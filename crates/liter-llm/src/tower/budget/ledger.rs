//! [`InMemoryBudgetLedger`] construction, sliding-window bookkeeping, and its
//! [`BudgetLedger`] implementation.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime};

use arc_swap::ArcSwap;
use dashmap::DashMap;

use super::{
    BudgetConfig, BudgetDimension, BudgetLedger, BudgetSnapshot, BudgetVerdict, CostCheckContext, CostRecordContext,
    DimensionLimits, InMemoryBudgetLedger, WindowEntry,
};

impl InMemoryBudgetLedger {
    /// Default cap on distinct entries tracked by `per_user` and
    /// `per_api_key`.
    ///
    /// Both maps are keyed by attacker-controlled, arbitrary-cardinality input
    /// (the request's free-text `user` field / a caller-supplied API-key id),
    /// so without a cap a caller can grow either map without bound simply by
    /// varying that field across requests — unbounded memory growth.
    /// `per_model`/`per_tenant` are operator-controlled and naturally
    /// bounded, so they are not capped. ~keep
    pub const DEFAULT_MAX_PRINCIPAL_ENTRIES: usize = 100_000;

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
            limits: ArcSwap::from_pointee(limits),
            window,
            max_principal_entries: Self::DEFAULT_MAX_PRINCIPAL_ENTRIES,
        }
    }

    /// Override the cap on tracked `per_user` / `per_api_key` entries.
    ///
    /// Opt-in escape hatch for deployments with either a much larger or much
    /// smaller expected principal cardinality than
    /// [`Self::DEFAULT_MAX_PRINCIPAL_ENTRIES`].
    #[must_use]
    pub fn with_max_principal_entries(mut self, max_principal_entries: usize) -> Self {
        self.max_principal_entries = max_principal_entries.max(1);
        self
    }

    /// Replace the configured per-dimension limits in place, preserving every
    /// sliding-window spend entry already accumulated.
    ///
    /// This is the fix for a config-reload-time budget reset: rebuilding a
    /// fresh [`InMemoryBudgetLedger`] to pick up new limits used to discard
    /// all `DashMap` spend entries along with the stale limits. Swapping only
    /// the `limits` pointer leaves `global`/`per_model`/`per_tenant`/
    /// `per_user`/`per_api_key` untouched, so month-to-date spend survives a
    /// reload.
    ///
    /// # Behavior on lowered limits
    ///
    /// If a limit drops below a dimension's already-accumulated spend, the
    /// next [`BudgetLedger::check`] call sees the (unchanged) spend against
    /// the new, lower limit and rejects immediately — existing spend is never
    /// forgiven. This is deliberate: silently resetting spend on a limit
    /// change is the exact failure mode this method exists to close.
    ///
    /// # Behavior on removed dimension keys
    ///
    /// A tenant/user/API-key dropped from `limits` (e.g. a virtual key
    /// removed from config) keeps its `DashMap` window entry — only the
    /// enforced cap disappears, so the key becomes unconstrained until a
    /// limit is configured for it again. Spend history is retained in case
    /// the same key is re-added later, rather than being silently discarded.
    pub fn update_limits(&self, limits: DimensionLimits) {
        self.limits.store(Arc::new(limits));
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
        let zero_secs = SystemTime::UNIX_EPOCH
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.global.spend_mc.store(0, Ordering::Relaxed);
        self.global.window_start_secs.store(zero_secs, Ordering::Relaxed);
        let _ = self.global.spend_usd(now);

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

    /// Like [`Self::entry_add`], but bounds `map` at `self.max_principal_entries`
    /// — used for `per_user` / `per_api_key`, whose keys are attacker-controlled.
    ///
    /// An already-tracked key always gets its spend recorded, regardless of
    /// how full `map` is: the cap only ever gates *new* keys, never starves an
    /// existing principal of accounting. See the [`InMemoryBudgetLedger`]
    /// doc comment for why a full map declines new principals instead of
    /// evicting a live one.
    fn entry_add_capped(
        &self,
        map: &DashMap<String, WindowEntry>,
        dimension_name: &'static str,
        key: &str,
        usd: f64,
        now: SystemTime,
    ) {
        let max_entries = self.max_principal_entries;
        let window = self.window;

        if !map.contains_key(key) && map.len() >= max_entries {
            Self::reclaim_zero_spend_entries(map, dimension_name, now);
        }

        if !map.contains_key(key) && map.len() >= max_entries {
            // ~keep Refuse to allocate tracking state for a brand-new principal rather
            // ~keep than evict a live one to make room — see the InMemoryBudgetLedger
            // ~keep doc comment. Global/per-model spend for this call is still recorded
            // ~keep by the caller; only this one dimension's enforcement is affected.
            tracing::warn!(
                dimension = dimension_name,
                cap = max_entries,
                "budget ledger at capacity; declining to track new principal, spend for this call was not recorded"
            );
            return;
        }

        map.entry(key.to_owned())
            .or_insert_with(|| WindowEntry::new(window))
            .add(usd, now);
    }

    /// Remove entries from `map` whose recorded spend reads as exactly zero,
    /// reclaiming capacity without ever discarding a live spend record.
    fn reclaim_zero_spend_entries(map: &DashMap<String, WindowEntry>, dimension_name: &'static str, now: SystemTime) {
        let mut evicted_live_spend_usd = 0.0_f64;

        map.retain(|_, entry| {
            let spend = entry.spend_usd(now);
            if spend > 0.0 {
                return true;
            }
            // ~keep Defensive invariant, not expected to ever be non-zero: this branch is
            // ~keep reached only when `spend <= 0.0`, so `evicted_live_spend_usd` should
            // ~keep never accumulate anything. Kept as a canary — if this ever fires, a
            // ~keep future change broke the "eviction never touches live spend" guarantee
            // ~keep this ledger relies on to avoid becoming a budget bypass.
            evicted_live_spend_usd += spend;
            false
        });

        if evicted_live_spend_usd > 0.0 {
            tracing::warn!(
                dimension = dimension_name,
                evicted_spend_usd = evicted_live_spend_usd,
                "budget ledger eviction reclaimed entries carrying non-zero spend; budget enforcement was weakened"
            );
        }
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
                self.entry_add_capped(&self.per_user, "user", user, ctx.cost_usd, now);
            }
            if let Some(key) = ctx.api_key_id {
                self.entry_add_capped(&self.per_api_key, "api_key", key, ctx.cost_usd, now);
            }

            #[cfg(feature = "otel")]
            {
                use crate::tower::metrics;
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
            // ~keep load_full (owned Arc) rather than load (thread-local Guard) because this
            // ~keep async block must produce a Send future; an owned Arc<DimensionLimits> is
            // ~keep unambiguously Send, avoiding any question about Guard's Send-ness here.
            let limits = self.limits.load_full();

            if let Some(limit) = limits.global {
                let spend = self.global.spend_usd(now);
                if let Some(v) = Self::check_limit(spend, limit, BudgetDimension::Global, "global") {
                    return v;
                }
            }

            if let Some(&limit) = limits.per_model.get(ctx.model) {
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

            if let Some(tenant) = ctx.tenant_id
                && let Some(&limit) = limits.per_tenant.get(tenant)
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

            if let Some(user) = ctx.user_id
                && let Some(&limit) = limits.per_user.get(user)
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

            if let Some(key) = ctx.api_key_id
                && let Some(&limit) = limits.per_api_key.get(key)
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
        let limits = self.limits.load();

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
            limit_global: limits.global,
            limits_per_user: limits.per_user.clone(),
            limits_per_api_key: limits.per_api_key.clone(),
            limits_per_tenant: limits.per_tenant.clone(),
        }
    }
}
