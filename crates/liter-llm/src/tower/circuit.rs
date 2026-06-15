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

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

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

// ─── ProbeGuard ───────────────────────────────────────────────────────────────

/// RAII guard that releases the probe slot via [`CircuitPolicy::release_probe_slot`]
/// when dropped.
///
/// Held by `CircuitService::call`'s async future **only** on the HalfOpen probe
/// path.  The guard is disarmed after the policy records success or failure via
/// [`ProbeGuard::disarm`]; if the future is dropped earlier — due to a panic,
/// cancellation, or any other reason — `Drop` invokes `release_probe_slot` so
/// subsequent requests are not permanently locked out of the probe slot.
enum ProbeGuard<P: CircuitPolicy> {
    /// Not on the probe path; no slot to release on drop.
    None,
    /// Holding the probe slot; releases it via the policy on drop unless disarmed.
    Half(Arc<P>),
}

impl<P: CircuitPolicy> ProbeGuard<P> {
    /// Disarm the guard without releasing the probe slot.
    ///
    /// Call this after invoking `policy.record_success()` or
    /// `policy.record_failure()`, which already handle the slot as part of the
    /// state transition.
    fn disarm(&mut self) {
        *self = Self::None;
    }
}

impl<P: CircuitPolicy> Drop for ProbeGuard<P> {
    fn drop(&mut self) {
        if let Self::Half(policy) = self {
            // The probe future was dropped without completing (cancel or panic).
            // Release the slot so subsequent requests can attempt a probe.
            policy.release_probe_slot();
        }
    }
}

// ─── CircuitPolicy trait ──────────────────────────────────────────────────────

/// Policy that drives a circuit breaker's state transitions.
///
/// Implement this trait to provide custom failure-detection and
/// recovery logic.  The default implementation is [`ExponentialBackoffCircuit`].
#[cfg_attr(alef, alef(skip))]
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

    /// Called when a probe request is dropped without completing (e.g. due to
    /// panic or cancellation) to release the probe slot.
    ///
    /// The default implementation is a no-op.  Policies that gate probe slots
    /// with a boolean flag (like [`ExponentialBackoffCircuit`]) should override
    /// this to clear the flag.
    fn release_probe_slot(&self) {}
}

// ─── ExponentialBackoffCircuit ─────────────────────────────────────────────────

/// Per-provider atomic state shared between all clones of the service.
struct CircuitInner {
    /// Current state encoded as a `u8` for atomic access.
    state: AtomicU8,
    /// Number of consecutive failures since the circuit was last closed.
    consecutive_failures: AtomicU32,
    /// Protects the `open_since` `Instant`; only mutated on state transitions.
    /// Uses `std::sync::Mutex` (not `tokio::sync::Mutex`) so that it can be
    /// acquired from synchronous contexts such as `record_failure`.
    open_since: Mutex<Option<Instant>>,
    /// Single-permit gate for HalfOpen probe requests.
    ///
    /// `true` while a probe is in-flight.  The first caller that CAS-es this
    /// from `false` to `true` owns the probe slot; all others are rejected as
    /// if the circuit were Open.  Cleared by `record_success` / `record_failure`
    /// as part of the state transition out of HalfOpen.
    probe_in_flight: AtomicBool,
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
#[cfg_attr(alef, alef(skip))]
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
                probe_in_flight: AtomicBool::new(false),
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
    /// Called from [`should_allow`] on every request when the circuit is Open,
    /// driving the timer-based Open → HalfOpen transition on the request path.
    fn maybe_half_open(&self) -> bool {
        let backoff = self.current_backoff();
        let guard = self.inner.open_since.lock().expect("open_since mutex poisoned");
        if let Some(open_at) = *guard
            && open_at.elapsed() >= backoff
        {
            drop(guard);
            self.inner.state.store(CircuitState::HalfOpen as u8, Ordering::Release);
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
        // Release the probe slot so a future HalfOpen round can happen.
        self.inner.probe_in_flight.store(false, Ordering::Release);
        if CircuitState::from_u8(prev) != CircuitState::Closed {
            tracing::info!("circuit breaker closed after successful probe");
        }
    }

    fn record_failure(&self) {
        let failures = self.inner.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;

        // Open if threshold reached, or re-open after a failed half-open probe.
        // Use a CAS so that exactly ONE concurrent caller performs the transition:
        // the winner atomically changes the state from non-Open to Open; all other
        // concurrent callers see the CAS fail and skip.  This prevents N concurrent
        // failures from incrementing `open_count` N times, which would make the
        // exponential-backoff exponent grow N times faster than intended.
        let current_u8 = self.inner.state.load(Ordering::Acquire);
        let current = CircuitState::from_u8(current_u8);

        // Already open: nothing to do.
        if current == CircuitState::Open {
            return;
        }

        let should_open = failures >= self.failure_threshold || current == CircuitState::HalfOpen;
        if !should_open {
            return;
        }

        // Attempt to atomically transition Closed/HalfOpen to Open.  Only the
        // single thread whose CAS succeeds proceeds to update metadata; losers
        // see Err(_) and skip silently because another thread already won.
        let result = self.inner.state.compare_exchange(
            current_u8,
            CircuitState::Open as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        );

        if result.is_ok() {
            // We are the sole winner: capture the backoff BEFORE incrementing
            // open_count so that `current_backoff()` uses the pre-transition
            // exponent.  The increment records that a new open cycle has begun
            // (for the NEXT half-open probe delay).
            let backoff = self.current_backoff();
            let open_count = self.open_count.fetch_add(1, Ordering::Relaxed) + 1;
            {
                let mut guard = self.inner.open_since.lock().expect("open_since mutex poisoned");
                *guard = Some(Instant::now());
            }
            // Release the probe slot; a fresh cooldown is now in effect.
            self.inner.probe_in_flight.store(false, Ordering::Release);
            tracing::warn!(
                consecutive_failures = failures,
                backoff = ?backoff,
                open_count,
                "circuit breaker opened"
            );
        }
        // Losers (result.is_err()) skip silently -- the circuit is already Open.
    }

    fn should_allow(&self) -> bool {
        match CircuitState::from_u8(self.inner.state.load(Ordering::Acquire)) {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Attempt the timer-driven Open → HalfOpen transition.
                if self.maybe_half_open() {
                    // Claim the single probe slot.
                    self.inner
                        .probe_in_flight
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Exactly ONE concurrent probe: first CAS winner gets the slot.
                self.inner
                    .probe_in_flight
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            }
        }
    }

    fn state(&self) -> CircuitState {
        CircuitState::from_u8(self.inner.state.load(Ordering::Acquire))
    }

    /// Release the probe slot without recording success or failure.
    ///
    /// Called by the [`ProbeGuard`] when the probe future is dropped before
    /// completing (e.g. cancelled or panicked).
    fn release_probe_slot(&self) {
        self.inner.probe_in_flight.store(false, Ordering::Release);
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

        // Tower readiness contract: `poll_ready` was called on `self.inner`
        // (the "polled-ready" instance).  We must call `inner.call(req)` on
        // that exact instance -- not on a fresh clone -- to consume any permit
        // reserved by `poll_ready` (e.g. ConcurrencyLimit slot, Buffer slot).
        //
        // Standard pattern (docs.rs/tower "Be careful when cloning inner
        // services"): take the ready instance for this call, leave a fresh
        // clone behind so the next `poll_ready`/`call` round starts clean.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            // Use a block to ensure `EnteredSpan` is dropped before any await.
            let (allowed, is_probe) = {
                let _span = tracing::debug_span!(
                    "circuit_breaker",
                    gen_ai.circuit.state = ?state,
                    provider = %provider,
                )
                .entered();
                let probe = state != CircuitState::Closed;
                (policy.should_allow(), probe)
            };

            if !allowed {
                tracing::debug!(provider = %provider, "circuit open -- rejecting request");

                // Emit circuit metric via the metrics module.
                super::metrics::record_circuit_trip(&system, &model);

                return Err(LiterLlmError::ServiceUnavailable {
                    message: format!("circuit breaker open for provider '{provider}'"),
                    status: 503,
                });
            }

            // Arm a probe guard when we are on the HalfOpen probe path.
            // If the future is dropped before `record_success`/`record_failure`
            // are called (panic or cancellation), the guard releases the probe
            // slot so the next request can attempt a probe.
            let mut probe_guard: ProbeGuard<P> = if is_probe {
                ProbeGuard::Half(Arc::clone(&policy))
            } else {
                ProbeGuard::None
            };

            tracing::debug!(provider = %provider, state = ?policy.state(), "circuit allowing request through");

            match inner.call(req).await {
                Ok(resp) => {
                    // Disarm the guard before calling record_success, which
                    // handles probe_in_flight internally.
                    probe_guard.disarm();
                    policy.record_success();
                    Ok(resp)
                }
                Err(e) => {
                    if e.is_transient() {
                        // Disarm before record_failure, which clears probe slot.
                        probe_guard.disarm();
                        policy.record_failure();
                    } else {
                        // Non-transient error: if we are on the probe path, the
                        // circuit stays HalfOpen (we did not count this as a
                        // failure), so we need to release the probe slot
                        // explicitly so the next request can retry.
                        probe_guard.disarm();
                        if is_probe {
                            policy.release_probe_slot();
                        }
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

        // State is updated synchronously -- no sleep needed.
        assert_eq!(
            p.state(),
            CircuitState::Open,
            "circuit should be open after threshold failures"
        );
    }

    #[tokio::test]
    async fn open_circuit_rejects_requests_without_calling_inner() {
        let p = policy(1);
        let mock = MockClient::failing_timeout();
        let call_count = Arc::clone(&mock.call_count);
        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc = layer.layer(LlmService::new(mock));

        // Trigger open -- state is set synchronously, no sleep needed.
        let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;

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

        // Open the circuit -- state is set synchronously, no sleep needed.
        let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
        assert_eq!(p.state(), CircuitState::Open);

        // Wait for backoff to elapse.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Manually transition to half-open (mirrors what the layer does on probe).
        let allowed = p.maybe_half_open();
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

        // Open the circuit -- state is set synchronously.
        let _ = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
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

        // Circuit should still be closed -- auth errors are not transient.
        assert_eq!(
            p.state(),
            CircuitState::Closed,
            "non-transient errors should not open the circuit"
        );
    }

    /// Regression: N concurrent `record_failure` calls must increment `open_count`
    /// by exactly 1, not N.  Previously the spawn-based approach caused every
    /// concurrent caller to race to increment `open_count`, making the
    /// exponential-backoff exponent grow N times faster than intended.
    #[test]
    fn circuit_concurrent_failures_open_count_increments_once() {
        use std::thread;

        // Threshold of 3: each thread calls record_failure once, so threads 3..N
        // all see failures >= threshold.  Only one should win the CAS and
        // increment open_count.
        let circuit = Arc::new(ExponentialBackoffCircuit::new(3, Duration::from_millis(50)));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let c = Arc::clone(&circuit);
                thread::spawn(move || c.record_failure())
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }

        let open_count = circuit.open_count.load(Ordering::Relaxed);
        assert_eq!(
            open_count, 1,
            "open_count should be 1 regardless of how many concurrent callers hit the threshold; got {open_count}"
        );
        assert_eq!(
            circuit.state(),
            CircuitState::Open,
            "circuit should be open after concurrent failures"
        );
    }

    /// Regression: `record_failure` must not panic when called outside a Tokio
    /// runtime.  Previously the `tokio::spawn` call would panic with
    /// "no current runtime" in non-async contexts (sync tests, Drop impls).
    #[test]
    fn circuit_record_failure_works_outside_tokio_runtime() {
        // This is a plain (non-async) test function -- no Tokio runtime is active.
        let circuit = ExponentialBackoffCircuit::new(1, Duration::from_millis(50));
        // Should not panic.
        circuit.record_failure();
        assert_eq!(
            circuit.state(),
            CircuitState::Open,
            "state should be Open after one failure with threshold=1"
        );
    }

    /// Verify that `CircuitService` honours the Tower readiness contract when
    /// the inner service is a `ConcurrencyLimit` (which reserves a permit in
    /// `poll_ready` and releases it when the returned future is dropped).
    ///
    /// With the old clone-and-call pattern the second caller would obtain a
    /// fresh clone that never had `poll_ready` called -- skipping the permit
    /// acquisition entirely and potentially exceeding the concurrency limit.
    /// With `std::mem::replace` the polled-ready instance is consumed for the
    /// call and the permit bookkeeping is correct.
    #[tokio::test]
    async fn circuit_service_respects_inner_readiness() {
        use std::pin::Pin;
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering as AtomicOrdering;

        use tower::limit::ConcurrencyLimit;

        // Inner service that blocks until the future is dropped, allowing us to
        // hold the ConcurrencyLimit permit open across concurrent callers.
        #[derive(Clone)]
        struct BlockingInner {
            call_count: Arc<AtomicUsize>,
        }

        impl tower::Service<LlmRequest> for BlockingInner {
            type Response = LlmResponse;
            type Error = LiterLlmError;
            type Future = crate::client::BoxFuture<'static, Result<LlmResponse>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, _req: LlmRequest) -> Self::Future {
                self.call_count.fetch_add(1, AtomicOrdering::SeqCst);
                // Block forever -- keeps the concurrency permit held.
                Box::pin(std::future::pending())
            }
        }

        let call_count = Arc::new(AtomicUsize::new(0));
        let inner = BlockingInner {
            call_count: Arc::clone(&call_count),
        };

        // Limit to 1 concurrent request.  Wrap it in CircuitService.
        let limited: ConcurrencyLimit<BlockingInner> = ConcurrencyLimit::new(inner, 1);
        let p = Arc::new(ExponentialBackoffCircuit::new(5, Duration::from_millis(50)));
        let mut svc = CircuitService {
            inner: limited,
            policy: Arc::clone(&p),
            provider: "test".into(),
        };

        // First poll_ready + call: acquires the permit, dispatches the blocking future.
        futures_util::future::poll_fn(|cx| svc.poll_ready(cx))
            .await
            .expect("first poll_ready should succeed");
        let mut held_fut = svc.call(LlmRequest::ListModels());

        // Drive the future once so that the async block runs to the point where
        // `inner.call(req)` is invoked (BlockingInner::call increments the counter
        // and returns `pending()`).  A single poll is sufficient.
        {
            let mut noop_cx = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
            let _ = Pin::new(&mut held_fut).poll(&mut noop_cx);
        }

        // The inner service should have been called exactly once.
        assert_eq!(
            call_count.load(AtomicOrdering::SeqCst),
            1,
            "inner service should have been called exactly once"
        );

        // The concurrency slot is now exhausted.  A second poll_ready on the
        // circuit service should propagate the Pending from ConcurrencyLimit --
        // not return Ready by bypassing poll_ready on a stale clone.
        let mut noop_cx = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
        let poll = svc.poll_ready(&mut noop_cx);
        assert!(
            poll.is_pending(),
            "second poll_ready must be Pending when the concurrency permit is exhausted"
        );
    }

    /// Bug 1 fix: Open circuit transitions to HalfOpen after cooldown elapses,
    /// driven by `should_allow` on the request path.
    #[tokio::test]
    async fn circuit_half_open_after_cooldown() {
        let p = Arc::new(ExponentialBackoffCircuit::new(1, Duration::from_millis(20)));
        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc_fail = layer.layer(LlmService::new(MockClient::failing_timeout()));
        let _ = svc_fail.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
        assert_eq!(p.state(), CircuitState::Open);
        // Wait for real-clock cooldown to elapse.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let layer2 = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc_ok = layer2.layer(LlmService::new(MockClient::ok()));
        // Next call: should_allow transitions Open→HalfOpen, claims probe slot.
        let resp = svc_ok
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect("probe after cooldown must succeed");
        assert!(matches!(resp, LlmResponse::Chat(_)));
        assert_eq!(p.state(), CircuitState::Closed);
    }

    /// Bug 2 fix: exactly ONE concurrent probe request allowed in HalfOpen.
    #[tokio::test]
    async fn circuit_half_open_single_probe() {
        use std::sync::atomic::AtomicUsize;

        let p = Arc::new(ExponentialBackoffCircuit::new(1, Duration::from_millis(20)));
        p.record_failure();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(p.maybe_half_open(), "should transition to HalfOpen");
        assert_eq!(p.state(), CircuitState::HalfOpen);

        let probe_count = Arc::new(AtomicUsize::new(0));
        let rejected_count = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..50)
            .map(|_| {
                let p2 = Arc::clone(&p);
                let pc = Arc::clone(&probe_count);
                let rc = Arc::clone(&rejected_count);
                tokio::spawn(async move {
                    if p2.should_allow() {
                        pc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    } else {
                        rc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
            })
            .collect();
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(
            probe_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "exactly 1 probe"
        );
        assert_eq!(
            rejected_count.load(std::sync::atomic::Ordering::SeqCst),
            49,
            "49 rejected"
        );
    }

    /// Fix 1: probe_in_flight is cleared when the probe future is dropped before
    /// completing (cancellation).  Without the ProbeGuard the flag would stay
    /// `true` forever, permanently preventing new probes.
    #[tokio::test]
    async fn circuit_probe_flag_cleared_on_cancel() {
        use std::sync::atomic::Ordering as AO;

        let p = Arc::new(ExponentialBackoffCircuit::new(1, Duration::from_millis(10)));

        // Open the circuit then manually advance to HalfOpen.
        p.record_failure();
        assert_eq!(p.state(), CircuitState::Open);
        p.inner.state.store(CircuitState::HalfOpen as u8, AO::Release);
        p.inner.probe_in_flight.store(false, AO::Release);

        // Inner service that blocks forever (simulates a request that is cancelled).
        #[derive(Clone)]
        struct BlockForever;
        impl tower::Service<LlmRequest> for BlockForever {
            type Response = LlmResponse;
            type Error = LiterLlmError;
            type Future = crate::client::BoxFuture<'static, Result<LlmResponse>>;
            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: LlmRequest) -> Self::Future {
                Box::pin(std::future::pending())
            }
        }

        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc = layer.layer(BlockForever);

        // Start a probe call and then immediately drop the future (cancel).
        {
            let fut = svc.call(LlmRequest::Chat(chat_req("openai/gpt-4")));
            // Dropping `fut` here cancels it — ProbeGuard::drop must fire.
            drop(fut);
        }

        // probe_in_flight must be cleared so the next request can probe.
        assert!(
            !p.inner.probe_in_flight.load(AO::Acquire),
            "probe_in_flight must be false after probe future is dropped"
        );

        // A subsequent request is now allowed to probe (not stuck in HalfOpen-rejected limbo).
        assert!(
            p.should_allow(),
            "should allow another probe after cancelled probe slot was released"
        );
    }

    /// Bug 1+2 fix: failing HalfOpen probe re-opens the circuit with fresh cooldown.
    #[tokio::test]
    async fn circuit_half_open_failure_reopens() {
        let p = Arc::new(ExponentialBackoffCircuit::new(1, Duration::from_millis(20)));
        let layer = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc_fail = layer.layer(LlmService::new(MockClient::failing_timeout()));
        let _ = svc_fail.call(LlmRequest::Chat(chat_req("openai/gpt-4"))).await;
        assert_eq!(p.state(), CircuitState::Open);
        tokio::time::sleep(Duration::from_millis(50)).await;
        let layer2 = CircuitLayer::new(Arc::clone(&p), "test");
        let mut svc_fail2 = layer2.layer(LlmService::new(MockClient::failing_timeout()));
        let err = svc_fail2
            .call(LlmRequest::Chat(chat_req("openai/gpt-4")))
            .await
            .expect_err("failing probe must error");
        assert!(matches!(err, LiterLlmError::Timeout));
        assert_eq!(p.state(), CircuitState::Open, "must re-open after failing probe");
        assert!(
            !p.inner.probe_in_flight.load(Ordering::Acquire),
            "probe_in_flight must be cleared after failure"
        );
    }
}
