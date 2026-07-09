//! Guardrail plugin system for liter-llm.
//!
//! Provides a vendor-neutral, trait-based plugin system for content filtering,
//! safety checks, and policy enforcement across all LLM request/response stages.
//!
//! # Architecture
//!
//! - [`Guardrail`] — the core trait that every plugin must implement.
//! - [`GuardrailStage`] — the lifecycle stage at which a guardrail runs.
//! - [`GuardrailContext`] — the payload passed to each guardrail check.
//! - [`GuardrailDecision`] — the outcome: allow, block, or mutate.
//! - [`GuardrailRegistry`] — ordered registry; first `Block` short-circuits.
//!
//! # Vendor neutrality
//!
//! No vendor-specific guardrails (Presidio, Lakera, Bedrock Guardrails) ship
//! in this module. Users plug their own implementations via the [`Guardrail`]
//! trait and register them with the global registry or a local
//! [`GuardrailRegistry`] instance.
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use liter_llm::guardrail::{GuardrailRegistry, builtin::DenyListGuardrail};
//!
//! let mut registry = GuardrailRegistry::new();
//! registry.register(Arc::new(DenyListGuardrail::new(
//!     "blocked-tenants",
//!     ["tenant-evil"].into_iter().map(String::from).collect(),
//!     "tenant_id",
//! )));
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub mod builtin;
#[cfg(feature = "guardrail-cel")]
pub mod cel;
pub mod registry;
#[cfg(test)]
mod tests;

pub use registry::GuardrailRegistry;

/// Core trait for all guardrail implementations.
///
/// Implement this trait to create a guardrail plugin. All implementations must
/// be `Send + Sync + 'static` to support concurrent request handling.
///
/// Vendor-specific guardrails (Presidio, Lakera, Bedrock Guardrails) are
/// intentionally excluded from this crate. Users plug them in via this trait.
#[cfg_attr(alef, alef(skip))]
pub trait Guardrail: Send + Sync + 'static {
    /// Human-readable identifier used in logs and metrics.
    ///
    /// Must be `'static` so it can be used as a metric label without allocation.
    fn name(&self) -> &'static str;

    /// The stages at which this guardrail can run.
    ///
    /// The guardrail will only be invoked at stages listed here.
    /// Returning an empty slice is valid (the guardrail never fires).
    fn supported_stages(&self) -> &'static [GuardrailStage];

    /// Run the guardrail check at the given stage with the provided context.
    ///
    /// Return [`GuardrailDecision::Allow`] to pass through, [`GuardrailDecision::Block`]
    /// to short-circuit with a rejection, or [`GuardrailDecision::Mutate`] to rewrite
    /// the payload (for redaction use cases).
    ///
    /// For [`GuardrailStage::OutputChunk`] implementations, this is called once
    /// per streaming chunk. Implementations may return `Mutate` to redact
    /// individual chunks or `Block` to terminate the stream.
    fn check<'a>(
        &'a self,
        stage: GuardrailStage,
        ctx: &'a GuardrailContext<'a>,
    ) -> Pin<Box<dyn Future<Output = GuardrailDecision> + Send + 'a>>;
}

/// The lifecycle stage at which a guardrail runs.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuardrailStage {
    /// The outgoing prompt / request, before forwarding to the upstream provider.
    Input,
    /// The full response from the upstream provider (non-streaming).
    Output,
    /// A single chunk in a streaming response. Guardrails here are called once
    /// per chunk and may block or mutate individual chunks.
    OutputChunk,
}

/// Per-call context passed to every guardrail check.
///
/// At `Input` stage: `request` is populated, `response` and `chunk` are `None`.
/// At `Output` stage: both `request` and `response` are populated, `chunk` is `None`.
/// At `OutputChunk` stage: `request` is populated, `chunk` holds the raw chunk
/// text, and `response` is `None` (the full response is not yet available).
#[cfg_attr(alef, alef(skip))]
pub struct GuardrailContext<'a> {
    /// The full JSON request body sent to the provider.
    pub request: &'a serde_json::Value,
    /// The full JSON response body received from the provider (`Output` stage only).
    pub response: Option<&'a serde_json::Value>,
    /// The raw text of the current streaming chunk (`OutputChunk` stage only).
    pub chunk: Option<&'a str>,
    /// Per-call metadata tags such as `user_id`, `tenant_id`, and `route`.
    /// Populated by the caller (e.g., the Tower layer or the application).
    pub metadata: &'a HashMap<String, String>,
}

/// The outcome of a guardrail check.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug)]
pub enum GuardrailDecision {
    /// The check passed. Continue to the next guardrail or to the inner service.
    Allow,
    /// The check failed. Short-circuit the request/response with this reason.
    ///
    /// `code` should be ≥ 1000 to avoid collision with HTTP status codes and
    /// to facilitate cross-language error mapping.
    Block {
        /// Human-readable explanation of why the request/response was blocked.
        reason: String,
        /// Numeric error code (≥ 1000) for programmatic handling and FFI mapping.
        code: u32,
    },
    /// Rewrite the payload. The provided `new_payload` replaces the original
    /// `request` or `response` before it reaches the next stage.
    ///
    /// For `OutputChunk` stage: `new_payload` replaces the chunk content.
    Mutate {
        /// The replacement JSON payload.
        new_payload: serde_json::Value,
    },
}

impl GuardrailDecision {
    /// Returns `true` if this decision blocks the request/response.
    #[must_use]
    pub fn is_block(&self) -> bool {
        matches!(self, Self::Block { .. })
    }

    /// Returns `true` if this decision allows the request/response through.
    #[must_use]
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow)
    }
}
