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

use tower::{Layer, Service, ServiceExt as _};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

/// Classification of a single attempt error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryClass {
    /// Transient error — advance to the next service in the chain.
    Transient,
    /// Terminal error — return immediately without consulting further services.
    Terminal,
}

/// Classifies an error as transient or terminal for fallback chain decisions.
///
/// Implement this to provide custom retry logic (e.g. to treat 429 as
/// terminal when the caller wants to surface rate-limit errors immediately).
pub trait RetryPolicy: Send + Sync + 'static {
    /// Classify `error` as [`RetryClass::Transient`] or [`RetryClass::Terminal`].
    fn classify(&self, error: &LiterLlmError) -> RetryClass;
}

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

/// `FallbackChainLayer` implements `Layer<()>` rather than the generic `Layer<S>`.
///
/// Unlike most Tower layers, `FallbackChainLayer` owns its entire service chain
/// internally (supplied to `FallbackChainLayer::new`).  The standard
/// `ServiceBuilder::new().layer(layer).service(svc)` composition pattern would
/// pass `svc` as the `inner` argument — but this layer has no single inner
/// service, it has a list.
///
/// # Usage
///
/// Pass `()` as the placeholder inner when using `layer()` directly:
///
/// ```rust,ignore
/// let layer = FallbackChainLayer::new(vec![svc_a, svc_b, svc_c]);
/// let svc = layer.layer(());
/// ```
///
/// To add a service at the head of the chain, use `prepend`:
/// ```rust,ignore
/// let layer = FallbackChainLayer::new(vec![svc_b, svc_c]).prepend(svc_a);
/// let svc = layer.layer(());
/// ```
impl<S: Clone, R: RetryPolicy> Layer<()> for FallbackChainLayer<S, R> {
    type Service = FallbackChainService<S, R>;

    fn layer(&self, _inner: ()) -> Self::Service {
        FallbackChainService {
            chain: Arc::clone(&self.chain),
            policy: Arc::clone(&self.policy),
        }
    }
}

impl<S: Clone, R: RetryPolicy> FallbackChainLayer<S, R> {
    /// Create a new [`FallbackChainLayer`] with `head` prepended to the chain.
    ///
    /// This is the ergonomic alternative to `ServiceBuilder` composition for
    /// `FallbackChainLayer`.  Use it when the first fallback candidate is
    /// logically the primary service:
    ///
    /// ```rust,ignore
    /// let layer = FallbackChainLayer::new(vec![backup_a, backup_b]).prepend(primary);
    /// let svc = layer.layer(());
    /// ```
    #[must_use]
    pub fn prepend(mut self, head: S) -> Self {
        let chain = Arc::make_mut(&mut self.chain);
        chain.insert(0, head);
        self
    }
}

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

                // ~keep Drive each fallback service to ready so permit-based readiness is honored.
                let svc = match svc.ready().await {
                    Ok(s) => s,
                    Err(e) => match policy.classify(&e) {
                        RetryClass::Terminal => {
                            tracing::debug!(
                                attempt,
                                error = %e,
                                "fallback chain: terminal error in poll_ready, aborting"
                            );
                            return Err(e);
                        }
                        RetryClass::Transient => {
                            tracing::warn!(
                                attempt,
                                chain_len,
                                error = %e,
                                "fallback chain: transient error in poll_ready, trying next service"
                            );
                            last_err = Some(e);
                            continue;
                        }
                    },
                };

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll};

    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::error::LiterLlmError;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::{LlmRequest, LlmResponse};

    #[tokio::test]
    async fn fallback_chain_succeeds_on_first_service() {
        let svc = FallbackChainService {
            chain: Arc::new(vec![LlmService::new(MockClient::ok())]),
            policy: Arc::new(DefaultRetryPolicy),
        };
        let mut svc = svc;
        let resp = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("first service must succeed");
        assert!(matches!(resp, LlmResponse::Chat(_)));
    }

    #[tokio::test]
    async fn fallback_chain_advances_on_transient_error() {
        let failing = LlmService::new(MockClient::failing_timeout());
        let succeeding = LlmService::new(MockClient::ok());
        let call_count = Arc::clone(&MockClient::ok().call_count);
        let _ = call_count;

        let ok_client = MockClient::ok();
        let ok_calls = Arc::clone(&ok_client.call_count);
        let mut svc = FallbackChainService {
            chain: Arc::new(vec![failing, LlmService::new(ok_client)]),
            policy: Arc::new(DefaultRetryPolicy),
        };
        let _ = succeeding;

        let resp = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("fallback must succeed on second service");
        assert!(matches!(resp, LlmResponse::Chat(_)));
        assert_eq!(ok_calls.load(Ordering::SeqCst), 1, "second service must be called");
    }

    #[tokio::test]
    async fn fallback_chain_aborts_on_terminal_error() {
        let failing_auth = LlmService::new(MockClient::failing_auth());
        let ok_client = MockClient::ok();
        let ok_calls = Arc::clone(&ok_client.call_count);
        let mut svc = FallbackChainService {
            chain: Arc::new(vec![failing_auth, LlmService::new(ok_client)]),
            policy: Arc::new(DefaultRetryPolicy),
        };

        let err = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect_err("terminal error must abort chain");
        assert!(
            matches!(err, LiterLlmError::BadRequest { .. }),
            "expected BadRequest (terminal), got {err:?}"
        );
        assert_eq!(
            ok_calls.load(Ordering::SeqCst),
            0,
            "second service must NOT be called after terminal error"
        );
    }

    #[tokio::test]
    async fn fallback_chain_empty_returns_server_error() {
        let mut svc = FallbackChainService::<LlmService<MockClient>, DefaultRetryPolicy> {
            chain: Arc::new(vec![]),
            policy: Arc::new(DefaultRetryPolicy),
        };
        let err = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect_err("empty chain must return error");
        assert!(
            matches!(err, LiterLlmError::ServerError { .. }),
            "expected ServerError for empty chain, got {err:?}"
        );
    }

    /// `FallbackChainLayer::prepend(head)` inserts `head` at position 0 so that
    /// it is tried first.  Without this method there was no ergonomic way to
    /// compose a primary service with a fallback chain; callers had to manually
    /// construct the full Vec including the primary.
    #[tokio::test]
    async fn fallback_chain_prepend_inserts_at_head() {
        let ok_client = MockClient::ok();
        let ok_calls = Arc::clone(&ok_client.call_count);
        let head_svc = LlmService::new(ok_client);

        let chain_svc = LlmService::new(MockClient::failing_timeout());
        let layer = FallbackChainLayer::new(vec![chain_svc]).prepend(head_svc);
        let mut svc = layer.layer(());

        let resp = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("prepended service (head) must be tried first and succeed");
        assert!(matches!(resp, LlmResponse::Chat(_)));
        assert_eq!(
            ok_calls.load(Ordering::SeqCst),
            1,
            "prepended head service must be called"
        );
    }

    /// `FallbackChainService::call` must invoke `poll_ready` on each cloned
    /// service before calling it.  Without `svc.ready().await`, services that
    /// reserve a resource in `poll_ready` (e.g. `ConcurrencyLimit`) would have
    /// their readiness bypassed, potentially exceeding the concurrency limit.
    #[tokio::test]
    async fn fallback_chain_respects_inner_readiness() {
        #[derive(Clone)]
        struct CountingService {
            concurrent: Arc<AtomicUsize>,
            peak: Arc<AtomicUsize>,
        }

        impl Service<LlmRequest> for CountingService {
            type Response = LlmResponse;
            type Error = LiterLlmError;
            type Future = crate::client::BoxFuture<'static, Result<LlmResponse>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, _req: LlmRequest) -> Self::Future {
                let concurrent = Arc::clone(&self.concurrent);
                let peak = Arc::clone(&self.peak);
                Box::pin(async move {
                    let current = concurrent.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                    concurrent.fetch_sub(1, Ordering::SeqCst);
                    Ok(LlmResponse::Chat(crate::tower::tests_common::make_chat_response(
                        "gpt-4",
                    )))
                })
            }
        }

        let concurrent = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let inner = CountingService {
            concurrent: Arc::clone(&concurrent),
            peak: Arc::clone(&peak),
        };
        let limited = tower::limit::ConcurrencyLimit::new(inner, 1);
        let mut svc = FallbackChainService {
            chain: Arc::new(vec![limited]),
            policy: Arc::new(DefaultRetryPolicy),
        };

        for _ in 0..5 {
            svc.call(LlmRequest::Chat(chat_req("openai/gpt-4")))
                .await
                .expect("each call must succeed");
        }

        assert_eq!(
            peak.load(Ordering::SeqCst),
            1,
            "peak concurrent calls must be 1 (ConcurrencyLimit respected)"
        );

        let concurrent2 = Arc::new(AtomicUsize::new(0));
        let peak2 = Arc::new(AtomicUsize::new(0));
        let inner2 = CountingService {
            concurrent: Arc::clone(&concurrent2),
            peak: Arc::clone(&peak2),
        };
        let limited2 = tower::limit::ConcurrencyLimit::new(inner2, 1);
        let svc2 = FallbackChainService {
            chain: Arc::new(vec![limited2]),
            policy: Arc::new(DefaultRetryPolicy),
        };

        let handles: Vec<_> = (0..5)
            .map(|_| {
                let mut s = svc2.clone();
                tokio::spawn(async move { s.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await })
            })
            .collect();
        for h in handles {
            h.await.expect("task panicked").expect("call must succeed");
        }
        assert!(
            peak2.load(Ordering::SeqCst) <= 1,
            "peak concurrent calls must not exceed ConcurrencyLimit of 1, got {}",
            peak2.load(Ordering::SeqCst)
        );
    }
}
