// Error variants are documented through `thiserror`'s `#[error("...")]`
// messages, which double as user-facing display strings and rustdoc-style
// summaries. `missing_docs` would force us to duplicate those messages.
#![allow(missing_docs)]

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Error response from an OpenAI-compatible API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ErrorResponse {
    error: ApiError,
}

/// Inner error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ApiError {
    message: String,
    #[serde(default)]
    code: Option<String>,
}

/// All errors that can occur when using `liter-llm`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LiterLlmError {
    /// `status` preserves the exact HTTP status code received (401 or 403).
    #[error("authentication failed: {message}")]
    Authentication { message: String, status: u16 },

    #[error("rate limited: {message}")]
    RateLimited {
        message: String,
        retry_after: Option<Duration>,
    },

    /// `status` preserves the exact HTTP status code received (400, 405, 413, 422, …).
    #[error("bad request: {message}")]
    BadRequest { message: String, status: u16 },

    #[error("context window exceeded: {message}")]
    ContextWindowExceeded { message: String },

    #[error("content policy violation: {message}")]
    ContentPolicy { message: String },

    #[error("not found: {message}")]
    NotFound { message: String },

    /// `status` preserves the exact HTTP status code received (500, or other 5xx not covered
    /// by `ServiceUnavailable`).
    #[error("server error: {message}")]
    ServerError { message: String, status: u16 },

    /// `status` preserves the exact HTTP status code received (502, 503, or 504).
    #[error("service unavailable: {message}")]
    ServiceUnavailable { message: String, status: u16 },

    #[error("request timeout")]
    Timeout,

    #[cfg(any(feature = "native-http", feature = "wasm-http"))]
    #[error(transparent)]
    Network(#[from] reqwest::Error),

    /// A catch-all for errors that occur during streaming response processing.
    ///
    /// This variant covers multiple sub-conditions including UTF-8 decoding
    /// failures, CRC/checksum mismatches (AWS EventStream), JSON parse errors
    /// in individual SSE chunks, and buffer overflow conditions.  The `message`
    /// field contains a human-readable description of the specific failure.
    #[error("streaming error: {message}")]
    Streaming { message: String },

    #[error("provider {provider} does not support {endpoint}")]
    EndpointNotSupported { endpoint: String, provider: String },

    #[error("invalid header {name:?}: {reason}")]
    InvalidHeader { name: String, reason: String },

    #[error("serialization error: {0}")]
    Serialization(
        #[from]
        #[cfg_attr(alef, alef(skip))]
        serde_json::Error,
    ),

    #[error("budget exceeded: {message}")]
    BudgetExceeded { message: String, model: Option<String> },

    #[error("hook rejected: {message}")]
    HookRejected { message: String },

    /// An internal logic error (e.g. unexpected Tower response variant).
    ///
    /// This should never surface in normal operation — if it does, it
    /// indicates a bug in the library.
    #[error("internal error: {message}")]
    InternalError { message: String },

    /// An outbound request was blocked by the active [`crate::provider::OutboundPolicy`].
    ///
    /// Returned when `register_custom_provider` is called with a `base_url` that
    /// violates the policy (e.g. a private-range IP under `DenyPrivate`), or when
    /// the per-connection DNS resolver detects a forbidden address at connect time.
    #[error("outbound request to {url} forbidden: {reason}")]
    OutboundForbidden { url: String, reason: String },

    /// A different request body was submitted for an existing `Idempotency-Key`.
    ///
    /// Per the OpenAI `Idempotency-Key` convention, once a key is used with a
    /// particular request body, subsequent requests using the same key must carry
    /// an identical body.  A body mismatch is a hard error (not retryable).
    ///
    /// HTTP equivalent: 409 Conflict.
    #[error("idempotency conflict: key '{key}' was already used with a different request body")]
    IdempotencyConflict { key: String },

    /// The same `Idempotency-Key` is already in-flight (another request with the
    /// same key is currently being processed).
    ///
    /// The caller should wait briefly and retry.  The response is not yet
    /// available, and this request has been short-circuited to avoid running
    /// the operation twice.
    ///
    /// HTTP equivalent: 409 Conflict (retryable after a brief delay).
    #[error("idempotency key '{key}' is currently in-flight; retry after the first request completes")]
    IdempotencyInFlight { key: String },
}

impl LiterLlmError {
    /// Returns the canonical HTTP status code associated with this error.
    ///
    /// Maps error variants to their originating HTTP status code as set by
    /// [`LiterLlmError::from_status`].  Used by e2e assertions that check
    /// `error.status_code` against the expected HTTP status.
    #[must_use]
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Authentication { status, .. } => *status,
            Self::RateLimited { .. } => 429,
            Self::BadRequest { status, .. } => *status,
            Self::ContextWindowExceeded { .. } => 400,
            Self::ContentPolicy { .. } => 400,
            Self::NotFound { .. } => 404,
            Self::ServerError { status, .. } => *status,
            Self::ServiceUnavailable { status, .. } => *status,
            Self::Timeout => 408,
            #[cfg(any(feature = "native-http", feature = "wasm-http"))]
            Self::Network(_) => 0,
            Self::Streaming { .. } => 0,
            Self::EndpointNotSupported { .. } => 400,
            Self::InvalidHeader { .. } => 400,
            Self::Serialization(_) => 0,
            Self::BudgetExceeded { .. } => 0,
            Self::HookRejected { .. } => 0,
            Self::InternalError { .. } => 0,
            Self::OutboundForbidden { .. } => 0,
            Self::IdempotencyConflict { .. } => 409,
            Self::IdempotencyInFlight { .. } => 409,
        }
    }

    /// Returns `true` for errors that are worth retrying on a different service
    /// or deployment (transient failures).
    ///
    /// Used by [`crate::tower::fallback::FallbackService`] and
    /// [`crate::tower::router::Router`] to decide whether to route to an
    /// alternative endpoint.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        match self {
            Self::RateLimited { .. } | Self::ServiceUnavailable { .. } | Self::Timeout | Self::ServerError { .. } => {
                true
            }
            #[cfg(any(feature = "native-http", feature = "wasm-http"))]
            Self::Network(_) => true,
            _ => false,
        }
    }

    /// Return the OpenTelemetry `error.type` string for this error variant.
    ///
    /// Used by the tracing middleware to record the `error.type` span attribute
    /// on failed requests per the GenAI semantic conventions.
    #[must_use]
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::Authentication { .. } => "Authentication",
            Self::RateLimited { .. } => "RateLimited",
            Self::BadRequest { .. } => "BadRequest",
            Self::ContextWindowExceeded { .. } => "ContextWindowExceeded",
            Self::ContentPolicy { .. } => "ContentPolicy",
            Self::NotFound { .. } => "NotFound",
            Self::ServerError { .. } => "ServerError",
            Self::ServiceUnavailable { .. } => "ServiceUnavailable",
            Self::Timeout => "Timeout",
            #[cfg(any(feature = "native-http", feature = "wasm-http"))]
            Self::Network(_) => "Network",
            Self::Streaming { .. } => "Streaming",
            Self::EndpointNotSupported { .. } => "EndpointNotSupported",
            Self::InvalidHeader { .. } => "InvalidHeader",
            Self::Serialization(_) => "Serialization",
            Self::BudgetExceeded { .. } => "BudgetExceeded",
            Self::HookRejected { .. } => "HookRejected",
            Self::InternalError { .. } => "InternalError",
            Self::OutboundForbidden { .. } => "OutboundForbidden",
            Self::IdempotencyConflict { .. } => "IdempotencyConflict",
            Self::IdempotencyInFlight { .. } => "IdempotencyInFlight",
        }
    }

    /// Create a version of this error suitable for broadcasting via singleflight.
    ///
    /// `LiterLlmError` is not `Clone` because some variants hold non-Clone types
    /// (e.g. `reqwest::Error`).  This method produces a semantically equivalent
    /// error that *is* owned and can be placed behind an `Arc` for broadcast.
    /// Variants that cannot be cloned exactly are converted to their nearest
    /// owned equivalent while preserving the error class (variant discriminant).
    #[cfg(feature = "tower")]
    pub(crate) fn to_singleflight_error(&self) -> Self {
        match self {
            Self::Authentication { message, status } => Self::Authentication {
                message: message.clone(),
                status: *status,
            },
            Self::RateLimited { message, retry_after } => Self::RateLimited {
                message: message.clone(),
                retry_after: *retry_after,
            },
            Self::BadRequest { message, status } => Self::BadRequest {
                message: message.clone(),
                status: *status,
            },
            Self::ContextWindowExceeded { message } => Self::ContextWindowExceeded {
                message: message.clone(),
            },
            Self::ContentPolicy { message } => Self::ContentPolicy {
                message: message.clone(),
            },
            Self::NotFound { message } => Self::NotFound {
                message: message.clone(),
            },
            Self::ServerError { message, status } => Self::ServerError {
                message: message.clone(),
                status: *status,
            },
            Self::ServiceUnavailable { message, status } => Self::ServiceUnavailable {
                message: message.clone(),
                status: *status,
            },
            Self::Timeout => Self::Timeout,
            #[cfg(any(feature = "native-http", feature = "wasm-http"))]
            Self::Network(e) => Self::InternalError { message: e.to_string() },
            Self::Streaming { message } => Self::Streaming {
                message: message.clone(),
            },
            Self::EndpointNotSupported { endpoint, provider } => Self::EndpointNotSupported {
                endpoint: endpoint.clone(),
                provider: provider.clone(),
            },
            Self::InvalidHeader { name, reason } => Self::InvalidHeader {
                name: name.clone(),
                reason: reason.clone(),
            },
            Self::Serialization(e) => Self::InternalError { message: e.to_string() },
            Self::BudgetExceeded { message, model } => Self::BudgetExceeded {
                message: message.clone(),
                model: model.clone(),
            },
            Self::HookRejected { message } => Self::HookRejected {
                message: message.clone(),
            },
            Self::InternalError { message } => Self::InternalError {
                message: message.clone(),
            },
            Self::OutboundForbidden { url, reason } => Self::OutboundForbidden {
                url: url.clone(),
                reason: reason.clone(),
            },
            Self::IdempotencyConflict { key } => Self::IdempotencyConflict { key: key.clone() },
            Self::IdempotencyInFlight { key } => Self::IdempotencyInFlight { key: key.clone() },
        }
    }

    /// Create from an HTTP status code, an API error response body, and an
    /// optional `Retry-After` duration already parsed from the response header.
    ///
    /// The `retry_after` value is forwarded into [`LiterLlmError::RateLimited`]
    /// so callers can honour the server-requested delay without re-parsing the
    /// header.
    pub(crate) fn from_status(status: u16, body: &str, retry_after: Option<Duration>) -> Self {
        let parsed = serde_json::from_str::<ErrorResponse>(body).ok();
        let code = parsed.as_ref().and_then(|r| r.error.code.clone());
        let message = parsed.map(|r| r.error.message).unwrap_or_else(|| body.to_string());

        match status {
            401 | 403 => Self::Authentication { message, status },
            429 => Self::RateLimited { message, retry_after },
            400 | 422 => {
                // Check the structured `code` field first — it is more reliable
                // than substring matching on the human-readable message.
                if code.as_deref() == Some("context_length_exceeded") {
                    Self::ContextWindowExceeded { message }
                } else if code.as_deref() == Some("content_policy_violation")
                    || code.as_deref() == Some("content_filter")
                {
                    Self::ContentPolicy { message }
                }
                // Fall back to message-based heuristics for providers that do not
                // populate the `code` field.
                else if message.contains("context_length_exceeded")
                    || message.contains("context window")
                    || message.contains("maximum context length")
                {
                    Self::ContextWindowExceeded { message }
                } else if message.contains("content_policy") || message.contains("content_filter") {
                    Self::ContentPolicy { message }
                } else {
                    Self::BadRequest { message, status }
                }
            }
            404 => Self::NotFound { message },
            405 | 413 => Self::BadRequest { message, status },
            408 => Self::Timeout,
            500 => Self::ServerError { message, status },
            502..=504 => Self::ServiceUnavailable { message, status },
            // Map remaining 4xx codes to BadRequest (client errors) and
            // everything else (5xx, unknown) to ServerError.
            400..=499 => Self::BadRequest { message, status },
            _ => Self::ServerError { message, status },
        }
    }
}

#[cfg_attr(alef, alef(skip))]
pub type Result<T> = std::result::Result<T, LiterLlmError>;
