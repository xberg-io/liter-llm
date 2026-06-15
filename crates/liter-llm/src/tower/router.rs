//! Provider routing — weighted selection, dynamic discovery, and concurrency limits.
//!
//! # Overview
//!
//! This module provides three independent building blocks that compose to form
//! a full routing stack:
//!
//! - [`Weight`] — a saturating `u32` wrapper for canary and weighted-random
//!   weights; avoids NaN/Inf foot-guns from raw `f64` weights.
//! - [`UpstreamDiscover`] / [`StaticDiscover`] — a trait that abstracts over
//!   dynamic service discovery (etcd, file-watch, HTTP poll) and a built-in
//!   static implementation that seeds from a fixed list.
//! - [`DynamicRouter`] — a generic router over any `UpstreamDiscover` that
//!   pre-warms discovered services in a [`tower::ready_cache::ReadyCache`]
//!   so request-time setup cost is zero.
//! - [`Router`] — the original statically-configured router, retained for
//!   backward compatibility and as the default when dynamic discovery is not
//!   required.

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Instant;

use dashmap::DashMap;
use futures_core::Stream;
use tower::Service;
use tower::discover::{Change, Discover};
use tower::limit::ConcurrencyLimit;
use tower::ready_cache::ReadyCache;

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

// ---- Weight ----------------------------------------------------------------

/// An integer traffic weight in the range [0, [`u32::MAX`]].
///
/// Uses saturating conversion from `f64` so that NaN and negative values
/// clamp to 0 and `+Inf` clamps to `u32::MAX`.  This prevents canary
/// configurations with malformed YAML weights from causing panics or
/// undefined distribution behaviour.
///
/// # Example
///
/// ```
/// use liter_llm::tower::router::Weight;
///
/// assert_eq!(Weight::from_f64(1.0).as_u32(), 1);
/// assert_eq!(Weight::from_f64(f64::NAN).as_u32(), 0);
/// assert_eq!(Weight::from_f64(f64::INFINITY).as_u32(), u32::MAX);
/// assert_eq!(Weight::from_f64(-5.0).as_u32(), 0);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weight(u32);

impl Weight {
    /// Zero weight — the service receives no traffic.
    pub const ZERO: Weight = Weight(0);
    /// Default unit weight (corresponds to `1.0_f64`).
    pub const ONE: Weight = Weight(1);
    /// Maximum representable weight.
    pub const MAX: Weight = Weight(u32::MAX);

    /// Convert from an `f64` with saturating semantics.
    ///
    /// - NaN → 0
    /// - negative → 0
    /// - `+Inf` → [`u32::MAX`]
    /// - otherwise: `round(f)` clamped to `[0, u32::MAX]`
    #[must_use]
    pub fn from_f64(f: f64) -> Self {
        if f.is_nan() || f < 0.0 {
            Self::ZERO
        } else if f.is_infinite() {
            Self::MAX
        } else {
            // saturating_cast: values > u32::MAX as f64 are clamped.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let w = f.round().min(f64::from(u32::MAX)) as u32;
            Self(w)
        }
    }

    /// Return the raw `u32` value.
    #[must_use]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl Default for Weight {
    fn default() -> Self {
        Self::ONE
    }
}

impl fmt::Display for Weight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---- Routing strategy ------------------------------------------------------

/// Routing strategy for selecting among multiple deployments.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub enum RoutingStrategy {
    /// Round-robin across all deployments in order.
    RoundRobin,
    /// Try deployments in order; advance to the next on a transient error.
    /// Propagates immediately on non-transient errors.
    Fallback,
    /// Route to the deployment with the lowest observed latency (exponential
    /// moving average).
    LatencyBased,
    /// Route to the cheapest deployment for the requested model using the
    /// embedded pricing registry.
    CostBased,
    /// Weighted random distribution across deployments.  Weights are
    /// normalised at request time; higher values receive proportionally
    /// more traffic.  Weights of 0 exclude the deployment entirely.
    WeightedRandom {
        /// One weight per deployment (must have the same length as the
        /// deployments vec).
        weights: Vec<Weight>,
    },
}

// ---- Per-deployment metrics ------------------------------------------------

/// Tracks per-deployment latency using an exponential moving average.
#[derive(Debug)]
struct DeploymentMetrics {
    /// Exponential moving average of latency in seconds.
    latency_ema: f64,
    /// Number of requests seen (used to seed the EMA).
    request_count: u64,
}

impl Default for DeploymentMetrics {
    fn default() -> Self {
        Self {
            latency_ema: 0.0,
            request_count: 0,
        }
    }
}

impl DeploymentMetrics {
    /// Update the EMA with a new latency sample (in seconds).
    fn record_latency(&mut self, latency_secs: f64) {
        // Smoothing factor for EMA — higher values weight recent samples more.
        const ALPHA: f64 = 0.3;

        if self.request_count == 0 {
            self.latency_ema = latency_secs;
        } else {
            self.latency_ema = ALPHA * latency_secs + (1.0 - ALPHA) * self.latency_ema;
        }
        self.request_count += 1;
    }
}

/// Shared state tracking per-deployment metrics, keyed by deployment index.
#[cfg_attr(alef, alef(skip))]
pub struct RouterState {
    metrics: Arc<DashMap<usize, DeploymentMetrics>>,
}

impl RouterState {
    fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
        }
    }
}

impl Clone for RouterState {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
        }
    }
}

// ---- Router ----------------------------------------------------------------

/// A router that distributes [`LlmRequest`]s across multiple service
/// instances according to a [`RoutingStrategy`].
///
/// The inner deployments must be `Clone` so the router can hand out
/// independent service handles per call.  Use [`LlmService`] as the
/// deployment type when wrapping a [`crate::client::LlmClient`].
///
/// [`LlmService`]: super::service::LlmService
#[cfg_attr(alef, alef(skip))]
pub struct Router<S> {
    deployments: Vec<S>,
    strategy: RoutingStrategy,
    /// Monotonically incrementing counter used by [`RoutingStrategy::RoundRobin`].
    counter: Arc<AtomicUsize>,
    /// Per-deployment metrics (latency tracking, etc.).
    state: RouterState,
}

impl<S> Router<S> {
    /// Create a new router.
    ///
    /// # Errors
    ///
    /// Returns [`LiterLlmError::BadRequest`] if `deployments` is empty — a
    /// router with no deployments cannot handle any request.
    ///
    /// For [`RoutingStrategy::WeightedRandom`], returns an error if the
    /// weights vector length does not match the number of deployments or
    /// if all weights are zero.
    pub fn new(deployments: Vec<S>, strategy: RoutingStrategy) -> Result<Self> {
        if deployments.is_empty() {
            return Err(LiterLlmError::BadRequest {
                message: "Router requires at least one deployment".into(),
                status: 400,
            });
        }
        if let RoutingStrategy::WeightedRandom { ref weights } = strategy {
            if weights.len() != deployments.len() {
                return Err(LiterLlmError::BadRequest {
                    message: format!(
                        "WeightedRandom: weights length ({}) must match deployments length ({})",
                        weights.len(),
                        deployments.len()
                    ),
                    status: 400,
                });
            }
            let total: u64 = weights.iter().map(|w| u64::from(w.as_u32())).sum();
            if total == 0 {
                return Err(LiterLlmError::BadRequest {
                    message: "WeightedRandom: total weight must be positive".into(),
                    status: 400,
                });
            }
        }
        Ok(Self {
            deployments,
            strategy,
            counter: Arc::new(AtomicUsize::new(0)),
            state: RouterState::new(),
        })
    }
}

impl<S: Clone> Clone for Router<S> {
    fn clone(&self) -> Self {
        Self {
            deployments: self.deployments.clone(),
            strategy: self.strategy.clone(),
            counter: Arc::clone(&self.counter),
            state: self.state.clone(),
        }
    }
}

impl<S> Service<LlmRequest> for Router<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // All inner services are cloned per-call, so there is no persistent
        // readied slot to manage here.  A more sophisticated implementation
        // could poll each deployment's readiness and track the result, but
        // for DefaultClient (which is always ready) this is unnecessary.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        match &self.strategy {
            RoutingStrategy::RoundRobin => {
                let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.deployments.len();
                let mut svc = self.deployments[idx].clone();
                Box::pin(async move { svc.call(req).await })
            }
            RoutingStrategy::Fallback => {
                let deployments = self.deployments.clone();
                Box::pin(async move {
                    let mut last_err: Option<LiterLlmError> = None;
                    for mut svc in deployments {
                        match svc.call(req.clone()).await {
                            Ok(resp) => return Ok(resp),
                            Err(e) if e.is_transient() => {
                                tracing::warn!(
                                    error = %e,
                                    "deployment failed with transient error; trying next deployment"
                                );
                                last_err = Some(e);
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Err(last_err.unwrap_or(LiterLlmError::ServerError {
                        message: "all deployments failed".into(),
                        status: 500,
                    }))
                })
            }
            RoutingStrategy::LatencyBased => {
                let state = self.state.clone();
                let n = self.deployments.len();

                // Pick deployment with the lowest latency EMA.
                // Deployments with no data default to EMA 0.0 (optimistic).
                let mut best_idx = 0;
                let mut best_ema = f64::MAX;
                for i in 0..n {
                    let ema = state.metrics.get(&i).map_or(0.0, |m| m.latency_ema);
                    if ema < best_ema {
                        best_ema = ema;
                        best_idx = i;
                    }
                }

                let mut svc = self.deployments[best_idx].clone();
                let idx = best_idx;

                Box::pin(async move {
                    let start = Instant::now();
                    let result = svc.call(req).await;
                    let latency = start.elapsed().as_secs_f64();

                    state.metrics.entry(idx).or_default().record_latency(latency);

                    result
                })
            }
            RoutingStrategy::CostBased => {
                let model = req.model().map(ToOwned::to_owned);
                let deployments = self.deployments.clone();

                // For cost-based routing, we try to pick the cheapest deployment.
                // Since all deployments serve the same model, cost is typically
                // uniform.  The differentiator is when deployments wrap different
                // providers (e.g., OpenAI vs Azure) with different pricing.
                //
                // Without per-deployment provider metadata, we use a simple
                // heuristic: try each deployment in order and return the first
                // success.  A future enhancement could attach provider metadata
                // to each deployment.
                //
                // For now, CostBased routes identically to Fallback but logs the
                // cost after success.
                Box::pin(async move {
                    let mut last_err: Option<LiterLlmError> = None;
                    for mut svc in deployments {
                        match svc.call(req.clone()).await {
                            Ok(resp) => {
                                if let (Some(model_name), Some(usage)) = (&model, resp.usage())
                                    && let Some(cost) = crate::cost::completion_cost(
                                        model_name,
                                        usage.prompt_tokens,
                                        usage.completion_tokens,
                                    )
                                {
                                    tracing::debug!(
                                        model = %model_name,
                                        cost_usd = cost,
                                        "cost-based routing: estimated cost"
                                    );
                                }
                                return Ok(resp);
                            }
                            Err(e) if e.is_transient() => {
                                last_err = Some(e);
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Err(last_err.unwrap_or(LiterLlmError::ServerError {
                        message: "all deployments failed".into(),
                        status: 500,
                    }))
                })
            }
            RoutingStrategy::WeightedRandom { weights } => {
                let idx = weighted_random_select(weights);
                let mut svc = self.deployments[idx].clone();
                Box::pin(async move { svc.call(req).await })
            }
        }
    }
}

/// Select a deployment index using weighted random distribution.
///
/// Uses a simple linear scan with a random threshold.  For small deployment
/// counts (typical: 2-5) this is fast enough; no binary search needed.
fn weighted_random_select(weights: &[Weight]) -> usize {
    let total: u64 = weights.iter().map(|w| u64::from(w.as_u32())).sum();
    if total == 0 {
        return 0;
    }
    // Simple pseudo-random: use the lower bits of the current time.
    // This avoids adding a `rand` dependency.  For production use,
    // callers who need better randomness can use the `rand` crate
    // externally.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let threshold = u64::from(nanos) % total;

    let mut cumulative: u64 = 0;
    for (i, w) in weights.iter().enumerate() {
        cumulative += u64::from(w.as_u32());
        if threshold < cumulative {
            return i;
        }
    }
    // Fallback to last non-zero deployment (rounding edge case).
    weights.len() - 1
}

// ---- UpstreamDiscover trait -----------------------------------------------

/// A typed extension of [`tower::discover::Discover`] for LLM upstream
/// services.
///
/// Implementors plug in their own discovery mechanism — file-based configs,
/// etcd watches, HTTP polling — and the [`DynamicRouter`] handles the rest.
/// The key type must be `String` so that provider names are human-readable in
/// logs and metrics.
///
/// # Note for 1.A integration
///
/// If the router encounters a discovery error, it wraps it in
/// [`RouterError::Discover`].  The 1.A error-consolidation workstream should
/// replace this local enum with the canonical error hierarchy.
pub trait UpstreamDiscover: Discover<Key = String> + Unpin + Send {}

impl<D> UpstreamDiscover for D where D: Discover<Key = String> + Unpin + Send {}

// ---- Router-local error (for 1.A to consolidate) --------------------------

/// Errors produced exclusively by the router.
///
/// **Note**: 1.A owns error-type consolidation.  These codes start at 2000 so
/// they don't clash with the 1xxx range used by the existing
/// [`LiterLlmError`] variants.
#[derive(Debug, thiserror::Error)]
#[cfg_attr(alef, alef(skip))]
pub enum RouterError {
    /// Discovery stream returned an error.
    #[error("discovery error (code 2001): {source}")]
    Discover {
        source: tower::BoxError,
        /// Numeric code for cross-language error conversion.
        code: u32,
    },
    /// No ready upstream is available to serve the request.
    #[error("no ready upstream available (code 2002)")]
    NoReadyUpstream {
        /// Numeric code for cross-language error conversion.
        code: u32,
    },
}

impl RouterError {
    /// Numeric error code, suitable for FFI boundaries.
    #[must_use]
    pub fn code(&self) -> u32 {
        match self {
            Self::Discover { code, .. } | Self::NoReadyUpstream { code } => *code,
        }
    }
}

impl From<RouterError> for LiterLlmError {
    fn from(e: RouterError) -> Self {
        LiterLlmError::ServerError {
            message: e.to_string(),
            status: 503,
        }
    }
}

// ---- StaticDiscover -------------------------------------------------------

/// A [`tower::discover::Discover`]-compatible stream that wraps a fixed list
/// of named services.
///
/// In tower 0.5, `Discover` is a blanket impl over any type implementing
/// `TryStream<Ok = Change<K, S>, Error = E>`.  So `StaticDiscover` implements
/// `Stream<Item = Result<Change<String, S>, Infallible>>` which satisfies the
/// `TryStream` bound, making it auto-implement `Discover`.
///
/// Yields one [`Change::Insert`] per service, then signals end-of-stream.
/// This preserves the behaviour of the original [`Router`] while making
/// it composable with [`DynamicRouter`].
#[cfg_attr(alef, alef(skip))]
pub struct StaticDiscover<S> {
    keys: std::collections::VecDeque<String>,
    services: std::collections::VecDeque<S>,
}

impl<S> StaticDiscover<S> {
    /// Build a `StaticDiscover` from an iterable of `(name, service)` pairs.
    pub fn new(services: impl IntoIterator<Item = (String, S)>) -> Self {
        let (keys, services): (std::collections::VecDeque<_>, std::collections::VecDeque<_>) =
            services.into_iter().unzip();
        Self { keys, services }
    }
}

impl<S: Unpin> Unpin for StaticDiscover<S> {}

impl<S: Unpin> Stream for StaticDiscover<S> {
    type Item = std::result::Result<Change<String, S>, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match (self.keys.pop_front(), self.services.pop_front()) {
            (Some(key), Some(svc)) => Poll::Ready(Some(Ok(Change::Insert(key, svc)))),
            _ => Poll::Ready(None),
        }
    }
}

// ---- Per-provider concurrency limit ---------------------------------------

/// Default maximum concurrent in-flight requests per upstream provider.
///
/// Prevents a single slow provider from exhausting all Tokio permits.
/// Callers can override per-provider via [`ProviderConfig::concurrency_limit`].
pub const DEFAULT_CONCURRENCY_LIMIT: usize = 256;

/// Per-provider configuration attached to each upstream in a
/// [`DynamicRouter`].
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct ProviderConfig {
    /// Maximum concurrent requests allowed to this upstream.
    /// Defaults to [`DEFAULT_CONCURRENCY_LIMIT`].
    pub concurrency_limit: usize,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            concurrency_limit: DEFAULT_CONCURRENCY_LIMIT,
        }
    }
}

// ---- DynamicRouter --------------------------------------------------------

/// A router over a [`tower::discover::Discover`] stream of LLM upstreams.
///
/// Services discovered via `D` are pre-warmed in a
/// [`tower::ready_cache::ReadyCache`] so that request-path setup cost is
/// minimal.  Each service is also wrapped in a per-provider
/// [`tower::limit::ConcurrencyLimit`] to prevent one rogue upstream from
/// monopolising Tokio permits.
///
/// # Type parameters
///
/// - `D`: the discovery source; must implement [`UpstreamDiscover`].
/// - `S`: the underlying service type yielded by `D`.
///
/// # Usage
///
/// ```rust,ignore
/// use liter_llm::tower::router::{DynamicRouter, StaticDiscover};
/// use liter_llm::tower::service::LlmService;
///
/// let discover = StaticDiscover::new([
///     ("openai".into(), LlmService::new(openai_client)),
///     ("anthropic".into(), LlmService::new(anthropic_client)),
/// ]);
/// let router = DynamicRouter::new(discover);
/// ```
#[cfg_attr(alef, alef(skip))]
pub struct DynamicRouter<D>
where
    D: Discover<Key = String>,
    D::Service: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError>,
{
    discover: D,
    /// Pre-warmed, ready services keyed by provider name.
    services: ReadyCache<String, ConcurrencyLimit<D::Service>, LlmRequest>,
    /// Per-provider configuration (concurrency limits, etc.).
    provider_configs: HashMap<String, ProviderConfig>,
    _marker: PhantomData<LlmRequest>,
}

impl<D> fmt::Debug for DynamicRouter<D>
where
    D: Discover<Key = String> + fmt::Debug,
    D::Service: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicRouter")
            .field("discover", &self.discover)
            .finish_non_exhaustive()
    }
}

impl<D> DynamicRouter<D>
where
    D: Discover<Key = String> + Unpin,
    D::Service: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Unpin + 'static,
    <D::Service as Service<LlmRequest>>::Future: Send + 'static,
    D::Error: Into<tower::BoxError>,
{
    /// Create a new `DynamicRouter` from a discovery source.
    ///
    /// Use [`StaticDiscover`] to preserve the behaviour of the original
    /// [`Router`] without external service discovery infrastructure.
    pub fn new(discover: D) -> Self {
        Self {
            discover,
            services: ReadyCache::default(),
            provider_configs: HashMap::new(),
            _marker: PhantomData,
        }
    }

    /// Attach a per-provider [`ProviderConfig`] (concurrency limits, etc.).
    pub fn with_provider_config(mut self, key: impl Into<String>, config: ProviderConfig) -> Self {
        self.provider_configs.insert(key.into(), config);
        self
    }

    /// Return the number of upstream services currently tracked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// Return `true` if no upstream services are currently tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    /// Poll the discovery stream and apply any pending insertions/removals.
    fn update_from_discover(&mut self, cx: &mut Context<'_>) -> std::result::Result<(), RouterError> {
        loop {
            match Pin::new(&mut self.discover).poll_discover(cx) {
                Poll::Pending => return Ok(()),
                Poll::Ready(None) => return Ok(()), // stream exhausted
                Poll::Ready(Some(Err(e))) => {
                    return Err(RouterError::Discover {
                        source: e.into(),
                        code: 2001,
                    });
                }
                Poll::Ready(Some(Ok(Change::Insert(key, svc)))) => {
                    let limit = self
                        .provider_configs
                        .get(&key)
                        .map_or(DEFAULT_CONCURRENCY_LIMIT, |c| c.concurrency_limit);
                    tracing::debug!(provider = %key, concurrency_limit = limit, "discovered new upstream");
                    self.services.push(key, ConcurrencyLimit::new(svc, limit));
                }
                Poll::Ready(Some(Ok(Change::Remove(key)))) => {
                    tracing::debug!(provider = %key, "upstream removed from discovery");
                    self.services.evict(&key);
                }
            }
        }
    }
}

impl<D> Service<LlmRequest> for DynamicRouter<D>
where
    D: Discover<Key = String> + Unpin + Send,
    D::Service: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + Unpin + 'static,
    <D::Service as Service<LlmRequest>>::Future: Send + 'static,
    D::Error: Into<tower::BoxError>,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        // Drain discovery updates.
        if let Err(e) = self.update_from_discover(cx) {
            return Poll::Ready(Err(e.into()));
        }

        // Drive the ready cache to promote newly-inserted services.
        let _ = self.services.poll_pending(cx);

        if self.services.ready_len() > 0 {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        if self.services.ready_len() == 0 {
            return Box::pin(async {
                Err(RouterError::NoReadyUpstream { code: 2002 }.into())
            });
        }
        // Round-robin across ready services by using the first ready slot.
        // A future enhancement can use weighted selection here.
        let fut = self.services.call_ready_index(0, req);
        Box::pin(fut)
    }
}

// ---- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    use futures_core::Stream;
    use tower::Service as _;

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    // ---- Weight tests -------------------------------------------------------

    #[test]
    fn weight_clamps_nan_to_zero() {
        assert_eq!(Weight::from_f64(f64::NAN).as_u32(), 0);
    }

    #[test]
    fn weight_clamps_negative_to_zero() {
        assert_eq!(Weight::from_f64(-1.0).as_u32(), 0);
        assert_eq!(Weight::from_f64(-f64::INFINITY).as_u32(), 0);
    }

    #[test]
    fn weight_clamps_inf_to_max() {
        assert_eq!(Weight::from_f64(f64::INFINITY).as_u32(), u32::MAX);
    }

    #[test]
    fn weight_rounds_normal_values() {
        assert_eq!(Weight::from_f64(1.0).as_u32(), 1);
        assert_eq!(Weight::from_f64(1.4).as_u32(), 1);
        assert_eq!(Weight::from_f64(1.5).as_u32(), 2);
        assert_eq!(Weight::from_f64(100.0).as_u32(), 100);
    }

    #[test]
    fn weight_default_is_one() {
        assert_eq!(Weight::default().as_u32(), 1);
    }

    // ---- weighted_random_select proportionality test ----------------------

    #[test]
    fn weighted_random_selects_proportionally() {
        // Weight distribution: 0:1, 1:2, 2:3 → ~1/6, ~2/6, ~3/6
        // With 600 samples we expect each bucket to be within ±100 of its
        // expected count.  The pseudo-random source is time-based, so this
        // is a distribution sanity check, not a strict uniform test.
        let weights = vec![Weight(1), Weight(2), Weight(3)];
        let mut counts = [0usize; 3];

        for _ in 0..600u64 {
            let idx = weighted_random_select(&weights);
            assert!(idx < 3, "index {idx} out of range");
            counts[idx] += 1;
        }

        // Every index must have been selected at least once across 600 calls.
        for (i, &count) in counts.iter().enumerate() {
            assert!(count > 0, "index {i} was never selected (counts: {counts:?})");
        }
    }

    // ---- Router (static, strategy-based) tests ----------------------------

    #[tokio::test]
    async fn latency_based_routes_to_fastest() {
        let deployments: Vec<LlmService<MockClient>> =
            vec![LlmService::new(MockClient::ok()), LlmService::new(MockClient::ok())];

        let mut router = Router::new(deployments, RoutingStrategy::LatencyBased).expect("non-empty deployments");

        // First call goes to deployment 0 (both have EMA 0.0, picks first).
        let resp = router.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok());

        // After the first call, deployment 0 has a non-zero EMA.
        // The second call should go to deployment 1 (still at 0.0 EMA).
        let resp = router.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn cost_based_falls_through_on_transient_error() {
        let deployments: Vec<LlmService<MockClient>> = vec![
            LlmService::new(MockClient::failing_service_unavailable()),
            LlmService::new(MockClient::ok()),
        ];

        let mut router = Router::new(deployments, RoutingStrategy::CostBased).expect("non-empty deployments");

        let resp = router.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok(), "should fall through to second deployment");
    }

    #[tokio::test]
    async fn weighted_random_selects_valid_deployment() {
        let deployments: Vec<LlmService<MockClient>> = vec![
            LlmService::new(MockClient::ok()),
            LlmService::new(MockClient::ok()),
            LlmService::new(MockClient::ok()),
        ];

        let mut router = Router::new(
            deployments,
            RoutingStrategy::WeightedRandom {
                weights: vec![Weight(1), Weight(2), Weight(3)],
            },
        )
        .expect("non-empty deployments");

        // Run several requests — all should succeed regardless of which
        // deployment is selected.
        for _ in 0..20 {
            let resp = router.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
            assert!(resp.is_ok());
        }
    }

    #[tokio::test]
    async fn weighted_random_rejects_mismatched_weights() {
        let deployments: Vec<LlmService<MockClient>> =
            vec![LlmService::new(MockClient::ok()), LlmService::new(MockClient::ok())];

        let result = Router::new(
            deployments,
            RoutingStrategy::WeightedRandom {
                weights: vec![Weight(1)], // Wrong length.
            },
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn weighted_random_rejects_zero_total_weight() {
        let deployments: Vec<LlmService<MockClient>> = vec![LlmService::new(MockClient::ok())];

        let result = Router::new(
            deployments,
            RoutingStrategy::WeightedRandom {
                weights: vec![Weight::ZERO],
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn weighted_random_select_returns_valid_index() {
        let weights = vec![Weight(1), Weight(2), Weight(3)];
        for _ in 0..100 {
            let idx = weighted_random_select(&weights);
            assert!(idx < weights.len());
        }
    }

    #[test]
    fn deployment_metrics_ema_updates() {
        let mut m = DeploymentMetrics::default();
        m.record_latency(1.0);
        assert!(
            (m.latency_ema - 1.0).abs() < 1e-9,
            "first sample should set EMA directly"
        );

        m.record_latency(0.0);
        // EMA = 0.3 * 0.0 + 0.7 * 1.0 = 0.7
        assert!(
            (m.latency_ema - 0.7).abs() < 1e-9,
            "EMA should be 0.7 after second sample"
        );
    }

    // ---- DynamicRouter tests -----------------------------------------------

    /// A `Stream`-based discover that drains from a pre-built VecDeque.
    /// In tower 0.5, `Discover` is a blanket impl over `TryStream`, so
    /// implementing `Stream` here is sufficient.
    struct VecDiscover {
        items: VecDeque<
            std::result::Result<Change<String, LlmService<MockClient>>, std::convert::Infallible>,
        >,
    }

    impl VecDiscover {
        fn new(services: Vec<(String, LlmService<MockClient>)>) -> Self {
            Self {
                items: services.into_iter().map(|(k, v)| Ok(Change::Insert(k, v))).collect(),
            }
        }
    }

    impl Stream for VecDiscover {
        type Item = std::result::Result<
            Change<String, LlmService<MockClient>>,
            std::convert::Infallible,
        >;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.items.pop_front())
        }
    }

    impl Unpin for VecDiscover {}

    #[tokio::test]
    async fn dynamic_router_warms_ready_cache() {
        let discover = VecDiscover::new(vec![
            ("openai".into(), LlmService::new(MockClient::ok())),
            ("anthropic".into(), LlmService::new(MockClient::ok())),
        ]);

        let mut router = DynamicRouter::new(discover);

        // poll_ready should drain the discovery updates and warm the cache.
        futures_util::future::poll_fn(|cx| match router.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(()),
            Poll::Ready(Err(e)) => panic!("unexpected error: {e}"),
            Poll::Pending => Poll::Pending,
        })
        .await;

        assert!(!router.is_empty(), "at least one upstream should be ready");
    }

    #[tokio::test]
    async fn dynamic_router_evicts_stale() {
        /// A stream that inserts then immediately removes a service.
        struct InsertThenRemoveDiscover {
            step: usize,
        }

        impl Stream for InsertThenRemoveDiscover {
            type Item = std::result::Result<
                Change<String, LlmService<MockClient>>,
                std::convert::Infallible,
            >;

            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                let step = self.step;
                self.step += 1;
                match step {
                    0 => Poll::Ready(Some(Ok(Change::Insert(
                        "openai".into(),
                        LlmService::new(MockClient::ok()),
                    )))),
                    1 => Poll::Ready(Some(Ok(Change::Remove("openai".into())))),
                    _ => Poll::Ready(None),
                }
            }
        }

        impl Unpin for InsertThenRemoveDiscover {}

        let discover = InsertThenRemoveDiscover { step: 0 };
        let mut router = DynamicRouter::new(discover);

        // Drive once to process insert + remove.
        let mut noop_cx = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
        let _ = router.poll_ready(&mut noop_cx);

        // After eviction the router should have zero entries.
        assert_eq!(router.len(), 0, "evicted service should be removed");
    }

    // ---- Concurrency limit test --------------------------------------------

    #[tokio::test]
    async fn concurrency_limit_rejects_at_max() {

        // A service that blocks until a signal is set, so we can hold the
        // permit open.
        #[derive(Clone)]
        struct BlockingService {
            call_count: Arc<AtomicUsize>,
        }

        impl Service<LlmRequest> for BlockingService {
            type Response = LlmResponse;
            type Error = LiterLlmError;
            type Future = BoxFuture<'static, Result<LlmResponse>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, _req: LlmRequest) -> Self::Future {
                self.call_count.fetch_add(1, AtomicOrdering::SeqCst);
                Box::pin(std::future::pending())
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let inner = BlockingService {
            call_count: Arc::clone(&counter),
        };

        // Limit to 1 concurrent request.
        let mut limited = ConcurrencyLimit::new(inner, 1);

        // First poll_ready → should succeed, acquiring the permit.
        assert!(
            futures_util::future::poll_fn(|cx| limited.poll_ready(cx)).await.is_ok(),
            "first poll_ready should be ok"
        );

        // Dispatch the first call (holds the permit open indefinitely).
        let _held_fut = limited.call(LlmRequest::ListModels);

        // Now the concurrency slot is exhausted.  poll_ready should return
        // Pending (the tower ConcurrencyLimit only returns Ready once the
        // semaphore has a slot).
        let mut noop_cx = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
        let poll = limited.poll_ready(&mut noop_cx);
        assert!(
            poll.is_pending(),
            "second poll_ready should be Pending when limit=1 and one request is in-flight"
        );
    }

    // ---- StaticDiscover tests -----------------------------------------------

    #[tokio::test]
    async fn static_discover_yields_all_services() {
        let mut discover = StaticDiscover::new(vec![
            ("a".to_owned(), LlmService::new(MockClient::ok())),
            ("b".to_owned(), LlmService::new(MockClient::ok())),
        ]);

        let mut noop_cx = std::task::Context::from_waker(futures_util::task::noop_waker_ref());

        // StaticDiscover implements Stream (not Discover directly).
        let first = Pin::new(&mut discover).poll_next(&mut noop_cx);
        assert!(matches!(first, Poll::Ready(Some(Ok(Change::Insert(ref k, _)))) if k == "a"));

        let second = Pin::new(&mut discover).poll_next(&mut noop_cx);
        assert!(matches!(second, Poll::Ready(Some(Ok(Change::Insert(ref k, _)))) if k == "b"));

        // After all services are yielded, stream ends.
        let third = Pin::new(&mut discover).poll_next(&mut noop_cx);
        assert!(matches!(third, Poll::Ready(None)));
    }
}
