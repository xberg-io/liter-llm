mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use common::test_proxy::{TestProxy, empty_config};

#[tokio::test]
async fn health_returns_200_with_models() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = TestProxy::new(&upstream.url);

    let resp = proxy
        .router()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "healthy");
    assert!(
        !json["models"].as_array().unwrap().is_empty(),
        "expected at least one model in health response"
    );

    upstream.shutdown();
}

#[tokio::test]
async fn health_returns_degraded_without_models() {
    let proxy = TestProxy::with_config(empty_config());

    let resp = proxy
        .router()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "degraded");
    assert!(json["models"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn liveness_returns_200() {
    let proxy = TestProxy::with_config(empty_config());

    let resp = proxy
        .router()
        .oneshot(Request::get("/health/liveness").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn readiness_returns_503_without_models() {
    let proxy = TestProxy::with_config(empty_config());

    let resp = proxy
        .router()
        .oneshot(Request::get("/health/readiness").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn readiness_returns_200_with_models() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = TestProxy::new(&upstream.url);

    let resp = proxy
        .router()
        .oneshot(Request::get("/health/readiness").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    upstream.shutdown();
}

#[tokio::test]
async fn healthz_returns_200() {
    let proxy = TestProxy::with_config(empty_config());

    let resp = proxy
        .router()
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn healthz_returns_ok_status_and_version_fields() {
    let proxy = TestProxy::with_config(empty_config());

    let resp = proxy
        .router()
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok", "status field must be 'ok'");
    assert!(json["uptime_seconds"].is_u64(), "uptime_seconds must be an integer");
    assert!(
        json["version"].is_string(),
        "version must be a string, got: {}",
        json["version"]
    );
    let version = json["version"].as_str().unwrap();
    assert!(
        version.contains('.'),
        "version should look like a semver string, got: {version}"
    );
}

#[tokio::test]
async fn healthz_is_always_200_even_without_models() {
    let proxy = TestProxy::with_config(empty_config());

    let resp = proxy
        .router()
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "/healthz must return 200 regardless of model config"
    );
}

#[tokio::test]
async fn readyz_returns_503_when_no_models_configured() {
    let proxy = TestProxy::with_config(empty_config());

    let resp = proxy
        .router()
        .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn readyz_returns_503_body_with_failed_probe_name() {
    let proxy = TestProxy::with_config(empty_config());

    let resp = proxy
        .router()
        .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "not_ready", "status must be 'not_ready' on 503");
    assert!(
        json["failed_probe"].is_string(),
        "failed_probe must be present on 503, got: {}",
        json
    );
    assert!(
        json["reason"].is_string(),
        "reason must be present on 503, got: {}",
        json
    );
}

#[tokio::test]
async fn readyz_returns_200_when_models_are_configured() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = TestProxy::new(&upstream.url);

    let resp = proxy
        .router()
        .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    upstream.shutdown();
}

#[tokio::test]
async fn readyz_returns_200_body_with_ready_status() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = TestProxy::new(&upstream.url);

    let resp = proxy
        .router()
        .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ready", "status must be 'ready' on 200");
    assert!(
        json["uptime_seconds"].is_u64(),
        "uptime_seconds must be present on 200, got: {}",
        json
    );

    upstream.shutdown();
}

#[tokio::test]
async fn readyz_no_auth_required() {
    let upstream = common::mock_upstream::MockUpstream::start(vec![]).await;
    let proxy = TestProxy::new(&upstream.url);

    let resp = proxy
        .router()
        .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_ne!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "/readyz must not require authentication"
    );

    upstream.shutdown();
}
