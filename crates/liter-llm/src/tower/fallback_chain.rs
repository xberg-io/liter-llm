//! Multi-step fallback chain layer.
//!
//! Walks an ordered `Vec<S>` of services, advancing to the next candidate on
//! transient errors. Terminal errors abort the chain immediately. Per-attempt
//! retry classification is delegated to a pluggable [`RetryPolicy`] trait.
//!
//! The default policy ([`DefaultRetryPolicy`]) treats rate-limit (429),
//! service-unavailable (502/503/504), timeout, server error (5xx), and network
//! errors as transient; authentication (401/403), bad-request (400/422),
//! context-window-exceeded, content-policy, and not-found (404) as terminal.

use std::sync::Arc;
use std::task::{Context, Poll};

use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

// ─── RetryClass ───────────────────────────────────────────────────────────────

/// Classification of a single attempt error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryClass {
    /// Transient error — advance to the next service in the chain.
    Transient,
    /// Terminal error — return immediately without consulting further services.
    Terminal,
}

// ─── RetryPolicy trait ────────────────────────────────────────────────────────

/// Classifies an error as transient or terminal for fallback chain decisions.
///
/// Implement this to provide custom retry logic (e.g. to treat 429 as
/// terminal when the caller wants to surface rate-limit errors immediately).
pub trait RetryPolicy: Send + Sync + 'static {
    /// Classify `error` as [`RetryClass::Transient`] or [`RetryClass::Terminal`].
    fn classify(&self, error: &LiterLlmError) -> RetryClass;
}

// ─── DefaultRetryPolicy ───────────────────────────────────────────────────────

/// Default [`RetryPolicy`]: delegates to [`LiterLlmError::is_transient`].
///
/// Transient: rate-limited (429), service unavailable (502/503/504), timeout,
/// server error (5xx), network errors.
/// Terminal: authentication, bad request, context-window-exceeded, content
/// policy, not-found, budget-exceeded, hook-rejected, and all others.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultRetryPolicy;

impl RetryPolicy for DefaultRetryPolicy {
    fn classify(&self, error: &LiterLlmError) -> RetryClass {
        if error.is_transient() {
            RetryClass::Transient
        } else {
            RetryClass::Terminal
        }
    }
}

// ─── FallbackChainLayer ───────────────────────────────────────────────────────

/// Tower [`Layer`] that walks an ordered list of services on transient errors.
///
/// On each call the layer iterates through the chain in order, invoking the
/// next service only when the previous one returns a
/// [`RetryClass::Transient`] error. The first successful response or terminal
/// error is returned immediately.
///
/// When all services are exhausted the last observed transient error is
/// returned.
///
/// # Cloning
///
/// The inner chain is stored behind an [`Arc`] so that [`FallbackChainLayer`]
/// and the produced [`FallbackChainService`] can be cloned cheaply. Each
/// service in the chain must implement `Clone`.
#[cfg_attr(alef, alef(skip))]
pub struct FallbackChainLayer<S, R: RetryPolicy = DefaultRetryPolicy> {
    chain: Arc<Vec<S>>,
    policy: Arc<R>,
}

impl<S> FallbackChainLayer<S, DefaultRetryPolicy> {
    /// Create a new layer with the given ordered service chain and the
    /// default retry policy.
    ///
    /// # Panics
    ///
    /// Does not panic; an empty chain will resolve to `ServerError` on any
    /// call, as there are no services to try.
    #[must_use]
    pub fn new(chain: Vec<S>) -> Self {
        Self {
            chain: Arc::new(chain),
            policy: Arc::new(DefaultRetryPolicy),
        }
    }
}

impl<S, R: RetryPolicy> FallbackChainLayer<S, R> {
    /// Create a new layer with the given ordered service chain and a custom
    /// retry policy.
    #[must_use]
    pub fn with_policy(chain: Vec<S>, policy: R) -> Self {
        Self {
            chain: Arc::new(chain),
            policy: Arc::new(policy),
        }
    }
}

impl<S: Clone, R: RetryPolicy> Clone for FallbackChainLayer<S, R> {
    fn clone(&self) -> Self {
        Self {
            chain: Arc::clone(&self.chain),
            policy: Arc::clone(&self.policy),
        }
    }
}

impl<S, R, Inner> Layer<Inner> for FallbackChainLayer<S, R>
where
    R: RetryPolicy,
{
    type Service = FallbackChainService<S, R>;

    fn layer(&self, _inner: Inner) -> Self::Service {
        // The chain _is_ the set of inner services; the `Inner` parameter is
        // accepted to satisfy Tower's `Layer<S>` interface convention but is
        // not used. Callers should pass the first service as the chain head
        // and supply the remainder via `FallbackChainLayer::new(chain)`.
        FallbackChainService {
            chain: Arc::clone(&self.chain),
            policy: Arc::clone(&self.policy),
        }
    }
}

// ─── FallbackChainService ─────────────────────────────────────────────────────

/// Tower service produced by [`FallbackChainLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct FallbackChainService<S, R: RetryPolicy = DefaultRetryPolicy> {
    chain: Arc<Vec<S>>,
    policy: Arc<R>,
}

impl<S: Clone, R: RetryPolicy> Clone for FallbackChainService<S, R> {
    fn clone(&self) -> Self {
        Self {
            chain: Arc::clone(&self.chain),
            policy: Arc::clone(&self.policy),
        }
    }
}

impl<S, R> Service<LlmRequest> for FallbackChainService<S, R>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + Sync + 'static,
    S::Future: Send + 'static,
    R: RetryPolicy,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // Services are cloned per-call (same rationale as `Router`); no
        // persistent readied slot to manage here.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: LlmRequest) -> Self::Future {
        let chain = Arc::clone(&self.chain);
        let policy = Arc::clone(&self.policy);

        Box::pin(async move {
            let chain_len = chain.len();
            tracing::debug!(chain_len, "fallback chain: starting walk");

            if chain.is_empty() {
                return Err(LiterLlmError::ServerError {
                    message: "fallback chain is empty".into(),
                    status: 500,
                });
            }

            let mut last_err: Option<LiterLlmError> = None;

            for (attempt, svc_template) in chain.iter().enumerate() {
                let mut svc = svc_template.clone();
                let span = tracing::debug_span!(
                    "fallback_chain.attempt",
                    chain_len,
                    attempt,
                    outcome = tracing::field::Empty,
                );
                let _guard = span.enter();

                match svc.call(request.clone()).await {
                    Ok(resp) => {
                        tracing::debug!(attempt, "fallback chain: success");
                        span.record("outcome", "success");
                        return Ok(resp);
                    }
                    Err(err) => match policy.classify(&err) {
                        RetryClass::Terminal => {
                            tracing::debug!(
                                attempt,
                                error = %err,
                                "fallback chain: terminal error, aborting"
                            );
                            span.record("outcome", "terminal");
                            return Err(err);
                        }
                        RetryClass::Transient => {
                            tracing::warn!(
                                attempt,
                                chain_len,
                                error = %err,
                                "fallback chain: transient error, trying next service"
                            );
                            span.record("outcome", "transient");
                            last_err = Some(err);
                        }
                    },
                }
            }

            Err(last_err.unwrap_or(LiterLlmError::ServerError {
                message: "fallback chain exhausted all services".into(),
                status: 503,
            }))
        })
    }
}
