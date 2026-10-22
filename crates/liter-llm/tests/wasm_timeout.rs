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

use std::time::Duration;

use futures_util::StreamExt;
use js_sys::{Date, Function, Promise, Reflect};
use liter_llm::{
    ChatCompletionRequest, ClientConfigBuilder, CreateFileRequest, DefaultClient, FileClient, LiterLlmError, LlmClient,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;

/// The body of a JavaScript function `(mode, millis)` that starts the mock server and returns a
/// promise for its handle. `mode` picks how the server answers:
///
/// - `silent`: read the request and never answer.
/// - `late`: answer after `millis`.
/// - `late_body`: send the headers and the start of the body, then the rest after `millis`.
/// - `busy_then_silent`: answer the first request with 503 and never answer the next ones.
/// - `stream`: send one server-sent event at once, then five more, `millis` apart.
/// - `stream_stall`: send one server-sent event at once, then go silent.
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
    if (mode === 'late_body') {
      res.writeHead(200, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) });
      res.write(body.slice(0, 10));
      setTimeout(() => res.end(body.slice(10)), millis);
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

const SLOW: u32 = 4000;
const TIMEOUT: Duration = Duration::from_millis(500);
/// A call that times out must end after about one timeout, not after the slow server answers.
const TIMEOUT_BOUND_MS: f64 = 2000.0;

#[wasm_bindgen_test]
async fn a_chat_call_that_outlives_the_timeout_fails_with_timeout_and_is_sent_once() {
    let server = MockServer::start("late", SLOW).await;
    let client = server.client(TIMEOUT, 3);

    let started = Date::now();
    let result = client.chat(chat_request()).await;
    let elapsed = Date::now() - started;
    server.close();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
    assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    assert_eq!(server.requests(), 1, "a timed-out request is not sent again");
}

#[wasm_bindgen_test]
async fn a_chat_reply_that_arrives_in_time_succeeds() {
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
}

#[wasm_bindgen_test]
async fn a_reply_body_that_outlives_the_timeout_fails_with_timeout() {
    let server = MockServer::start("late_body", SLOW).await;
    let client = server.client(TIMEOUT, 0);

    let started = Date::now();
    let result = client.chat(chat_request()).await;
    let elapsed = Date::now() - started;
    server.close();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
    assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
}

#[wasm_bindgen_test]
async fn a_timeout_after_a_server_error_ends_the_call() {
    let server = MockServer::start("busy_then_silent", 0).await;
    let client = server.client(TIMEOUT, 3);

    let result = client.chat(chat_request()).await;
    server.close();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
    assert_eq!(server.requests(), 2, "the 503 is retried once and the timeout is not");
}

#[wasm_bindgen_test]
async fn a_get_call_that_outlives_the_timeout_fails_with_timeout() {
    let server = MockServer::start("late", SLOW).await;
    let client = server.client(TIMEOUT, 0);

    let started = Date::now();
    let result = client.list_models().await;
    let elapsed = Date::now() - started;
    server.close();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
    assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
}

#[wasm_bindgen_test]
async fn a_get_reply_that_arrives_in_time_succeeds() {
    let server = MockServer::start("late", 100).await;
    let client = server.client(Duration::from_secs(3), 0);

    let result = client.list_models().await;
    server.close();

    assert!(result.expect("a reply inside the timeout is returned").data.is_empty());
}

#[wasm_bindgen_test]
async fn an_upload_that_outlives_the_timeout_fails_with_timeout() {
    let server = MockServer::start("late", SLOW).await;
    let client = server.client(TIMEOUT, 0);
    let request: CreateFileRequest = serde_json::from_value(serde_json::json!({
        "file": "YQ==",
        "purpose": "assistants",
    }))
    .expect("file request");

    let started = Date::now();
    let result = client.create_file(request).await;
    let elapsed = Date::now() - started;
    server.close();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {result:?}");
    assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
}

#[wasm_bindgen_test]
async fn an_upload_reply_that_arrives_in_time_succeeds() {
    let server = MockServer::start("late", 100).await;
    let client = server.client(Duration::from_secs(3), 0);
    let request: CreateFileRequest = serde_json::from_value(serde_json::json!({
        "file": "YQ==",
        "purpose": "assistants",
    }))
    .expect("file request");

    let result = client.create_file(request).await;
    server.close();

    assert_eq!(result.expect("a reply inside the timeout is returned").id, "f1");
}

#[wasm_bindgen_test]
async fn a_stream_with_no_reply_fails_with_timeout_and_is_sent_once() {
    let server = MockServer::start("silent", 0).await;
    let client = server.client(TIMEOUT, 3);

    let started = Date::now();
    let result = client.chat_stream(chat_request()).await;
    let elapsed = Date::now() - started;
    server.close();

    assert!(matches!(result, Err(LiterLlmError::Timeout)), "got {:?}", result.err());
    assert!(elapsed < TIMEOUT_BOUND_MS, "the call ended after {elapsed} ms");
    assert_eq!(server.requests(), 1, "a timed-out request is not sent again");
}

#[wasm_bindgen_test]
async fn a_stream_that_goes_idle_ends_with_timeout() {
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
}

#[wasm_bindgen_test]
async fn a_stream_longer_than_the_timeout_completes_while_chunks_keep_arriving() {
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
}
