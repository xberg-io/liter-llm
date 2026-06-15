//! Memory-bound constants and helpers for bounded buffer guards.
//!
//! All stream parsers and body accumulators in the library must obey a limit to
//! prevent unbounded memory growth under adversarial or broken upstreams.  This
//! module centralises those limits so they can be tuned in a single place and
//! referenced consistently across SSE, EventStream, and request-body paths.

use crate::error::{LiterLlmError, Result};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum bytes buffered in the SSE line parser before the stream is aborted.
///
/// This matches the value in `http::streaming` and is re-exported here so
/// application code can reference it without depending on internal modules.
pub const SSE_BUFFER_MAX_BYTES: usize = 1024 * 1024; // 1 MiB

/// Maximum bytes buffered in the AWS EventStream binary parser before abort.
///
/// Matches `http::eventstream::MAX_FRAME_SIZE`.
pub const EVENT_STREAM_BUFFER_MAX_BYTES: usize = 16 * 1024 * 1024; // 16 MiB

/// Maximum bytes accepted in a non-streaming response body before abort.
///
/// Protects against upstreams that send unexpectedly large JSON bodies.
/// Set conservatively at 32 MiB — the largest plausible chat completion body.
pub const RESPONSE_BODY_MAX_BYTES: usize = 32 * 1024 * 1024; // 32 MiB

/// Maximum bytes accumulated when collecting streamed chunks into a Vec for
/// non-streaming response assembly.
///
/// Set to the same value as [`RESPONSE_BODY_MAX_BYTES`].
pub const CHUNK_ACCUMULATION_MAX_BYTES: usize = RESPONSE_BODY_MAX_BYTES;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Assert that `current_len + incoming` does not exceed `limit`.
///
/// Call this before appending `incoming` bytes to any buffer that must
/// stay below `limit`.  Returns `Err(LiterLlmError::Streaming)` on overflow
/// and emits a `tracing::warn!` with context.
///
/// # Example
///
/// ```ignore
/// check_bound("SSE buffer", buffer.len(), chunk.len(), SSE_BUFFER_MAX_BYTES)?;
/// buffer.push_str(chunk_str);
/// ```
pub fn check_bound(context: &str, current_len: usize, incoming: usize, limit: usize) -> Result<()> {
    if current_len.saturating_add(incoming) > limit {
        #[cfg(feature = "tracing")]
        tracing::warn!(
            context,
            current_len,
            incoming,
            limit,
            "buffer limit exceeded; aborting stream"
        );
        return Err(LiterLlmError::Streaming {
            message: format!("{context} buffer exceeded {limit} bytes; aborting"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_bound_passes_when_within_limit() {
        assert!(check_bound("test", 100, 50, 200).is_ok());
    }

    #[test]
    fn check_bound_passes_at_exact_limit() {
        assert!(check_bound("test", 100, 100, 200).is_ok());
    }

    #[test]
    fn check_bound_fails_when_exceeds_limit() {
        let err = check_bound("test ctx", 100, 101, 200).unwrap_err();
        assert!(err.to_string().contains("test ctx"));
        assert!(err.to_string().contains("200"));
    }

    #[test]
    fn check_bound_saturating_add_does_not_overflow() {
        // Ensure usize::MAX additions don't panic.
        let err = check_bound("overflow", usize::MAX, 1, 1024).unwrap_err();
        assert!(err.to_string().contains("overflow"));
    }

    #[test]
    fn sse_constant_is_one_mib() {
        assert_eq!(SSE_BUFFER_MAX_BYTES, 1024 * 1024);
    }

    #[test]
    fn event_stream_constant_is_sixteen_mib() {
        assert_eq!(EVENT_STREAM_BUFFER_MAX_BYTES, 16 * 1024 * 1024);
    }

    #[test]
    fn response_body_constant_is_thirty_two_mib() {
        assert_eq!(RESPONSE_BODY_MAX_BYTES, 32 * 1024 * 1024);
    }
}
