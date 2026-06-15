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
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::cost;
use crate::error::{LiterLlmError, Result};

// ─── Ledger trait types ───────────────────────────────────────────────────────

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

// ─── InMemoryBudgetLedger ─────────────────────────────────────────────────────

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
    /// thread wins the rollover: it zeroes `spend_mc` and advances
    /// `window_start_secs`.  Threads that lose the CAS (the window was already
    /// rolled by the winner) simply re-read the (now-zeroed) counter.
    fn spend_usd(&self, now: SystemTime) -> f64 {
        let now_secs = now.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
        let start = self.window_start_secs.load(Ordering::Acquire);
        if now_secs.saturating_sub(start) >= self.window_secs {
            // CAS: only one thread advances the window start.  The loser sees
            // `Err` and skips the reset — the winner already zeroed `spend_mc`.
            if self
                .window_start_secs
                .compare_exchange(start, now_secs, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // We won the race — zero the accumulator.
                self.spend_mc.store(0, Ordering::Release);
            }
            // Whether we won or lost, the window has been reset.  Re-read the
            // (now potentially zeroed) counter so we return the correct value.
        }
        microcents_to_usd(self.spend_mc.load(Ordering::Acquire))
    }

    /// Add `usd` to this entry, respecting the sliding window.
    fn add(&self, usd: f64, now: SystemTime) {
        // Trigger window reset via `spend_usd` first.  The CAS inside ensures
        // exactly one thread performs the reset even under concurrent calls.
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
#[derive(Debug)]
pub struct InMemoryBudgetLedger {
    limits: DimensionLimits,
    window: Duration,
    global: Arc<WindowEntry>,
    per_model: Arc<DashMap<String, WindowEntry>>,
    per_tenant: Arc<DashMap<String, WindowEntry>>,
    per_user: Arc<DashMap<String, WindowEntry>>,
    per_api_key: Arc<DashMap<String, WindowEntry>>,
}

impl InMemoryBudgetLedger {
    /// Create a new ledger with explicit limits and a shared window duration.
    ///
    /// The `window` controls how long spend is accumulated before the
    /// per-dimension counters reset (e.g. `Duration::from_secs(86400)` for
    /// daily budgets).
    #[must_use]
    pub fn new(limits: DimensionLimits, window: Duration) -> Self {
        Self {
            global: Arc::new(WindowEntry::new(window)),
            per_model: Arc::new(DashMap::new()),
            per_tenant: Arc::new(DashMap::new()),
            per_user: Arc::new(DashMap::new()),
            per_api_key: Arc::new(DashMap::new()),
            limits,
            window,
        }
    }

    /// Build from a legacy [`BudgetConfig`].
    ///
    /// Global and per-model limits from `config` are mapped directly.
    /// Tenant, user, and API-key limits are left empty.
    /// The sliding window defaults to 30 days (a calendar month approximation).
    #[must_use]
    pub fn from_config(config: &BudgetConfig) -> Self {
        let limits = DimensionLimits {
            global: config.global_limit,
            per_model: config.model_limits.clone(),
            ..Default::default()
        };
        // Default window: 30 days — resets monthly.
        Self::new(limits, Duration::from_secs(30 * 24 * 3600))
    }

    /// Export a CSV of the current spend snapshot to `writer`.
    ///
    /// The CSV has two columns: `dimension,spend_usd`.  Each tracked key is
    /// emitted as one row.  Designed for cron-job extraction into a chargeback
    /// pipeline.
    ///
    /// # Errors
    ///
    /// Returns `Err(io::Error)` if writing to `writer` fails.
    pub fn export_csv(&self, mut writer: impl io::Write) -> io::Result<()> {
        let snap = self.snapshot();
        writeln!(writer, "dimension,spend_usd")?;
        writeln!(writer, "global,{}", snap.global_spend_usd)?;
        for (model, spend) in &snap.per_model {
            writeln!(writer, "model:{model},{spend}")?;
        }
        for (tenant, spend) in &snap.per_tenant {
            writeln!(writer, "tenant:{tenant},{spend}")?;
        }
        for (user, spend) in &snap.per_user {
            writeln!(writer, "user:{user},{spend}")?;
        }
        for (key, spend) in &snap.per_api_key {
            writeln!(writer, "api_key:{key},{spend}")?;
        }
        Ok(())
    }

    /// Reset all dimension counters to zero (useful for tests and manual overrides).
    pub fn reset(&self) {
        let now = SystemTime::now();
        // Force window expiry on the global entry by back-dating start.
        let zero_secs = SystemTime::UNIX_EPOCH
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.global.spend_mc.store(0, Ordering::Relaxed);
        self.global.window_start_secs.store(zero_secs, Ordering::Relaxed);
        let _ = self.global.spend_usd(now); // re-arm window

        self.per_model.clear();
        self.per_tenant.clear();
        self.per_user.clear();
        self.per_api_key.clear();
    }

    fn entry_spend(map: &DashMap<String, WindowEntry>, key: &str, now: SystemTime) -> f64 {
        map.get(key).map(|e| e.spend_usd(now)).unwrap_or(0.0)
    }

    fn entry_add(map: &DashMap<String, WindowEntry>, key: &str, usd: f64, window: Duration, now: SystemTime) {
        map.entry(key.to_owned())
            .or_insert_with(|| WindowEntry::new(window))
            .add(usd, now);
    }

    fn check_limit(spend: f64, limit: f64, dimension: BudgetDimension, key: &str) -> Option<BudgetVerdict> {
        if spend >= limit {
            Some(BudgetVerdict::Reject {
                reason: format!("{key} budget exceeded: spent ${spend:.6}, limit ${limit:.6}"),
                dimension,
            })
        } else {
            None
        }
    }
}

impl BudgetLedger for InMemoryBudgetLedger {
    fn record<'a>(&'a self, ctx: &'a CostRecordContext<'a>) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let now = ctx.timestamp;
            self.global.add(ctx.cost_usd, now);
            Self::entry_add(&self.per_model, ctx.model, ctx.cost_usd, self.window, now);
            if let Some(tenant) = ctx.tenant_id {
                Self::entry_add(&self.per_tenant, tenant, ctx.cost_usd, self.window, now);
            }
            if let Some(user) = ctx.user_id {
                Self::entry_add(&self.per_user, user, ctx.cost_usd, self.window, now);
            }
            if let Some(key) = ctx.api_key_id {
                Self::entry_add(&self.per_api_key, key, ctx.cost_usd, self.window, now);
            }

            // OTel: emit per-dimension spend gauge
            #[cfg(feature = "otel")]
            {
                use super::metrics;
                metrics::record_budget_spend(
                    ctx.model,
                    ctx.provider,
                    ctx.tenant_id,
                    ctx.user_id,
                    ctx.api_key_id,
                    ctx.cost_usd,
                );
            }
        })
    }

    fn check<'a>(&'a self, ctx: &'a CostCheckContext<'a>) -> Pin<Box<dyn Future<Output = BudgetVerdict> + Send + 'a>> {
        Box::pin(async move {
            let now = ctx.timestamp;

            // Global
            if let Some(limit) = self.limits.global {
                let spend = self.global.spend_usd(now);
                if let Some(v) = Self::check_limit(spend, limit, BudgetDimension::Global, "global") {
                    return v;
                }
            }

            // Per-model
            if let Some(&limit) = self.limits.per_model.get(ctx.model) {
                let spend = Self::entry_spend(&self.per_model, ctx.model, now);
                if let Some(v) = Self::check_limit(
                    spend,
                    limit,
                    BudgetDimension::Model(ctx.model.to_owned()),
                    &format!("model:{}", ctx.model),
                ) {
                    return v;
                }
            }

            // Per-tenant
            if let Some(tenant) = ctx.tenant_id
                && let Some(&limit) = self.limits.per_tenant.get(tenant)
            {
                let spend = Self::entry_spend(&self.per_tenant, tenant, now);
                if let Some(v) = Self::check_limit(
                    spend,
                    limit,
                    BudgetDimension::Tenant(tenant.to_owned()),
                    &format!("tenant:{tenant}"),
                ) {
                    return v;
                }
            }

            // Per-user
            if let Some(user) = ctx.user_id
                && let Some(&limit) = self.limits.per_user.get(user)
            {
                let spend = Self::entry_spend(&self.per_user, user, now);
                if let Some(v) = Self::check_limit(
                    spend,
                    limit,
                    BudgetDimension::User(user.to_owned()),
                    &format!("user:{user}"),
                ) {
                    return v;
                }
            }

            // Per-API-key
            if let Some(key) = ctx.api_key_id
                && let Some(&limit) = self.limits.per_api_key.get(key)
            {
                let spend = Self::entry_spend(&self.per_api_key, key, now);
                if let Some(v) = Self::check_limit(
                    spend,
                    limit,
                    BudgetDimension::ApiKey(key.to_owned()),
                    &format!("api_key:{key}"),
                ) {
                    return v;
                }
            }

            BudgetVerdict::Allow
        })
    }

    fn snapshot(&self) -> BudgetSnapshot {
        let now = SystemTime::now();

        let global_spend_usd = self.global.spend_usd(now);

        let per_model = self
            .per_model
            .iter()
            .map(|e| (e.key().clone(), e.value().spend_usd(now)))
            .collect();

        let per_tenant = self
            .per_tenant
            .iter()
            .map(|e| (e.key().clone(), e.value().spend_usd(now)))
            .collect();

        let per_user = self
            .per_user
            .iter()
            .map(|e| (e.key().clone(), e.value().spend_usd(now)))
            .collect();

        let per_api_key = self
            .per_api_key
            .iter()
            .map(|e| (e.key().clone(), e.value().spend_usd(now)))
            .collect();

        BudgetSnapshot {
            global_spend_usd,
            per_model,
            per_tenant,
            per_user,
            per_api_key,
            limit_global: self.limits.global,
            limits_per_user: self.limits.per_user.clone(),
            limits_per_api_key: self.limits.per_api_key.clone(),
            limits_per_tenant: self.limits.per_tenant.clone(),
        }
    }
}

// ─── Hedge helper ─────────────────────────────────────────────────────────────

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
    // A hedge issues two copies of the request.
    let hedge_cost = 2.0 * estimated_cost_usd;
    let margin = safety_margin_pct.clamp(0.0, 0.999);

    // Returns `true` when `spend + hedge_cost` fits within the effective limit.
    let has_headroom = |spend: f64, limit: f64| -> bool {
        let effective_limit = limit * (1.0 - margin);
        spend + hedge_cost < effective_limit
    };

    // Global dimension.
    if let Some(global_limit) = snap.limit_global
        && !has_headroom(snap.global_spend_usd, global_limit)
    {
        return false;
    }

    // Per-user dimension.
    if let Some(user) = ctx.user_id
        && let Some(&user_limit) = snap.limits_per_user.get(user)
    {
        let user_spend = snap.per_user.get(user).copied().unwrap_or(0.0);
        if !has_headroom(user_spend, user_limit) {
            return false;
        }
    }

    // Per-API-key dimension.
    if let Some(key) = ctx.api_key_id
        && let Some(&key_limit) = snap.limits_per_api_key.get(key)
    {
        let key_spend = snap.per_api_key.get(key).copied().unwrap_or(0.0);
        if !has_headroom(key_spend, key_limit) {
            return false;
        }
    }

    // Per-tenant dimension.
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

// ── Types -----------------------------------------------------------------

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

// ── Config ----------------------------------------------------------------

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

// ── State -----------------------------------------------------------------

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

// ── Conversions -----------------------------------------------------------

fn usd_to_microcents(usd: f64) -> u64 {
    // Clamp negative values to zero to avoid wrapping in unsigned arithmetic.
    if usd <= 0.0 {
        return 0;
    }
    (usd * 1_000_000.0).round() as u64
}

fn microcents_to_usd(mc: u64) -> f64 {
    mc as f64 / 1_000_000.0
}

// ── Layer -----------------------------------------------------------------

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

// ── Service ---------------------------------------------------------------

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

        // --- Pre-flight: hard enforcement check ---
        if config.enforcement == Enforcement::Hard
            && let Some(err) = check_budget(&config, &state, &model)
        {
            return Box::pin(async move { Err(err) });
        }

        let fut = self.inner.call(req);

        Box::pin(async move {
            let resp = fut.await?;

            // --- Post-flight: record cost ---
            if let Some(usage) = resp.usage()
                && let Some(usd) = cost::completion_cost(&model, usage.prompt_tokens, usage.completion_tokens)
            {
                state.record(&model, usd);

                // Soft enforcement: warn after recording.
                if config.enforcement == Enforcement::Soft {
                    emit_soft_warnings(&config, &state, &model);
                }
            }

            Ok(resp)
        })
    }
}

// ── Helpers ---------------------------------------------------------------

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
    // Global limit check.
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

    // Per-model limit check.
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

// ── Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    /// Helper: build a budget layer + service with the given config.
    fn build_service(config: BudgetConfig, state: Arc<BudgetState>) -> BudgetService<LlmService<MockClient>> {
        let layer = BudgetLayer::new(config, state);
        let inner = LlmService::new(MockClient::ok());
        layer.layer(inner)
    }

    // ── Hard enforcement ────────────────────────────────────────────────────

    #[tokio::test]
    async fn hard_enforcement_rejects_when_global_limit_exceeded() {
        let state = Arc::new(BudgetState::new());
        // Pre-seed spend above the limit.
        state.global_spend.store(usd_to_microcents(10.0), Ordering::Relaxed);

        let config = BudgetConfig {
            global_limit: Some(5.0),
            enforcement: Enforcement::Hard,
            ..Default::default()
        };

        let mut svc = build_service(config, state);
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should reject over-budget request");
        assert!(matches!(err, LiterLlmError::BudgetExceeded { .. }));
    }

    #[tokio::test]
    async fn hard_enforcement_rejects_when_model_limit_exceeded() {
        let state = Arc::new(BudgetState::new());
        // Pre-seed per-model spend above the model limit.
        state
            .model_spend
            .entry("gpt-4".to_owned())
            .or_insert_with(|| AtomicU64::new(0))
            .store(usd_to_microcents(2.0), Ordering::Relaxed);

        let mut limits = HashMap::new();
        limits.insert("gpt-4".into(), 1.0);

        let config = BudgetConfig {
            global_limit: None,
            model_limits: limits,
            enforcement: Enforcement::Hard,
        };

        let mut svc = build_service(config, state);
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should reject over-budget model request");

        match &err {
            LiterLlmError::BudgetExceeded { model, .. } => {
                assert_eq!(model.as_deref(), Some("gpt-4"));
            }
            other => panic!("expected BudgetExceeded, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn hard_enforcement_allows_requests_under_limit() {
        let state = Arc::new(BudgetState::new());
        let config = BudgetConfig {
            global_limit: Some(100.0),
            enforcement: Enforcement::Hard,
            ..Default::default()
        };

        let mut svc = build_service(config, state);
        let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok(), "request under budget should succeed");
    }

    // ── Soft enforcement ────────────────────────────────────────────────────

    #[tokio::test]
    async fn soft_enforcement_allows_requests_over_global_limit() {
        let state = Arc::new(BudgetState::new());
        state.global_spend.store(usd_to_microcents(100.0), Ordering::Relaxed);

        let config = BudgetConfig {
            global_limit: Some(5.0),
            enforcement: Enforcement::Soft,
            ..Default::default()
        };

        let mut svc = build_service(config, state);
        let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok(), "soft mode should never reject");
    }

    #[tokio::test]
    async fn soft_enforcement_allows_requests_over_model_limit() {
        let state = Arc::new(BudgetState::new());
        state
            .model_spend
            .entry("gpt-4".to_owned())
            .or_insert_with(|| AtomicU64::new(0))
            .store(usd_to_microcents(10.0), Ordering::Relaxed);

        let mut limits = HashMap::new();
        limits.insert("gpt-4".into(), 1.0);

        let config = BudgetConfig {
            global_limit: None,
            model_limits: limits,
            enforcement: Enforcement::Soft,
        };

        let mut svc = build_service(config, state);
        let resp = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(resp.is_ok(), "soft mode should never reject");
    }

    // ── Cost accumulation ───────────────────────────────────────────────────

    #[tokio::test]
    async fn accumulates_cost_after_response() {
        let state = Arc::new(BudgetState::new());
        let config = BudgetConfig {
            global_limit: Some(100.0),
            enforcement: Enforcement::Hard,
            ..Default::default()
        };

        let mut svc = build_service(config, Arc::clone(&state));
        // MockClient returns usage: prompt=10, completion=5 for the model.
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");

        // gpt-4 pricing: input=0.00003/token, output=0.00006/token
        // 10 * 0.00003 + 5 * 0.00006 = 0.0003 + 0.0003 = 0.0006
        assert!(state.global_spend() > 0.0, "global spend should be recorded");
        assert!(state.model_spend("gpt-4") > 0.0, "model spend should be recorded");
    }

    // ── Per-model limits (independent) ──────────────────────────────────────

    #[tokio::test]
    async fn per_model_limits_are_independent() {
        let state = Arc::new(BudgetState::new());
        // Set gpt-4 over its limit, but gpt-3.5-turbo has no model limit.
        state
            .model_spend
            .entry("gpt-4".to_owned())
            .or_insert_with(|| AtomicU64::new(0))
            .store(usd_to_microcents(5.0), Ordering::Relaxed);

        let mut limits = HashMap::new();
        limits.insert("gpt-4".into(), 1.0);

        let config = BudgetConfig {
            global_limit: None,
            model_limits: limits,
            enforcement: Enforcement::Hard,
        };

        let mut svc = build_service(config, state);

        // gpt-4 should be rejected.
        let err = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(err.is_err(), "gpt-4 should be rejected");

        // gpt-3.5-turbo has no per-model limit, should succeed.
        let ok = svc.call(LlmRequest::Chat(chat_req("gpt-3.5-turbo"))).await;
        assert!(ok.is_ok(), "gpt-3.5-turbo should not be limited");
    }

    // ── Reset ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn reset_clears_all_counters() {
        let state = Arc::new(BudgetState::new());
        state.global_spend.store(usd_to_microcents(50.0), Ordering::Relaxed);
        state
            .model_spend
            .entry("gpt-4".to_owned())
            .or_insert_with(|| AtomicU64::new(0))
            .store(usd_to_microcents(25.0), Ordering::Relaxed);

        assert!(state.global_spend() > 0.0);
        assert!(state.model_spend("gpt-4") > 0.0);

        state.reset();

        assert_eq!(state.global_spend(), 0.0, "global spend should be zero after reset");
        assert_eq!(
            state.model_spend("gpt-4"),
            0.0,
            "model spend should be zero after reset"
        );
    }

    // ── Reset then allow ────────────────────────────────────────────────────

    #[tokio::test]
    async fn reset_allows_previously_blocked_requests() {
        let state = Arc::new(BudgetState::new());
        state.global_spend.store(usd_to_microcents(10.0), Ordering::Relaxed);

        let config = BudgetConfig {
            global_limit: Some(5.0),
            enforcement: Enforcement::Hard,
            ..Default::default()
        };

        let mut svc = build_service(config, Arc::clone(&state));

        // Should be rejected.
        let err = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(err.is_err());

        // Reset and retry.
        state.reset();
        let ok = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(ok.is_ok(), "should succeed after reset");
    }

    // ── Unlimited config ────────────────────────────────────────────────────

    #[tokio::test]
    async fn unlimited_config_allows_all_requests() {
        let state = Arc::new(BudgetState::new());
        let config = BudgetConfig::default();

        let mut svc = build_service(config, state);
        for _ in 0..20 {
            assert!(svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.is_ok());
        }
    }

    // ── Propagates inner errors ─────────────────────────────────────────────

    #[tokio::test]
    async fn propagates_inner_service_errors() {
        let state = Arc::new(BudgetState::new());
        let config = BudgetConfig {
            global_limit: Some(100.0),
            enforcement: Enforcement::Hard,
            ..Default::default()
        };

        let layer = BudgetLayer::new(config, state);
        let inner = LlmService::new(MockClient::failing_timeout());
        let mut svc = layer.layer(inner);

        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should propagate inner error");
        assert!(matches!(err, LiterLlmError::Timeout));
    }

    // ── BudgetLedger: per-key and per-user recording ─────────────────────────

    #[tokio::test]
    async fn budget_ledger_records_per_key_and_per_user() {
        let limits = DimensionLimits::default();
        let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

        let ctx1 = CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: Some("acme"),
            user_id: Some("alice"),
            api_key_id: Some("key-1"),
            cost_usd: 0.10,
            tokens_in: 1000,
            tokens_out: 500,
            timestamp: SystemTime::now(),
        };
        ledger.record(&ctx1).await;

        let ctx2 = CostRecordContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: Some("acme"),
            user_id: Some("bob"),
            api_key_id: Some("key-2"),
            cost_usd: 0.20,
            tokens_in: 2000,
            tokens_out: 1000,
            timestamp: SystemTime::now(),
        };
        ledger.record(&ctx2).await;

        let snap = ledger.snapshot();
        assert!(
            (snap.global_spend_usd - 0.30).abs() < 1e-9,
            "global: {}",
            snap.global_spend_usd
        );
        assert!((snap.per_model["gpt-4"] - 0.30).abs() < 1e-9);
        assert!((snap.per_tenant["acme"] - 0.30).abs() < 1e-9);
        assert!((snap.per_user["alice"] - 0.10).abs() < 1e-9);
        assert!((snap.per_user["bob"] - 0.20).abs() < 1e-9);
        assert!((snap.per_api_key["key-1"] - 0.10).abs() < 1e-9);
        assert!((snap.per_api_key["key-2"] - 0.20).abs() < 1e-9);
    }

    // ── BudgetLedger: reject when user limit exceeded ─────────────────────────

    #[tokio::test]
    async fn budget_ledger_rejects_when_user_limit_exceeded() {
        let mut limits = DimensionLimits::default();
        limits.per_user.insert("alice".to_owned(), 0.05);

        let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

        // Record spend that pushes alice over her $0.05 cap.
        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: Some("alice"),
                api_key_id: None,
                cost_usd: 0.10,
                tokens_in: 100,
                tokens_out: 50,
                timestamp: SystemTime::now(),
            })
            .await;

        let verdict = ledger
            .check(&CostCheckContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: Some("alice"),
                api_key_id: None,
                timestamp: SystemTime::now(),
            })
            .await;

        match verdict {
            BudgetVerdict::Reject { dimension, .. } => {
                assert!(
                    matches!(dimension, BudgetDimension::User(ref u) if u == "alice"),
                    "expected User(alice) dimension, got {dimension:?}"
                );
            }
            BudgetVerdict::Allow => panic!("expected Reject, got Allow"),
        }
    }

    // ── BudgetLedger: resets at window boundary ───────────────────────────────

    #[tokio::test]
    async fn budget_ledger_resets_at_window_boundary() {
        let limits = DimensionLimits {
            global: Some(100.0),
            ..Default::default()
        };
        // Very short window — 1 second.
        let window = Duration::from_secs(1);
        let ledger = InMemoryBudgetLedger::new(limits, window);

        // Record spend inside the window.
        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: None,
                api_key_id: None,
                cost_usd: 50.0,
                tokens_in: 1_000_000,
                tokens_out: 0,
                timestamp: SystemTime::now(),
            })
            .await;

        assert!(ledger.snapshot().global_spend_usd > 0.0);

        // Advance mock time past the window boundary by using a timestamp
        // that is 2 seconds in the future.
        let future = SystemTime::now() + Duration::from_secs(2);

        // spend_usd on the global entry will detect the elapsed window when queried.
        let spend_after_window = ledger.global.spend_usd(future);
        assert_eq!(spend_after_window, 0.0, "spend should reset to 0 after window boundary");
    }

    // ── BudgetSnapshot CSV export ─────────────────────────────────────────────

    #[tokio::test]
    async fn budget_snapshot_csv_export_round_trips() {
        let ledger = InMemoryBudgetLedger::new(DimensionLimits::default(), Duration::from_secs(3600));

        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: Some("tenant-x"),
                user_id: Some("user-y"),
                api_key_id: Some("key-z"),
                cost_usd: 1.23,
                tokens_in: 100,
                tokens_out: 50,
                timestamp: SystemTime::now(),
            })
            .await;

        let mut csv_bytes: Vec<u8> = Vec::new();
        ledger.export_csv(&mut csv_bytes).expect("CSV export must not fail");
        let csv = String::from_utf8(csv_bytes).expect("CSV must be valid UTF-8");

        // Verify the header row is present.
        assert!(csv.starts_with("dimension,spend_usd\n"), "missing header: {csv}");

        // Parse the CSV back and verify each row.
        let mut found_global = false;
        let mut found_model = false;
        let mut found_tenant = false;
        let mut found_user = false;
        let mut found_key = false;

        for line in csv.lines().skip(1) {
            let parts: Vec<&str> = line.splitn(2, ',').collect();
            assert_eq!(parts.len(), 2, "malformed CSV line: {line}");
            let dimension = parts[0];
            let spend: f64 = parts[1].parse().expect("spend must be a float");

            match dimension {
                "global" => {
                    assert!((spend - 1.23).abs() < 1e-6, "global spend mismatch: {spend}");
                    found_global = true;
                }
                "model:gpt-4" => {
                    assert!((spend - 1.23).abs() < 1e-6);
                    found_model = true;
                }
                "tenant:tenant-x" => {
                    assert!((spend - 1.23).abs() < 1e-6);
                    found_tenant = true;
                }
                "user:user-y" => {
                    assert!((spend - 1.23).abs() < 1e-6);
                    found_user = true;
                }
                "api_key:key-z" => {
                    assert!((spend - 1.23).abs() < 1e-6);
                    found_key = true;
                }
                _ => {}
            }
        }

        assert!(found_global, "global row missing from CSV");
        assert!(found_model, "model row missing from CSV");
        assert!(found_tenant, "tenant row missing from CSV");
        assert!(found_user, "user row missing from CSV");
        assert!(found_key, "api_key row missing from CSV");
    }

    // ── Window rollover concurrency ──────────────────────────────────────────

    /// Spawn 100 threads each calling `add($0.10)` exactly at the window
    /// boundary and assert the total is $10.00, not less.
    ///
    /// The CAS in `spend_usd` guarantees exactly one thread resets the window;
    /// the other 99 threads see the already-zeroed counter but still add their
    /// $0.10 contribution via `fetch_add`.  Without the CAS fix, both threads
    /// that race on the boundary would zero `spend_mc` independently, causing
    /// each other's prior `add` to be dropped.
    #[test]
    fn window_rollover_under_concurrent_threads_does_not_undercount() {
        use std::sync::Barrier;
        use std::thread;

        // Very short window (1 second) so we can trigger a rollover without
        // actually sleeping: we manually pass a "future" timestamp.
        let entry = Arc::new(WindowEntry::new(Duration::from_secs(1)));

        // All 100 threads will add $0.10 using a timestamp that is 2 seconds
        // past the window start — i.e., all threads see the window as expired
        // and race for the CAS.
        let future_now = SystemTime::now() + Duration::from_secs(2);

        let barrier = Arc::new(Barrier::new(100));
        let mut handles = Vec::with_capacity(100);

        for _ in 0..100 {
            let entry_clone = Arc::clone(&entry);
            let barrier_clone = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                // Synchronise all threads so they hit the window boundary at
                // the same instant, maximising the chance of a race.
                barrier_clone.wait();
                entry_clone.add(0.10, future_now);
            }));
        }

        for h in handles {
            h.join().expect("thread must not panic");
        }

        let total = microcents_to_usd(entry.spend_mc.load(Ordering::Acquire));
        // Allow a tiny floating-point rounding tolerance (microcents are
        // integers, so the real tolerance is 0 but we allow 1 µ$ of drift).
        assert!(
            (total - 10.0_f64).abs() < 1e-4,
            "expected $10.00 total, got ${total:.6} — window rollover race caused under-counting"
        );
    }

    /// Bug 6 fix: 200 parallel `add($0.10)` calls at a rollover boundary must
    /// total exactly $20.00 — no contribution lost due to TOCTOU.
    #[test]
    fn budget_window_rollover_no_torn_read() {
        use std::sync::Barrier;
        use std::thread;

        let entry = Arc::new(WindowEntry::new(Duration::from_secs(1)));
        let future_now = SystemTime::now() + Duration::from_secs(2);

        const WRITERS: usize = 200;
        let barrier = Arc::new(Barrier::new(WRITERS));
        let mut handles = Vec::with_capacity(WRITERS);
        for _ in 0..WRITERS {
            let e = Arc::clone(&entry);
            let b = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                b.wait();
                e.add(0.10, future_now);
            }));
        }
        for h in handles {
            h.join().expect("writer must not panic");
        }
        let total = microcents_to_usd(entry.spend_mc.load(Ordering::Acquire));
        assert!(
            (total - 20.0_f64).abs() < 1e-4,
            "expected $20.00 total after 200 concurrent adds at rollover; got ${total:.6}"
        );
    }

    // ── should_hedge: respects configured user budget ────────────────────────

    /// $10 user budget, $9.50 spend, estimated_cost=$0.50, safety_margin=0.10
    /// → effective limit = $10 × 0.90 = $9.00.
    /// $9.50 + 2×$0.50 = $10.50 ≥ $9.00 → hedging must be suppressed.
    #[tokio::test]
    async fn should_hedge_respects_user_budget() {
        let mut limits = DimensionLimits::default();
        limits.per_user.insert("alice".to_owned(), 10.0);

        let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: Some("alice"),
                api_key_id: None,
                cost_usd: 9.50,
                tokens_in: 100,
                tokens_out: 50,
                timestamp: SystemTime::now(),
            })
            .await;

        let ctx = CostCheckContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            timestamp: SystemTime::now(),
        };

        let result = should_hedge(&ledger, &ctx, 0.50, 0.10);
        assert!(
            !result,
            "hedging should be suppressed when user spend + 2×cost would exceed 90% of budget"
        );
    }

    /// Same $10 user budget but only $1.00 spend.
    /// $1.00 + 2×$0.50 = $2.00 < $9.00 → hedging must be allowed.
    #[tokio::test]
    async fn should_hedge_allows_when_far_below_budget() {
        let mut limits = DimensionLimits::default();
        limits.per_user.insert("alice".to_owned(), 10.0);

        let ledger = InMemoryBudgetLedger::new(limits, Duration::from_secs(3600));

        ledger
            .record(&CostRecordContext {
                model: "gpt-4",
                provider: "openai",
                tenant_id: None,
                user_id: Some("alice"),
                api_key_id: None,
                cost_usd: 1.00,
                tokens_in: 100,
                tokens_out: 50,
                timestamp: SystemTime::now(),
            })
            .await;

        let ctx = CostCheckContext {
            model: "gpt-4",
            provider: "openai",
            tenant_id: None,
            user_id: Some("alice"),
            api_key_id: None,
            timestamp: SystemTime::now(),
        };

        let result = should_hedge(&ledger, &ctx, 0.50, 0.10);
        assert!(
            result,
            "hedging should be allowed when user spend + 2×cost is well below 90% of budget"
        );
    }
}
