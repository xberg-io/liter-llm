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

use super::cost::observe_stream_usage;
use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::cost;
use crate::error::{LiterLlmError, Result};
use crate::types::Usage;

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

            entry.request_count += 1;
        }

        let fut = self.inner.call(req);

        Box::pin(async move {
            let resp = fut.await?;

            match resp {
                // ~keep LlmResponse::usage() always returns None for ChatStream; record once the stream
                // ~keep completes instead of silently skipping every streamed request's token count.
                LlmResponse::ChatStream(stream) => {
                    let model_for_completion = model.clone();
                    let state_for_completion = Arc::clone(&state);
                    let config_for_completion = config.clone();
                    let wrapped = observe_stream_usage(stream, move |usage| {
                        record_tokens(
                            &state_for_completion,
                            &config_for_completion,
                            &model_for_completion,
                            usage.as_ref(),
                        );
                    });
                    Ok(LlmResponse::ChatStream(wrapped))
                }
                other => {
                    record_tokens(&state, &config, &model, other.usage());
                    Ok(other)
                }
            }
        })
    }
}

/// Add `usage`'s token count to `model`'s rate-limit window, resetting the
/// window first if it has elapsed. No-op when `usage` is `None`.
///
/// Shared by the non-streaming response path (usage known immediately) and
/// the `ChatStream` completion callback (usage only known once the stream is
/// fully consumed).
fn record_tokens(
    state: &DashMap<String, ModelRateState>,
    config: &RateLimitConfig,
    model: &str,
    usage: Option<&Usage>,
) {
    let Some(usage) = usage else { return };
    let total_tokens = usage.prompt_tokens + usage.completion_tokens;
    if let Some(mut entry) = state.get_mut(model) {
        entry.maybe_reset(config.window);
        entry.token_count += total_tokens;
    }
}

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
    ///
    /// Uses a `compare_exchange` CAS so that under concurrent calls exactly
    /// one thread wins the rollover, mirroring
    /// [`super::budget::WindowEntry::spend_usd`]. The previous
    /// unconditional `store(0, ..)` let every thread that observed an
    /// elapsed window reset the counter independently — two threads racing
    /// at the boundary could each zero `spend_mc` after the other's
    /// `fetch_add` in [`Self::add`] had already landed, silently dropping
    /// that contribution (a torn read/write, not just a redundant reset).
    fn spend_usd(&self, now_secs: u64) -> f64 {
        let start = self.window_start_secs.load(Ordering::Acquire);
        if now_secs.saturating_sub(start) >= self.window_secs {
            // ~keep Snapshot before CAS so racing increments after this point are preserved.
            let old_mc = self.spend_mc.load(Ordering::Acquire);

            // ~keep Only the CAS winner performs rollover; losers keep the winner's reset.
            if self
                .window_start_secs
                .compare_exchange(start, now_secs, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // ~keep Subtract only the old-window amount so new-window racing increments remain.
                self.spend_mc.fetch_sub(old_mc, Ordering::AcqRel);
            }
        }
        self.spend_mc.load(Ordering::Acquire) as f64 / 1_000_000.0
    }

    /// Add `usd` to the window accumulator.
    fn add(&self, usd: f64, now_secs: u64) {
        let _ = self.spend_usd(now_secs);
        if usd > 0.0 {
            let mc = (usd * 1_000_000.0).round() as u64;
            self.spend_mc.fetch_add(mc, Ordering::AcqRel);
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

        if let Some(err) = state.check(&config) {
            return Box::pin(async move { Err(err) });
        }

        let fut = self.inner.call(req);

        Box::pin(async move {
            let resp = fut.await?;

            match resp {
                // ~keep LlmResponse::usage() always returns None for ChatStream; record once the stream
                // ~keep completes instead of silently skipping every streamed request's cost.
                LlmResponse::ChatStream(stream) => {
                    let model_for_completion = model.clone();
                    let state_for_completion = Arc::clone(&state);
                    let wrapped = observe_stream_usage(stream, move |usage| {
                        record_cost_window(&state_for_completion, &model_for_completion, usage.as_ref());
                    });
                    Ok(LlmResponse::ChatStream(wrapped))
                }
                other => {
                    record_cost_window(&state, &model, other.usage());
                    Ok(other)
                }
            }
        })
    }
}

/// Compute the cost of `usage` and record it into `state`'s minute/hour/day
/// windows. No-op when `usage` is `None` or the model has no pricing data.
///
/// Shared by the non-streaming response path (usage known immediately) and
/// the `ChatStream` completion callback (usage only known once the stream is
/// fully consumed).
fn record_cost_window(state: &CostRateLimitState, model: &str, usage: Option<&Usage>) {
    let Some(usage) = usage else { return };
    if let Some(usd) = cost::completion_cost(model, usage.prompt_tokens, usage.completion_tokens) {
        state.record(usd);
    }
}

#[cfg(test)]
mod tests {

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

        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");

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
        svc.call(LlmRequest::Chat(chat_req("gpt-3.5-turbo")))
            .await
            .expect("service call should not fail");
    }

    #[tokio::test]
    async fn tpm_limit_rejects_after_threshold() {
        let config = RateLimitConfig {
            rpm: None,
            tpm: Some(10),
            window: Duration::from_secs(60),
        };
        let layer = ModelRateLimitLayer::new(config);
        let inner = LlmService::new(MockClient::ok());
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");

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

        let config2 = CostRateLimitConfig {
            max_usd_per_minute: Some(0.000001),
            max_usd_per_hour: None,
            max_usd_per_day: None,
        };
        let layer2 = CostRateLimitLayer::new(config2);
        let inner2 = LlmService::new(MockClient::ok());
        let mut svc2 = layer2.layer(inner2);

        svc2.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("first call should succeed");

        let err = svc2
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should be rate limited by cost");
        assert!(
            matches!(err, LiterLlmError::RateLimited { .. }),
            "expected RateLimited, got {err:?}"
        );

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

    /// Regression for the `CostWindow` rollover race: 200 parallel `add($0.10)`
    /// calls at a rollover boundary must total exactly $20.00. Before the CAS
    /// fix, `spend_usd` reset the counter unconditionally
    /// (`self.spend_mc.store(0, ..)`) whenever it observed an elapsed window,
    /// so two threads racing at the boundary could each zero the counter
    /// independently, silently dropping any `fetch_add` from [`CostWindow::add`]
    /// that landed between the two resets.
    #[test]
    fn cost_window_rollover_under_concurrent_threads_does_not_undercount() {
        use std::sync::Barrier;
        use std::thread;

        let window = Arc::new(CostWindow::new(Duration::from_secs(1)));
        let future_now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + 2;

        const WRITERS: usize = 200;
        let barrier = Arc::new(Barrier::new(WRITERS));
        let mut handles = Vec::with_capacity(WRITERS);
        for _ in 0..WRITERS {
            let w = Arc::clone(&window);
            let b = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                b.wait();
                w.add(0.10, future_now);
            }));
        }
        for h in handles {
            h.join().expect("writer must not panic");
        }

        let total = window.spend_mc.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        assert!(
            (total - 20.0_f64).abs() < 1e-4,
            "expected $20.00 total after 200 concurrent adds at rollover; got ${total:.6}"
        );
    }

    use std::collections::VecDeque;

    use futures_core::Stream;

    use crate::client::BoxStream;
    use crate::types::ChatCompletionChunk;

    struct ChunkStream(VecDeque<ChatCompletionChunk>);
    impl Stream for ChunkStream {
        type Item = Result<ChatCompletionChunk>;
        fn poll_next(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            std::task::Poll::Ready(self.0.pop_front().map(Ok))
        }
    }

    fn usage_chunk(usage: Option<Usage>) -> ChatCompletionChunk {
        ChatCompletionChunk {
            id: "chunk".into(),
            object: "chat.completion.chunk".into(),
            created: 0,
            model: "gpt-4".into(),
            choices: vec![],
            usage,
            system_fingerprint: None,
            service_tier: None,
        }
    }

    #[derive(Clone)]
    struct StreamingUsageService;
    impl tower::Service<LlmRequest> for StreamingUsageService {
        type Response = LlmResponse;
        type Error = LiterLlmError;
        type Future = BoxFuture<'static, Result<LlmResponse>>;
        fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }
        fn call(&mut self, _req: LlmRequest) -> Self::Future {
            Box::pin(async move {
                let usage = Usage {
                    prompt_tokens: 30,
                    completion_tokens: 20,
                    total_tokens: 50,
                    prompt_tokens_details: None,
                    completion_tokens_details: None,
                };
                let chunks = VecDeque::from([usage_chunk(None), usage_chunk(Some(usage))]);
                let stream: BoxStream<'static, Result<ChatCompletionChunk>> = Box::pin(ChunkStream(chunks));
                Ok(LlmResponse::ChatStream(stream))
            })
        }
    }

    /// Regression for the "streaming bypasses rate-limit accounting" bug:
    /// `LlmResponse::usage()` always returns `None` for `ChatStream`, so a
    /// naive post-response check never sees a streamed call's token count and
    /// `ModelRateLimitService` never updates its TPM window for it — a caller
    /// could stream unlimited tokens through a TPM-limited model.
    #[tokio::test]
    async fn model_rate_limit_records_tokens_for_streamed_response() {
        use futures_util::StreamExt as _;

        let config = RateLimitConfig {
            rpm: None,
            tpm: Some(50),
            window: Duration::from_secs(60),
        };
        let layer = ModelRateLimitLayer::new(config);
        let mut svc = layer.layer(StreamingUsageService);

        let resp = svc
            .call(LlmRequest::ChatStream(chat_req("gpt-4")))
            .await
            .expect("streamed call should succeed");
        let LlmResponse::ChatStream(mut stream) = resp else {
            panic!("expected a ChatStream response");
        };
        while stream.next().await.is_some() {}

        // The TPM window is now at exactly its 50-token limit; a second call must be rejected.
        let err = svc
            .call(LlmRequest::ChatStream(chat_req("gpt-4")))
            .await
            .expect_err("second call must be rejected once the streamed call's 50 tokens are recorded");
        assert!(
            matches!(err, LiterLlmError::RateLimited { .. }),
            "expected RateLimited once streamed tokens push the window to its TPM limit; got {err:?}"
        );
    }
}
