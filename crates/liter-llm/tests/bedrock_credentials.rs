#![cfg(feature = "bedrock")]

mod common;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use futures_util::StreamExt;
use liter_llm::auth::{Credential, CredentialProvider};
use liter_llm::client::{BoxFuture, ClientConfigBuilder, DefaultClient, LlmClient, LlmClientRaw};
use liter_llm::error::{LiterLlmError, Result};
use liter_llm::types::{ChatCompletionRequest, Message, UserMessage};
use sha2::{Digest, Sha256};

const MODEL: &str = "bedrock/anthropic.claude-3-haiku-20240307-v1:0";
const RESPONSE: &str = r#"{"output":{"message":{"role":"assistant","content":[{"text":"Hello"}]}},"stopReason":"end_turn","usage":{"inputTokens":1,"outputTokens":1,"totalTokens":2}}"#;

struct RotatingCredentials(AtomicUsize);

impl CredentialProvider for RotatingCredentials {
    fn resolve(&self) -> BoxFuture<'_, Result<Credential>> {
        let generation = self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            Ok(Credential::AwsCredentials {
                access_key_id: format!("resolved-key-{generation}").into(),
                secret_access_key: format!("resolved-secret-{generation}").into(),
                session_token: (generation == 0).then(|| "resolved-token".into()),
            })
        })
    }
}

struct FailedCredentials;

impl CredentialProvider for FailedCredentials {
    fn resolve(&self) -> BoxFuture<'_, Result<Credential>> {
        Box::pin(async {
            Err(LiterLlmError::Authentication {
                message: "credential refresh failed".into(),
                status: 401,
            })
        })
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Chat,
    RawChat,
    Stream,
    RawStream,
}

async fn send(client: &DefaultClient, mode: Mode) -> Result<()> {
    let request = ChatCompletionRequest {
        model: MODEL.into(),
        messages: vec![Message::User(UserMessage {
            content: "Hello".into(),
            ..Default::default()
        })],
        ..Default::default()
    };
    let mut stream = match mode {
        Mode::Chat => return client.chat(request).await.map(|_| ()),
        Mode::RawChat => return client.chat_raw(request).await.map(|_| ()),
        Mode::Stream => client.chat_stream(request).await?,
        Mode::RawStream => client.chat_stream_raw(request).await?.stream,
    };
    // ~keep The fixture sends no events, so the typed stream and the raw stream both report truncation
    // (#295, #308): Bedrock's event stream must end with a final event.
    let first = stream.next().await;
    assert!(
        matches!(first, Some(Err(LiterLlmError::Streaming { .. }))),
        "fixture sends an empty event stream, got {first:?}"
    );
    Ok(())
}

fn client(base_url: String, credentials: Arc<dyn CredentialProvider>) -> DefaultClient {
    let mut config = ClientConfigBuilder::new("")
        .load_env(false)
        .base_url(base_url)
        .credential_provider(credentials)
        .build();
    config.timeout = Duration::from_secs(2);
    config.max_retries = 0;
    config.bedrock_region = Some("us-east-1".into());
    config.bedrock_access_key_id = Some("fallback-key".into());
    config.bedrock_secret_access_key = Some("fallback-secret".into());
    config.bedrock_session_token = Some("fallback-token".into());
    DefaultClient::new(config, Some(MODEL)).expect("construct Bedrock fixture client")
}

struct CapturedRequest {
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn capture(listener: TcpListener, mode: Mode) -> Vec<CapturedRequest> {
    (0..2)
        .map(|_| {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream.set_read_timeout(Some(Duration::from_secs(5))).expect("read timeout");
            let mut reader = BufReader::new(stream.try_clone().expect("clone request stream"));
            let mut line = String::new();
            reader.read_line(&mut line).expect("read request line");
            let path = line.split_whitespace().nth(1).expect("request path").to_owned();
            let mut headers = HashMap::new();
            loop {
                line.clear();
                reader.read_line(&mut line).expect("read headers");
                if line.trim().is_empty() {
                    break;
                }
                let (name, value) = line.split_once(':').expect("header pair");
                headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
            }
            let length: usize = headers["content-length"].parse().expect("body length");
            let mut body = vec![0; length];
            reader.read_exact(&mut body).expect("request body");
            let (content_type, response) = match mode {
                Mode::Chat | Mode::RawChat => ("application/json", RESPONSE),
                Mode::Stream | Mode::RawStream => ("application/vnd.amazon.eventstream", ""),
            };
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len())
                .expect("write fixture response");
            CapturedRequest { path, headers, body }
        })
        .collect()
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    const BLOCK_SIZE: usize = 64;
    let mut normalized = [0; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        normalized[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let inner_key = normalized.map(|byte| byte ^ 0x36);
    let outer_key = normalized.map(|byte| byte ^ 0x5c);
    let inner = Sha256::new().chain_update(inner_key).chain_update(message).finalize();
    Sha256::new()
        .chain_update(outer_key)
        .chain_update(inner)
        .finalize()
        .to_vec()
}

fn assert_signature(request: &CapturedRequest, generation: usize, secret: &str) {
    let authorization = &request.headers["authorization"];
    let (_, fields) = authorization.split_once(' ').expect("signature algorithm");
    let fields: HashMap<_, _> = fields
        .split(", ")
        .map(|part| part.split_once('=').expect("signing field"))
        .collect();
    let (key, scope) = fields["Credential"].split_once('/').expect("credential scope");
    assert_eq!(key, format!("resolved-key-{generation}"));
    assert_eq!(
        request.headers.get("x-amz-security-token").map(String::as_str),
        (generation == 0).then_some("resolved-token")
    );
    let signed_headers = fields["SignedHeaders"];
    let canonical_headers: String = signed_headers
        .split(';')
        .map(|name| format!("{name}:{}\n", request.headers[name]))
        .collect();
    // ~keep SigV4 defaults double-encode the already percent-encoded model path.
    let canonical = format!(
        "POST\n{}\n\n{canonical_headers}\n{signed_headers}\n{}",
        request.path.replace('%', "%25"),
        hex::encode(Sha256::digest(&request.body))
    );
    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{scope}\n{}",
        request.headers["x-amz-date"],
        hex::encode(Sha256::digest(canonical))
    );
    let date = scope.split('/').next().expect("scope date");
    let date_key = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let region_key = hmac_sha256(&date_key, b"us-east-1");
    let service_key = hmac_sha256(&region_key, b"bedrock");
    let signing_key = hmac_sha256(&service_key, b"aws4_request");
    assert_eq!(
        fields["Signature"],
        hex::encode(hmac_sha256(&signing_key, to_sign.as_bytes()))
    );
}

async fn assert_rotating_credentials(mode: Mode) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let credentials = Arc::new(RotatingCredentials(AtomicUsize::new(0)));
    let client = client(
        format!("http://{}", listener.local_addr().expect("fixture address")),
        credentials.clone(),
    );
    let server = thread::spawn(move || capture(listener, mode));
    for _ in 0..2 {
        send(&client, mode).await.expect("Bedrock request succeeds");
    }
    let requests = server.join().expect("fixture completes");
    assert_eq!(requests.len(), 2);
    assert_eq!(credentials.0.load(Ordering::SeqCst), 2);
    for (generation, request) in requests.iter().enumerate() {
        assert_signature(request, generation, &format!("resolved-secret-{generation}"));
    }
}

#[test]
fn should_match_independent_hmac_reference_vector() {
    assert_eq!(
        hex::encode(hmac_sha256(&[0x0b; 20], b"Hi There")),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

#[tokio::test]
async fn should_sign_standard_chat_with_rotating_resolved_credentials() {
    assert_rotating_credentials(Mode::Chat).await;
}

#[tokio::test]
async fn should_sign_raw_chat_with_rotating_resolved_credentials() {
    assert_rotating_credentials(Mode::RawChat).await;
}

#[tokio::test]
async fn should_sign_streaming_chat_with_rotating_resolved_credentials() {
    assert_rotating_credentials(Mode::Stream).await;
}

#[tokio::test]
async fn should_sign_raw_streaming_chat_with_rotating_resolved_credentials() {
    assert_rotating_credentials(Mode::RawStream).await;
}

#[tokio::test]
async fn should_prevent_http_requests_when_credential_resolution_fails() {
    for mode in [Mode::Chat, Mode::RawChat, Mode::Stream, Mode::RawStream] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        listener.set_nonblocking(true).expect("nonblocking fixture");
        let client = client(
            format!("http://{}", listener.local_addr().expect("fixture address")),
            Arc::new(FailedCredentials),
        );
        let error = send(&client, mode).await.expect_err("resolver failure propagates");
        assert!(
            matches!(error, LiterLlmError::Authentication { message, status: 401 } if message == "credential refresh failed")
        );
        assert_eq!(
            listener.accept().expect_err("no HTTP request is sent").kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[tokio::test]
async fn should_allow_resolved_credentials_without_static_aws_credentials() {
    const CHILD_MARKER: &str = "LITER_LLM_BEDROCK_RESOLVER_ONLY_TEST";
    if std::env::var_os(CHILD_MARKER).is_none() {
        let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "should_allow_resolved_credentials_without_static_aws_credentials",
            ])
            .env(CHILD_MARKER, "1")
            .env_remove("AWS_ACCESS_KEY_ID")
            .env_remove("AWS_SECRET_ACCESS_KEY")
            .env_remove("AWS_BEARER_TOKEN_BEDROCK")
            .env("AWS_SESSION_TOKEN", "unrelated-session-token")
            .status()
            .expect("run isolated resolver-only regression");
        assert!(status.success(), "isolated resolver-only regression passes");
        return;
    }
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let credentials = Arc::new(RotatingCredentials(AtomicUsize::new(0)));
    let mut config = ClientConfigBuilder::new("")
        .load_env(false)
        .base_url(format!("http://{}", listener.local_addr().expect("fixture address")))
        .credential_provider(credentials.clone())
        .build();
    config.bedrock_region = Some("us-east-1".into());
    let client = DefaultClient::new(config, Some(MODEL)).expect("resolver supplies all required AWS credentials");
    let server = thread::spawn(move || capture(listener, Mode::Chat));
    for _ in 0..2 {
        send(&client, Mode::Chat).await.expect("resolved-only request succeeds");
    }
    let requests = server.join().expect("fixture completes");
    assert_eq!(requests.len(), 2);
    assert_eq!(credentials.0.load(Ordering::SeqCst), 2);
    for (generation, request) in requests.iter().enumerate() {
        assert_signature(request, generation, &format!("resolved-secret-{generation}"));
    }
}

#[tokio::test]
async fn should_use_resolved_bearer_instead_of_static_bedrock_auth() {
    for mode in [Mode::Chat, Mode::RawChat, Mode::Stream, Mode::RawStream] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        let client = client(
            format!("http://{}", listener.local_addr().expect("fixture address")),
            Arc::new(liter_llm::auth::StaticTokenProvider::new("resolved-bearer".into())),
        );
        let server = thread::spawn(move || capture(listener, mode));
        for _ in 0..2 {
            send(&client, mode).await.expect("resolved bearer request succeeds");
        }
        let requests = server.join().expect("fixture completes");
        assert_eq!(requests.len(), 2);
        for request in requests {
            assert_eq!(request.headers["authorization"], "Bearer resolved-bearer");
            assert_eq!(request.headers.get("x-amz-security-token"), None);
            assert_eq!(request.headers.get("x-amz-date"), None);
        }
    }
}

struct FixedCredentials(Credential);

impl CredentialProvider for FixedCredentials {
    fn resolve(&self) -> BoxFuture<'_, Result<Credential>> {
        Box::pin(async { Ok(self.0.clone()) })
    }
}

async fn assert_invalid_credentials_prevent_http(credential: Credential) {
    for mode in [Mode::Chat, Mode::RawChat, Mode::Stream, Mode::RawStream] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        listener.set_nonblocking(true).expect("nonblocking fixture");
        let client = client(
            format!("http://{}", listener.local_addr().expect("fixture address")),
            Arc::new(FixedCredentials(credential.clone())),
        );
        let result = send(&client, mode).await;
        assert_eq!(
            listener
                .accept()
                .expect_err("invalid credentials send no HTTP request")
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert!(matches!(result, Err(LiterLlmError::Authentication { status: 401, .. })));
    }
}

#[tokio::test]
async fn should_reject_empty_resolved_bearer_before_http() {
    assert_invalid_credentials_prevent_http(Credential::BearerToken("".into())).await;
}

#[tokio::test]
async fn should_reject_empty_resolved_access_key_before_http() {
    assert_invalid_credentials_prevent_http(Credential::AwsCredentials {
        access_key_id: "".into(),
        secret_access_key: "resolved-secret".into(),
        session_token: None,
    })
    .await;
}

#[tokio::test]
async fn should_reject_empty_resolved_secret_key_before_http() {
    assert_invalid_credentials_prevent_http(Credential::AwsCredentials {
        access_key_id: "resolved-key".into(),
        secret_access_key: "".into(),
        session_token: None,
    })
    .await;
}

async fn assert_whitespace_credentials_prevent_all_http_routes(credential: Credential) {
    let mut failures = Vec::new();
    let mut routes = 0;
    for mode in [Mode::Chat, Mode::RawChat, Mode::Stream, Mode::RawStream] {
        routes += 1;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind whitespace guard fixture");
        listener.set_nonblocking(true).expect("nonblocking fixture");
        let client = client(
            format!("http://{}", listener.local_addr().expect("fixture address")),
            Arc::new(FixedCredentials(credential.clone())),
        );
        let result = send(&client, mode).await;
        let sent_http = match listener.accept() {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => false,
            Err(error) => panic!("failed to inspect HTTP receipt: {error}"),
        };
        let authentication_error = matches!(result, Err(LiterLlmError::Authentication { status: 401, .. }));
        if sent_http || !authentication_error {
            failures.push((routes, sent_http, authentication_error));
        }
    }
    assert_eq!(routes, 4);
    assert!(
        failures.is_empty(),
        "route, sent HTTP, authentication error: {failures:?}"
    );
}

#[tokio::test]
async fn should_reject_whitespace_only_resolved_bearer_before_all_http_routes() {
    assert_whitespace_credentials_prevent_all_http_routes(Credential::BearerToken("   ".into())).await;
}

#[tokio::test]
async fn should_reject_whitespace_only_resolved_access_key_before_all_http_routes() {
    assert_whitespace_credentials_prevent_all_http_routes(Credential::AwsCredentials {
        access_key_id: "   ".into(),
        secret_access_key: "resolved-secret".into(),
        session_token: None,
    })
    .await;
}

#[tokio::test]
async fn should_reject_whitespace_only_resolved_secret_key_before_all_http_routes() {
    assert_whitespace_credentials_prevent_all_http_routes(Credential::AwsCredentials {
        access_key_id: "resolved-key".into(),
        secret_access_key: "   ".into(),
        session_token: None,
    })
    .await;
}

#[tokio::test]
async fn should_preserve_nonblank_resolved_secret_bytes_in_all_signatures() {
    const SECRET: &str = " \tresolved-secret-0 \t";
    for mode in [Mode::Chat, Mode::RawChat, Mode::Stream, Mode::RawStream] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind exact-secret fixture");
        let client = client(
            format!("http://{}", listener.local_addr().expect("fixture address")),
            Arc::new(FixedCredentials(Credential::AwsCredentials {
                access_key_id: "resolved-key-0".into(),
                secret_access_key: SECRET.into(),
                session_token: Some("resolved-token".into()),
            })),
        );
        let server = thread::spawn(move || capture(listener, mode));
        for _ in 0..2 {
            send(&client, mode)
                .await
                .expect("nonblank secret signs without trimming");
        }
        let requests = server.join().expect("exact-secret fixture completes");
        assert_eq!(requests.len(), 2);
        for request in requests {
            assert_signature(&request, 0, SECRET);
        }
    }
}
