pub mod key_store;

pub use key_store::{KeyContext, KeyStore, MASTER_TENANT_ID};

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;
use liter_llm::tenant::KeyResolverError;

use crate::error::ProxyError;
use crate::state::AppState;

/// Axum middleware that validates the `Authorization: Bearer <token>` header
/// against the configured master key and virtual key store.
///
/// On success the resolved [`KeyContext`] — including a populated `tenant_id`
/// — is inserted into request extensions so downstream handlers can inspect
/// model-access permissions and attach the tenant to outbound [`liter_llm::tower::types::LlmRequest`]s.
pub async fn validate_api_key(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProxyError> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ProxyError::authentication("Missing or invalid Authorization header"))?;

    if state.key_store.is_master_key(token) {
        request.extensions_mut().insert(KeyContext::master());
        return Ok(next.run(request).await);
    }

    let token_owned = token.to_owned();
    let resolved = state.key_resolver.resolve(token_owned).await.map_err(|e| match e {
        KeyResolverError::NotFound | KeyResolverError::Inactive => ProxyError::authentication("Invalid API key"),
        KeyResolverError::Backend(msg) => ProxyError::internal(format!("key resolver backend error: {msg}")),
    })?;

    let ctx = KeyContext::from_resolved(token, &resolved);
    request.extensions_mut().insert(ctx);
    Ok(next.run(request).await)
}
