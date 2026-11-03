//! Per-client blocked networks, through the real client and HTTP stack.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use liter_llm::client::{ClientConfigBuilder, DefaultClient, LlmClient};
use liter_llm::{ChatCompletionRequest, ClientOptions, LiterLlmError, create_client_with_options};

fn chat_request() -> ChatCompletionRequest {
    serde_json::from_value(serde_json::json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Hi"}]
    }))
    .expect("chat request")
}

/// A loopback chat server that answers every connection and counts them.
struct ChatServer {
    port: u16,
    connections: Arc<AtomicUsize>,
}

fn chat_server() -> ChatServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind chat server");
    let port = listener.local_addr().expect("chat server address").port();
    let connections = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&connections);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            counter.fetch_add(1, Ordering::SeqCst);
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let body = serde_json::json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "created": 1_700_000_000_u64,
                "model": "test-model",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }]
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    ChatServer { port, connections }
}

fn client(base_url: String, blocked: &[&str]) -> DefaultClient {
    let config = ClientConfigBuilder::new("test-key")
        .base_url(base_url)
        .max_retries(0)
        .blocked_networks(blocked.iter().copied())
        .build();
    DefaultClient::new(config, None).expect("client creation")
}

fn assert_forbidden<T: std::fmt::Debug>(result: liter_llm::Result<T>) {
    assert!(
        matches!(result, Err(LiterLlmError::OutboundForbidden { .. })),
        "expected OutboundForbidden, got {result:?}"
    );
}

#[tokio::test]
async fn ip_literal_host_in_a_blocked_network_is_refused_before_send() {
    let server = chat_server();

    let result = client(format!("http://127.0.0.1:{}", server.port), &["127.0.0.0/8"])
        .chat(chat_request())
        .await;

    assert_forbidden(result);
    assert_eq!(server.connections.load(Ordering::SeqCst), 0, "no connection is opened");
}

#[tokio::test]
async fn hostname_resolving_into_a_blocked_network_is_refused_at_connect() {
    let server = chat_server();

    let result = client(format!("http://localhost:{}", server.port), &["127.0.0.0/8", "::1"])
        .chat(chat_request())
        .await;

    assert_forbidden(result);
    assert_eq!(server.connections.load(Ordering::SeqCst), 0, "no connection is opened");
}

#[tokio::test]
async fn addresses_outside_the_blocked_networks_still_connect() {
    let server = chat_server();

    let literal = client(format!("http://127.0.0.1:{}", server.port), &["10.0.0.0/8"])
        .chat(chat_request())
        .await;
    let hostname = client(format!("http://localhost:{}", server.port), &["10.0.0.0/8", "fd00::/8"])
        .chat(chat_request())
        .await;

    assert!(literal.is_ok(), "IP literal: {literal:?}");
    assert!(hostname.is_ok(), "hostname: {hostname:?}");
}

#[tokio::test]
async fn client_options_carry_blocked_networks() {
    let server = chat_server();

    let client = create_client_with_options(ClientOptions {
        api_key: "test-key".into(),
        base_url: Some(format!("http://127.0.0.1:{}", server.port)),
        max_retries: Some(0),
        blocked_networks: vec!["127.0.0.1".into()],
        ..Default::default()
    })
    .expect("create client");

    assert_forbidden(client.chat(chat_request()).await);
    assert_eq!(server.connections.load(Ordering::SeqCst), 0, "no connection is opened");
}

#[test]
fn client_options_json_without_blocked_networks_still_parses() {
    let options: ClientOptions = serde_json::from_str(r#"{"api_key": "k"}"#).expect("options JSON");
    assert!(options.blocked_networks.is_empty());
}

#[test]
fn an_invalid_entry_is_an_error_when_the_client_is_created() {
    for build in [
        DefaultClient::new(
            ClientConfigBuilder::new("test-key")
                .blocked_networks(["10.0.0.0/40"])
                .build(),
            None,
        )
        .map(|_| ()),
        create_client_with_options(ClientOptions {
            api_key: "test-key".into(),
            blocked_networks: vec!["metadata.internal".into()],
            ..Default::default()
        })
        .map(|_| ()),
    ] {
        match build {
            Err(LiterLlmError::BadRequest { message, status: 400 }) => {
                assert!(message.contains("blocked_networks"), "message = {message}");
            }
            other => panic!("an invalid entry must be a BadRequest error, got {other:?}"),
        }
    }
}
