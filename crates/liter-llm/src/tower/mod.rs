//! Tower middleware integration for [`crate::client::LlmClient`].
//!
//! This module is only compiled when the `tower` feature is enabled.  It
//! provides:
//!
//! - [`types::LlmRequest`] / [`types::LlmResponse`] — the request/response
//!   enums that cross the tower `Service` boundary.
//! - [`service::LlmService`] — a thin `tower::Service` wrapper around any
//!   [`crate::client::LlmClient`].
//! - [`tracing::TracingLayer`] / [`tracing::TracingService`] — OTEL-compatible
//!   tracing middleware.
//! - [`fallback::FallbackLayer`] / [`fallback::FallbackService`] — route to a
//!   backup service on transient errors.
//! - [`cost::CostTrackingLayer`] / [`cost::CostTrackingService`] — emit
//!   `gen_ai.usage.cost` tracing span attribute from embedded pricing data.
//! - [`rate_limit::ModelRateLimitLayer`] / [`rate_limit::ModelRateLimitService`]
//!   — per-model RPM / TPM rate limiting.
//! - [`cache::CacheLayer`] / [`cache::CacheService`] — in-memory response
//!   caching for non-streaming requests.
//! - [`cooldown::CooldownLayer`] / [`cooldown::CooldownService`] — deployment
//!   cooldowns after transient errors.
//! - [`health::HealthCheckLayer`] / [`health::HealthCheckService`] — periodic
//!   health probes with automatic request rejection on failure.
//! - [`budget::BudgetLayer`] / [`budget::BudgetService`] — global and per-model
//!   spending budget enforcement (hard reject or soft warn).
//! - [`hooks::HooksLayer`] / [`hooks::HooksService`] — user-defined pre/post
//!   request hooks for guardrails, logging, and auditing.
//! - [`metrics::MetricsLayer`] / [`metrics::MetricsService`] — OTel-native
//!   GenAI semantic-convention metrics (histograms + counters).
//! - [`circuit::CircuitLayer`] / [`circuit::CircuitService`] — circuit breaker
//!   with pluggable [`circuit::CircuitPolicy`].
//! - [`hedge::HedgeLayer`] / [`hedge::HedgeService`] — hedged retry that races
//!   concurrent requests and cancels losers.
//!
//! # Example
//!
//! ```rust,ignore
//! use liter_llm::tower::{CostTrackingLayer, LlmService, TracingLayer};
//! use tower::ServiceBuilder;
//!
//! let client = liter_llm::DefaultClient::new(config, None)?;
//! let service = ServiceBuilder::new()
//!     .layer(TracingLayer)
//!     .layer(CostTrackingLayer)
//!     .service(LlmService::new(client));
//! ```

/// Token / cost budget enforcement layer.
pub mod budget;
/// Response-cache layer with pluggable in-memory backend.
pub mod cache;
/// Guardrail enforcement layer (content filtering, safety checks, policy evaluation).
pub mod guardrail;
#[cfg(feature = "opendal-cache")]
/// OpenDAL-backed cache backend for the response cache layer.
pub mod cache_opendal;
/// Circuit-breaker layer with pluggable [`circuit::CircuitPolicy`].
pub mod circuit;
/// Cooldown layer that backs off after upstream failures.
pub mod cooldown;
/// Cost-tracking layer that attaches token / dollar accounting to each call.
pub mod cost;
/// Staging area for new error variants (circuit-open, hedge-exhausted).
pub(crate) mod error;
/// Fallback layer that retries a failed call against a sibling provider.
pub mod fallback;
/// Health-probe layer used by the router to score upstream providers.
pub mod health;
/// Hedged-retry layer that races concurrent requests and cancels losers.
pub mod hedge;
/// User-supplied request/response hooks (mutators, observers).
pub mod hooks;
/// OTel-native GenAI semantic-convention metrics layer.
pub mod metrics;
/// Per-provider rate limiter.
pub mod rate_limit;
/// Provider routing strategies (round-robin, weighted, latency-aware).
pub mod router;
/// Wired-up Tower service type alias plus the public [`service::ManagedService`] entry-point.
pub mod service;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod tests_common;
/// Tracing spans / OpenTelemetry attributes attached at each Tower layer.
pub mod tracing;
/// Internal types shared by the Tower layers (errors, builder enums).
pub mod types;

// Re-export tower core types for convenient access
pub use tower::ServiceExt;

pub use budget::{BudgetConfig, BudgetLayer, BudgetService, BudgetState, Enforcement};
pub use cache::{CacheBackend, CacheConfig, CacheLayer, CacheService, CacheStore, CachedResponse, InMemoryStore};
pub use guardrail::{GuardrailLayer, GuardrailService};
#[cfg(feature = "opendal-cache")]
pub use cache_opendal::OpenDalCacheStore;
pub use circuit::{CircuitLayer, CircuitPolicy, CircuitService, CircuitState, ExponentialBackoffCircuit};
pub use cooldown::{CooldownLayer, CooldownService};
pub use cost::{CostTrackingLayer, CostTrackingService};
pub use fallback::{FallbackLayer, FallbackService};
pub use health::{
    HealthCheckConfig, HealthCheckLayer, HealthCheckService, HealthChecker, HealthStatus, HttpProbeHealthChecker,
    PerProviderHealthCheck,
};
pub use hedge::{FixedDelayHedge, HedgeLayer, HedgePolicy, HedgeService};
pub use hooks::{HooksLayer, HooksService, LlmHook};
pub use metrics::{MetricsLayer, MetricsService};
pub use rate_limit::{ModelRateLimitLayer, ModelRateLimitService, RateLimitConfig};
pub use router::{
    DEFAULT_CONCURRENCY_LIMIT, DynamicRouter, ProviderConfig, Router, RouterError, RoutingStrategy, StaticDiscover,
    UpstreamDiscover, Weight,
};
pub use service::LlmService;
pub use tracing::{TracingLayer, TracingService};
pub use types::{LlmRequest, LlmResponse};
