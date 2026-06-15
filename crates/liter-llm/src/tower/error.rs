//! Tower-layer error variants for circuit breaker and hedged retry.
//!
//! This file is a staging area for new error variants that workstream 1.A
//! will merge into [`crate::error::LiterLlmError`].  Until that merge lands,
//! these variants convert to [`crate::error::LiterLlmError::InternalError`]
//! via the `From` impls below.

use std::time::Duration;

/// Error produced by [`super::circuit::CircuitLayer`] when the circuit is open.
#[derive(Debug, thiserror::Error)]
#[error("circuit open for provider {provider}: retry after {retry_after:?}")]
pub struct CircuitOpenError {
    /// Provider or deployment name.
    pub provider: String,
    /// Suggested retry delay (exponential-backoff computed by the policy).
    pub retry_after: Duration,
}

/// Error produced by [`super::hedge::HedgeLayer`] when all attempts are
/// exhausted before any response arrives.
#[derive(Debug, thiserror::Error)]
#[error("all {attempts} hedged attempts exhausted")]
pub struct HedgeExhaustedError {
    /// Number of attempts that were launched.
    pub attempts: u32,
}

// ─── Conversions ─────────────────────────────────────────────────────────────

impl From<CircuitOpenError> for crate::error::LiterLlmError {
    fn from(e: CircuitOpenError) -> Self {
        // TODO(1.A): replace with a dedicated CircuitOpen variant.
        Self::ServiceUnavailable {
            message: e.to_string(),
            status: 503,
        }
    }
}

impl From<HedgeExhaustedError> for crate::error::LiterLlmError {
    fn from(_e: HedgeExhaustedError) -> Self {
        // TODO(1.A): replace with a dedicated HedgeExhausted variant.
        Self::Timeout
    }
}
