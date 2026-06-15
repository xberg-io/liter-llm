//! Health-check middleware — global and per-provider.
//!
//! # Overview
//!
//! This module provides two levels of health-checking:
//!
//! ## Global health gate (backward-compatible)
//!
//! [`HealthCheckLayer`] wraps a service and spawns a background task that
//! periodically probes the service by sending a [`LlmRequest::ListModels`]
//! request.  If the probe fails, the service is marked unhealthy and incoming
//! requests are immediately rejected with [`LiterLlmError::ServiceUnavailable`].
//!
//! The health flag is an [`AtomicBool`] shared between the background probe
//! task and the request path, so checking health adds minimal overhead (a
//! single atomic load).
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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

// ---- HealthStatus ----------------------------------------------------------

/// The result of a single health probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// The probe succeeded; the upstream is reachable.
    Healthy,
    /// The probe failed; the upstream may be down.
    Unhealthy,
}

// ---- HealthCheckConfig -----------------------------------------------------

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

// ---- HealthChecker trait ---------------------------------------------------

/// Abstraction over a health probe strategy.
///
/// Implementors issue a lightweight probe against `upstream` (typically a
/// provider base URL or named identifier) and report [`HealthStatus`].
///
/// # Note for 1.A integration
///
/// The `upstream` parameter is intentionally a plain `&str` so that it works
/// across the FFI boundary without allocation.  1.A's error consolidation
/// should ensure that `Err` values carry numeric codes ≥ 2000.
pub trait HealthChecker: Send + Sync + 'static {
    /// Probe `upstream` and return its current [`HealthStatus`].
    ///
    /// The future must be `'static + Send`.
    fn check(
        &self,
        upstream: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthStatus> + Send + 'static>>;
}

// ---- HttpProbeHealthChecker ------------------------------------------------

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
pub struct HttpProbeHealthChecker {
    /// HTTP client used for probes.  Shared across all probe tasks.
    /// The per-probe timeout is baked in at construction time via
    /// `reqwest::ClientBuilder::timeout`.
    client: reqwest::Client,
    /// Per-provider health-check URL overrides.
    /// Key: provider base URL or name; Value: dedicated probe endpoint.
    probe_urls: std::collections::HashMap<String, String>,
}

impl HttpProbeHealthChecker {
    /// Create a new checker with the given timeout and optional URL overrides.
    ///
    /// `probe_urls`: maps provider base URL / name → dedicated health-check
    /// URL.  If a provider is not in this map, the prober issues a GET
    /// request on the upstream URL itself.
    pub fn new(timeout: Duration, probe_urls: impl IntoIterator<Item = (String, String)>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| LiterLlmError::BadRequest {
                message: format!("failed to build HTTP client for health checker: {e}"),
                status: 500,
            })?;
        Ok(Self {
            client,
            probe_urls: probe_urls.into_iter().collect(),
        })
    }
}

impl HealthChecker for HttpProbeHealthChecker {
    fn check(
        &self,
        upstream: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthStatus> + Send + 'static>> {
        let url = self
            .probe_urls
            .get(upstream)
            .cloned()
            .unwrap_or_else(|| upstream.to_owned());
        let client = self.client.clone();

        Box::pin(async move {
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

// ---- Per-provider probe state ----------------------------------------------

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

// ---- Per-provider probe background task -----------------------------------

async fn run_provider_health_probe<C: HealthChecker>(
    checker: Arc<C>,
    upstream: String,
    state: Arc<ProviderHealthState>,
    config: HealthCheckConfig,
) {
    loop {
        tokio::time::sleep(config.interval).await;

        // Stop when the state Arc is only held by this task — all service
        // clones have been dropped.
        if Arc::strong_count(&state) <= 1 {
            break;
        }

        let status = checker.check(&upstream).await;
        state.record(status, &config);
    }
}

// ---- PerProviderHealthCheck service ---------------------------------------

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
        // Start healthy; the first failure within threshold won't block requests.
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
                })
            });
        }
        let fut = self.inner.call(req);
        Box::pin(fut)
    }
}

// ---- Backward-compatible global health gate --------------------------------

/// Tower [`Layer`] that monitors service health via periodic probes.
///
/// The background health-check task is spawned when the layer wraps a service
/// (i.e. when [`Layer::layer`] is called).  The task runs until the
/// [`HealthCheckService`] (and all its clones) are dropped.
#[cfg_attr(alef, alef(skip))]
pub struct HealthCheckLayer {
    interval: Duration,
}

impl HealthCheckLayer {
    /// Create a new health-check layer that probes every `interval`.
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }
}

impl<S> Layer<S> for HealthCheckLayer
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = HealthCheckService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let healthy = Arc::new(AtomicBool::new(true));

        // Spawn the background probe task.
        let probe_svc = inner.clone();
        let probe_healthy = Arc::clone(&healthy);
        let interval = self.interval;

        tokio::spawn(async move {
            run_health_probe(probe_svc, probe_healthy, interval).await;
        });

        HealthCheckService { inner, healthy }
    }
}

// ---- Background probe ------------------------------------------------------

async fn run_health_probe<S>(mut svc: S, healthy: Arc<AtomicBool>, interval: Duration)
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + 'static,
    S::Future: Send + 'static,
{
    loop {
        tokio::time::sleep(interval).await;

        // If the Arc is held only by us, all service clones have been dropped
        // and we should stop probing.
        if Arc::strong_count(&healthy) <= 1 {
            break;
        }

        let result = svc.call(LlmRequest::ListModels).await;
        let is_healthy = result.is_ok();
        healthy.store(is_healthy, Ordering::Release);

        if !is_healthy {
            tracing::warn!("health check failed; marking service as unhealthy");
        }
    }
}

// ---- Service ---------------------------------------------------------------

/// Tower service produced by [`HealthCheckLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct HealthCheckService<S> {
    inner: S,
    healthy: Arc<AtomicBool>,
}

impl<S: Clone> Clone for HealthCheckService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            healthy: Arc::clone(&self.healthy),
        }
    }
}

impl<S> HealthCheckService<S> {
    /// Returns `true` if the last health probe succeeded.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Acquire)
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
        if !self.healthy.load(Ordering::Acquire) {
            return Poll::Ready(Err(LiterLlmError::ServiceUnavailable {
                message: "service is unhealthy (health check failed)".into(),
                status: 503,
            }));
        }
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        if !self.healthy.load(Ordering::Acquire) {
            return Box::pin(async {
                Err(LiterLlmError::ServiceUnavailable {
                    message: "service is unhealthy (health check failed)".into(),
                    status: 503,
                })
            });
        }
        let fut = self.inner.call(req);
        Box::pin(fut)
    }
}

// ---- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use tower::Service as _;

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    // ---- HealthCheckService (backward-compat) tests -------------------------

    #[tokio::test]
    async fn healthy_service_passes_through() {
        let inner = LlmService::new(MockClient::ok());
        let healthy = Arc::new(AtomicBool::new(true));
        let mut svc = HealthCheckService {
            inner,
            healthy: Arc::clone(&healthy),
        };

        let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn unhealthy_service_rejects_requests() {
        let inner = LlmService::new(MockClient::ok());
        let healthy = Arc::new(AtomicBool::new(false));
        let mut svc = HealthCheckService {
            inner,
            healthy: Arc::clone(&healthy),
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
        let healthy = Arc::new(AtomicBool::new(true));
        let svc = HealthCheckService {
            inner,
            healthy: Arc::clone(&healthy),
        };

        assert!(svc.is_healthy());
        healthy.store(false, Ordering::Release);
        assert!(!svc.is_healthy());
    }

    #[tokio::test]
    async fn recovery_after_becoming_healthy_again() {
        let inner = LlmService::new(MockClient::ok());
        let healthy = Arc::new(AtomicBool::new(false));
        let mut svc = HealthCheckService {
            inner,
            healthy: Arc::clone(&healthy),
        };

        // Unhealthy — should reject.
        assert!(svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.is_err());

        // Mark as healthy again.
        healthy.store(true, Ordering::Release);
        assert!(svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.is_ok());
    }

    // ---- HealthCheckConfig defaults ----------------------------------------

    #[test]
    fn health_check_config_default_values() {
        let config = HealthCheckConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.unhealthy_threshold, 3);
        assert_eq!(config.healthy_threshold, 2);
    }

    // ---- ProviderHealthState threshold tests --------------------------------

    #[test]
    fn health_checker_marks_down_after_threshold() {
        let config = HealthCheckConfig {
            unhealthy_threshold: 3,
            healthy_threshold: 1,
            ..Default::default()
        };
        let state = ProviderHealthState::new(true);

        // Two failures — still healthy (below threshold).
        state.record(HealthStatus::Unhealthy, &config);
        assert!(state.is_healthy(), "should still be healthy after 1 failure");

        state.record(HealthStatus::Unhealthy, &config);
        assert!(state.is_healthy(), "should still be healthy after 2 failures");

        // Third failure — crosses threshold.
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
        let state = ProviderHealthState::new(false); // start unhealthy

        // One success — still unhealthy (below threshold of 2).
        state.record(HealthStatus::Healthy, &config);
        assert!(!state.is_healthy(), "should still be unhealthy after 1 success");

        // Second success — crosses healthy threshold.
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

        // One failure.
        state.record(HealthStatus::Unhealthy, &config);
        // A success resets the failure counter.
        state.record(HealthStatus::Healthy, &config);
        // Now need two more failures to mark down.
        state.record(HealthStatus::Unhealthy, &config);
        assert!(state.is_healthy(), "one failure after reset should not mark unhealthy");
        state.record(HealthStatus::Unhealthy, &config);
        assert!(!state.is_healthy(), "second failure after reset should mark unhealthy");
    }

    // ---- PerProviderHealthCheck service tests --------------------------------

    #[tokio::test]
    async fn per_provider_healthy_passes_through() {
        // Build a no-op checker that always returns Healthy.
        struct AlwaysHealthy;
        impl HealthChecker for AlwaysHealthy {
            fn check(
                &self,
                _upstream: &str,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthStatus> + Send + 'static>> {
                Box::pin(async { HealthStatus::Healthy })
            }
        }

        let inner = LlmService::new(MockClient::ok());
        let config = HealthCheckConfig::default();
        let checker = Arc::new(AlwaysHealthy);
        let mut svc = PerProviderHealthCheck::new(inner, checker, "test-provider".into(), config);

        assert!(svc.is_healthy());
        let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn per_provider_unhealthy_rejects() {
        struct AlwaysUnhealthy;
        impl HealthChecker for AlwaysUnhealthy {
            fn check(
                &self,
                _upstream: &str,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthStatus> + Send + 'static>> {
                Box::pin(async { HealthStatus::Unhealthy })
            }
        }

        let inner = LlmService::new(MockClient::ok());
        // Use threshold=1 so first failure immediately marks down.
        let config = HealthCheckConfig {
            unhealthy_threshold: 1,
            healthy_threshold: 1,
            ..Default::default()
        };
        let checker = Arc::new(AlwaysUnhealthy);
        let mut svc = PerProviderHealthCheck::new(inner, checker, "test-provider".into(), config);

        // Manually drive the state to unhealthy (the background probe hasn't
        // fired in this synchronous test; we test the state logic directly).
        svc.state.record(
            HealthStatus::Unhealthy,
            &HealthCheckConfig {
                unhealthy_threshold: 1,
                healthy_threshold: 1,
                ..Default::default()
            },
        );

        assert!(!svc.is_healthy());
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("unhealthy provider should reject");
        assert!(matches!(err, LiterLlmError::ServiceUnavailable { .. }));
    }
}
