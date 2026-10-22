//! The client timeout applies on wasm32, as it does on native targets.
//!
//! Each test starts a mock HTTP server inside Node.js, points a `DefaultClient` at it and checks
//! how the call ends. The native counterparts are `http_timeout_retry.rs` and `stream_idle_timeout.rs`.
//!
//! Run with `wasm-bindgen-test-runner` on Node.js 22.3 or later:
//!
//! ```text
//! CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
//!   cargo test --target wasm32-unknown-unknown -p liter-llm \
//!   --no-default-features --features wasm-http --test wasm_timeout
//! ```

#![cfg(all(target_arch = "wasm32", feature = "wasm-http"))]

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use js_sys::{Date, Function, Promise, Reflect};
use liter_llm::auth::StaticTokenProvider;
use liter_llm::{
    ChatCompletionRequest, ClientConfigBuilder, CreateFileRequest, CreateSpeechRequest, DefaultClient, FileClient,
    LiterLlmError, LlmClient,
};
use secrecy::SecretString;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;

/// The body of a JavaScript function `(mode, millis)` that starts the mock server and returns a
/// promise for its handle. `mode` picks how the server answers:
///
/// - `silent`: read the request and never answer.
/// - `late`: answer after `millis`.
/// - `late_body`: send the headers and the start of the body, then the rest after `millis`.
/// - `late_error_body`: send a 503 with the start of its body, then the rest after `millis`.
/// - `busy_then_silent`: answer the first request with 503 and never answer the next ones.
/// - `stream`: send one server-sent event at once, then five more, `millis` apart.
/// - `stream_stall`: send one server-sent event at once, then go silent.
/// - `headers_only`: send the headers of an event stream reply and no body.
const MOCK_SERVER: &str = r#"
const http = process.getBuiltinModule('node:http');
const chat = JSON.stringify({
  id: 'c', object: 'chat.completion', created: 1, model: 'm',
  choices: [{ index: 0, finish_reason: 'stop', message: { role: 'assistant', content: 'in time' } }],
});
const models = JSON.stringify({ object: 'list', data: [] });
const file = JSON.stringify({
  id: 'f1', object: 'file', bytes: 1, created_at: 1, filename: 'a', purpose: 'assistants',
});
const event = (text, finish) => 'data: ' + JSON.stringify({
  id: 'c1', object: 'chat.completion.chunk', created: 1, model: 'm',
  choices: [{ index: 0, delta: finish ? {} : { content: text }, finish_reason: finish ? 'stop' : null }],
}) + '\n\n';
const handle = { requests: 0, port: 0, close() {} };
const server = http.createServer((req, res) => {
  const attempt = handle.requests++;
  req.resume();
  req.on('end', () => {
    const body = req.url.endsWith('/models') ? models : req.url.endsWith('/files') ? file : chat;
    const json = (status, payload) => {
      res.writeHead(status, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(payload) });
      res.end(payload);
    };
    if (mode === 'silent') return;
    if (mode === 'late') { setTimeout(() => json(200, body), millis); return; }
    if (mode === 'busy_then_silent') { if (attempt === 0) json(503, '{"error":{"message":"busy"}}'); return; }
    if (mode === 'late_error_body') {
      res.writeHead(503, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) });
      res.write(body.slice(0, 10));
      setTimeout(() => res.end(body.slice(10)), millis);
      return;
    }
    if (mode === 'late_body') {
      res.writeHead(200, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) });
      res.write(body.slice(0, 10));
      setTimeout(() => res.end(body.slice(10)), millis);
      return;
    }
    if (mode === 'headers_only') {
      res.writeHead(200, { 'content-type': 'application/vnd.amazon.eventstream' });
      res.flushHeaders();
      return;
    }
    res.writeHead(200, { 'content-type': 'text/event-stream' });
    res.write(event('w0 '));
    if (mode === 'stream_stall') return;
    let next = 1;
    const tick = () => {
      if (next <= 5) {
        res.write(event('w' + next + ' '));
        next += 1;
        setTimeout(tick, millis);
      } else {
        res.write(event('', true));
        res.end('data: [DONE]\n\n');
      }
    };
    setTimeout(tick, millis);
  });
});
return new Promise((resolve) => server.listen(0, () => {
  handle.port = server.address().port;
  handle.close = () => { server.closeAllConnections(); server.close(); };
  resolve(handle);
}));
"#;

struct MockServer {
    handle: JsValue,
    base_url: String,
}

impl MockServer {
    async fn start(mode: &str, millis: u32) -> Self {
        let start = Function::new_with_args("mode, millis", MOCK_SERVER);
        let promise: Promise = start
            .call2(&JsValue::NULL, &JsValue::from_str(mode), &JsValue::from(millis))
            .expect("start the mock server")
            .unchecked_into();
        let handle = JsFuture::from(promise).await.expect("mock server listens");
        let port = Reflect::get(&handle, &JsValue::from_str("port"))
            .expect("read the port")
            .as_f64()
            .expect("port is a number");
        // ~keep `localhost` is a name, not a literal address, so the default outbound policy allows it.
        Self {
            handle,
            base_url: format!("http://localhost:{port}/v1"),
        }
    }

    fn requests(&self) -> u32 {
        Reflect::get(&self.handle, &JsValue::from_str("requests"))
            .expect("read the request count")
            .as_f64()
            .expect("count is a number") as u32
    }

    fn client(&self, timeout: Duration, max_retries: u32) -> DefaultClient {
        let config = ClientConfigBuilder::new("test-key")
            .base_url(self.base_url.clone())
            .timeout(timeout)
            .max_retries(max_retries)
            .build();
        DefaultClient::new(config, Some("openai")).expect("create client")
    }

    fn close(&self) {
        let close: Function = Reflect::get(&self.handle, &JsValue::from_str("close"))
            .expect("read close")
            .unchecked_into();
        close.call0(&self.handle).expect("close the mock server");
    }
}

fn chat_request() -> ChatCompletionRequest {
    serde_json::from_value(serde_json::json!({
        "model": "m",
        "messages": [{"role": "user", "content": "hi"}],
    }))
    .expect("chat request")
}

fn file_request() -> CreateFileRequest {
    serde_json::from_value(serde_json::json!({
        "file": "YQ==",
        "purpose": "assistants",
    }))
    .expect("file request")
}

/// The most a test may take. A test that waits longer than this has found a call that never ends,
/// and fails instead of hanging until the harness gives up. It is above the slowest healthy test.
const TEST_LIMIT: Duration = Duration::from_secs(10);

async fn within<F: std::future::Future<Output = ()>>(test: F) {
    let test = std::pin::pin!(test);
    let limit = std::pin::pin!(gloo_timers::future::sleep(TEST_LIMIT));
    if let futures_util::future::Either::Right(_) = futures_util::future::select(test, limit).await {
        panic!("the test did not end within {TEST_LIMIT:?}: a call that must time out never ended");
    }
}

const SLOW: u32 = 4000;
const TIMEOUT: Duration = Duration::from_millis(500);
/// A call that times out must end after about one timeout, not after the slow server answers.
const TIMEOUT_BOUND_MS: f64 = 2000.0;

#[wasm_bindgen_test]
async fn a_chat_call_that_outlives_the_timeout_fails_with_timeout_and_is_sent_once() {
    within(async {
        let server = MockServer::start("late", SLOW).await;
        let client = server.client(TIMEOUT, 3);

        let started = Date::now();
        let result = client.chat(chat_request()).await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
        assert_eq!(server.requests(), 1, "a timed-out request is not sent again");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_chat_reply_that_arrives_in_time_succeeds() {
    within(async {
        let server = MockServer::start("late", 100).await;
        let client = server.client(Duration::from_secs(3), 0);

        let result = client.chat(chat_request()).await;
        server.close();

        let reply = result.expect("a reply inside the timeout is returned");
        let reply = serde_json::to_value(&reply).expect("serialize reply");
        assert_eq!(
            reply.pointer("/choices/0/message/content"),
            Some(&serde_json::json!("in time")),
            "the reply body is read in full"
        );
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_reply_body_that_outlives_the_timeout_fails_with_timeout() {
    within(async {
        let server = MockServer::start("late_body", SLOW).await;
        let client = server.client(TIMEOUT, 0);

        let started = Date::now();
        let result = client.chat(chat_request()).await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_timeout_after_a_server_error_ends_the_call() {
    within(async {
        let server = MockServer::start("busy_then_silent", 0).await;
        let client = server.client(TIMEOUT, 3);

        let result = client.chat(chat_request()).await;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert_eq!(server.requests(), 2, "the 503 is retried once and the timeout is not");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_get_call_that_outlives_the_timeout_fails_with_timeout() {
    within(async {
        let server = MockServer::start("late", SLOW).await;
        let client = server.client(TIMEOUT, 0);

        let started = Date::now();
        let result = client.list_models().await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_get_reply_that_arrives_in_time_succeeds() {
    within(async {
        let server = MockServer::start("late", 100).await;
        let client = server.client(Duration::from_secs(3), 0);

        let result = client.list_models().await;
        server.close();

        assert!(result.expect("a reply inside the timeout is returned").data.is_empty());
    })
    .await;
}

#[wasm_bindgen_test]
async fn an_upload_that_outlives_the_timeout_fails_with_timeout() {
    within(async {
        let server = MockServer::start("late", SLOW).await;
        let client = server.client(TIMEOUT, 0);
        let request = file_request();

        let started = Date::now();
        let result = client.create_file(request).await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    })
    .await;
}

#[wasm_bindgen_test]
async fn an_error_body_that_outlives_the_timeout_fails_with_timeout() {
    within(async {
        let server = MockServer::start("late_error_body", SLOW).await;
        let client = server.client(TIMEOUT, 0);

        let started = Date::now();
        let result = client.chat(chat_request()).await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    })
    .await;
}

#[wasm_bindgen_test]
async fn an_upload_error_body_that_outlives_the_timeout_fails_with_timeout() {
    within(async {
        let server = MockServer::start("late_error_body", SLOW).await;
        let client = server.client(TIMEOUT, 0);

        let started = Date::now();
        let result = client.create_file(file_request()).await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_delete_call_that_outlives_the_timeout_fails_with_timeout() {
    within(async {
        let server = MockServer::start("late", SLOW).await;
        let client = server.client(TIMEOUT, 0);

        let started = Date::now();
        let result = client.delete_file("f1").await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_binary_download_that_outlives_the_timeout_fails_with_timeout() {
    within(async {
        let server = MockServer::start("late", SLOW).await;
        let client = server.client(TIMEOUT, 0);

        let started = Date::now();
        let result = client.file_content("f1").await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_binary_download_that_arrives_in_time_succeeds() {
    within(async {
        let server = MockServer::start("late", 100).await;
        let client = server.client(Duration::from_secs(3), 0);

        let result = client.file_content("f1").await;
        server.close();

        assert!(!result.expect("a reply inside the timeout is returned").is_empty());
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_speech_call_that_outlives_the_timeout_fails_with_timeout() {
    within(async {
        let server = MockServer::start("late", SLOW).await;
        let client = server.client(TIMEOUT, 0);
        let request: CreateSpeechRequest = serde_json::from_value(serde_json::json!({
            "model": "m",
            "input": "hi",
            "voice": "alloy",
        }))
        .expect("speech request");

        let started = Date::now();
        let result = client.speech(request).await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    })
    .await;
}

#[wasm_bindgen_test]
async fn an_upload_reply_that_arrives_in_time_succeeds() {
    within(async {
        let server = MockServer::start("late", 100).await;
        let client = server.client(Duration::from_secs(3), 0);
        let request = file_request();

        let result = client.create_file(request).await;
        server.close();

        assert_eq!(result.expect("a reply inside the timeout is returned").id, "f1");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_stream_with_no_reply_fails_with_timeout_and_is_sent_once() {
    within(async {
        let server = MockServer::start("silent", 0).await;
        let client = server.client(TIMEOUT, 3);

        let started = Date::now();
        let result = client.chat_stream(chat_request()).await;
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {:?}", result.err());
        assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
        assert_eq!(server.requests(), 1, "a timed-out request is not sent again");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_stream_that_goes_idle_ends_with_timeout() {
    within(async {
        let server = MockServer::start("stream_stall", 0).await;
        let client = server.client(TIMEOUT, 0);

        let started = Date::now();
        let mut stream = client.chat_stream(chat_request()).await.expect("stream opens");
        let first = stream
            .next()
            .await
            .expect("the first chunk arrives")
            .expect("a healthy chunk");
        assert_eq!(first.choices[0].delta.content.as_deref(), Some("w0 "));
        let idle = stream.next().await.expect("the idle stream reports an error");
        let elapsed = Date::now() - started;
        let after = stream.next().await;
        server.close();

        assert!(matches!(idle, Err(LiterLlmError::Timeout)), "got {idle:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the stream ended after {elapsed} ms");
        assert!(after.is_none(), "the stream is finished after the timeout");
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_stream_longer_than_the_timeout_completes_while_chunks_keep_arriving() {
    within(async {
        // Five chunks 300 ms apart take about 1.5 s, against a 700 ms timeout.
        let server = MockServer::start("stream", 300).await;
        let client = server.client(Duration::from_millis(700), 0);

        let started = Date::now();
        let mut stream = client.chat_stream(chat_request()).await.expect("stream opens");
        let mut text = String::new();
        while let Some(item) = stream.next().await {
            let chunk = item.expect("a healthy stream does not fail");
            if let Some(part) = chunk.choices.first().and_then(|choice| choice.delta.content.as_deref()) {
                text.push_str(part);
            }
        }
        let elapsed = Date::now() - started;
        server.close();

        assert_eq!(text, "w0 w1 w2 w3 w4 w5 ");
        assert!(
            elapsed > 1200.0,
            "the stream must outlast the timeout, took {elapsed} ms"
        );
    })
    .await;
}

#[wasm_bindgen_test]
async fn a_bedrock_stream_that_goes_idle_ends_with_timeout() {
    within(async {
        const MODEL: &str = "bedrock/anthropic.claude-3-haiku-20240307-v1:0";
        let server = MockServer::start("headers_only", 0).await;
        // ~keep wasm32 has no environment and no SigV4 signer, so a Bedrock client authenticates with a
        // ~keep credential provider that returns a Bedrock API key.
        let mut config = ClientConfigBuilder::new("")
            .load_env(false)
            .base_url(server.base_url.clone())
            .timeout(TIMEOUT)
            .max_retries(0)
            .credential_provider(Arc::new(StaticTokenProvider::new(SecretString::from("bedrock-key"))))
            .build();
        config.bedrock_region = Some("us-east-1".into());
        let client = DefaultClient::new(config, Some(MODEL)).expect("create client");
        let mut request = chat_request();
        request.model = MODEL.to_owned();

        let started = Date::now();
        let mut stream = client.chat_stream(request).await.expect("stream opens");
        let idle = stream.next().await.expect("the idle stream reports an error");
        let elapsed = Date::now() - started;
        server.close();

        assert!(matches!(idle, Err(LiterLlmError::Timeout)), "got {idle:?}");
        assert!(elapsed < TIMEOUT_BOUND_MS, "the stream ended after {elapsed} ms");
    })
    .await;
}
