//! Canonical per-request usage events and pluggable sinks.
//!
//! [`UsageEvent`] is the billing/observability-agnostic shape emitted by
//! [`crate::tower::hooks::HooksLayer`] after every request completion.
//! Downstream sinks translate it into Prometheus metrics, OTel events,
//! append-only ledgers, or any other target without needing to define
//! their own event schema.
//!
//! # Sink implementations
//!
//! | Type | Behaviour |
//! |------|-----------|
//! | [`LoggingUsageSink`] | Emits a structured `tracing` INFO event — good for development and smoke tests. |
//! | [`MultiUsageSink`] | Fan-out: emits to multiple inner sinks concurrently; sink errors are logged, not returned. |
//!
//! # Example
//!
//! ```rust
//! use std::sync::Arc;
//! use liter_llm::observability::{LoggingUsageSink, MultiUsageSink, UsageSink};
//!
//! let sink: Arc<LoggingUsageSink> = Arc::new(LoggingUsageSink);
//! let _multi = MultiUsageSink::from_sinks(vec![sink]);
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::tenant::TenantId;

// ─── Event shape ─────────────────────────────────────────────────────────────

/// Canonical per-request usage event emitted by [`crate::tower::hooks::HooksLayer`]
/// after every request (success or failure).
///
/// This type is intentionally decoupled from any specific billing vendor or
/// observability stack. Sinks receive it and translate as needed.
#[cfg_attr(alef, alef(skip))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageEvent {
    /// Tenant that issued the request, when tenant context was attached via
    /// [`crate::tower::types::LlmRequest::with_tenant_id`].
    pub tenant_id: Option<TenantId>,

    /// Opaque request identifier.
    ///
    /// Set to the idempotency key when one is present; otherwise a
    /// monotonically-increasing counter string scoped to the process lifetime.
    /// Use this for deduplication and cross-log correlation.
    pub request_id: String,

    /// Model name as submitted in the request.
    pub model: String,

    /// Provider prefix extracted from `model` (the part before the first `/`,
    /// e.g. `"openai"` from `"openai/gpt-4o"`). Empty when the model string
    /// contains no prefix.
    pub provider: String,

    /// Prompt token count from the response usage block.
    /// Zero for request types that do not report token counts (image, speech, …).
    pub prompt_tokens: u64,
    /// Completion token count from the response usage block.
    pub completion_tokens: u64,
    /// Provider-reported cached prompt tokens (zero when not reported).
    pub cached_tokens: u64,
    /// Total tokens (prompt + completion).
    pub total_tokens: u64,

    /// Estimated cost in USD, or `Decimal::ZERO` when the model has no pricing entry.
    pub cost_usd: Decimal,

    /// Cache layer outcome for this request.
    ///
    /// Set by [`crate::tower::cache::CacheService`] via a task-local cell read
    /// by [`crate::tower::hooks::HooksService`] after the inner service resolves.
    pub cache_state: CacheState,

    /// Provider-echoed model name from the response, when available.
    ///
    /// Differs from [`Self::model`] when routing or fallback substitutes a
    /// different model than was requested (e.g. request asks for `"gpt-4o"` but
    /// the provider echoes `"gpt-4o-2024-08-06"`). `None` for response variants
    /// that do not carry a model field (streaming, speech, image, transcription,
    /// rerank, list-models) and on error paths where no response is available.
    pub effective_model: Option<String>,

    /// The `finish_reason` string from a chat response choice, when present.
    pub finish_reason: Option<String>,

    /// Whether the overall request succeeded, errored, timed out, or was cancelled.
    pub outcome: UsageEventOutcome,

    /// Request latency measured by the hooks layer (inner `call` duration, ms).
    pub latency_ms: u64,

    /// Free-form metadata that sinks can inspect without adding fields to this struct.
    pub metadata: HashMap<String, String>,

    /// Wall-clock time at which this event was created (just after the request completed).
    pub received_at: SystemTime,
}

// ─── CacheState ──────────────────────────────────────────────────────────────

/// Cache outcome for a single request.
#[cfg_attr(alef, alef(skip))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheState {
    /// No cache entry found; request was sent to the provider.
    Miss,
    /// Exact-match cache hit; provider was not called.
    ExactHit,
    /// Semantic-similarity cache hit; provider was not called.
    SemanticHit,
    /// Stale entry served (TTL expired but no fresh entry was available).
    StaleHit,
    /// Cache lookup was skipped (bypass policy, streaming request, etc.).
    #[default]
    Bypass,
}

// ─── UsageEventOutcome ───────────────────────────────────────────────────────

/// High-level outcome of the request.
#[cfg_attr(alef, alef(skip))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageEventOutcome {
    /// Inner service returned a successful response.
    Success,
    /// Inner service returned an error (non-timeout).
    Error,
    /// Request was cancelled before the inner service responded.
    Cancelled,
    /// Inner service timed out.
    TimedOut,
}

// ─── UsageSink ───────────────────────────────────────────────────────────────

/// Pluggable consumer of [`UsageEvent`]s.
///
/// Implementations should be cheap on the hot path — defer heavy I/O to
/// their own background task or channel. The `emit` future is awaited
/// directly in the Tower request path, so blocking I/O increases tail latency.
#[cfg_attr(alef, alef(skip))]
pub trait UsageSink: Send + Sync + 'static {
    /// Emit a single usage event.
    ///
    /// Errors from this call are logged but do not propagate to the caller
    /// of the LLM request.
    fn emit(&self, event: UsageEvent) -> impl Future<Output = Result<(), UsageSinkError>> + Send;
}

// ─── Object-safe erased helper ───────────────────────────────────────────────

// `UsageSink` uses RPITIT which is not object-safe. This sealed helper trait
// lets `HooksLayer` and `MultiUsageSink` store sinks behind `dyn` pointers.
//
// It is `pub` (not `pub(crate)`) so that users can pass heterogeneous sink
// collections to [`MultiUsageSink::from_erased`] without reaching into the
// crate internals.  The trait is intentionally not re-exported from the crate
// root to discourage direct implementation — use [`UsageSink`] instead.
#[cfg_attr(alef, alef(skip))]
pub trait UsageSinkErased: Send + Sync + 'static {
    fn emit_erased<'a>(
        &'a self,
        event: UsageEvent,
    ) -> Pin<Box<dyn Future<Output = Result<(), UsageSinkError>> + Send + 'a>>;
}

impl<T: UsageSink> UsageSinkErased for T {
    fn emit_erased<'a>(
        &'a self,
        event: UsageEvent,
    ) -> Pin<Box<dyn Future<Output = Result<(), UsageSinkError>> + Send + 'a>> {
        Box::pin(self.emit(event))
    }
}

// ─── UsageSinkError ──────────────────────────────────────────────────────────

/// Error returned by a [`UsageSink`] implementation.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, thiserror::Error)]
pub enum UsageSinkError {
    /// The sink's backend failed to accept the event.
    #[error("usage sink backend error: {0}")]
    Backend(String),
}

// ─── LoggingUsageSink ────────────────────────────────────────────────────────

/// `tracing`-backed sink that emits each [`UsageEvent`] as a structured
/// `INFO` event on the `gen_ai.usage` target.
///
/// Useful in development and as a smoke-test default. No I/O is performed;
/// the sink is always cheap.
#[cfg_attr(alef, alef(skip))]
#[derive(Clone, Debug, Default)]
pub struct LoggingUsageSink;

impl UsageSink for LoggingUsageSink {
    #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
    async fn emit(&self, event: UsageEvent) -> Result<(), UsageSinkError> {
        #[cfg(feature = "tracing")]
        tracing::info!(
            target: "gen_ai.usage",
            tenant_id = event.tenant_id.as_ref().map(|t| t.as_ref()),
            request_id = %event.request_id,
            model = %event.model,
            effective_model = event.effective_model.as_deref(),
            provider = %event.provider,
            prompt_tokens = event.prompt_tokens,
            completion_tokens = event.completion_tokens,
            cost_usd = %event.cost_usd,
            cache_state = ?event.cache_state,
            outcome = ?event.outcome,
            latency_ms = event.latency_ms,
            "usage_event"
        );
        Ok(())
    }
}

// ─── MultiUsageSink ──────────────────────────────────────────────────────────

/// Fan-out sink that delivers each event to multiple inner sinks concurrently.
///
/// Individual sink errors are logged but do not cause `emit` to return `Err`.
/// Use this to layer, say, a logging sink and a database sink without
/// coupling the error semantics of one to the other.
#[cfg_attr(alef, alef(skip))]
pub struct MultiUsageSink {
    sinks: Vec<Arc<dyn UsageSinkErased>>,
}

impl MultiUsageSink {
    /// Build a fan-out sink from a homogeneous list of inner sinks.
    ///
    /// Each element must be `Arc<S>` where `S: UsageSink`. For heterogeneous
    /// compositions (mixing sink types), use [`Self::from_erased`] instead.
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use liter_llm::observability::{LoggingUsageSink, MultiUsageSink};
    ///
    /// let _multi = MultiUsageSink::from_sinks(vec![Arc::new(LoggingUsageSink)]);
    /// ```
    #[must_use]
    pub fn from_sinks<S: UsageSink>(sinks: Vec<Arc<S>>) -> Self {
        Self {
            sinks: sinks.into_iter().map(|s| s as Arc<dyn UsageSinkErased>).collect(),
        }
    }

    /// Build a fan-out sink from a heterogeneous list of already-erased sinks.
    ///
    /// Use this when you need to combine sinks of different concrete types in a
    /// single `MultiUsageSink`:
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use liter_llm::observability::{
    ///     LoggingUsageSink, MultiUsageSink, UsageEvent, UsageSink, UsageSinkError,
    ///     UsageSinkErased,
    /// };
    ///
    /// #[derive(Default)]
    /// struct MetricsSink;
    /// impl UsageSink for MetricsSink {
    ///     async fn emit(&self, _event: UsageEvent) -> Result<(), UsageSinkError> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let multi = MultiUsageSink::from_erased(vec![
    ///     Arc::new(LoggingUsageSink) as Arc<dyn UsageSinkErased>,
    ///     Arc::new(MetricsSink::default()) as Arc<dyn UsageSinkErased>,
    /// ]);
    /// ```
    #[must_use]
    pub fn from_erased(sinks: Vec<Arc<dyn UsageSinkErased>>) -> Self {
        Self { sinks }
    }

    /// Build an empty fan-out sink.
    #[must_use]
    pub fn empty() -> Self {
        Self { sinks: Vec::new() }
    }

    /// Append an inner sink.
    pub fn push<S: UsageSink>(&mut self, sink: Arc<S>) {
        self.sinks.push(sink as Arc<dyn UsageSinkErased>);
    }

    /// Append an already-erased sink.
    pub fn push_erased(&mut self, sink: Arc<dyn UsageSinkErased>) {
        self.sinks.push(sink);
    }
}

impl UsageSink for MultiUsageSink {
    async fn emit(&self, event: UsageEvent) -> Result<(), UsageSinkError> {
        // Sequential emit — fan-out sinks are best-effort observability; if
        // one sink is slow, it adds latency but does not block correctness.
        // Sequentialising avoids pulling `futures_util` into minimal builds
        // (JNI/wasm umbrella crates).  Use `join_all` from `futures_util`
        // explicitly in your own `UsageSink` impl if concurrent fan-out is
        // important — it's a workspace dep gated behind the `tower` feature.
        for sink in &self.sinks {
            if let Err(_err) = sink.emit_erased(event.clone()).await {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    target: "gen_ai.usage",
                    error = %_err,
                    "usage sink emit failed"
                );
            }
        }
        Ok(())
    }
}
