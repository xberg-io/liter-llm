/// Tests for embedder-supplied `KeyResolver` and `UsageSink` injection.
///
/// These tests verify that `ProxyServer::with_key_resolver` and
/// `ProxyServer::with_usage_sink` wire correctly through `AppState` and the
/// Tower stack.  Each test builds a `TestProxy` using `with_injection` so
/// the integration layer is exercised end-to-end without binding a port.
mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use liter_llm::observability::{UsageEvent, UsageSink, UsageSinkErased, UsageSinkError};
use liter_llm::tenant::{KeyResolver, KeyResolverError, ResolvedKey, TenantId};

use common::test_proxy::TestProxy;

// ─── Mock fixtures ───────────────────────────────────────────────────────────

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

fn chat_request_with_model(model: &str) -> String {
    format!(r#"{{"model":"{model}","messages":[{{"role":"user","content":"hi"}}]}}"#)
}

/// Key resolver that accepts one specific token and counts how many times
/// `resolve` is called.
struct MockKeyResolver {
    expected_key: String,
    call_count: Arc<AtomicUsize>,
}

impl MockKeyResolver {
    fn new(expected_key: impl Into<String>) -> (Self, Arc<AtomicUsize>) {
        let call_count = Arc::new(AtomicUsize::new(0));
        (
            Self {
                expected_key: expected_key.into(),
                call_count: Arc::clone(&call_count),
            },
            call_count,
        )
    }
}

impl KeyResolver for MockKeyResolver {
    fn resolve(
        &self,
        api_key: String,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'static>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let expected = self.expected_key.clone();
        Box::pin(async move {
            if api_key == expected {
                Ok(ResolvedKey {
                    tenant_id: TenantId::from(api_key),
                    allowed_models: vec![],
                    monthly_budget: None,
                    currency: None,
                    metadata: std::collections::HashMap::new(),
                    active: true,
                })
            } else {
                Err(KeyResolverError::NotFound)
            }
        })
    }
}

/// Usage sink that collects every emitted event into a shared `Vec`.
#[derive(Default)]
struct VecUsageSink {
    events: Arc<Mutex<Vec<UsageEvent>>>,
}

impl VecUsageSink {
    fn events(&self) -> Vec<UsageEvent> {
        self.events.lock().expect("lock poisoned").clone()
    }
}

impl UsageSink for VecUsageSink {
    async fn emit(&self, event: UsageEvent) -> Result<(), UsageSinkError> {
        self.events.lock().expect("lock poisoned").push(event);
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// Verify that a proxy built with no builder calls serves requests the same
/// way it does today (no regression in default behaviour).
#[tokio::test]
async fn default_construction_still_works() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![chat_route()]).await;
    let proxy = TestProxy::new(&upstream.url);

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer sk-master")
                .header("content-type", "application/json")
                .body(Body::from(chat_request_with_model("test-model")))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = Body::new(resp.into_body()).collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["object"], "chat.completion");

    upstream.shutdown();
}

/// Verify that a custom `KeyResolver` is consulted for virtual-key requests.
///
/// A virtual key (`sk-vk`) is presented; the proxy must call
/// `MockKeyResolver::resolve` exactly once with that token.  The master-key
/// path bypasses the resolver (checked in other tests), so we intentionally
/// use a virtual key here.
#[tokio::test]
async fn custom_key_resolver_is_consulted() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![chat_route()]).await;

    let (mock_resolver, call_count) = MockKeyResolver::new("sk-vk");
    let proxy = TestProxy::with_injection(&upstream.url, Some(Arc::new(mock_resolver)), None);

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer sk-vk")
                .header("content-type", "application/json")
                .body(Body::from(chat_request_with_model("test-model")))
                .unwrap(),
        )
        .await
        .unwrap();

    // The resolver returns an unrestricted `ResolvedKey`, so the request
    // should proceed through to the upstream and succeed.
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "request should succeed with custom resolver"
    );
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        1,
        "resolver must be called exactly once per request"
    );

    upstream.shutdown();
}

/// Verify that a custom `UsageSink` receives exactly one `Success` event when
/// a chat request completes normally.
#[tokio::test]
async fn custom_usage_sink_receives_events() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![chat_route()]).await;

    let sink = Arc::new(VecUsageSink::default());
    let erased: Arc<dyn UsageSinkErased> = sink.clone() as Arc<dyn UsageSinkErased>;
    let proxy = TestProxy::with_injection(&upstream.url, None, Some(erased));

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer sk-master")
                .header("content-type", "application/json")
                .body(Body::from(chat_request_with_model("test-model")))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // The sink emit is fire-and-forget via `tokio::spawn` inside `HooksLayer`.
    // Yield a few times so the spawned task can run.
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let events = sink.events();
    assert_eq!(events.len(), 1, "exactly one usage event must be emitted per request");
    assert_eq!(
        events[0].outcome,
        liter_llm::observability::UsageEventOutcome::Success,
        "outcome must be Success for a completed request"
    );

    upstream.shutdown();
}

/// Verify that omitting the sink builder causes no panics and no events.
///
/// This is an explicit null case: `with_injection` is called with no sink,
/// the proxy still serves requests, and no `VecUsageSink` exists to be
/// accidentally populated.
#[tokio::test]
async fn default_path_has_no_sink_no_panic() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![chat_route()]).await;

    // No sink injected — HooksLayer must not be in the stack.
    let proxy = TestProxy::with_injection(&upstream.url, None, None);

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", "Bearer sk-master")
                .header("content-type", "application/json")
                .body(Body::from(chat_request_with_model("test-model")))
                .unwrap(),
        )
        .await
        .unwrap();

    // Request must succeed; no panic may have occurred.
    assert_eq!(resp.status(), StatusCode::OK);

    upstream.shutdown();
}
