//! Budget enforcement middleware.
//!
//! [`BudgetLayer`] wraps any [`Service<LlmRequest>`] and enforces spending
//! limits (global and per-model) in USD.  Cost is calculated after each
//! successful response using [`crate::cost::completion_cost`] and accumulated
//! atomically in [`BudgetState`].
//!
//! Two enforcement modes are supported:
//!
//! - **Hard** — pre-request check rejects with [`LiterLlmError::BudgetExceeded`]
//!   when the accumulated spend is at or above the configured limit.  Note that
//!   hard enforcement is **best-effort** under concurrent load: because cost is
//!   recorded after the response, concurrent in-flight requests may collectively
//!   overshoot the limit.  See [`check_budget`] for details.
//! - **Soft** — requests are never rejected; a `tracing::warn!` is emitted when
//!   the limit is exceeded.
//!
//! # Pluggable ledger
//!
//! [`BudgetLedger`] is the extension point for custom per-key / per-user cost
//! tracking and multi-dimensional budgets.  The built-in [`InMemoryBudgetLedger`]
//! tracks spend across the global, per-model, per-tenant, per-user, and
//! per-API-key dimensions using sliding-window accumulators backed by
//! [`DashMap`]s.  Supply any type implementing [`BudgetLedger`] to plug in a
//! database-backed or remote ledger.
//!
//! # Example
//!
//! ```rust,ignore
//! use liter_llm::tower::{BudgetConfig, BudgetLayer, BudgetState, Enforcement, LlmService};
//! use tower::ServiceBuilder;
//! use std::sync::Arc;
//!
//! let state = Arc::new(BudgetState::new());
//! let config = BudgetConfig {
//!     global_limit: Some(10.0),
//!     model_limits: Default::default(),
//!     enforcement: Enforcement::Hard,
//! };
//!
//! let client = liter_llm::DefaultClient::new(cfg, None)?;
//! let service = ServiceBuilder::new()
//!     .layer(BudgetLayer::new(config, Arc::clone(&state)))
//!     .service(LlmService::new(client));
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};

use arc_swap::ArcSwap;
use dashmap::DashMap;
use tower::{Layer, Service};

use super::cost::observe_stream_usage;
use super::types::{LlmRequest, LlmRequestKind, LlmResponse};
use crate::client::BoxFuture;
use crate::cost;
use crate::error::{LiterLlmError, Result};
use crate::types::Usage;

/// The dimension along which a budget rejection was triggered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetDimension {
    /// Cumulative spend across all dimensions.
    Global,
    /// Spend for a specific model.
    Model(String),
    /// Spend for a tenant (organisation-level grouping).
    Tenant(String),
    /// Spend for an individual end-user.
    User(String),
    /// Spend for a specific API key.
    ApiKey(String),
}

/// Decision returned by [`BudgetLedger::check`].
#[derive(Debug, Clone)]
pub enum BudgetVerdict {
    /// The request may proceed.
    Allow,
    /// The request should be rejected because a budget limit was exceeded.
    Reject {
        /// Human-readable reason.
        reason: String,
        /// Which limit was triggered.
        dimension: BudgetDimension,
    },
}

/// Contextual metadata passed to [`BudgetLedger::record`] after a successful
/// completion.
pub struct CostRecordContext<'a> {
    /// The model name (e.g. `"gpt-4"`).
    pub model: &'a str,
    /// The provider name (e.g. `"openai"`).
    pub provider: &'a str,
    /// Optional organisation / tenant identifier.
    pub tenant_id: Option<&'a str>,
    /// Optional end-user identifier.
    pub user_id: Option<&'a str>,
    /// Optional API-key identifier (not the raw secret — an opaque handle).
    pub api_key_id: Option<&'a str>,
    /// Actual cost of this call in US dollars.
    pub cost_usd: f64,
    /// Number of prompt (input) tokens consumed.
    pub tokens_in: u64,
    /// Number of completion (output) tokens consumed.
    pub tokens_out: u64,
    /// Wall-clock time at which the response was received.
    pub timestamp: SystemTime,
}

/// Contextual metadata passed to [`BudgetLedger::check`] before a call is
/// dispatched.  Identical to [`CostRecordContext`] except that `cost_usd`,
/// `tokens_in`, and `tokens_out` are not yet known.
pub struct CostCheckContext<'a> {
    /// The model name (e.g. `"gpt-4"`).
    pub model: &'a str,
    /// The provider name (e.g. `"openai"`).
    pub provider: &'a str,
    /// Optional organisation / tenant identifier.
    pub tenant_id: Option<&'a str>,
    /// Optional end-user identifier.
    pub user_id: Option<&'a str>,
    /// Optional API-key identifier (not the raw secret — an opaque handle).
    pub api_key_id: Option<&'a str>,
    /// Wall-clock time at which the pre-flight check is performed.
    pub timestamp: SystemTime,
}

/// A point-in-time snapshot of cumulative spend across all tracked dimensions.
///
/// Used for observability dashboards and as the primitive for chargeback-ready
/// CSV export via [`InMemoryBudgetLedger::export_csv`].  The `limits_*` fields
/// carry the configured caps so that helpers such as [`should_hedge`] can make
/// limit-aware decisions without requiring access to ledger internals.
#[derive(Debug, Clone, Default)]
pub struct BudgetSnapshot {
    /// Total spend across all dimensions, in USD.
    pub global_spend_usd: f64,
    /// Per-model spend, keyed by model name, in USD.
    pub per_model: HashMap<String, f64>,
    /// Per-tenant spend, keyed by tenant identifier, in USD.
    pub per_tenant: HashMap<String, f64>,
    /// Per-user spend, keyed by user identifier, in USD.
    pub per_user: HashMap<String, f64>,
    /// Per-API-key spend, keyed by API-key identifier, in USD.
    pub per_api_key: HashMap<String, f64>,
    /// Configured global spending cap in USD, if any.
    pub limit_global: Option<f64>,
    /// Configured per-user spending caps in USD.
    pub limits_per_user: HashMap<String, f64>,
    /// Configured per-API-key spending caps in USD.
    pub limits_per_api_key: HashMap<String, f64>,
    /// Configured per-tenant spending caps in USD.
    pub limits_per_tenant: HashMap<String, f64>,
}

/// Pluggable cost-tracking and budget-enforcement backend.
///
/// Implement this trait to plug in a database-backed, Redis-backed, or remote
/// ledger.  The built-in implementation is [`InMemoryBudgetLedger`].
///
/// # Object safety
///
/// The trait is object-safe; you can store it as `Arc<dyn BudgetLedger>`.
pub trait BudgetLedger: Send + Sync + 'static {
    /// Record the cost of a successful call against all relevant ledgers.
    ///
    /// This is called **after** the inner service returns a successful response.
    /// Implementations must be non-blocking; long-running work should be
    /// spawned as a background task.
    fn record<'a>(&'a self, ctx: &'a CostRecordContext<'a>) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

    /// Check whether the *next* call would exceed any configured budget limit.
    ///
    /// This is called **before** the inner service is invoked.  Return
    /// [`BudgetVerdict::Reject`] to short-circuit the call without forwarding
    /// to the upstream provider.
    fn check<'a>(&'a self, ctx: &'a CostCheckContext<'a>) -> Pin<Box<dyn Future<Output = BudgetVerdict> + Send + 'a>>;

    /// Return a point-in-time snapshot of all tracked spend dimensions.
    ///
    /// Callers use this for dashboards and for the cost-aware rate-limiter.
    fn snapshot(&self) -> BudgetSnapshot;
}

/// Sliding-window accumulator for a single budget dimension.
///
/// Each dimension (global, model, tenant, user, API-key) maintains its own
/// pair of `(spend_microcents, window_start)`.  When the window elapses the
/// counters are atomically zeroed so that the limit applies fresh each period.
///
/// All values are stored in **microcents** (`USD × 1_000_000`) to avoid
/// floating-point atomics while retaining sub-cent precision.
#[derive(Debug)]
struct WindowEntry {
    /// Accumulated spend in microcents (USD × 1_000_000).
    spend_mc: AtomicU64,
    /// Epoch seconds at which the current window started.
    window_start_secs: AtomicU64,
    /// Window duration in seconds.
    window_secs: u64,
}

impl WindowEntry {
    fn new(window: Duration) -> Self {
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            spend_mc: AtomicU64::new(0),
            window_start_secs: AtomicU64::new(now_secs),
            window_secs: window.as_secs(),
        }
    }

    /// Return current spend in USD, resetting if the window has elapsed.
    ///
    /// Uses a `compare_exchange` CAS so that under concurrent calls exactly one
    /// thread wins the rollover.  The winner subtracts the snapshot of
    /// `spend_mc` taken **before** the CAS (the old-window accumulation), so
    /// that any concurrent `fetch_add` calls that land after the snapshot —
    /// whether before or after the CAS — are preserved in the counter.
    /// Threads that lose the CAS simply re-read the counter.
    fn spend_usd(&self, now: SystemTime) -> f64 {
        let now_secs = now.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
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
        microcents_to_usd(self.spend_mc.load(Ordering::Acquire))
    }

    /// Add `usd` to this entry, respecting the sliding window.
    fn add(&self, usd: f64, now: SystemTime) {
        let _ = self.spend_usd(now);
        self.spend_mc.fetch_add(usd_to_microcents(usd), Ordering::AcqRel);
    }
}

/// Per-dimension limits configuration used by [`InMemoryBudgetLedger`].
#[derive(Debug, Clone, Default)]
pub struct DimensionLimits {
    /// Global spending cap in USD.  `None` means unlimited.
    pub global: Option<f64>,
    /// Per-model spending caps in USD.
    pub per_model: HashMap<String, f64>,
    /// Per-tenant spending caps in USD.
    pub per_tenant: HashMap<String, f64>,
    /// Per-user spending caps in USD.
    pub per_user: HashMap<String, f64>,
    /// Per-API-key spending caps in USD.
    pub per_api_key: HashMap<String, f64>,
}

/// In-memory [`BudgetLedger`] backed by [`DashMap`]s with sliding-window reset.
///
/// Use [`InMemoryBudgetLedger::new`] for full control or
/// [`InMemoryBudgetLedger::from_config`] to build from an existing
/// [`BudgetConfig`] (for backward compatibility).
///
/// `limits` lives behind an [`ArcSwap`] rather than a plain field so that
/// [`InMemoryBudgetLedger::update_limits`] can hot-swap the configured caps —
/// e.g. on a config reload — without touching the per-tenant/per-user/
/// per-API-key spend already accumulated in the `DashMap`s below. This keeps
/// the read path (`check`/`snapshot`) lock-free: a swap is a single atomic
/// pointer load, matching the concurrency style the sliding-window
/// [`WindowEntry`] accumulators already use.
///
/// # Bounded dimensions
///
/// `per_user` is keyed by the request's free-text `user` field and
/// `per_api_key` by a caller-supplied API-key identifier — both are
/// attacker-controlled, arbitrary-cardinality inputs, so they are capped at
/// `max_principal_entries` entries (see [`Self::with_max_principal_entries`]
/// to override). `per_model` and `per_tenant` are keyed by operator-controlled
/// values (the deployment's configured model list / tenant roster) with
/// naturally bounded cardinality, so they are deliberately left uncapped —
/// capping them would add eviction-policy risk (see below) for a dimension
/// that was never the actual memory-exhaustion vector.
///
/// # Why eviction never touches recorded spend
///
/// This is a *spend ledger*, not a cache: an entry's value is money already
/// billed against a principal. Evicting a live (non-zero-spend) entry to make
/// room for a newcomer would silently forgive that spend, which is a budget
/// bypass — a caller could burn through their limit, get evicted by
/// cardinality pressure from other callers, and come back with a fresh
/// budget. So the cap (`entry_add_capped`) only ever reclaims entries
/// whose recorded spend reads as exactly zero, and once no such entries
/// remain it **declines to track new principals** rather than evict a live
/// one — the call still succeeds and global/per-model spend is still
/// recorded, but that one principal's per-user/per-API-key budget is
/// unenforced until capacity frees up. This is a known, deliberate trade-off:
/// bounded memory always wins, at the cost of enforcement granularity (never
/// of already-recorded spend) under sustained cardinality pressure.
#[derive(Debug)]
pub struct InMemoryBudgetLedger {
    limits: ArcSwap<DimensionLimits>,
    window: Duration,
    global: Arc<WindowEntry>,
    per_model: Arc<DashMap<String, WindowEntry>>,
    per_tenant: Arc<DashMap<String, WindowEntry>>,
    per_user: Arc<DashMap<String, WindowEntry>>,
    per_api_key: Arc<DashMap<String, WindowEntry>>,
    max_principal_entries: usize,
}

/// Advise the hedge layer wiring whether to issue a speculative duplicate
/// request for the given pre-flight context.
///
/// Returns `false` (suppress hedging) when issuing a second speculative copy
/// of the request would push any budget dimension over its limit.  The hedge
/// wiring callsite should consult this before enabling the hedge policy.
///
/// # Parameters
///
/// * `ledger` — the live budget ledger to consult.
/// * `ctx` — pre-flight context identifying the user / key / model.
/// * `estimated_cost_usd` — expected cost of **one** copy of the request.  A
///   hedged call doubles this cost, so the check uses `2 × estimated_cost`.
/// * `safety_margin_pct` — fraction of each limit to reserve before blocking
///   hedging (e.g. `0.10` stops hedging when spend would exceed 90 % of the
///   limit).  Must be in `[0.0, 1.0)`.
///
/// # Logic
///
/// For each budget dimension that is both tracked in the ledger snapshot and
/// has a configured limit on `ledger`, hedging is suppressed when:
///
/// ```text
/// current_spend + 2 × estimated_cost  >=  limit × (1 − safety_margin_pct)
/// ```
///
/// Returns `true` only if **all** applicable dimensions have sufficient
/// headroom for two copies of the call.
#[must_use]
pub fn should_hedge<L: BudgetLedger>(
    ledger: &L,
    ctx: &CostCheckContext<'_>,
    estimated_cost_usd: f64,
    safety_margin_pct: f64,
) -> bool {
    let snap = ledger.snapshot();
    let hedge_cost = 2.0 * estimated_cost_usd;
    let margin = safety_margin_pct.clamp(0.0, 0.999);

    let has_headroom = |spend: f64, limit: f64| -> bool {
        let effective_limit = limit * (1.0 - margin);
        spend + hedge_cost < effective_limit
    };

    if let Some(global_limit) = snap.limit_global
        && !has_headroom(snap.global_spend_usd, global_limit)
    {
        return false;
    }

    if let Some(user) = ctx.user_id
        && let Some(&user_limit) = snap.limits_per_user.get(user)
    {
        let user_spend = snap.per_user.get(user).copied().unwrap_or(0.0);
        if !has_headroom(user_spend, user_limit) {
            return false;
        }
    }

    if let Some(key) = ctx.api_key_id
        && let Some(&key_limit) = snap.limits_per_api_key.get(key)
    {
        let key_spend = snap.per_api_key.get(key).copied().unwrap_or(0.0);
        if !has_headroom(key_spend, key_limit) {
            return false;
        }
    }

    if let Some(tenant) = ctx.tenant_id
        && let Some(&tenant_limit) = snap.limits_per_tenant.get(tenant)
    {
        let tenant_spend = snap.per_tenant.get(tenant).copied().unwrap_or(0.0);
        if !has_headroom(tenant_spend, tenant_limit) {
            return false;
        }
    }

    true
}

/// Derive the OpenTelemetry GenAI `gen_ai.system` provider prefix from a
/// model identifier (e.g. `"openai"` from `"openai/gpt-4o"`), matching the
/// convention [`crate::tower::tracing::TracingService`] uses. Returns `""`
/// when the model has no `<provider>/` prefix.
pub(crate) fn provider_of(model: &str) -> &str {
    model.split_once('/').map_or("", |(prefix, _)| prefix)
}

/// Extract the end-user identifier from a `Chat`/`ChatStream`/`Embed`
/// request's `user` field.
///
/// Returns `None` for request kinds that carry no `user` field (image,
/// audio, moderation, etc.) or when the field is unset.
pub(crate) fn user_id_of(req: &LlmRequest) -> Option<&str> {
    match &req.kind {
        LlmRequestKind::Chat(r) | LlmRequestKind::ChatStream(r) => r.user.as_deref(),
        LlmRequestKind::Embed(r) => r.user.as_deref(),
        _ => None,
    }
}

/// Tower [`Layer`] that enforces and records spend via a pluggable
/// [`BudgetLedger`], adding per-tenant / per-user / per-API-key budget
/// dimensions on top of what [`BudgetLayer`] provides.
///
/// # Why this layer exists
///
/// [`BudgetLedger`] (and its default [`InMemoryBudgetLedger`] implementation)
/// is a fully-built, independently tested trait for multi-dimensional spend
/// tracking — but nothing in the Tower stack ever constructed a `Service`
/// around it: [`BudgetLayer`] only ever touches the simpler [`BudgetState`]
/// atomic counters, which track just the global and per-model dimensions.
/// `BudgetLedgerLayer` is the missing wiring. Compose it alongside (or
/// instead of) [`BudgetLayer`] to get tenant/user-scoped enforcement and
/// recording.
///
/// # Context extraction
///
/// - `model` / `provider` come from [`LlmRequest::model`] (`provider` is the
///   `<provider>/` prefix, see [`provider_of`]).
/// - `tenant_id` comes from [`LlmRequest::tenant_id`].
/// - `user_id` comes from the `user` field on `Chat`/`ChatStream`/`Embed`
///   requests (see [`user_id_of`]).
/// - `api_key_id` is always `None` — [`LlmRequest`] does not currently carry
///   an API-key identifier anywhere in its public surface. This dimension is
///   therefore inert (never checked, never recorded) until a caller extends
///   `LlmRequest` (or a wrapping layer) with one.
///
/// # Streaming
///
/// Like [`BudgetLayer`], the pre-flight check applies uniformly to every
/// request kind. Post-response recording uses
/// [`observe_stream_usage`][crate::tower::cost::observe_stream_usage] so
/// `ChatStream` responses are recorded once the stream completes instead of
/// being silently skipped — recording happens on a spawned task since
/// [`BudgetLedger::record`] is async but the stream's completion callback is
/// synchronous.
#[cfg_attr(alef, alef(skip))]
pub struct BudgetLedgerLayer<L: BudgetLedger> {
    ledger: Arc<L>,
    enforcement: Enforcement,
}

impl<L: BudgetLedger> BudgetLedgerLayer<L> {
    /// Create a new layer backed by `ledger`.
    // ~keep Redundant with the `alef(skip)` on the type itself: alef does not propagate a
    // ~keep type-level skip to that type's impl blocks, so it still reports this generic
    // ~keep constructor as an unrepresentable public item and fails generation outright.
    #[cfg_attr(alef, alef(skip))]
    #[must_use]
    pub fn new(ledger: Arc<L>, enforcement: Enforcement) -> Self {
        Self { ledger, enforcement }
    }
}

impl<L: BudgetLedger, S> Layer<S> for BudgetLedgerLayer<L> {
    type Service = BudgetLedgerService<L, S>;

    fn layer(&self, inner: S) -> Self::Service {
        BudgetLedgerService {
            inner,
            ledger: Arc::clone(&self.ledger),
            enforcement: self.enforcement,
        }
    }
}

/// Tower service produced by [`BudgetLedgerLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct BudgetLedgerService<L: BudgetLedger, S> {
    inner: S,
    ledger: Arc<L>,
    enforcement: Enforcement,
}

impl<L: BudgetLedger, S: Clone> Clone for BudgetLedgerService<L, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            ledger: Arc::clone(&self.ledger),
            enforcement: self.enforcement,
        }
    }
}

/// How budget limits are enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Enforcement {
    /// Reject requests that would exceed the budget with
    /// [`LiterLlmError::BudgetExceeded`].
    Hard,
    /// Allow requests through but emit a `tracing::warn!` when the budget is
    /// exceeded.
    Soft,
}

/// Configuration for budget enforcement.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BudgetConfig {
    /// Maximum total spend across all models, in USD.  `None` means unlimited.
    pub global_limit: Option<f64>,
    /// Per-model spending limits in USD.  Models not listed here are only
    /// constrained by `global_limit`.
    pub model_limits: HashMap<String, f64>,
    /// Whether to reject requests or merely warn when a limit is exceeded.
    pub enforcement: Enforcement,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            global_limit: None,
            model_limits: HashMap::new(),
            enforcement: Enforcement::Hard,
        }
    }
}

/// Shared, thread-safe budget accumulator.
///
/// All values are stored in **microcents** (USD * 1_000_000) as `AtomicU64` to
/// avoid floating-point atomics while retaining sub-cent precision.
#[derive(Debug)]
pub struct BudgetState {
    /// Total spend across all models (microcents).
    global_spend: AtomicU64,
    /// Per-model spend (microcents).
    model_spend: DashMap<String, AtomicU64>,
}

impl BudgetState {
    /// Create a new, zeroed budget state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            global_spend: AtomicU64::new(0),
            model_spend: DashMap::new(),
        }
    }

    /// Return the total global spend in USD.
    #[must_use]
    pub fn global_spend(&self) -> f64 {
        microcents_to_usd(self.global_spend.load(Ordering::Relaxed))
    }

    /// Return the spend for a specific model in USD, or `0.0` if the model has
    /// not been seen.
    #[must_use]
    pub fn model_spend(&self, model: &str) -> f64 {
        self.model_spend
            .get(model)
            .map(|v| microcents_to_usd(v.load(Ordering::Relaxed)))
            .unwrap_or(0.0)
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        self.global_spend.store(0, Ordering::Relaxed);
        self.model_spend.clear();
    }

    /// Add `usd` to the global and per-model counters.
    fn record(&self, model: &str, usd: f64) {
        let mc = usd_to_microcents(usd);
        self.global_spend.fetch_add(mc, Ordering::Relaxed);
        self.model_spend
            .entry(model.to_owned())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(mc, Ordering::Relaxed);
    }
}

#[cfg_attr(alef, alef(skip))]
impl Default for BudgetState {
    fn default() -> Self {
        Self::new()
    }
}

fn usd_to_microcents(usd: f64) -> u64 {
    if usd <= 0.0 {
        return 0;
    }
    (usd * 1_000_000.0).round() as u64
}

fn microcents_to_usd(mc: u64) -> f64 {
    mc as f64 / 1_000_000.0
}

/// Tower [`Layer`] that enforces spending budgets.
#[cfg_attr(alef, alef(skip))]
pub struct BudgetLayer {
    config: BudgetConfig,
    state: Arc<BudgetState>,
}

#[cfg_attr(alef, alef(skip))]
impl BudgetLayer {
    /// Create a new budget layer with the given configuration and shared state.
    ///
    /// The caller retains an `Arc<BudgetState>` for runtime introspection
    /// (e.g. dashboard queries, manual resets).
    #[must_use]
    pub fn new(config: BudgetConfig, state: Arc<BudgetState>) -> Self {
        Self { config, state }
    }
}

impl<S> Layer<S> for BudgetLayer {
    type Service = BudgetService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BudgetService {
            inner,
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

/// Tower service produced by [`BudgetLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct BudgetService<S> {
    inner: S,
    config: BudgetConfig,
    state: Arc<BudgetState>,
}

impl<S: Clone> Clone for BudgetService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: self.config.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

impl<S> Service<LlmRequest> for BudgetService<S>
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

        if config.enforcement == Enforcement::Hard
            && let Some(err) = check_budget(&config, &state, &model)
        {
            return Box::pin(async move { Err(err) });
        }

        let fut = self.inner.call(req);

        Box::pin(async move {
            let resp = fut.await?;

            match resp {
                // ~keep LlmResponse::usage() always returns None for ChatStream (usage isn't known until the
                // ~keep stream completes), so recording must happen in the stream's completion callback instead.
                LlmResponse::ChatStream(stream) => {
                    let model_for_completion = model.clone();
                    let state_for_completion = Arc::clone(&state);
                    let config_for_completion = config.clone();
                    let wrapped = observe_stream_usage(stream, move |usage| {
                        record_usage(
                            &config_for_completion,
                            &state_for_completion,
                            &model_for_completion,
                            usage.as_ref(),
                        );
                    });
                    Ok(LlmResponse::ChatStream(wrapped))
                }
                other => {
                    record_usage(&config, &state, &model, other.usage());
                    Ok(other)
                }
            }
        })
    }
}

/// Compute the cost of `usage` and record it against `state`, emitting soft
/// enforcement warnings if configured. No-op when `usage` is `None` or the
/// model has no pricing data.
///
/// Shared by the non-streaming response path (usage known immediately) and
/// the `ChatStream` completion callback (usage only known once the stream is
/// fully consumed).
fn record_usage(config: &BudgetConfig, state: &BudgetState, model: &str, usage: Option<&Usage>) {
    let Some(usage) = usage else { return };
    let Some(usd) = cost::completion_cost(model, usage.prompt_tokens, usage.completion_tokens) else {
        return;
    };

    state.record(model, usd);

    if config.enforcement == Enforcement::Soft {
        emit_soft_warnings(config, state, model);
    }
}

/// Check whether the current spend exceeds any configured limit.  Returns
/// `Some(LiterLlmError)` if the budget is exceeded under hard enforcement.
///
/// **Concurrency note:** This check is best-effort under concurrent load.
/// Because the budget is checked (read) before the request and recorded
/// (write) after the response, concurrent requests may all pass the
/// pre-flight check before any of them record their cost.  This means
/// hard enforcement can slightly overshoot the configured limit by up to
/// `N * max_single_request_cost` where `N` is the number of concurrent
/// in-flight requests.  For strict dollar-accurate enforcement, use an
/// external budget service with transactional semantics.
fn check_budget(config: &BudgetConfig, state: &BudgetState, model: &str) -> Option<LiterLlmError> {
    if let Some(limit) = config.global_limit
        && state.global_spend() >= limit
    {
        return Some(LiterLlmError::BudgetExceeded {
            message: format!(
                "global budget exceeded: spent ${:.6}, limit ${:.6}",
                state.global_spend(),
                limit,
            ),
            model: None,
        });
    }

    if let Some(&limit) = config.model_limits.get(model)
        && state.model_spend(model) >= limit
    {
        return Some(LiterLlmError::BudgetExceeded {
            message: format!(
                "model {model} budget exceeded: spent ${:.6}, limit ${:.6}",
                state.model_spend(model),
                limit,
            ),
            model: Some(model.to_owned()),
        });
    }

    None
}

/// Emit `tracing::warn!` messages for any exceeded limits (soft mode).
fn emit_soft_warnings(config: &BudgetConfig, state: &BudgetState, model: &str) {
    if let Some(limit) = config.global_limit
        && state.global_spend() >= limit
    {
        tracing::warn!(
            spend = state.global_spend(),
            limit,
            "global budget exceeded (soft enforcement)"
        );
    }

    if let Some(&limit) = config.model_limits.get(model)
        && state.model_spend(model) >= limit
    {
        tracing::warn!(
            model,
            spend = state.model_spend(model),
            limit,
            "model budget exceeded (soft enforcement)"
        );
    }
}

mod ledger;
mod ledger_service;

#[cfg(test)]
mod ledger_tests;
#[cfg(test)]
mod tests;
