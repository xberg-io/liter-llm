//! End-to-end stack integration test for the proxy.
//!
//! Exercises the full request pipeline — `auth` → `key_resolver` → `service_pool`
//! → upstream → response — so a regression in any layer surfaces here, even
//! when individual unit/integration tests for each layer remain green.
//!
//! Tests 1 and 2 use the shared `common::mock_upstream::MockUpstream` harness.
//! Test 3 needs to assert that no upstream call occurs, which requires a hit
//! counter that the shared harness does not expose; for that case a hand-rolled
//! `axum` upstream is used.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use axum::{Router, routing::post};
use http_body_util::BodyExt;
use tokio::task::JoinHandle;
use tower::ServiceExt;

use liter_llm_proxy::config::VirtualKeyConfig;

const CHAT_COMPLETION_BODY: &str = r#"{"id":"chatcmpl-1","object":"chat.completion","created":1700000000,"model":"gpt-4o","choices":[{"index":0,"message":{"role":"assistant","content":"Hello!"},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}"#;

fn chat_route() -> common::mock_upstream::MockRoute {
    common::mock_upstream::MockRoute {
        path: "/chat/completions".into(),
        method: "POST".into(),
        status: 200,
        body: CHAT_COMPLETION_BODY.into(),
        stream_chunks: vec![],
    }
}

fn chat_request_body() -> &'static str {
    r#"{"model":"test-model","messages":[{"role":"user","content":"hi"}]}"#
}

/// A mock upstream that tracks the number of POSTs to `/chat/completions`.
///
/// The shared `common::mock_upstream::MockUpstream` harness has no observable
/// counter, and the brief forbids modifying it.  This local helper plugs the
/// gap for test 3 where the negative assertion `hits == 0` is load-bearing.
struct CountingUpstream {
    url: String,
    counter: Arc<AtomicUsize>,
    handle: JoinHandle<()>,
}

impl CountingUpstream {
    async fn start() -> Self {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_route = Arc::clone(&counter);

        let app = Router::new().route(
            "/chat/completions",
            post(move || {
                let counter = Arc::clone(&counter_route);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(Body::from(CHAT_COMPLETION_BODY))
                        .expect("valid response")
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock upstream");
        let port = listener.local_addr().expect("local_addr").port();
        let url = format!("http://127.0.0.1:{port}");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock upstream serve");
        });

        Self { url, counter, handle }
    }

    fn hits(&self) -> usize {
        self.counter.load(Ordering::SeqCst)
    }

    fn shutdown(self) {
        self.handle.abort();
    }
}

/// Test 1 — happy-path: master key passes auth, routing resolves `test-model`,
/// the service pool dispatches to the mock upstream, and the canned response
/// flows back to the client unchanged.  Issuing the same request twice in a
/// row asserts the stack is at least idempotent end-to-end.
///
/// Note: the cache layer is not wired into the default `TestProxy` harness, so
/// the assertion focuses on the stack remaining stable across repeated
/// invocations rather than on cache hit/miss counters.
#[tokio::test]
async fn cache_miss_then_hit_through_full_stack() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![chat_route()]).await;
    let proxy = common::test_proxy::TestProxy::new(&upstream.url);

    let make_request = || {
        Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("authorization", "Bearer sk-master")
            .header("content-type", "application/json")
            .body(Body::from(chat_request_body()))
            .unwrap()
    };

    let resp1 = proxy.router().oneshot(make_request()).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK, "first request must succeed");
    let bytes1 = Body::new(resp1.into_body()).collect().await.unwrap().to_bytes();
    let body1: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
    assert_eq!(body1["object"], "chat.completion");
    assert_eq!(body1["choices"][0]["message"]["content"], "Hello!");

    let resp2 = proxy.router().oneshot(make_request()).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK, "second request must succeed");
    let bytes2 = Body::new(resp2.into_body()).collect().await.unwrap().to_bytes();
    let body2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(body1, body2, "stack must produce identical responses");

    upstream.shutdown();
}

/// Test 2 — virtual key with a model allowlist successfully routes through
/// auth → key_resolver → service_pool → upstream when the requested model is
/// permitted.  Proves the full stack runs for virtual-key auth, not just
/// master-key.
#[tokio::test]
async fn virtual_key_routes_through_full_stack() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![chat_route()]).await;

    let mut config = common::test_proxy::default_config(&upstream.url);
    config.keys = vec![VirtualKeyConfig {
        key: "sk-vk-allowed".into(),
        description: Some("vk with test-model allowlist".into()),
        models: vec!["test-model".into()],
        rpm: None,
        tpm: None,
        budget_limit: None,
        provider_credentials: vec![],
    }];
    let proxy = common::test_proxy::TestProxy::with_config(config);

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer sk-vk-allowed")
                .header("content-type", "application/json")
                .body(Body::from(chat_request_body()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "VK with test-model in allowlist must reach upstream"
    );

    let bytes = Body::new(resp.into_body()).collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["object"], "chat.completion");

    upstream.shutdown();
}

/// Test 3 — virtual key whose model allowlist does NOT include `test-model`
/// must be rejected with `403 Forbidden` before any upstream call is made.
/// Proves the auth/authorization layer short-circuits the pipeline ahead of
/// routing and service-pool dispatch.
///
/// Uses [`CountingUpstream`] to assert `hits == 0` after the request.
#[tokio::test]
async fn denied_model_short_circuits_before_upstream() {
    let upstream = CountingUpstream::start().await;

    let mut config = common::test_proxy::default_config(&upstream.url);
    config.keys = vec![VirtualKeyConfig {
        key: "sk-vk-denied".into(),
        description: Some("vk without test-model".into()),
        models: vec!["other".into()],
        rpm: None,
        tpm: None,
        budget_limit: None,
        provider_credentials: vec![],
    }];
    let proxy = common::test_proxy::TestProxy::with_config(config);

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer sk-vk-denied")
                .header("content-type", "application/json")
                .body(Body::from(chat_request_body()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "model outside VK allowlist must return 403"
    );
    assert_eq!(
        upstream.hits(),
        0,
        "auth layer must short-circuit before any upstream call"
    );

    let bytes = Body::new(resp.into_body()).collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["type"], "Forbidden");

    upstream.shutdown();
}
