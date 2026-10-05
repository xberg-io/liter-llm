//! `create_client_from_json` must apply the middleware and provider keys it parses.
//!
//! The returned binding client is a plain `DefaultClient`; these tests prove that cache, budget
//! and `providers` entries from the JSON config take effect on real requests against a local
//! mock server.

#![cfg(all(feature = "native-http", feature = "tower"))]
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use liter_llm::client::LlmClient;
use liter_llm::types::ChatCompletionRequest;
use liter_llm::{LiterLlmError, create_client_from_json, unregister_custom_provider};
use serde_json::json;
use serial_test::serial;

/// Minimal HTTP/1.1 server that records each request body and answers with a fixed chat completion.
struct MockServer {
    url: String,
    bodies: Arc<Mutex<Vec<serde_json::Value>>>,
    _handle: thread::JoinHandle<()>,
}

impl MockServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let url = format!("http://127.0.0.1:{}", listener.local_addr().expect("addr").port());
        let bodies: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let writer = Arc::clone(&bodies);

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));

                let mut request_line = String::new();
                if reader.read_line(&mut request_line).is_err() {
                    continue;
                }
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.trim().split_once(':')
                        && name.trim().eq_ignore_ascii_case("content-length")
                    {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0u8; content_length];
                let _ = reader.read_exact(&mut body);
                writer
                    .lock()
                    .expect("bodies lock")
                    .push(serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null));

                // ~keep Token counts are large enough that one gpt-4 call costs far more than the test budgets.
                let response = json!({
                    "id": "chatcmpl-test",
                    "object": "chat.completion",
                    "created": 1_700_000_000_u64,
                    "model": "gpt-4",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "Hello from mock"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1000, "completion_tokens": 1000, "total_tokens": 2000}
                })
                .to_string();
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.len(),
                        response
                    )
                    .as_bytes(),
                );
                let _ = stream.flush();
            }
        });

        Self {
            url,
            bodies,
            _handle: handle,
        }
    }

    fn request_bodies(&self) -> Vec<serde_json::Value> {
        self.bodies.lock().expect("bodies lock").clone()
    }
}

fn chat_request(model: &str) -> ChatCompletionRequest {
    serde_json::from_value(json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hi"}]
    }))
    .expect("minimal chat request should deserialize")
}

#[tokio::test]
async fn client_without_middleware_keys_sends_every_request() {
    let mock = MockServer::start();
    let client = create_client_from_json(&json!({"api_key": "k", "base_url": mock.url, "max_retries": 0}).to_string())
        .expect("client");

    client.chat(chat_request("gpt-4")).await.expect("first call");
    client.chat(chat_request("gpt-4")).await.expect("second call");

    assert_eq!(
        mock.request_bodies().len(),
        2,
        "no cache configured, so both calls reach the server"
    );
}

#[tokio::test]
async fn cache_config_serves_second_identical_chat_without_hitting_server() {
    let mock = MockServer::start();
    let client = create_client_from_json(
        &json!({
            "api_key": "k",
            "base_url": mock.url,
            "max_retries": 0,
            "cache": {"max_entries": 16, "ttl_seconds": 60}
        })
        .to_string(),
    )
    .expect("client");

    let first = client.chat(chat_request("gpt-4")).await.expect("first call");
    let second = client.chat(chat_request("gpt-4")).await.expect("second call");

    assert_eq!(first.id, second.id);
    assert_eq!(
        mock.request_bodies().len(),
        1,
        "second identical call must be a cache hit"
    );
}

#[tokio::test]
async fn hard_budget_rejects_second_call_once_limit_is_spent() {
    let mock = MockServer::start();
    let client = create_client_from_json(
        &json!({
            "api_key": "k",
            "base_url": mock.url,
            "max_retries": 0,
            "budget": {"global_limit": 0.0001, "enforcement": "hard"}
        })
        .to_string(),
    )
    .expect("client");

    client
        .chat(chat_request("gpt-4"))
        .await
        .expect("first call is within budget");
    let err = client
        .chat(chat_request("gpt-4"))
        .await
        .expect_err("second call must exceed the budget");

    assert!(matches!(err, LiterLlmError::BudgetExceeded { .. }), "got {err:?}");
    assert_eq!(
        mock.request_bodies().len(),
        1,
        "rejected call must not reach the server"
    );
}

#[tokio::test]
#[serial]
async fn providers_entry_routes_custom_prefix_and_strips_it() {
    let mock = MockServer::start();
    let client = create_client_from_json(
        &json!({
            "api_key": "k",
            "max_retries": 0,
            "providers": [{
                "name": "json-cfg-provider",
                "base_url": mock.url,
                "auth_header": "X-Json-Cfg-Key",
                "model_prefixes": ["json-cfg-provider/"]
            }]
        })
        .to_string(),
    )
    .expect("client");

    let result = client.chat(chat_request("json-cfg-provider/my-model")).await;
    let _ = unregister_custom_provider("json-cfg-provider");
    result.expect("request routed to the registered provider");

    let bodies = mock.request_bodies();
    assert_eq!(bodies.len(), 1, "request must reach the provider's base_url");
    assert_eq!(bodies[0]["model"], "my-model");
}
