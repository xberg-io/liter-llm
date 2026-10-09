use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

/// Determine whether to retry based on status code and attempt number.
///
/// Returns `Some(delay)` if the request should be retried, `None` otherwise.
///
/// When `retry_after` is provided (parsed from the `Retry-After` response
/// header) it takes precedence over exponential backoff for 429 and 529
/// and 503 responses.
///
/// Exponential backoff includes jitter to prevent thundering-herd effects
/// when multiple clients retry simultaneously. The jitter scales the delay
/// to a random value in `[0.5 * base, 1.0 * base]` using the low-order bits
/// of the system clock as a lightweight entropy source.
pub fn should_retry(status: u16, attempt: u32, max_retries: u32, retry_after: Option<Duration>) -> Option<Duration> {
    if attempt >= max_retries {
        return None;
    }

    // ~keep 529 is Anthropic's non-standard "Overloaded" status (not in the IANA HTTP
    // status registry), used the same way other providers use 429/503: back off and retry.
    if !matches!(status, 429 | 500 | 502 | 503 | 504 | 529) {
        return None;
    }

    if matches!(status, 429 | 503 | 529)
        && let Some(server_delay) = retry_after
    {
        return Some(server_delay.min(Duration::from_secs(60)));
    }

    Some(exponential_backoff(attempt))
}

/// Determine whether to retry after a transport-level failure (connection
/// reset, DNS failure, TLS handshake failure, request timeout) rather than an
/// HTTP status response.
///
/// No response was received, so there is no status code or `Retry-After`
/// header to consult — this always falls back to the same jittered
/// exponential backoff used for 5xx / 429 status retries.
pub fn should_retry_transport_error(attempt: u32, max_retries: u32) -> Option<Duration> {
    if attempt >= max_retries {
        return None;
    }

    Some(exponential_backoff(attempt))
}

/// Jittered exponential backoff shared by status-based and transport-error retries.
///
/// Base delay doubles per attempt (`2^attempt` seconds), capped at 30s, then
/// jittered to `[0.5, 1.0]` of that value.
fn exponential_backoff(attempt: u32) -> Duration {
    let base_delay = Duration::from_secs(1u64.checked_shl(attempt).unwrap_or(u64::MAX));
    let capped = base_delay.min(Duration::from_secs(30));
    jittered(capped)
}

/// Apply jitter to a retry delay.
///
/// Scales `delay` to a random value in `[0.5 * delay, 1.0 * delay]` using the
/// low-order bits of the system clock as a lightweight entropy source.
///
/// On `wasm32-unknown-unknown` `SystemTime::now()` panics with `unreachable`
/// (time is not implemented in the bare wasm target). On wasm we skip jitter
/// and return the delay unchanged — a deterministic exponential backoff is
/// acceptable for browser/SDK use.
#[cfg(not(target_arch = "wasm32"))]
fn jittered(delay: Duration) -> Duration {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let jitter_factor = 0.5 + (f64::from(nanos % 1000) / 2000.0);
    delay.mul_f64(jitter_factor)
}

#[cfg(target_arch = "wasm32")]
fn jittered(delay: Duration) -> Duration {
    delay
}

/// Parse the value of a `Retry-After` header into a `Duration`.
///
/// The header may be:
/// - A non-negative integer (number of seconds to wait), or
/// - An HTTP-date (RFC 7231 format).
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    let trimmed = value.trim();

    if let Ok(secs) = trimmed.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(deadline) = httpdate::parse_http_date(trimmed) {
        return deadline.duration_since(SystemTime::now()).ok();
    }

    tracing::warn!(
        retry_after = trimmed,
        "invalid Retry-After header; falling back to exponential backoff"
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_on_529_overloaded() {
        // ~keep Regression test for #50: Anthropic's 529 "Overloaded" was not in the
        // retryable status set at all, so it was never retried.
        assert!(should_retry(529, 0, 3, None).is_some());
        assert!(should_retry(529, 2, 3, None).is_some());
        assert!(
            should_retry(529, 3, 3, None).is_none(),
            "must still respect max_retries"
        );
    }

    #[test]
    fn retry_after_header_respected_on_529() {
        // ~keep Regression test for #50: Retry-After was only honored for 429, so a
        // 529 response with a server-provided delay fell through to blind exponential
        // backoff instead of the delay the server actually asked for.
        let server_delay = Duration::from_secs(17);
        let delay = should_retry(529, 0, 3, Some(server_delay)).expect("should retry on 529 with Retry-After");
        assert_eq!(delay, server_delay);
    }

    #[test]
    fn retry_after_header_respected_on_503() {
        let server_delay = Duration::from_secs(3);
        assert_eq!(should_retry(503, 0, 3, Some(server_delay)), Some(server_delay));
    }

    #[test]
    fn retry_after_still_capped_at_60s_on_529() {
        let server_delay = Duration::from_secs(999);
        let delay = should_retry(529, 0, 3, Some(server_delay)).expect("should retry on 529 with Retry-After");
        assert_eq!(delay, Duration::from_secs(60));
    }

    #[test]
    fn retry_after_still_ignored_on_500() {
        // ~keep Unchanged prior behavior: a generic 5xx does not honor Retry-After,
        // only the explicit throttling signals (429, 529) do.
        let server_delay = Duration::from_secs(42);
        let delay = should_retry(500, 0, 3, Some(server_delay)).expect("should retry on 500");
        assert!(
            delay < server_delay,
            "500 must fall back to exponential backoff, not the server delay"
        );
    }

    #[test]
    fn should_retry_transport_error_respects_max_retries() {
        assert!(should_retry_transport_error(0, 3).is_some());
        assert!(should_retry_transport_error(2, 3).is_some());
        assert!(should_retry_transport_error(3, 3).is_none());
    }

    #[test]
    fn should_retry_transport_error_none_when_retries_disabled() {
        assert!(should_retry_transport_error(0, 0).is_none());
    }
}
