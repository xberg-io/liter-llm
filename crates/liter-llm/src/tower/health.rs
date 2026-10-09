//! Health-check middleware — global and per-provider.
//!
//! # Overview
//!
//! This module provides two levels of health-checking:
//!
//! ## Global health gate
//!
//! [`HealthCheckLayer`] wraps a service and spawns a background task that
//! periodically probes the service by sending a [`LlmRequest::ListModels`]
//! request.  The gate shares the same [`HealthCheckConfig`] threshold
//! machinery as the per-provider checker below: it only opens after
//! `unhealthy_threshold` consecutive probe failures, and only closes again
//! after `healthy_threshold` consecutive successes, rather than flipping on a
//! single probe result. A probe that times out (per `HealthCheckConfig::timeout`)
//! counts as a failure. A probe that fails with
//! [`LiterLlmError::EndpointNotSupported`] is not counted at all — it means
//! the provider does not implement `ListModels`, not that it is down, and
//! ordinary requests through it may still succeed.
//!
//! While the gate is open, incoming requests are immediately rejected with
//! [`LiterLlmError::ServiceUnavailable`].
//!
//! ## Per-provider health-check (1.E addition)
//!
//! [`HealthChecker`] is a trait that abstracts the probe strategy.
//! [`HttpProbeHealthChecker`] implements it by issuing a GET request to a
//! provider-specific health-check URL (falling back to a HEAD on the base URL
//! when no explicit endpoint is configured).
//!
//! [`HealthCheckConfig`] carries per-provider thresholds so that a
//! flaky provider is only marked down after `unhealthy_threshold` consecutive
//! failures, and only recovered after `healthy_threshold` consecutive successes.
//! `HealthCheckConfig::timeout` bounds every individual probe call — regardless
//! of `HealthChecker` implementation — via `tokio::time::timeout`, so a stalled
//! checker cannot leave a dead provider marked healthy forever.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

/// The result of a single health probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// The probe succeeded; the upstream is reachable.
    Healthy,
    /// The probe failed; the upstream may be down.
    Unhealthy,
}

/// Per-provider health-check configuration.
///
/// Controls probe timing and the number of consecutive successes/failures
/// required to transition between healthy and unhealthy states.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct HealthCheckConfig {
    /// How often to run the probe.
    pub interval: Duration,
    /// Maximum time to wait for a probe response before marking it failed.
    pub timeout: Duration,
    /// Number of consecutive probe failures before marking the upstream down.
    pub unhealthy_threshold: u32,
    /// Number of consecutive probe successes before marking the upstream up.
    pub healthy_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            unhealthy_threshold: 3,
            healthy_threshold: 2,
        }
    }
}

/// Abstraction over a health probe strategy.
///
/// Implementors issue a lightweight probe against `upstream` (typically a
/// provider base URL or named identifier) and report [`HealthStatus`].
pub trait HealthChecker: Send + Sync + 'static {
    /// Probe `upstream` and return its current [`HealthStatus`].
    ///
    /// The parameter is taken by value (`String`) so that implementations can
    /// move it into the returned future without a clone, making the
    /// `'static + Send` bound on the future trivially satisfiable.
    fn check(
        &self,
        upstream: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthStatus> + Send + 'static>>;
}

/// A [`HealthChecker`] that probes an HTTP endpoint.
///
/// For each provider it first looks up a dedicated health-check URL.  If none
/// is configured, it falls back to a HEAD request on the base URL.
///
/// The implementation is intentionally simple: a successful HTTP response
/// (any 2xx or 3xx) is treated as [`HealthStatus::Healthy`]; timeouts,
/// connection errors, and 4xx/5xx responses are [`HealthStatus::Unhealthy`].
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
#[cfg(feature = "native-http")]
pub struct HttpProbeHealthChecker {
    /// HTTP client used for probes.  Shared across all probe tasks.
    ///
    /// This client intentionally carries no client-level timeout.
    /// `HealthCheckConfig::timeout` is the single source of truth for the
    /// probe timeout: callers (`run_provider_health_probe`) wrap every
    /// `check()` invocation in `tokio::time::timeout(config.timeout, ..)`, so
    /// baking a second timeout in here would create two independent,
    /// potentially disagreeing deadlines for the same probe.
    client: reqwest::Client,
    /// Per-provider health-check URL overrides.
    /// Key: provider base URL or name; Value: dedicated probe endpoint.
    probe_urls: std::collections::HashMap<String, String>,
}

#[cfg(feature = "native-http")]
impl HttpProbeHealthChecker {
    /// Create a new checker with optional URL overrides.
    ///
    /// `probe_urls`: maps provider base URL / name → dedicated health-check
    /// URL.  If a provider is not in this map, the prober issues a GET
    /// request on the upstream URL itself.
    ///
    /// The probe timeout is controlled entirely by the [`HealthCheckConfig::timeout`]
    /// passed to the probe loop, not by this constructor.
    pub fn new(probe_urls: impl IntoIterator<Item = (String, String)>) -> Result<Self> {
        let builder = crate::provider::configure_outbound_client_builder(reqwest::Client::builder(), None);
        let client = builder.build().map_err(|e| LiterLlmError::BadRequest {
            message: format!("failed to build HTTP client for health checker: {e}"),
            status: 500,
        })?;
        Ok(Self {
            client,
            probe_urls: probe_urls.into_iter().collect(),
        })
    }
}

#[cfg(feature = "native-http")]
impl HealthChecker for HttpProbeHealthChecker {
    fn check(
        &self,
        upstream: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthStatus> + Send + 'static>> {
        let url = self.probe_urls.get(&upstream).cloned().unwrap_or(upstream);
        let client = self.client.clone();

        Box::pin(async move {
            if let Err(error) = crate::provider::validate_outbound_url(&url).await {
                tracing::debug!(upstream = %url, error = %error, "health probe blocked by outbound policy");
                return HealthStatus::Unhealthy;
            }
            let result = client.get(&url).send().await;
            match result {
                Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => HealthStatus::Healthy,
                Ok(resp) => {
                    tracing::debug!(
                        upstream = %url,
                        status = resp.status().as_u16(),
                        "health probe returned non-success status"
                    );
                    HealthStatus::Unhealthy
                }
                Err(e) => {
                    tracing::debug!(
                        upstream = %url,
                        error = %e,
                        "health probe failed"
                    );
                    HealthStatus::Unhealthy
                }
            }
        })
    }
}

/// Shared state for a single provider's health probe.
///
/// Tracks consecutive success/failure counts and the current health flag using
/// atomics so the probe task and the request path can share it cheaply.
#[derive(Debug)]
struct ProviderHealthState {
    healthy: AtomicBool,
    consecutive_failures: AtomicU32,
    consecutive_successes: AtomicU32,
}

impl ProviderHealthState {
    fn new(initially_healthy: bool) -> Arc<Self> {
        Arc::new(Self {
            healthy: AtomicBool::new(initially_healthy),
            consecutive_failures: AtomicU32::new(0),
            consecutive_successes: AtomicU32::new(0),
        })
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Acquire)
    }

    /// Record a probe result and update the health flag according to `config`.
    fn record(&self, status: HealthStatus, config: &HealthCheckConfig) {
        match status {
            HealthStatus::Healthy => {
                self.consecutive_failures.store(0, Ordering::Release);
                let successes = self.consecutive_successes.fetch_add(1, Ordering::AcqRel) + 1;
                if successes >= config.healthy_threshold {
                    let was_unhealthy = !self.healthy.load(Ordering::Acquire);
                    self.healthy.store(true, Ordering::Release);
                    if was_unhealthy {
                        tracing::info!(
                            consecutive_successes = successes,
                            "health probe: upstream marked healthy"
                        );
                    }
                }
            }
            HealthStatus::Unhealthy => {
                self.consecutive_successes.store(0, Ordering::Release);
                let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
                if failures >= config.unhealthy_threshold {
                    let was_healthy = self.healthy.load(Ordering::Acquire);
                    self.healthy.store(false, Ordering::Release);
                    if was_healthy {
                        tracing::warn!(
                            consecutive_failures = failures,
                            "health probe: upstream marked unhealthy"
                        );
                    }
                }
            }
        }
    }
}

async fn run_provider_health_probe<C: HealthChecker>(
    checker: Arc<C>,
    upstream: String,
    state: Arc<ProviderHealthState>,
    config: HealthCheckConfig,
) {
    loop {
        tokio::time::sleep(config.interval).await;

        if Arc::strong_count(&state) <= 1 {
            break;
        }

        match tokio::time::timeout(config.timeout, checker.check(upstream.clone())).await {
            Ok(status) => state.record(status, &config),
            Err(_elapsed) => {
                tracing::debug!(
                    upstream = %upstream,
                    timeout = ?config.timeout,
                    "health probe timed out"
                );
                state.record(HealthStatus::Unhealthy, &config);
            }
        }
    }
}

/// A service wrapper that enforces per-provider health-check thresholds.
///
/// Compared to [`HealthCheckService`], this wrapper uses [`HealthCheckConfig`]
/// thresholds (consecutive failures/successes) rather than a single global
/// atomic flip, and plugs in any [`HealthChecker`] implementation.
#[cfg_attr(alef, alef(skip))]
pub struct PerProviderHealthCheck<S> {
    inner: S,
    state: Arc<ProviderHealthState>,
}

impl<S: Clone> Clone for PerProviderHealthCheck<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

impl<S> PerProviderHealthCheck<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    /// Wrap `inner` with per-provider health checks using `checker` and `config`.
    ///
    /// Spawns a background probe task immediately.
    pub fn new<C: HealthChecker>(inner: S, checker: Arc<C>, upstream: String, config: HealthCheckConfig) -> Self {
        let state = ProviderHealthState::new(true);
        let probe_state = Arc::clone(&state);
        let probe_checker = Arc::clone(&checker);

        tokio::spawn(async move {
            run_provider_health_probe(probe_checker, upstream, probe_state, config).await;
        });

        Self { inner, state }
    }

    /// Returns `true` if this provider is currently considered healthy.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.state.is_healthy()
    }
}

impl<S> Service<LlmRequest> for PerProviderHealthCheck<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if !self.state.is_healthy() {
            return Poll::Ready(Err(LiterLlmError::ServiceUnavailable {
                message: "provider is unhealthy (health check failed)".into(),
                status: 503,
                retry_after: None,
            }));
        }
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        if !self.state.is_healthy() {
            return Box::pin(async {
                Err(LiterLlmError::ServiceUnavailable {
                    message: "provider is unhealthy (health check failed)".into(),
                    status: 503,
                    retry_after: None,
                })
            });
        }
        let fut = self.inner.call(req);
        Box::pin(fut)
    }
}

/// Tower [`Layer`] that monitors service health via periodic probes.
///
/// The background health-check task is spawned when the layer wraps a service
/// (i.e. when [`Layer::layer`] is called).  The task runs until the
/// [`HealthCheckService`] (and all its clones) are dropped.
#[cfg_attr(alef, alef(skip))]
pub struct HealthCheckLayer {
    config: HealthCheckConfig,
}

impl HealthCheckLayer {
    /// Create a new health-check layer that probes every `interval`, using
    /// [`HealthCheckConfig::default`] for everything else.
    ///
    /// This means the gate now requires 3 consecutive `ListModels` failures
    /// before it opens (previously: 1) and 2 consecutive successes before it
    /// closes again (previously: 1), and each probe is bounded by a 5s
    /// timeout (previously: unbounded). This is a deliberate behaviour change:
    /// a single failed probe — including one caused by a provider that simply
    /// does not implement `ListModels` — must not take the whole service
    /// down. Operators who want the old single-probe flip should use
    /// [`HealthCheckLayer::with_config`] with `unhealthy_threshold: 1,
    /// healthy_threshold: 1`.
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self::with_config(HealthCheckConfig {
            interval,
            ..Default::default()
        })
    }

    /// Create a new health-check layer with full control over the probe
    /// interval, timeout, and consecutive-failure/success thresholds.
    #[must_use]
    pub fn with_config(config: HealthCheckConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for HealthCheckLayer
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = HealthCheckService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let state = ProviderHealthState::new(true);

        let probe_svc = inner.clone();
        let probe_state = Arc::clone(&state);
        let config = self.config.clone();

        tokio::spawn(async move {
            run_health_probe(probe_svc, probe_state, config).await;
        });

        HealthCheckService { inner, state }
    }
}

/// Runs the global `ListModels`-probe loop that backs [`HealthCheckService`].
///
/// Reuses [`ProviderHealthState::record`] — the same consecutive-threshold
/// machinery as [`PerProviderHealthCheck`] — rather than flipping the gate on
/// a single probe result, and bounds each probe with `config.timeout` via
/// `tokio::time::timeout`. A probe failing with
/// [`LiterLlmError::EndpointNotSupported`] is not an availability signal (the
/// provider may simply not implement `ListModels`) and is skipped rather than
/// counted.
async fn run_health_probe<S>(mut svc: S, state: Arc<ProviderHealthState>, config: HealthCheckConfig)
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + 'static,
    S::Future: Send + 'static,
{
    loop {
        tokio::time::sleep(config.interval).await;

        if Arc::strong_count(&state) <= 1 {
            break;
        }

        match tokio::time::timeout(config.timeout, svc.call(LlmRequest::ListModels())).await {
            Ok(Ok(_)) => state.record(HealthStatus::Healthy, &config),
            Ok(Err(LiterLlmError::EndpointNotSupported { .. })) => {
                tracing::debug!("health probe: provider does not implement ListModels; skipping probe result");
            }
            Ok(Err(e)) => {
                tracing::debug!(error = %e, "health probe failed");
                state.record(HealthStatus::Unhealthy, &config);
            }
            Err(_elapsed) => {
                tracing::debug!(timeout = ?config.timeout, "health probe timed out");
                state.record(HealthStatus::Unhealthy, &config);
            }
        }
    }
}

/// Tower service produced by [`HealthCheckLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct HealthCheckService<S> {
    inner: S,
    state: Arc<ProviderHealthState>,
}

impl<S: Clone> Clone for HealthCheckService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

impl<S> HealthCheckService<S> {
    /// Returns `true` if the service is currently considered healthy (i.e.
    /// the consecutive-failure threshold has not been reached).
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.state.is_healthy()
    }
}

impl<S> Service<LlmRequest> for HealthCheckService<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if !self.state.is_healthy() {
            return Poll::Ready(Err(LiterLlmError::ServiceUnavailable {
                message: "service is unhealthy (health check failed)".into(),
                status: 503,
                retry_after: None,
            }));
        }
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        if !self.state.is_healthy() {
            return Box::pin(async {
                Err(LiterLlmError::ServiceUnavailable {
                    message: "service is unhealthy (health check failed)".into(),
                    status: 503,
                    retry_after: None,
                })
            });
        }
        let fut = self.inner.call(req);
        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll};

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;
    use crate::types::ModelsListResponse;

    fn list_models_ok() -> LlmResponse {
        LlmResponse::ListModels(ModelsListResponse {
            object: "list".into(),
            data: vec![],
        })
    }

    /// Scripted outcome for [`ScriptedProbeService`], used to drive the
    /// global `run_health_probe` loop deterministically under paused time.
    #[derive(Clone)]
    enum ScriptedOutcome {
        Ok,
        Unavailable,
        EndpointNotSupported,
        /// Sleeps for the given duration before resolving `Ok` — proves the
        /// probe timeout bounds a stalled service rather than waiting on it.
        Stall(Duration),
    }

    /// A `Service<LlmRequest>` whose `ListModels` responses are scripted, one
    /// per call, falling back to `Ok` once the script is exhausted. Used to
    /// prove `run_health_probe` actually invokes the wrapped service on each
    /// tick rather than being bypassed.
    #[derive(Clone)]
    struct ScriptedProbeService {
        script: Arc<Mutex<VecDeque<ScriptedOutcome>>>,
        calls: Arc<AtomicUsize>,
    }

    impl ScriptedProbeService {
        fn new(script: Vec<ScriptedOutcome>) -> Self {
            Self {
                script: Arc::new(Mutex::new(script.into_iter().collect())),
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl Service<LlmRequest> for ScriptedProbeService {
        type Response = LlmResponse;
        type Error = LiterLlmError;
        type Future = BoxFuture<'static, Result<LlmResponse>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: LlmRequest) -> Self::Future {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let outcome = self
                .script
                .lock()
                .expect("script mutex poisoned")
                .pop_front()
                .unwrap_or(ScriptedOutcome::Ok);

            Box::pin(async move {
                match outcome {
                    ScriptedOutcome::Ok => Ok(list_models_ok()),
                    ScriptedOutcome::Unavailable => Err(LiterLlmError::ServiceUnavailable {
                        message: "probe failed".into(),
                        status: 503,
                        retry_after: None,
                    }),
                    ScriptedOutcome::EndpointNotSupported => Err(LiterLlmError::EndpointNotSupported {
                        endpoint: "list_models".into(),
                        provider: "scripted".into(),
                    }),
                    ScriptedOutcome::Stall(d) => {
                        tokio::time::sleep(d).await;
                        Ok(list_models_ok())
                    }
                }
            })
        }
    }

    /// Scripted outcome for [`ScriptedChecker`].
    #[derive(Clone)]
    enum ScriptedHealthOutcome {
        Healthy,
        Unhealthy,
        /// Resolves `Healthy` only after sleeping `Duration` — proves the
        /// outer `tokio::time::timeout(config.timeout, ..)` bounds a checker
        /// whose own future stalls.
        Stall(Duration),
    }

    /// A [`HealthChecker`] whose results are scripted, one per call, falling
    /// back to `Healthy` once the script is exhausted.
    struct ScriptedChecker {
        script: Mutex<VecDeque<ScriptedHealthOutcome>>,
        calls: Arc<AtomicUsize>,
    }

    impl ScriptedChecker {
        fn new(script: Vec<ScriptedHealthOutcome>) -> (Arc<Self>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            let checker = Arc::new(Self {
                script: Mutex::new(script.into_iter().collect()),
                calls: Arc::clone(&calls),
            });
            (checker, calls)
        }
    }

    impl HealthChecker for ScriptedChecker {
        fn check(&self, _upstream: String) -> Pin<Box<dyn Future<Output = HealthStatus> + Send + 'static>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let outcome = self
                .script
                .lock()
                .expect("script mutex poisoned")
                .pop_front()
                .unwrap_or(ScriptedHealthOutcome::Healthy);

            Box::pin(async move {
                match outcome {
                    ScriptedHealthOutcome::Healthy => HealthStatus::Healthy,
                    ScriptedHealthOutcome::Unhealthy => HealthStatus::Unhealthy,
                    ScriptedHealthOutcome::Stall(d) => {
                        tokio::time::sleep(d).await;
                        HealthStatus::Healthy
                    }
                }
            })
        }
    }

    /// Advances the paused clock by `d`, then yields repeatedly so the woken
    /// probe task can be polled to completion before the assertion runs.
    ///
    /// ~keep One `yield_now` is not enough: after waking from `sleep` the probe
    /// ~keep still has to traverse `timeout(..)`, `svc.call(..).await` and the
    /// ~keep `state.record` that follows, which is several distinct await points.
    /// ~keep A single yield polls it once and the assertion then races the probe.
    async fn advance(d: Duration) {
        // ~keep Yield BEFORE advancing. `layer()` spawns the probe but does not poll it, so
        // ~keep on the first call its `sleep` timer is not registered yet — advancing the clock
        // ~keep past a timer that does not exist does nothing, and the probe then sleeps from
        // ~keep the new now, never firing. This is why every threshold test reported zero calls.
        tokio::task::yield_now().await;
        tokio::time::advance(d).await;
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test]
    async fn healthy_service_passes_through() {
        let inner = LlmService::new(MockClient::ok());
        let mut svc = HealthCheckService {
            inner,
            state: ProviderHealthState::new(true),
        };

        let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn unhealthy_service_rejects_requests() {
        let inner = LlmService::new(MockClient::ok());
        let mut svc = HealthCheckService {
            inner,
            state: ProviderHealthState::new(false),
        };

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("unhealthy service should reject");
        assert!(matches!(err, LiterLlmError::ServiceUnavailable { .. }));
    }

    #[tokio::test]
    async fn is_healthy_reflects_flag() {
        let inner = LlmService::new(MockClient::ok());
        let state = ProviderHealthState::new(true);
        let svc = HealthCheckService {
            inner,
            state: Arc::clone(&state),
        };

        assert!(svc.is_healthy());
        state.healthy.store(false, Ordering::Release);
        assert!(!svc.is_healthy());
    }

    #[tokio::test]
    async fn recovery_after_becoming_healthy_again() {
        let inner = LlmService::new(MockClient::ok());
        let state = ProviderHealthState::new(false);
        let mut svc = HealthCheckService {
            inner,
            state: Arc::clone(&state),
        };

        assert!(svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.is_err());

        state.healthy.store(true, Ordering::Release);
        assert!(svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.is_ok());
    }

    /// Revert target: the `if failures >= config.unhealthy_threshold` guard in
    /// `ProviderHealthState::record` (or reverting `run_health_probe` to the old
    /// `healthy.store(result.is_ok())` single-probe flip) makes this fail, because
    /// one failure would immediately open the gate.
    #[tokio::test(start_paused = true)]
    async fn single_failure_does_not_open_global_gate() {
        let config = HealthCheckConfig {
            interval: Duration::from_millis(10),
            timeout: Duration::from_millis(50),
            unhealthy_threshold: 3,
            healthy_threshold: 1,
        };
        let svc = ScriptedProbeService::new(vec![ScriptedOutcome::Unavailable]);
        let calls = Arc::clone(&svc.calls);
        let health_svc = HealthCheckLayer::with_config(config.clone()).layer(svc);

        advance(config.interval * 2).await;

        assert!(
            health_svc.is_healthy(),
            "one failure below unhealthy_threshold must not open the gate"
        );
        assert!(calls.load(Ordering::SeqCst) >= 1, "probe must actually have run");
    }

    /// Revert target: same as above — remove the threshold check (or the
    /// `state.record` call) from `run_health_probe` and this fails, because
    /// the gate would never close for `unhealthy_threshold` consecutive failures
    /// specifically (it would already be open after the first).
    #[tokio::test(start_paused = true)]
    async fn n_consecutive_failures_open_global_gate() {
        let config = HealthCheckConfig {
            interval: Duration::from_millis(10),
            timeout: Duration::from_millis(50),
            unhealthy_threshold: 3,
            healthy_threshold: 1,
        };
        let svc = ScriptedProbeService::new(vec![
            ScriptedOutcome::Unavailable,
            ScriptedOutcome::Unavailable,
            ScriptedOutcome::Unavailable,
        ]);
        let calls = Arc::clone(&svc.calls);
        let health_svc = HealthCheckLayer::with_config(config.clone()).layer(svc);

        advance(config.interval).await;
        assert!(health_svc.is_healthy(), "1 of 3 failures must not open the gate yet");

        advance(config.interval).await;
        assert!(health_svc.is_healthy(), "2 of 3 failures must not open the gate yet");

        advance(config.interval).await;
        assert!(!health_svc.is_healthy(), "3 consecutive failures must open the gate");
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    /// Revert target: the `Ok(Ok(_)) => state.record(HealthStatus::Healthy, ..)`
    /// arm in `run_health_probe` (or the healthy-threshold check in
    /// `ProviderHealthState::record`) — remove it and the gate never closes.
    #[tokio::test(start_paused = true)]
    async fn recovery_closes_global_gate() {
        let config = HealthCheckConfig {
            interval: Duration::from_millis(10),
            timeout: Duration::from_millis(50),
            unhealthy_threshold: 1,
            healthy_threshold: 2,
        };
        let svc = ScriptedProbeService::new(vec![
            ScriptedOutcome::Unavailable,
            ScriptedOutcome::Ok,
            ScriptedOutcome::Ok,
        ]);
        let health_svc = HealthCheckLayer::with_config(config.clone()).layer(svc);

        advance(config.interval).await;
        assert!(
            !health_svc.is_healthy(),
            "first failure must open the gate (threshold 1)"
        );

        advance(config.interval).await;
        assert!(
            !health_svc.is_healthy(),
            "one success below healthy_threshold must not close the gate yet"
        );

        advance(config.interval).await;
        assert!(
            health_svc.is_healthy(),
            "second consecutive success must close the gate"
        );
    }

    /// Revert target: the `Ok(Err(LiterLlmError::EndpointNotSupported { .. }))` arm
    /// in `run_health_probe` — remove it (folding it into the generic error arm) and
    /// this fails, because a provider that simply doesn't implement `ListModels`
    /// would be marked down and reject all traffic, per DEFECT 1 scenario (a).
    #[tokio::test(start_paused = true)]
    async fn endpoint_not_supported_does_not_open_global_gate() {
        let config = HealthCheckConfig {
            interval: Duration::from_millis(10),
            timeout: Duration::from_millis(50),
            unhealthy_threshold: 1,
            healthy_threshold: 1,
        };
        let svc = ScriptedProbeService::new(vec![
            ScriptedOutcome::EndpointNotSupported,
            ScriptedOutcome::EndpointNotSupported,
            ScriptedOutcome::EndpointNotSupported,
        ]);
        let calls = Arc::clone(&svc.calls);
        let health_svc = HealthCheckLayer::with_config(config.clone()).layer(svc);

        advance(config.interval * 3).await;

        assert!(
            health_svc.is_healthy(),
            "EndpointNotSupported is not an availability signal and must not open the gate"
        );
        assert!(calls.load(Ordering::SeqCst) >= 1, "probe must actually have run");
    }

    /// Revert target: the `tokio::time::timeout(config.timeout, svc.call(..))`
    /// wrapper in `run_health_probe` — replace it with a bare `.await` and this
    /// fails, because the probe task would still be parked on the stalled call
    /// at the point we assert, never having recorded a failure.
    #[tokio::test(start_paused = true)]
    async fn stalled_global_probe_bounded_by_timeout() {
        let config = HealthCheckConfig {
            interval: Duration::from_millis(10),
            timeout: Duration::from_millis(20),
            unhealthy_threshold: 1,
            healthy_threshold: 1,
        };
        let svc = ScriptedProbeService::new(vec![ScriptedOutcome::Stall(Duration::from_millis(500))]);
        let health_svc = HealthCheckLayer::with_config(config.clone()).layer(svc);

        // ~keep Two phases, not one big jump: the probe's timeout timer is only registered
        // ~keep after its sleep fires, so a single advance past both deadlines lands the timeout
        // ~keep deadline beyond the already-jumped clock and it never elapses.
        advance(config.interval).await;
        advance(config.timeout + Duration::from_millis(10)).await;

        assert!(
            !health_svc.is_healthy(),
            "a probe stuck past the configured timeout must be treated as a failure, not left pending forever"
        );
    }

    #[test]
    fn health_check_config_default_values() {
        let config = HealthCheckConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.unhealthy_threshold, 3);
        assert_eq!(config.healthy_threshold, 2);
    }

    #[test]
    fn health_checker_marks_down_after_threshold() {
        let config = HealthCheckConfig {
            unhealthy_threshold: 3,
            healthy_threshold: 1,
            ..Default::default()
        };
        let state = ProviderHealthState::new(true);

        state.record(HealthStatus::Unhealthy, &config);
        assert!(state.is_healthy(), "should still be healthy after 1 failure");

        state.record(HealthStatus::Unhealthy, &config);
        assert!(state.is_healthy(), "should still be healthy after 2 failures");

        state.record(HealthStatus::Unhealthy, &config);
        assert!(!state.is_healthy(), "should be unhealthy after 3 consecutive failures");
    }

    #[test]
    fn health_checker_marks_up_after_threshold() {
        let config = HealthCheckConfig {
            unhealthy_threshold: 1,
            healthy_threshold: 2,
            ..Default::default()
        };
        let state = ProviderHealthState::new(false);

        state.record(HealthStatus::Healthy, &config);
        assert!(!state.is_healthy(), "should still be unhealthy after 1 success");

        state.record(HealthStatus::Healthy, &config);
        assert!(state.is_healthy(), "should be healthy after 2 consecutive successes");
    }

    #[test]
    fn health_checker_resets_counters_on_state_change() {
        let config = HealthCheckConfig {
            unhealthy_threshold: 2,
            healthy_threshold: 2,
            ..Default::default()
        };
        let state = ProviderHealthState::new(true);

        state.record(HealthStatus::Unhealthy, &config);
        state.record(HealthStatus::Healthy, &config);
        state.record(HealthStatus::Unhealthy, &config);
        assert!(state.is_healthy(), "one failure after reset should not mark unhealthy");
        state.record(HealthStatus::Unhealthy, &config);
        assert!(!state.is_healthy(), "second failure after reset should mark unhealthy");
    }

    /// Proves the background probe task actually calls `HealthChecker::check`
    /// (the original version of this test used `interval: 30s` from
    /// `..Default::default()`, so the probe task never fired within the test
    /// and the checker's `check` was never invoked). Revert target: replace
    /// `tokio::time::timeout(config.timeout, checker.check(..))` with a bare
    /// `checker.check(..).await` call site removal, or delete the spawn in
    /// `PerProviderHealthCheck::new` — either way `calls` stays at 0.
    #[tokio::test(start_paused = true)]
    async fn per_provider_healthy_checker_is_actually_invoked() {
        let (checker, calls) = ScriptedChecker::new(vec![]);
        let inner = LlmService::new(MockClient::ok());
        let config = HealthCheckConfig {
            interval: Duration::from_millis(10),
            timeout: Duration::from_millis(50),
            unhealthy_threshold: 3,
            healthy_threshold: 1,
        };
        let mut svc = PerProviderHealthCheck::new(inner, checker, "test-provider".into(), config.clone());

        advance(config.interval * 2).await;

        assert!(svc.is_healthy());
        assert!(
            calls.load(Ordering::SeqCst) >= 1,
            "checker must actually be invoked by the probe loop, not just assumed healthy"
        );
        let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok());
    }

    /// Revert target: the `if failures >= config.unhealthy_threshold` guard in
    /// `ProviderHealthState::record` — without it a single scripted `Unhealthy`
    /// result would reject immediately instead of after 3 consecutive failures.
    #[tokio::test(start_paused = true)]
    async fn per_provider_unhealthy_rejects_after_threshold() {
        let (checker, calls) = ScriptedChecker::new(vec![
            ScriptedHealthOutcome::Unhealthy,
            ScriptedHealthOutcome::Unhealthy,
            ScriptedHealthOutcome::Unhealthy,
        ]);
        let inner = LlmService::new(MockClient::ok());
        let config = HealthCheckConfig {
            interval: Duration::from_millis(10),
            timeout: Duration::from_millis(50),
            unhealthy_threshold: 3,
            healthy_threshold: 1,
        };
        let mut svc = PerProviderHealthCheck::new(inner, checker, "test-provider".into(), config.clone());

        advance(config.interval).await;
        assert!(svc.is_healthy(), "one failure below threshold must not reject");

        // ~keep One advance per tick: the probe registers its next sleep only after the
        // ~keep previous one completes, so a single 2x jump fires one timer, not two.
        advance(config.interval).await;
        advance(config.interval).await;

        assert!(!svc.is_healthy());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "checker must be called once per interval tick"
        );
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("unhealthy provider should reject");
        assert!(matches!(err, LiterLlmError::ServiceUnavailable { .. }));
    }

    /// Revert target: the `tokio::time::timeout(config.timeout, checker.check(..))`
    /// wrapper in `run_provider_health_probe` — replace it with a bare `.await` and
    /// this fails, because the probe task stays parked on the checker's 500ms sleep
    /// and never calls `state.record`, leaving the provider marked healthy forever
    /// (the exact DEFECT 2 consequence: a dead provider keeps receiving traffic).
    #[tokio::test(start_paused = true)]
    async fn per_provider_stalled_checker_bounded_by_timeout() {
        let (checker, _calls) = ScriptedChecker::new(vec![ScriptedHealthOutcome::Stall(Duration::from_millis(500))]);
        let inner = LlmService::new(MockClient::ok());
        let config = HealthCheckConfig {
            interval: Duration::from_millis(10),
            timeout: Duration::from_millis(20),
            unhealthy_threshold: 1,
            healthy_threshold: 1,
        };
        let svc = PerProviderHealthCheck::new(inner, checker, "test-provider".into(), config.clone());

        // ~keep Two phases, not one big jump: the probe's timeout timer is only registered
        // ~keep after its sleep fires, so a single advance past both deadlines lands the timeout
        // ~keep deadline beyond the already-jumped clock and it never elapses.
        advance(config.interval).await;
        advance(config.timeout + Duration::from_millis(10)).await;

        assert!(
            !svc.is_healthy(),
            "a checker stuck past the configured timeout must be treated as a failure, not left pending forever"
        );
    }
}
