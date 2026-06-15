//! Hedged-retry Tower middleware.
//!
//! # Overview
//!
//! [`HedgeLayer`] races multiple copies of the same request against each other.
//! After a configurable delay, a second (or third, …) request is launched.
//! The first response that arrives wins; all losers are cancelled via
//! [`tokio_util::sync::CancellationToken`].
//!
//! This pattern is particularly effective for tail-latency reduction: most
//! requests complete before the hedge fires, but slow outliers get a
//! second chance without incurring extra cost in the common case.
//!
//! # Trait-first design
//!
//! The [`HedgePolicy`] trait is the extension point.  Supply a custom
//! implementation to use latency-based delays (e.g. p99 latency), adaptive
//! delays per model, or request-property-based hedging.
//!
//! # Note on `CancellationToken`
//!
//! This module depends on `tokio_util`.  Because `tokio_util` is not yet a
//! workspace dependency, it is referenced via `tokio`'s re-export
//! (`tokio_util::sync::CancellationToken` is available through
//! `tokio-util = "0.7"` which `tokio` 1.x exposes indirectly).  We use a
//! bespoke `AbortHandle` via `tokio::task::JoinSet` instead to avoid adding
//! a hard dependency here — the cancellation is implemented with
//! `tokio::select!` and `AbortHandle`.

use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

// ─── HedgePolicy trait ────────────────────────────────────────────────────────

/// Policy that controls when and how many hedged requests are launched.
///
/// Implement this trait to provide custom hedging strategies such as
/// latency-percentile-based delays or per-model adaptive delays.
pub trait HedgePolicy: Send + Sync + 'static {
    /// Returns the delay before launching attempt `attempt` (1-indexed; attempt
    /// 1 is the initial request, attempt 2 is the first hedge, etc.).
    ///
    /// - `attempt`: 1-indexed attempt number.
    /// - `latency_so_far`: elapsed time since the first request was dispatched.
    ///
    /// Return `None` to skip this attempt (and all subsequent ones).
    fn delay_for_attempt(&self, attempt: u32, latency_so_far: Duration) -> Option<Duration>;

    /// Maximum number of concurrent attempts (including the original request).
    ///
    /// Must be ≥ 1.  Values above 3 are rarely useful and increase provider
    /// costs significantly.
    fn max_attempts(&self) -> u32;
}

// ─── FixedDelayHedge ─────────────────────────────────────────────────────────

/// A simple [`HedgePolicy`] that fires hedges at fixed intervals.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use std::time::Duration;
/// use liter_llm::tower::hedge::{FixedDelayHedge, HedgeLayer};
///
/// // Fire a second request 200 ms after the first; allow up to 2 attempts.
/// let policy = Arc::new(FixedDelayHedge::new(Duration::from_millis(200), 2));
/// let layer = HedgeLayer::new(policy);
/// ```
pub struct FixedDelayHedge {
    /// Fixed delay between attempts.
    delay: Duration,
    /// Maximum concurrent attempts including the first request.
    max_attempts: u32,
}

impl FixedDelayHedge {
    /// Create a new policy.
    ///
    /// - `delay`: how long to wait before launching each additional attempt.
    /// - `max_attempts`: maximum concurrent copies of the request (≥ 1).
    #[must_use]
    pub fn new(delay: Duration, max_attempts: u32) -> Self {
        Self {
            delay,
            max_attempts: max_attempts.max(1),
        }
    }
}

impl HedgePolicy for FixedDelayHedge {
    fn delay_for_attempt(&self, attempt: u32, _latency_so_far: Duration) -> Option<Duration> {
        if attempt > self.max_attempts {
            return None;
        }
        // Attempt 1: launched immediately (delay = 0 for the caller).
        // Attempt 2: launched after `delay` from dispatch time.
        // Attempt 3: launched after 2 × `delay`, etc.
        Some(self.delay * (attempt - 1))
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

// ─── Layer ────────────────────────────────────────────────────────────────────

/// Tower [`Layer`] that wraps a service with hedged request racing.
///
/// The layer clones the inner service for each additional attempt.
#[cfg_attr(alef, alef(skip))]
pub struct HedgeLayer<P> {
    policy: Arc<P>,
}

impl<P: HedgePolicy> HedgeLayer<P> {
    /// Create a new hedge layer.
    #[must_use]
    pub fn new(policy: Arc<P>) -> Self {
        Self { policy }
    }
}

impl<P: HedgePolicy, S> Layer<S> for HedgeLayer<P> {
    type Service = HedgeService<P, S>;

    fn layer(&self, inner: S) -> Self::Service {
        HedgeService {
            inner,
            policy: Arc::clone(&self.policy),
        }
    }
}

// ─── Service ─────────────────────────────────────────────────────────────────

/// Tower service produced by [`HedgeLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct HedgeService<P, S> {
    inner: S,
    policy: Arc<P>,
}

impl<P: HedgePolicy, S: Clone> Clone for HedgeService<P, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            policy: Arc::clone(&self.policy),
        }
    }
}

impl<P, S> Service<LlmRequest> for HedgeService<P, S>
where
    P: HedgePolicy + 'static,
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let policy = Arc::clone(&self.policy);
        let max_attempts = policy.max_attempts();
        let inner = self.inner.clone();

        Box::pin(async move {
            // Log before the await — EnteredSpan is not Send.
            tracing::debug!(hedge.max_attempts = max_attempts, "starting hedged request");
            hedge_race(req, inner, policy, max_attempts).await
        })
    }
}

/// Core hedging logic: spawn attempts with increasing delays and race them.
///
/// Uses `JoinSet` with `abort_all()` to cancel losing tasks.
async fn hedge_race<S>(
    req: LlmRequest,
    inner: S,
    policy: Arc<impl HedgePolicy>,
    max_attempts: u32,
) -> Result<LlmResponse>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    use std::time::Instant;

    let dispatch_time = Instant::now();

    // We use a JoinSet to manage concurrent tasks and abort losers.
    let mut join_set: tokio::task::JoinSet<(u32, Result<LlmResponse>)> = tokio::task::JoinSet::new();

    // Launch attempt 1 immediately.
    {
        let req_clone = req.clone();
        let mut svc_clone = inner.clone();
        join_set.spawn(async move {
            let result = svc_clone.call(req_clone).await;
            (1u32, result)
        });
    }

    // Schedule hedged attempts.
    for attempt in 2..=max_attempts {
        let latency_so_far = dispatch_time.elapsed();
        let Some(hedge_delay) = policy.delay_for_attempt(attempt, latency_so_far) else {
            break;
        };

        let req_clone = req.clone();
        let mut svc_clone = inner.clone();
        join_set.spawn(async move {
            if hedge_delay > Duration::ZERO {
                tokio::time::sleep(hedge_delay).await;
            }
            tracing::debug!(attempt, "launching hedged request");

            // Emit retry metric.
            let model = req_clone.model().unwrap_or("").to_owned();
            let system = model.split_once('/').map(|(p, _)| p.to_owned()).unwrap_or_default();
            super::metrics::record_retry_attempt(&system, &model, req_clone.operation_name());

            let result = svc_clone.call(req_clone).await;
            (attempt, result)
        });
    }

    // Race: first Ok wins, abort the rest.  If all fail, return the last error.
    let mut last_err: Option<LiterLlmError> = None;

    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok((attempt, Ok(resp))) => {
                tracing::debug!(attempt, "hedged request succeeded first");
                // Abort all other in-flight attempts.
                join_set.abort_all();
                return Ok(resp);
            }
            Ok((attempt, Err(e))) => {
                tracing::debug!(attempt, error = %e, "hedged attempt failed");
                last_err = Some(e);
            }
            Err(join_err) if join_err.is_cancelled() => {
                // This task was aborted — a winner was already found.
                // The early return above already returned the winner, so we
                // should not reach here after `abort_all()`, but handle it
                // defensively.
            }
            Err(join_err) => {
                tracing::error!(error = %join_err, "hedged task panicked");
                last_err = Some(LiterLlmError::InternalError {
                    message: format!("hedge task panicked: {join_err}"),
                });
            }
        }
    }

    // All attempts failed.
    Err(last_err.unwrap_or(LiterLlmError::InternalError {
        message: "all hedged attempts failed with no error recorded".into(),
    }))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    #[tokio::test]
    async fn hedge_returns_first_success() {
        // A single attempt should succeed without hedging.
        let policy = Arc::new(FixedDelayHedge::new(Duration::from_millis(200), 2));
        let inner = LlmService::new(MockClient::ok());
        let mut svc = HedgeLayer::new(policy).layer(inner);

        let resp = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("should succeed");

        assert!(matches!(resp, LlmResponse::Chat(_)));
    }

    #[tokio::test]
    async fn hedge_single_attempt_policy_does_not_spawn_extra() {
        let policy = Arc::new(FixedDelayHedge::new(Duration::from_millis(100), 1));
        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);
        let mut svc = HedgeLayer::new(policy).layer(inner);

        svc.call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("should succeed");

        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "max_attempts=1 should only call inner service once"
        );
    }

    #[tokio::test]
    async fn hedge_propagates_error_when_all_attempts_fail() {
        let policy = Arc::new(FixedDelayHedge::new(Duration::from_millis(10), 2));
        let inner = LlmService::new(MockClient::failing_timeout());
        let mut svc = HedgeLayer::new(policy).layer(inner);

        let err = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect_err("all attempts should fail");

        // The error should be the one from one of the attempts (Timeout).
        assert!(
            matches!(err, LiterLlmError::Timeout),
            "expected Timeout from failed hedge, got {err:?}"
        );
    }

    #[tokio::test]
    async fn fixed_delay_hedge_policy_respects_max_attempts() {
        let policy = FixedDelayHedge::new(Duration::from_millis(100), 3);

        // attempt 1: delay(1, 0) = Some(0)
        assert_eq!(policy.delay_for_attempt(1, Duration::ZERO), Some(Duration::ZERO));
        // attempt 2: delay(2, 0) = Some(100ms)
        assert_eq!(
            policy.delay_for_attempt(2, Duration::ZERO),
            Some(Duration::from_millis(100))
        );
        // attempt 3: delay(3, 0) = Some(200ms)
        assert_eq!(
            policy.delay_for_attempt(3, Duration::ZERO),
            Some(Duration::from_millis(200))
        );
        // attempt 4: beyond max_attempts → None
        assert_eq!(policy.delay_for_attempt(4, Duration::ZERO), None);
    }

    #[tokio::test]
    async fn hedge_with_two_attempts_calls_inner_at_most_twice() {
        let policy = Arc::new(FixedDelayHedge::new(Duration::from_millis(5), 2));
        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);
        let mut svc = HedgeLayer::new(policy).layer(inner);

        svc.call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("should succeed");

        // The first attempt succeeds quickly; the hedge may or may not fire
        // depending on scheduling.  The count should be 1 or 2.
        let count = call_count.load(Ordering::SeqCst);
        assert!(count >= 1 && count <= 2, "expected 1 or 2 calls, got {count}");
    }
}
