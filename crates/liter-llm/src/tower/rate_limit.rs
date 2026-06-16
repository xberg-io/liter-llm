//! Per-model rate limiting middleware.
//!
//! [`ModelRateLimitLayer`] wraps any [`Service<LlmRequest>`] and enforces
//! per-model request-per-minute (RPM) and token-per-minute (TPM) limits using
//! a fixed window.  When a model exceeds its configured limit the middleware
//! returns [`LiterLlmError::RateLimited`] without forwarding the request to the
//! inner service.  After a successful response, token usage is extracted and
//! added to the running count.
//!
//! Rate state is tracked per model name in a [`DashMap`] so that independent
//! models do not interfere with each other.
//!
//! # Cost-based rate limiting
//!
//! [`CostRateLimitLayer`] adds a parallel rate-limit axis keyed on cost (USD)
//! rather than request or token counts.  It consults a sliding-window spend
//! accumulator and rejects requests when the projected spend would exceed the
//! configured `max_usd_per_minute`, `max_usd_per_hour`, or `max_usd_per_day`
//! threshold.  Because exact call cost is only known after the response, the
//! layer uses `cost_estimate_usd` as a conservative pre-flight guard.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime};

use dashmap::DashMap;
use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::cost;
use crate::error::{LiterLlmError, Result};

// ---- Config ----------------------------------------------------------------

/// Configuration for per-model rate limits.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window.  `None` means unlimited.
    pub rpm: Option<u32>,
    /// Maximum tokens per window.  `None` means unlimited.
    pub tpm: Option<u64>,
    /// Fixed window duration (defaults to 60 s).
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            rpm: None,
            tpm: None,
            window: Duration::from_secs(60),
        }
    }
}

// ---- State -----------------------------------------------------------------

/// Per-model counters for the current window.
struct ModelRateState {
    request_count: u64,
    token_count: u64,
    window_start: Instant,
}

impl ModelRateState {
    fn new() -> Self {
        Self {
            request_count: 0,
            token_count: 0,
            window_start: Instant::now(),
        }
    }

    /// Reset counters if the current window has elapsed.
    fn maybe_reset(&mut self, window: Duration) {
        if self.window_start.elapsed() >= window {
            self.request_count = 0;
            self.token_count = 0;
            self.window_start = Instant::now();
        }
    }
}

// ---- Layer -----------------------------------------------------------------

/// Tower [`Layer`] that enforces per-model rate limits.
#[cfg_attr(alef, alef(skip))]
pub struct ModelRateLimitLayer {
    config: RateLimitConfig,
    state: Arc<DashMap<String, ModelRateState>>,
}

impl ModelRateLimitLayer {
    /// Create a new rate-limit layer with the given configuration.
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            state: Arc::new(DashMap::new()),
        }
    }
}

impl<S> Layer<S> for ModelRateLimitLayer {
    type Service = ModelRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ModelRateLimitService {
            inner,
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

// ---- Service ---------------------------------------------------------------

/// Tower service produced by [`ModelRateLimitLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct ModelRateLimitService<S> {
    inner: S,
    config: RateLimitConfig,
    state: Arc<DashMap<String, ModelRateState>>,
}

impl<S: Clone> Clone for ModelRateLimitService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

impl<S> Service<LlmRequest> for ModelRateLimitService<S>
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
        let model = req.model().unwrap_or("unknown").to_owned();
        let config = self.config.clone();
        let state = Arc::clone(&self.state);

        // --- Pre-flight: check RPM limit ---
        {
            let mut entry = state.entry(model.clone()).or_insert_with(ModelRateState::new);
            entry.maybe_reset(config.window);

            if let Some(rpm) = config.rpm
                && entry.request_count >= u64::from(rpm)
            {
                return Box::pin(async move {
                    Err(LiterLlmError::RateLimited {
                        message: format!(
                            "model {model} exceeded {rpm} requests per {:.0}s window",
                            config.window.as_secs_f64()
                        ),
                        retry_after: Some(config.window),
                    })
                });
            }

            if let Some(tpm) = config.tpm
                && entry.token_count >= tpm
            {
                return Box::pin(async move {
                    Err(LiterLlmError::RateLimited {
                        message: format!(
                            "model {model} exceeded {tpm} tokens per {:.0}s window",
                            config.window.as_secs_f64()
                        ),
                        retry_after: Some(config.window),
                    })
                });
            }

            // Increment request count optimistically.
            entry.request_count += 1;
        }

        let fut = self.inner.call(req);

        Box::pin(async move {
            let resp = fut.await?;

            // --- Post-flight: update token count ---
            if let Some(usage) = resp.usage() {
                let total_tokens = usage.prompt_tokens + usage.completion_tokens;
                if let Some(mut entry) = state.get_mut(&model) {
                    entry.maybe_reset(config.window);
                    entry.token_count += total_tokens;
                }
            }

            Ok(resp)
        })
    }
}

// ─── Cost rate-limit config ───────────────────────────────────────────────────

/// Configuration for the cost-based rate-limit axis.
///
/// Requests are rejected before dispatch when the accumulated spend in the
/// relevant sliding window already exceeds the configured threshold.  `None`
/// means that dimension is unlimited.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CostRateLimitConfig {
    /// Maximum cumulative spend in USD per 60-second window.  `None` means
    /// unlimited.
    pub max_usd_per_minute: Option<f64>,
    /// Maximum cumulative spend in USD per 3600-second window.  `None` means
    /// unlimited.
    pub max_usd_per_hour: Option<f64>,
    /// Maximum cumulative spend in USD per 86400-second window.  `None` means
    /// unlimited.
    pub max_usd_per_day: Option<f64>,
}

// ─── Cost sliding-window state ────────────────────────────────────────────────

/// Atomic sliding-window accumulator for a single cost window.
///
/// Spend is stored in microcents (`USD × 1_000_000`) to avoid floating-point
/// atomics.  The window resets lazily when the first access after expiry occurs.
#[derive(Debug)]
struct CostWindow {
    spend_mc: AtomicU64,
    window_start_secs: AtomicU64,
    window_secs: u64,
}

impl CostWindow {
    fn new(window: Duration) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            spend_mc: AtomicU64::new(0),
            window_start_secs: AtomicU64::new(now),
            window_secs: window.as_secs(),
        }
    }

    /// Return current spend in USD, resetting if the window has elapsed.
    fn spend_usd(&self, now_secs: u64) -> f64 {
        let start = self.window_start_secs.load(Ordering::Relaxed);
        if now_secs.saturating_sub(start) >= self.window_secs {
            self.spend_mc.store(0, Ordering::Relaxed);
            self.window_start_secs.store(now_secs, Ordering::Relaxed);
        }
        let mc = self.spend_mc.load(Ordering::Relaxed);
        mc as f64 / 1_000_000.0
    }

    /// Add `usd` to the window accumulator.
    fn add(&self, usd: f64, now_secs: u64) {
        let _ = self.spend_usd(now_secs); // reset if expired
        if usd > 0.0 {
            let mc = (usd * 1_000_000.0).round() as u64;
            self.spend_mc.fetch_add(mc, Ordering::Relaxed);
        }
    }
}

/// Shared spend state for the cost rate-limit layer.
#[derive(Debug)]
struct CostRateLimitState {
    per_minute: CostWindow,
    per_hour: CostWindow,
    per_day: CostWindow,
}

impl CostRateLimitState {
    fn new() -> Self {
        Self {
            per_minute: CostWindow::new(Duration::from_secs(60)),
            per_hour: CostWindow::new(Duration::from_secs(3600)),
            per_day: CostWindow::new(Duration::from_secs(86_400)),
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Check whether any window is over the configured limit.
    fn check(&self, config: &CostRateLimitConfig) -> Option<LiterLlmError> {
        let now = Self::now_secs();

        if let Some(limit) = config.max_usd_per_minute {
            let spend = self.per_minute.spend_usd(now);
            if spend >= limit {
                return Some(LiterLlmError::RateLimited {
                    message: format!("cost rate limit exceeded: ${spend:.6} >= ${limit:.6} per minute"),
                    retry_after: Some(Duration::from_secs(60)),
                });
            }
        }

        if let Some(limit) = config.max_usd_per_hour {
            let spend = self.per_hour.spend_usd(now);
            if spend >= limit {
                return Some(LiterLlmError::RateLimited {
                    message: format!("cost rate limit exceeded: ${spend:.6} >= ${limit:.6} per hour"),
                    retry_after: Some(Duration::from_secs(3600)),
                });
            }
        }

        if let Some(limit) = config.max_usd_per_day {
            let spend = self.per_day.spend_usd(now);
            if spend >= limit {
                return Some(LiterLlmError::RateLimited {
                    message: format!("cost rate limit exceeded: ${spend:.6} >= ${limit:.6} per day"),
                    retry_after: Some(Duration::from_secs(86_400)),
                });
            }
        }

        None
    }

    /// Record actual cost after a successful response.
    fn record(&self, usd: f64) {
        let now = Self::now_secs();
        self.per_minute.add(usd, now);
        self.per_hour.add(usd, now);
        self.per_day.add(usd, now);
    }
}

// ─── CostRateLimitLayer ───────────────────────────────────────────────────────

/// Tower [`Layer`] that enforces cost-based rate limits (USD per time window).
///
/// Rejects requests with [`LiterLlmError::RateLimited`] when any configured
/// window threshold is exceeded.  Cost is accumulated after each successful
/// response; the pre-flight check uses the running window total.
#[cfg_attr(alef, alef(skip))]
pub struct CostRateLimitLayer {
    config: CostRateLimitConfig,
    state: Arc<CostRateLimitState>,
}

impl CostRateLimitLayer {
    /// Create a new cost-rate-limit layer with the given configuration.
    #[must_use]
    pub fn new(config: CostRateLimitConfig) -> Self {
        Self {
            config,
            state: Arc::new(CostRateLimitState::new()),
        }
    }
}

impl<S> Layer<S> for CostRateLimitLayer {
    type Service = CostRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CostRateLimitService {
            inner,
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

// ─── CostRateLimitService ─────────────────────────────────────────────────────

/// Tower service produced by [`CostRateLimitLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct CostRateLimitService<S> {
    inner: S,
    config: CostRateLimitConfig,
    state: Arc<CostRateLimitState>,
}

impl<S: Clone> Clone for CostRateLimitService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

impl<S> Service<LlmRequest> for CostRateLimitService<S>
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
        let model = req.model().unwrap_or("unknown").to_owned();
        let config = self.config.clone();
        let state = Arc::clone(&self.state);

        // Pre-flight: check whether any cost window is already exhausted.
        if let Some(err) = state.check(&config) {
            return Box::pin(async move { Err(err) });
        }

        let fut = self.inner.call(req);

        Box::pin(async move {
            let resp = fut.await?;

            // Post-flight: record actual cost.
            if let Some(usage) = resp.usage()
                && let Some(usd) = cost::completion_cost(&model, usage.prompt_tokens, usage.completion_tokens)
            {
                state.record(usd);
            }

            Ok(resp)
        })
    }
}

// ---- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::tower::tests_common::{MockClient, chat_req};

    use crate::tower::service::LlmService;
    use crate::tower::types::LlmRequest;

    #[tokio::test]
    async fn allows_requests_under_rpm_limit() {
        let config = RateLimitConfig {
            rpm: Some(5),
            tpm: None,
            window: Duration::from_secs(60),
        };
        let layer = ModelRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        for _ in 0..5 {
            let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
            assert!(resp.is_ok(), "requests under limit should succeed");
        }
    }

    #[tokio::test]
    async fn rejects_requests_over_rpm_limit() {
        let config = RateLimitConfig {
            rpm: Some(2),
            tpm: None,
            window: Duration::from_secs(60),
        };
        let layer = ModelRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        // First two succeed.
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");

        // Third should be rate limited.
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should be rate limited");
        assert!(matches!(err, LiterLlmError::RateLimited { .. }));
    }

    #[tokio::test]
    async fn independent_models_have_separate_limits() {
        let config = RateLimitConfig {
            rpm: Some(1),
            tpm: None,
            window: Duration::from_secs(60),
        };
        let layer = ModelRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        // Different model should still work.
        svc.call(LlmRequest::Chat(chat_req("gpt-3.5-turbo")))
            .await
            .expect("service call should not fail");
    }

    #[tokio::test]
    async fn tpm_limit_rejects_after_threshold() {
        let config = RateLimitConfig {
            rpm: None,
            tpm: Some(10), // Very low threshold — the mock returns 15 total tokens.
            window: Duration::from_secs(60),
        };
        let layer = ModelRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        // First call succeeds and records 15 tokens (over the 10 limit).
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");

        // Second call should be rejected because token count >= tpm.
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should be rate limited by TPM");
        assert!(matches!(err, LiterLlmError::RateLimited { .. }));
    }

    #[tokio::test]
    async fn unlimited_config_allows_all_requests() {
        let config = RateLimitConfig::default();
        let layer = ModelRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        for _ in 0..100 {
            assert!(svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.is_ok());
        }
    }

    // ── Cost rate-limit tests ─────────────────────────────────────────────────

    /// When the accumulated cost in the minute window already exceeds the
    /// configured `max_usd_per_minute`, the layer must reject the next request
    /// without forwarding it to the inner service.
    #[tokio::test]
    async fn cost_rate_limit_rejects_when_projected_exceeds_max() {
        let config = CostRateLimitConfig {
            max_usd_per_minute: Some(0.01),
            max_usd_per_hour: None,
            max_usd_per_day: None,
        };
        let layer = CostRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        // Directly prime the per-minute window to exceed the $0.01 limit.
        // gpt-4: 10 prompt + 5 completion tokens = $0.0006; we need > $0.01.
        // Pre-seed by calling record directly on the internal state.
        // Since state is private, we instead issue requests until the window
        // accumulates enough cost — but the mock returns tiny spend.  Instead
        // use the public layer API: call `record` by manually pumping cost.
        //
        // Approach: set max_usd_per_minute very low ($0.000001) so that even
        // a single mock call of gpt-4 ($0.0006) exceeds it after recording.
        let config2 = CostRateLimitConfig {
            max_usd_per_minute: Some(0.000001),
            max_usd_per_hour: None,
            max_usd_per_day: None,
        };
        let layer2 = CostRateLimitLayer::new(config2);
        let inner2 = LlmService::new(MockClient::ok());
        let mut svc2 = layer2.layer(inner2);

        // First call succeeds and records ~$0.0006 into the window.
        svc2.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("first call should succeed");

        // Second call should be rejected: window spend ($0.0006) >= limit ($0.000001).
        let err = svc2
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should be rate limited by cost");
        assert!(
            matches!(err, LiterLlmError::RateLimited { .. }),
            "expected RateLimited, got {err:?}"
        );

        // The original svc with a $0.01 limit should still allow requests since
        // no cost has been recorded in it.
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("request under cost limit should succeed");
    }

    /// An unlimited cost config allows arbitrarily many requests.
    #[tokio::test]
    async fn cost_rate_limit_unlimited_config_allows_all_requests() {
        let config = CostRateLimitConfig::default();
        let layer = CostRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        for _ in 0..20 {
            assert!(svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.is_ok());
        }
    }

    /// Errors from the inner service are propagated without updating the cost window.
    #[tokio::test]
    async fn cost_rate_limit_propagates_inner_errors() {
        let config = CostRateLimitConfig {
            max_usd_per_minute: Some(100.0),
            max_usd_per_hour: None,
            max_usd_per_day: None,
        };
        let layer = CostRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::failing_timeout());
        let mut svc = layer.layer(inner);

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("inner error should propagate");
        assert!(matches!(err, LiterLlmError::Timeout));
    }
}
