pub mod audio;
pub mod batches;
pub mod chat;
pub mod embeddings;
pub mod files;
pub mod health;
pub mod images;
pub mod models;
pub mod moderations;
pub mod ocr;
pub mod realtime;
pub mod rerank;
pub mod responses;
pub mod search;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::HeaderValue;
use axum::http::header::AUTHORIZATION;
use axum::middleware;
use axum::routing::{get, post};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::sensitive_headers::SetSensitiveHeadersLayer;
use tower_http::trace::TraceLayer;

use tower::Service;

use liter_llm::tower::types::{LlmRequest, LlmResponse};

use crate::auth;
use crate::auth::KeyContext;
use crate::error::ProxyError;
use crate::state::AppState;

/// Check model access for the authenticated key, attach the tenant identifier,
/// and dispatch the request through the Tower service stack.
///
/// `tenant_id` is propagated via [`LlmRequest::with_tenant_id`] so that every
/// Tower layer downstream — [`BudgetLedger::Tenant`], [`TenantScopedStrategy`],
/// and [`UsageEvent`] — receives the correct tenant dimension without each
/// handler needing to wire it independently.
pub(crate) async fn dispatch(
    state: &AppState,
    key_ctx: &KeyContext,
    model: &str,
    request: LlmRequest,
) -> Result<LlmResponse, ProxyError> {
    if !key_ctx.can_access_model(model) {
        return Err(ProxyError::forbidden(format!(
            "key '{}' is not allowed to access model '{model}'",
            key_ctx.key_id
        )));
    }
    let request = request.with_tenant_id(key_ctx.tenant_id.clone());
    let mut svc = state.service_pool.get_service(model)?;
    Ok(svc.call(request).await?)
}

/// Build the full axum router with all routes, middleware, and shared state.
pub fn build_router(state: AppState) -> Router {
    let v1_routes = Router::new()
        .route("/v1/chat/completions", post(chat::chat_completions))
        .route("/v1/embeddings", post(embeddings::create_embedding))
        .route("/v1/models", get(models::list_models))
        .route("/v1/images/generations", post(images::create_image))
        .route("/v1/audio/speech", post(audio::create_speech))
        .route("/v1/audio/transcriptions", post(audio::create_transcription))
        .route("/v1/moderations", post(moderations::create_moderation))
        .route("/v1/rerank", post(rerank::rerank))
        .route("/v1/search", post(search::search))
        .route("/v1/ocr", post(ocr::ocr))
        .route("/v1/files", post(files::create_file).get(files::list_files))
        .route(
            "/v1/files/{file_id}",
            get(files::retrieve_file).delete(files::delete_file),
        )
        .route("/v1/files/{file_id}/content", get(files::file_content))
        .route("/v1/batches", post(batches::create_batch).get(batches::list_batches))
        .route("/v1/batches/{batch_id}", get(batches::retrieve_batch))
        .route("/v1/batches/{batch_id}/cancel", post(batches::cancel_batch))
        .route("/v1/responses", post(responses::create_response))
        .route("/v1/responses/{response_id}", get(responses::retrieve_response))
        .route("/v1/responses/{response_id}/cancel", post(responses::cancel_response))
        .route("/v1/realtime", get(realtime::realtime_websocket))
        .layer(middleware::from_fn_with_state(state.clone(), auth::validate_api_key));

    let health_routes = Router::new()
        .route("/health", get(health::health))
        .route("/health/liveness", get(health::liveness))
        .route("/health/readiness", get(health::readiness))
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .route("/openapi.json", get(crate::openapi::openapi_schema));

    // ~keep Router-build config is startup-only; handlers must load config per request.
    let cfg_snapshot = state.config.load();

    // ~keep Wildcard CORS must not expose Authorization or it permits credentialed requests.
    let cors_layer: Option<CorsLayer> = if cfg_snapshot.server.cors_origins.is_empty() {
        None
    } else if cfg_snapshot.server.cors_origins.iter().any(|o| o == "*") {
        Some(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                // ~keep Deliberately exclude Authorization for wildcard origins.
                .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::ACCEPT]),
        )
    } else {
        let origins: Vec<HeaderValue> = cfg_snapshot
            .server
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        Some(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(Any)
                .allow_headers(Any),
        )
    };

    let mut router = Router::new()
        .merge(v1_routes)
        .merge(health_routes)
        .layer(SetSensitiveHeadersLayer::new([AUTHORIZATION]))
        .layer(DefaultBodyLimit::max(cfg_snapshot.server.body_limit_bytes))
        .layer(CompressionLayer::new())
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    if let Some(layer) = cors_layer {
        router = router.layer(layer);
    }

    router
}
