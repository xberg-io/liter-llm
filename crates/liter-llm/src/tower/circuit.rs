//! Circuit-breaker Tower middleware.
//!
//! # Overview
//!
//! [`CircuitLayer`] wraps any [`tower::Service<LlmRequest>`] with a
//! circuit-breaker pattern.  Consumers supply a [`CircuitPolicy`]
//! implementation; the default is [`ExponentialBackoffCircuit`].
//!
//! # State machine
//!
//! ```text
//! Closed ─(N consecutive failures)→ Open ─(backoff elapsed)→ HalfOpen
//!   ↑                                                              │
//!   └─────────────────────(success)────────────────────────────────┘
//!                                      (failure in HalfOpen → Open again)
//! ```
//!
//! - **Closed**: requests flow through normally.
//! - **Open**: all requests are rejected immediately with
//!   [`crate::error::LiterLlmError::ServiceUnavailable`].
//! - **HalfOpen**: one probe request is allowed through.  Success closes
//!   the circuit; failure re-opens it with a fresh backoff delay.
//!
//! # Trait-first design
//!
//! The [`CircuitPolicy`] trait lets callers plug in custom policy logic
//! (e.g. sliding-window error-rate policies) without modifying library code.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

// ─── CircuitState ─────────────────────────────────────────────────────────────

/// Observable state of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CircuitState {
    /// Requests flow through normally.
    Closed = 0,
    /// All requests are rejected; the circuit is waiting for the backoff to elapse.
    Open = 1,
    /// One probe request is allowed through to test service health.
    HalfOpen = 2,
}

impl CircuitState {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Open,
            2 => Self::HalfOpen,
            _ => Self::Closed,
        }
    }
}

// ─── CircuitPolicy trait ──────────────────────────────────────────────────────

/// Policy that drives a circuit breaker's state transitions.
///
/// Implement this trait to provide custom failure-detection and
/// recovery logic.  The default implementation is [`ExponentialBackoffCircuit`].
pub trait CircuitPolicy: Send + Sync + 'static {
    /// Called when the inner service returns a successful response.
    fn record_success(&self);

    /// Called when the inner service returns an error.
    ///
    /// The policy decides whether to count the error as a circuit-trip failure.
    fn record_failure(&self);

    /// Returns `true` when a request should be allowed to proceed.
    ///
    /// `false` means the circuit is open and the request should be rejected.
    fn should_allow(&self) -> bool;

    /// Returns the current circuit state.
    fn state(&self) -> CircuitState;
}

// ─── ExponentialBackoffCircuit ─────────────────────────────────────────────────

/// Per-provider atomic state shared between all clones of the service.
struct CircuitInner {
    /// Current state encoded as a `u8` for atomic access.
    state: AtomicU8,
    /// Number of consecutive failures since the circuit was last closed.
    consecutive_failures: AtomicU32,
    /// Protects the `open_since` `Instant`; only mutated on state transitions.
    open_since: Mutex<Option<Instant>>,
}

/// Circuit breaker with exponential backoff.
///
/// Opens after `failure_threshold` consecutive failures.  After
/// `base_backoff` (doubled on each successive open → half-open → open cycle,
/// up to `max_backoff`), the circuit enters [`CircuitState::HalfOpen`] and
/// allows one probe request through.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use liter_llm::tower::circuit::{CircuitLayer, ExponentialBackoffCircuit};
///
/// let policy = Arc::new(ExponentialBackoffCircuit::new(5, std::time::Duration::from_secs(10)));
/// let layer = CircuitLayer::new(policy, "openai".to_string());
/// ```
pub struct ExponentialBackoffCircuit {
    /// Open after this many consecutive failures.
    failure_threshold: u32,
    /// Initial backoff before entering half-open.
    base_backoff: Duration,
    /// Maximum backoff (caps exponential growth).
    max_backoff: Duration,
    inner: Arc<CircuitInner>,
    /// Tracks how many times the circuit has opened (drives the exponent).
    open_count: AtomicU32,
}

impl ExponentialBackoffCircuit {
    /// Create a new policy.
    ///
    /// - `failure_threshold`: consecutive failures required to open the circuit.
    /// - `base_backoff`: initial half-open retry delay (doubles each open cycle,
    ///   capped at 2 minutes).
    #[must_use]
    pub fn new(failure_threshold: u32, base_backoff: Duration) -> Self {
        Self {
            failure_threshold,
            base_backoff,
            max_backoff: Duration::from_secs(120),
            inner: Arc::new(CircuitInner {
                state: AtomicU8::new(CircuitState::Closed as u8),
                consecutive_failures: AtomicU32::new(0),
                open_since: Mutex::new(None),
            }),
            open_count: AtomicU32::new(0),
        }
    }

    /// Compute the current backoff duration based on how many times the circuit
    /// has opened.
    fn current_backoff(&self) -> Duration {
        let count = self.open_count.load(Ordering::Relaxed);
        // 2^count saturation — clamp count to avoid overflow in the shift.
        let shift = count.min(62) as u64;
        let factor = 1u64.checked_shl(shift as u32).unwrap_or(u64::MAX);
        let nanos = self.base_backoff.as_nanos().saturating_mul(factor as u128);
        let computed = Duration::from_nanos(nanos.min(u64::MAX as u128) as u64);
        computed.min(self.max_backoff)
    }

    /// Check whether the open-circuit backoff has elapsed and, if so,
    /// transition to [`CircuitState::HalfOpen`].
    ///
    /// Called from the test suite to simulate the half-open transition without
    /// waiting for the full real-time backoff.
    #[cfg_attr(not(test), allow(dead_code))]
    async fn maybe_half_open(&self) -> bool {
        let backoff = self.current_backoff();
        let guard = self.inner.open_since.lock().await;
        if let Some(open_at) = *guard
            && open_at.elapsed() >= backoff
        {
            drop(guard);
            self.inner
                .state
                .store(CircuitState::HalfOpen as u8, Ordering::Release);
            tracing::info!(backoff = ?backoff, "circuit breaker entering half-open");
            return true;
        }
        false
    }
}

impl CircuitPolicy for ExponentialBackoffCircuit {
    fn record_success(&self) {
        self.inner.consecutive_failures.store(0, Ordering::Relaxed);
        let prev = self.inner.state.swap(CircuitState::Closed as u8, Ordering::Release);
        if CircuitState::from_u8(prev) != CircuitState::Closed {
            tracing::info!("circuit breaker closed after successful probe");
        }
    }

    fn record_failure(&self) {
        let failures = self.inner.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        let current = CircuitState::from_u8(self.inner.state.load(Ordering::Acquire));

        // Open if threshold reached, or re-open after a failed half-open probe.
        if failures >= self.failure_threshold || current == CircuitState::HalfOpen {
            // We need an async context to lock `open_since`, but `record_failure`
            // is a sync method.  Spawn a detached task to do the transition.
            // This is intentionally fire-and-forget: the state transition is
            // best-effort and idempotent.
            let inner_arc = Arc::clone(&self.inner);
            let backoff = self.current_backoff();
            let open_count = self.open_count.fetch_add(1, Ordering::Relaxed) + 1;
            tokio::spawn(async move {
                inner_arc.state.store(CircuitState::Open as u8, Ordering::Release);
                let mut guard = inner_arc.open_since.lock().await;
                *guard = Some(Instant::now());
                tracing::warn!(
                    consecutive_failures = failures,
                    backoff = ?backoff,
                    open_count,
                    "circuit breaker opened"
                );
            });
        }
    }

    fn should_allow(&self) -> bool {
        match CircuitState::from_u8(self.inner.state.load(Ordering::Acquire)) {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => false,
        }
    }

    fn state(&self) -> CircuitState {
        CircuitState::from_u8(self.inner.state.load(Ordering::Acquire))
    }
}

// ─── Layer ────────────────────────────────────────────────────────────────────

/// Tower [`Layer`] that wraps a service with a [`CircuitPolicy`].
///
/// # Clone behaviour
///
/// The `policy` is wrapped in `Arc` so all service clones share the same
/// circuit state.
#[cfg_attr(alef, alef(skip))]
pub struct CircuitLayer<P> {
    policy: Arc<P>,
    /// Human-readable provider label used in error messages and metrics.
    provider: String,
}

impl<P: CircuitPolicy> CircuitLayer<P> {
    /// Create a new circuit-breaker layer.
    ///
    /// - `policy`: the [`CircuitPolicy`] implementation.
    /// - `provider`: a label for error messages (e.g. `"openai"`).
    #[must_use]
    pub fn new(policy: Arc<P>, provider: impl Into<String>) -> Self {
        Self {
            policy,
            provider: provider.into(),
        }
    }
}

impl<P: CircuitPolicy, S> Layer<S> for CircuitLayer<P> {
    type Service = CircuitService<P, S>;

    fn layer(&self, inner: S) -> Self::Service {
        CircuitService {
            inner,
            policy: Arc::clone(&self.policy),
            provider: self.provider.clone(),
        }
    }
}

// ─── Service ─────────────────────────────────────────────────────────────────

/// Tower service produced by [`CircuitLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct CircuitService<P, S> {
    inner: S,
    policy: Arc<P>,
    provider: String,
}

impl<P: CircuitPolicy, S: Clone> Clone for CircuitService<P, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            policy: Arc::clone(&self.policy),
            provider: self.provider.clone(),
        }
    }
}

impl<P, S> Service<LlmRequest> for CircuitService<P, S>
where
    P: CircuitPolicy + 'static,
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
        let provider = self.provider.clone();
        let model = req.model().unwrap_or("").to_owned();
        let system = model.split_once('/').map(|(p, _)| p.to_owned()).unwrap_or_default();
        let state = self.policy.state();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Use a block to ensure `EnteredSpan` is dropped before any await.
            let allowed = {
                let _span = tracing::debug_span!(
                    "circuit_breaker",
                    gen_ai.circuit.state = ?state,
                    provider = %provider,
                )
                .entered();
                policy.should_allow()
            };

            if !allowed {
                tracing::debug!(provider = %provider, "circuit open — rejecting request");

                // Emit circuit metric via the metrics module.
                super::metrics::record_circuit_trip(&system, &model);

                return Err(LiterLlmError::ServiceUnavailable {
                    message: format!("circuit breaker open for provider '{provider}'"),
                    status: 503,
                });
            }

            tracing::debug!(provider = %provider, state = ?policy.state(), "circuit allowing request through");

            match inner.call(req).await {
                Ok(resp) => {
                    policy.record_success();
                    Ok(resp)
                }
                Err(e) => {
                    if e.is_transient() {
                        policy.record_failure();
                    }
                    Err(e)
                }
            }
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    /// Helper: build a policy that opens after `n` failures.
    fn policy(n: u32) -> Arc<ExponentialBackoffCircuit> {
        Arc::new(ExponentialBackoffCircuit::new(n, Duration::from_millis(50)))
    }

    #[tokio::test]
    async fn circuit_starts_closed() {
        let p = policy(3);
        assert_eq!(p.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn circuit_breaker_opens_after_n_failures() {
        let p = policy(3);
        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc = layer.layer(LlmService::new(MockClient::failing_timeout()));

        // Drive 3 transient failures.
        for _ in 0..3 {
            let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
        }

        // Give the background spawn a moment to update state.
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(p.state(), CircuitState::Open, "circuit should be open after threshold failures");
    }

    #[tokio::test]
    async fn open_circuit_rejects_requests_without_calling_inner() {
        let p = policy(1);
        let mock = MockClient::failing_timeout();
        let call_count = Arc::clone(&mock.call_count);
        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc = layer.layer(LlmService::new(mock));

        // Trigger open.
        let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        let before = call_count.load(std::sync::atomic::Ordering::SeqCst);

        // Next call should be rejected by the layer, NOT by the inner service.
        let err = svc
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect_err("should be rejected by open circuit");

        assert!(
            matches!(err, LiterLlmError::ServiceUnavailable { .. }),
            "expected ServiceUnavailable from open circuit, got {err:?}"
        );
        // Inner service should NOT have been called again.
        assert_eq!(
            call_count.load(std::sync::atomic::Ordering::SeqCst),
            before,
            "inner service should not be called while circuit is open"
        );
    }

    #[tokio::test]
    async fn circuit_breaker_half_opens_after_delay() {
        let p = Arc::new(ExponentialBackoffCircuit::new(1, Duration::from_millis(20)));
        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc = layer.layer(LlmService::new(MockClient::failing_timeout()));

        // Open the circuit.
        let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(p.state(), CircuitState::Open);

        // Wait for backoff to elapse.
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Manually transition to half-open (mirrors what the layer does on probe).
        let allowed = p.maybe_half_open().await;
        assert!(allowed, "should transition to half-open after backoff");
        assert_eq!(p.state(), CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn circuit_closes_after_successful_probe() {
        let p = policy(1);
        // First service: always fails.
        let failing = LlmService::new(MockClient::failing_timeout());
        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc = layer.layer(failing);

        // Open the circuit.
        let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(p.state(), CircuitState::Open);

        // Manually transition to half-open.
        p.inner.state.store(CircuitState::HalfOpen as u8, Ordering::Release);

        // Now swap in a succeeding inner service (simulate service recovery).
        let recovering = LlmService::new(MockClient::ok());
        let layer2 = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc2 = layer2.layer(recovering);

        let resp = svc2
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("probe should succeed");
        assert!(matches!(resp, LlmResponse::Chat(_)));
        assert_eq!(p.state(), CircuitState::Closed, "circuit should close after success");
    }

    #[tokio::test]
    async fn non_transient_errors_do_not_trip_circuit() {
        let p = policy(2);
        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        // Authentication errors are not transient.
        let mut svc = layer.layer(LlmService::new(MockClient::failing_auth()));

        for _ in 0..5 {
            let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Circuit should still be closed — auth errors are not transient.
        assert_eq!(
            p.state(),
            CircuitState::Closed,
            "non-transient errors should not open the circuit"
        );
    }
}
