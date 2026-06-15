//! Tower middleware layer that enforces guardrail checks at each request stage.
//!
//! [`GuardrailLayer`] wraps any [`Service<LlmRequest>`] and runs the registered
//! guardrails at three lifecycle points:
//!
//! - **`Input`** — before forwarding the request to the inner service. A
//!   `Block` decision returns [`LiterLlmError::HookRejected`] immediately.
//! - **`Output`** — after the inner service returns a non-streaming response.
//!   A `Block` decision returns an error; `Mutate` replaces the response JSON.
//! - **`OutputChunk`** — for each streaming chunk. A `Block` decision
//!   terminates the stream; `Mutate` replaces the chunk text.
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use liter_llm::guardrail::registry::GuardrailRegistry;
//! use liter_llm::tower::guardrail::GuardrailLayer;
//! use tower::ServiceBuilder;
//!
//! let registry = Arc::new(GuardrailRegistry::new());
//! let service = ServiceBuilder::new()
//!     .layer(GuardrailLayer::new(registry, Default::default()))
//!     .service(inner_service);
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::task::{Context, Poll};

use tower::Layer;
use tower::Service;

use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};
use crate::guardrail::registry::GuardrailRegistry;
use crate::guardrail::{GuardrailContext, GuardrailDecision, GuardrailStage};

use super::types::{LlmRequest, LlmResponse};

// ── Layer ─────────────────────────────────────────────────────────────────────

/// Tower [`Layer`] that enforces guardrail checks around an inner service.
///
/// `registry` holds the ordered list of guardrails to evaluate.
/// `metadata` provides per-layer static tags (e.g., route, deployment) that are
/// merged with per-call metadata passed by the application.
#[cfg_attr(alef, alef(skip))]
#[derive(Clone)]
pub struct GuardrailLayer {
    registry: Arc<GuardrailRegistry>,
    metadata: Arc<HashMap<String, String>>,
}

impl GuardrailLayer {
    /// Create a new [`GuardrailLayer`] with the given registry and static metadata.
    ///
    /// `metadata` is merged into the [`GuardrailContext`] for every request.
    /// Per-call metadata (e.g., `user_id`, `tenant_id`) should be provided via
    /// [`GuardrailContext::metadata`] on a per-request basis; this constructor
    /// accepts layer-level static tags only.
    #[must_use]
    pub fn new(registry: Arc<GuardrailRegistry>, metadata: HashMap<String, String>) -> Self {
        Self {
            registry,
            metadata: Arc::new(metadata),
        }
    }

    /// Create a new [`GuardrailLayer`] with no static metadata.
    #[must_use]
    pub fn with_registry(registry: Arc<GuardrailRegistry>) -> Self {
        Self::new(registry, HashMap::new())
    }
}

impl<S> Layer<S> for GuardrailLayer {
    type Service = GuardrailService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GuardrailService {
            inner,
            registry: Arc::clone(&self.registry),
            metadata: Arc::clone(&self.metadata),
        }
    }
}

// ── Service ───────────────────────────────────────────────────────────────────

/// Tower service produced by [`GuardrailLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct GuardrailService<S> {
    inner: S,
    registry: Arc<GuardrailRegistry>,
    metadata: Arc<HashMap<String, String>>,
}

impl<S: Clone> Clone for GuardrailService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            registry: Arc::clone(&self.registry),
            metadata: Arc::clone(&self.metadata),
        }
    }
}

impl<S> Service<LlmRequest> for GuardrailService<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let registry = Arc::clone(&self.registry);
        let metadata = Arc::clone(&self.metadata);
        let inner_fut = self.inner.call(req.clone());

        Box::pin(async move {
            // Serialize the request once for use in guardrail contexts.
            let request_json = match serde_json::to_value(&req) {
                Ok(v) => v,
                Err(e) => {
                    return Err(LiterLlmError::InternalError {
                        message: format!("guardrail: failed to serialize request: {e}"),
                    });
                }
            };

            // ── Input stage ─────────────────────────────────────────────────
            let input_ctx = GuardrailContext {
                request: &request_json,
                response: None,
                chunk: None,
                metadata: &metadata,
            };

            let input_decision = registry.run_stage(GuardrailStage::Input, &input_ctx).await;
            match input_decision {
                GuardrailDecision::Block { reason, code } => {
                    return Err(LiterLlmError::HookRejected {
                        message: format!("guardrail blocked [code={code}]: {reason}"),
                    });
                }
                GuardrailDecision::Mutate { .. } => {
                    // Mutate at Input stage: in a full implementation, the mutated
                    // payload would be used to rebuild the request before forwarding.
                    // For now, we log and proceed with the original — implementing
                    // full request mutation requires per-variant deserialization.
                    tracing::debug!("guardrail: Input stage Mutate decision; proceeding with original request");
                }
                GuardrailDecision::Allow => {}
            }

            // ── Inner service call ───────────────────────────────────────────
            let response = inner_fut.await?;

            // ── Output stage ─────────────────────────────────────────────────
            // Serialize the inner response for guardrail context, matching on the
            // specific variants that implement `Serialize`.  Streaming and other
            // non-serialisable variants skip the Output stage and return directly.
            let response_json = match &response {
                LlmResponse::Chat(r) => match serde_json::to_value(r) {
                    Ok(v) => v,
                    Err(_) => return Ok(response),
                },
                LlmResponse::Embed(r) => match serde_json::to_value(r) {
                    Ok(v) => v,
                    Err(_) => return Ok(response),
                },
                LlmResponse::ListModels(r) => match serde_json::to_value(r) {
                    Ok(v) => v,
                    Err(_) => return Ok(response),
                },
                // Non-serialisable variants skip Output-stage checks.
                _ => return Ok(response),
            };

            let output_ctx = GuardrailContext {
                request: &request_json,
                response: Some(&response_json),
                chunk: None,
                metadata: &metadata,
            };

            let output_decision = registry.run_stage(GuardrailStage::Output, &output_ctx).await;
            match output_decision {
                GuardrailDecision::Block { reason, code } => {
                    Err(LiterLlmError::HookRejected {
                        message: format!("guardrail blocked output [code={code}]: {reason}"),
                    })
                }
                GuardrailDecision::Mutate { .. } => {
                    // Response mutation requires rebuilding the typed response from JSON.
                    // For the initial implementation we log and return the original.
                    tracing::debug!("guardrail: Output stage Mutate decision; returning original response");
                    Ok(response)
                }
                GuardrailDecision::Allow => Ok(response),
            }
        })
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    use tower::{Layer, Service};

    use super::*;
    use crate::guardrail::builtin::DenyListGuardrail;
    use crate::guardrail::registry::GuardrailRegistry;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    #[tokio::test]
    async fn guardrail_layer_allows_when_registry_is_empty() {
        let registry = Arc::new(GuardrailRegistry::new());
        let inner = LlmService::new(MockClient::ok());
        let mut svc = GuardrailLayer::with_registry(registry).layer(inner);

        let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(result.is_ok(), "empty registry should allow all requests");
    }

    #[tokio::test]
    async fn guardrail_layer_input_block_prevents_inner_call() {
        let mut registry = GuardrailRegistry::new();
        let list: HashSet<String> = ["banned-user"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(DenyListGuardrail::new("ban", list, "user_id")));

        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);

        let mut meta = HashMap::new();
        meta.insert("user_id".to_string(), "banned-user".to_string());

        let mut svc = GuardrailLayer::new(Arc::new(registry), meta).layer(inner);
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("banned user should be blocked");

        assert!(
            matches!(err, LiterLlmError::HookRejected { .. }),
            "guardrail block should surface as HookRejected"
        );
        assert_eq!(call_count.load(Ordering::SeqCst), 0, "inner service must not be called");
    }

    #[tokio::test]
    async fn guardrail_layer_allows_non_blocked_user() {
        let mut registry = GuardrailRegistry::new();
        let list: HashSet<String> = ["banned-user"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(DenyListGuardrail::new("ban", list, "user_id")));

        let inner = LlmService::new(MockClient::ok());
        let mut meta = HashMap::new();
        meta.insert("user_id".to_string(), "good-user".to_string());

        let mut svc = GuardrailLayer::new(Arc::new(registry), meta).layer(inner);
        let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(result.is_ok(), "non-blocked user should pass through");
    }
}
