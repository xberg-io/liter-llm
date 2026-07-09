mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use liter_llm_proxy::config::ProxyConfig;

fn config_with_cors(mock_url: &str, cors_origins: &[&str]) -> ProxyConfig {
    let origins_toml = cors_origins
        .iter()
        .map(|o| format!(r#""{o}""#))
        .collect::<Vec<_>>()
        .join(", ");
    ProxyConfig::from_toml_str(&format!(
        r#"
[server]
cors_origins = [{origins_toml}]

[general]
master_key = "sk-master"

[[models]]
name = "test-model"
provider_model = "openai/gpt-4o"
api_key = "sk-upstream"
base_url = "{mock_url}"
"#
    ))
    .expect("cors config TOML")
}

#[tokio::test]
async fn empty_cors_origins_produces_no_allow_origin_header() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = common::test_proxy::TestProxy::with_config(config_with_cors(&upstream.url, &[]));

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .header("origin", "https://evil.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "default (empty) cors_origins must not produce access-control-allow-origin"
    );

    upstream.shutdown();
}

#[tokio::test]
async fn empty_cors_origins_preflight_returns_no_cors_headers() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = common::test_proxy::TestProxy::with_config(config_with_cors(&upstream.url, &[]));

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/v1/chat/completions")
                .header("origin", "https://evil.example.com")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "authorization,content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "empty cors_origins: no access-control-allow-origin on preflight"
    );

    upstream.shutdown();
}

#[tokio::test]
async fn wildcard_cors_allows_any_origin() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = common::test_proxy::TestProxy::with_config(config_with_cors(&upstream.url, &["*"]));

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .header("origin", "https://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let allow_origin = resp
        .headers()
        .get("access-control-allow-origin")
        .map(|v| v.to_str().unwrap_or(""));
    assert!(
        allow_origin.is_some(),
        "cors_origins=[\"*\"] must set access-control-allow-origin"
    );

    upstream.shutdown();
}

#[tokio::test]
async fn wildcard_cors_does_not_allow_authorization_header() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = common::test_proxy::TestProxy::with_config(config_with_cors(&upstream.url, &["*"]));

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/v1/chat/completions")
                .header("origin", "https://example.com")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "authorization,content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let allowed_headers = resp
        .headers()
        .get("access-control-allow-headers")
        .map(|v| v.to_str().unwrap_or("").to_ascii_lowercase())
        .unwrap_or_default();

    assert!(
        !allowed_headers.contains("authorization"),
        "wildcard CORS must not allow the Authorization header; got: {allowed_headers}"
    );

    upstream.shutdown();
}

#[tokio::test]
async fn explicit_origin_is_echoed() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = common::test_proxy::TestProxy::with_config(config_with_cors(&upstream.url, &["https://example.com"]));

    let resp = proxy
        .router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .header("origin", "https://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let allow_origin = resp
        .headers()
        .get("access-control-allow-origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(
        allow_origin, "https://example.com",
        "explicit cors_origins must echo the allowed origin"
    );

    upstream.shutdown();
}
