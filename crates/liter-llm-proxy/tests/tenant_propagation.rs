mod common;

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use liter_llm::observability::{UsageEvent, UsageSink, UsageSinkErased, UsageSinkError};
use liter_llm::tenant::{InMemoryKeyResolver, ResolvedKey, TenantId};
use tower::ServiceExt;

use common::mock_upstream::{MockRoute, MockUpstream};

/// Captures every emitted [`UsageEvent`] in a `Vec` so tests can inspect them.
#[derive(Default, Clone)]
struct VecUsageSink {
    events: Arc<Mutex<Vec<UsageEvent>>>,
}

impl VecUsageSink {
    fn collected(&self) -> Vec<UsageEvent> {
        self.events.lock().expect("VecUsageSink lock poisoned").clone()
    }
}

impl UsageSink for VecUsageSink {
    async fn emit(&self, event: UsageEvent) -> Result<(), UsageSinkError> {
        self.events.lock().expect("VecUsageSink lock poisoned").push(event);
        Ok(())
    }
}

const CHAT_BODY: &str = r#"{"id":"cmpl-1","object":"chat.completion","created":1700000000,"model":"gpt-4o","choices":[{"index":0,"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}"#;

const EMBED_BODY: &str = r#"{"object":"list","data":[{"object":"embedding","index":0,"embedding":[0.1,0.2]}],"model":"text-embedding-3-small","usage":{"prompt_tokens":5,"total_tokens":5}}"#;

const RERANK_BODY: &str =
    r#"{"model":"rerank-v3","results":[{"index":0,"relevance_score":0.9,"document":{"text":"doc"}}]}"#;

/// Build a `ResolvedKey` for a given `tenant_id` that has unrestricted model access.
fn resolved_key(tenant_id: impl Into<String>) -> ResolvedKey {
    ResolvedKey {
        tenant_id: TenantId::from(tenant_id.into().as_str()),
        allowed_models: vec![],
        monthly_budget: None,
        currency: None,
        metadata: std::collections::HashMap::new(),
        active: true,
    }
}

/// Build a `TestProxy` with:
/// - a mock upstream at `mock_url`,
/// - an `InMemoryKeyResolver` pre-loaded with the supplied entries, and
/// - a `VecUsageSink` to capture emitted events.
///
/// Returns `(proxy, sink)`.
fn build_proxy_with_sink(
    mock_url: &str,
    resolver_entries: Vec<(&'static str, ResolvedKey)>,
) -> (common::test_proxy::TestProxy, VecUsageSink) {
    let sink = VecUsageSink::default();
    let sink_erased: Arc<dyn UsageSinkErased> = Arc::new(sink.clone());

    let resolver = InMemoryKeyResolver::with_entries(resolver_entries.into_iter().map(|(k, v)| (k.to_string(), v)));

    let proxy = common::test_proxy::TestProxy::with_injection(mock_url, Some(Arc::new(resolver)), Some(sink_erased));

    (proxy, sink)
}

/// POST a JSON request to `path` with an `Authorization: Bearer <token>` header
/// and return the HTTP response.
async fn post_json(
    proxy: &common::test_proxy::TestProxy,
    path: &str,
    token: &str,
    body: &str,
) -> axum::response::Response {
    proxy
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Chat requests made with a virtual key carry the key's `tenant_id` in the
/// emitted `UsageEvent`.
#[tokio::test]
async fn chat_request_carries_tenant_id() {
    let upstream = MockUpstream::start(vec![MockRoute {
        path: "/chat/completions".into(),
        method: "POST".into(),
        status: 200,
        body: CHAT_BODY.into(),
        stream_chunks: vec![],
    }])
    .await;

    let (proxy, sink) = build_proxy_with_sink(&upstream.url, vec![("sk-acme", resolved_key("acme"))]);

    let body = r#"{"model":"test-model","messages":[{"role":"user","content":"hi"}]}"#;
    let resp = post_json(&proxy, "/v1/chat/completions", "sk-acme", body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = Body::new(resp.into_body()).collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["object"], "chat.completion", "expected a chat completion response");

    tokio::task::yield_now().await;
    let events = sink.collected();
    assert_eq!(events.len(), 1, "expected exactly one usage event");
    assert_eq!(
        events[0].tenant_id,
        Some(TenantId::from("acme")),
        "tenant_id must match the resolved tenant, not the raw key token"
    );

    upstream.shutdown();
}

/// Embedding requests carry the tenant resolved from the virtual key.
#[tokio::test]
async fn embeddings_request_carries_tenant_id() {
    let upstream = MockUpstream::start(vec![MockRoute {
        path: "/embeddings".into(),
        method: "POST".into(),
        status: 200,
        body: EMBED_BODY.into(),
        stream_chunks: vec![],
    }])
    .await;

    let (proxy, sink) = build_proxy_with_sink(&upstream.url, vec![("sk-embed-tenant", resolved_key("embed-corp"))]);

    let body = r#"{"model":"test-model","input":"hello world"}"#;
    let resp = post_json(&proxy, "/v1/embeddings", "sk-embed-tenant", body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    tokio::task::yield_now().await;
    let events = sink.collected();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].tenant_id,
        Some(TenantId::from("embed-corp")),
        "embedding event must carry embed-corp tenant"
    );

    upstream.shutdown();
}

/// Rerank requests carry the tenant resolved from the virtual key.
#[tokio::test]
async fn rerank_request_carries_tenant_id() {
    let upstream = MockUpstream::start(vec![MockRoute {
        path: "/rerank".into(),
        method: "POST".into(),
        status: 200,
        body: RERANK_BODY.into(),
        stream_chunks: vec![],
    }])
    .await;

    let (proxy, sink) = build_proxy_with_sink(&upstream.url, vec![("sk-rerank-tenant", resolved_key("rerank-corp"))]);

    let body = r#"{"model":"test-model","query":"what is rust","documents":["doc one"]}"#;
    let resp = post_json(&proxy, "/v1/rerank", "sk-rerank-tenant", body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    tokio::task::yield_now().await;
    let events = sink.collected();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].tenant_id,
        Some(TenantId::from("rerank-corp")),
        "rerank event must carry rerank-corp tenant"
    );

    upstream.shutdown();
}

/// Requests authenticated with the master key resolve to `TenantId("master")`.
#[tokio::test]
async fn master_key_resolves_to_master_tenant() {
    let upstream = MockUpstream::start(vec![MockRoute {
        path: "/chat/completions".into(),
        method: "POST".into(),
        status: 200,
        body: CHAT_BODY.into(),
        stream_chunks: vec![],
    }])
    .await;

    let sink = VecUsageSink::default();
    let sink_erased: Arc<dyn UsageSinkErased> = Arc::new(sink.clone());

    let proxy = common::test_proxy::TestProxy::with_injection(&upstream.url, None, Some(sink_erased));

    let body = r#"{"model":"test-model","messages":[{"role":"user","content":"hi"}]}"#;
    let resp = post_json(&proxy, "/v1/chat/completions", "sk-master", body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    tokio::task::yield_now().await;
    let events = sink.collected();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].tenant_id,
        Some(TenantId::from("master")),
        "master-key auth must resolve to TenantId(\"master\")"
    );

    upstream.shutdown();
}

/// Each of the three primary LLM-routed endpoints (chat, embeddings, rerank)
/// propagates the tenant from the same virtual key.
#[tokio::test]
async fn parameterized_endpoints_carry_tenant() {
    let upstream = MockUpstream::start(vec![
        MockRoute {
            path: "/chat/completions".into(),
            method: "POST".into(),
            status: 200,
            body: CHAT_BODY.into(),
            stream_chunks: vec![],
        },
        MockRoute {
            path: "/embeddings".into(),
            method: "POST".into(),
            status: 200,
            body: EMBED_BODY.into(),
            stream_chunks: vec![],
        },
        MockRoute {
            path: "/rerank".into(),
            method: "POST".into(),
            status: 200,
            body: RERANK_BODY.into(),
            stream_chunks: vec![],
        },
    ])
    .await;

    let tenant = TenantId::from("multi-endpoint-corp");

    struct Case {
        path: &'static str,
        body: &'static str,
    }

    let cases = [
        Case {
            path: "/v1/chat/completions",
            body: r#"{"model":"test-model","messages":[{"role":"user","content":"hi"}]}"#,
        },
        Case {
            path: "/v1/embeddings",
            body: r#"{"model":"test-model","input":"hello"}"#,
        },
        Case {
            path: "/v1/rerank",
            body: r#"{"model":"test-model","query":"q","documents":["d"]}"#,
        },
    ];

    for case in &cases {
        let (proxy, sink) =
            build_proxy_with_sink(&upstream.url, vec![("sk-multi", resolved_key("multi-endpoint-corp"))]);

        let resp = post_json(&proxy, case.path, "sk-multi", case.body).await;
        assert_eq!(resp.status(), StatusCode::OK, "expected 200 for {}", case.path);

        tokio::task::yield_now().await;
        let events = sink.collected();
        assert_eq!(events.len(), 1, "expected one event for {}", case.path);
        assert_eq!(
            events[0].tenant_id,
            Some(tenant.clone()),
            "tenant must be propagated for {}",
            case.path
        );
    }

    upstream.shutdown();
}
