//! Only Google AI and Vertex send `API_KEY_INVALID` in the `details` of a 400
//! reply. A 400 with that reason from any other provider stays a bad request.

#![cfg(feature = "native-http")]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;

use liter_llm::LiterLlmError;
use liter_llm::client::{ClientConfigBuilder, DefaultClient, LlmClient};
use liter_llm::types::{ChatCompletionRequest, Message, UserMessage};

const INVALID_KEY: &str = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"API_KEY_INVALID","domain":"googleapis.com"}]}}"#;

fn request(prefix: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: format!("{prefix}/some-model"),
        messages: vec![Message::User(UserMessage {
            content: "Hello".into(),
            ..Default::default()
        })],
        ..Default::default()
    }
}

/// A client whose server answers its one request with a 400 and `body`.
fn client(prefix: &str, body: &'static str) -> (DefaultClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind response server");
    let base_url = format!("http://{}", listener.local_addr().expect("server address"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read request headers");
            if line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                content_length = value.trim().parse().expect("request content length");
            }
        }
        reader
            .read_exact(&mut vec![0; content_length])
            .expect("read request body");
        write!(
            stream,
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write response");
    });
    let config = ClientConfigBuilder::new("test-key").base_url(base_url).build();
    let client = DefaultClient::new(config, Some(&format!("{prefix}/some-model"))).expect("create client");
    (client, handle)
}

#[tokio::test]
async fn invalid_key_reason_from_a_provider_other_than_google_stays_bad_request() {
    for prefix in ["openai", "groq", "mistral", "anthropic"] {
        let (client, server) = client(prefix, INVALID_KEY);
        let err = client.chat(request(prefix)).await.expect_err("the reply is a 400");
        assert!(
            matches!(&err, LiterLlmError::BadRequest { status: 400, .. }),
            "{prefix}: {err:?}"
        );
        server.join().expect("response server completed");
    }
}

#[tokio::test]
async fn invalid_key_reason_from_google_ai_and_vertex_is_authentication() {
    for prefix in ["gemini", "google_ai", "vertex_ai"] {
        let (client, server) = client(prefix, INVALID_KEY);
        let err = client.chat(request(prefix)).await.expect_err("the reply is a 400");
        assert!(
            matches!(&err, LiterLlmError::Authentication { status: 400, .. }),
            "{prefix}: {err:?}"
        );
        server.join().expect("response server completed");
    }
}

/// Revert line: pass `invalid_key_as_400: false` in the options of the
/// `post_stream_framed` call in `chat_stream` to make this test fail.
#[tokio::test]
async fn streaming_request_with_invalid_key_reply_from_google_ai_and_vertex_is_authentication() {
    for prefix in ["gemini", "google_ai", "vertex_ai"] {
        let (client, server) = client(prefix, INVALID_KEY);
        let err = match client.chat_stream(request(prefix)).await {
            Ok(_) => panic!("{prefix}: the reply is a 400"),
            Err(err) => err,
        };
        assert!(
            matches!(&err, LiterLlmError::Authentication { status: 400, .. }),
            "{prefix}: {err:?}"
        );
        server.join().expect("response server completed");
    }
}

#[tokio::test]
async fn streaming_request_with_invalid_key_reply_from_another_provider_stays_bad_request() {
    for prefix in ["openai", "groq"] {
        let (client, server) = client(prefix, INVALID_KEY);
        let err = match client.chat_stream(request(prefix)).await {
            Ok(_) => panic!("{prefix}: the reply is a 400"),
            Err(err) => err,
        };
        assert!(
            matches!(&err, LiterLlmError::BadRequest { status: 400, .. }),
            "{prefix}: {err:?}"
        );
        server.join().expect("response server completed");
    }
}
