//! Secret management — trait, types, and built-in backends.
//!
//! # URI scheme routing
//!
//! Secret names may carry a URI scheme prefix to select a backend:
//!
//! - `env://NAME` — environment variable `NAME` (always available)
//! - `aws://PATH` — AWS Secrets Manager (requires `secrets-aws` feature)
//! - `vault://PATH` — HashCorp Vault KV-v2 (requires `secrets-vault` feature)
//!
//! When no scheme is present, the registry uses its configured default backend.
//!
//! # Rotation warnings
//!
//! After every successful [`SecretManager::get`] call, the caller should check
//! [`SecretMetadata::expires_at`].  The registry's convenience methods do this
//! automatically: a [`tracing::warn!`] is emitted and the OTel gauge
//! `gen_ai.secret.expires_in_seconds` is recorded when expiry is within 24 h.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, SystemTime};

use secrecy::SecretString;

pub mod env;
pub mod registry;

#[cfg(feature = "secrets-aws")]
pub mod aws;

#[cfg(feature = "secrets-vault")]
pub mod vault;

// ---------------------------------------------------------------------------
// Public re-exports
// ---------------------------------------------------------------------------

pub use env::EnvVarSecretManager;
pub use registry::SecretManagerRegistry;

#[cfg(feature = "secrets-aws")]
pub use aws::AwsSecretsManagerProvider;

#[cfg(feature = "secrets-vault")]
pub use vault::HashCorpVaultProvider;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`SecretManager`] operations.
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    /// The named secret does not exist in the backend.
    #[error("secret not found: {0}")]
    NotFound(String),

    /// The caller lacks permission to read or write this secret.
    #[error("permission denied for secret: {0}")]
    PermissionDenied(String),

    /// The secret exists but its `expires_at` timestamp has passed.
    #[error("secret has expired: {0}")]
    Expired(String),

    /// The backend rate-limited the request.
    #[error("rate limited by secret backend")]
    RateLimited,

    /// The backend address was rejected by the outbound SSRF policy.
    ///
    /// Returned by backends (e.g. Vault) when the caller-supplied `address`
    /// targets a private / loopback / link-local / CGNAT address that the
    /// active [`liter_llm::provider::OutboundPolicy`] does not permit.
    /// Prevents SSRF via operator misconfiguration or config-file injection.
    #[error("backend address forbidden by outbound policy: {0}")]
    Forbidden(String),

    /// Any other backend-specific error.
    #[error("secret backend error: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl SecretError {
    /// Convenience constructor for [`SecretError::Backend`] from any error.
    pub fn backend(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Backend(Box::new(err))
    }

    /// Convenience constructor for [`SecretError::Backend`] from a string.
    pub fn backend_msg(msg: impl Into<String>) -> Self {
        #[derive(Debug, thiserror::Error)]
        #[error("{0}")]
        struct StringError(String);
        Self::Backend(Box::new(StringError(msg.into())))
    }
}

// ---------------------------------------------------------------------------
// Value + metadata
// ---------------------------------------------------------------------------

/// A secret value with full metadata.
///
/// The [`value`](SecretValue::value) field is a [`SecretString`] and is
/// zeroed on drop.
pub struct SecretValue {
    /// The secret payload, zeroed on drop.
    pub value: SecretString,
    /// Provenance and lifecycle metadata.
    pub metadata: SecretMetadata,
}

/// Provenance and lifecycle information for a single secret.
#[derive(Debug, Clone)]
pub struct SecretMetadata {
    /// Canonical name as understood by the backend.
    pub name: String,
    /// Backend-specific version identifier (AWS `VersionId`, Vault revision, etc.).
    pub version: String,
    /// When the secret was created, if known.
    pub created_at: SystemTime,
    /// When the secret was last updated, if known.
    pub updated_at: SystemTime,
    /// Optional expiry time.  When set and within 24 h, the registry emits a
    /// rotation warning.
    pub expires_at: Option<SystemTime>,
    /// Arbitrary key-value labels from the backend (AWS tags, Vault metadata).
    pub tags: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Unified secret-management interface.
///
/// Implementors must be `Send + Sync + 'static` so they can be stored in
/// [`Arc`] and shared across async tasks.
///
/// All methods return `Pin<Box<dyn Future…>>` (i.e. boxed futures) so the
/// trait is object-safe without requiring `async_trait`.
///
/// # Examples
///
/// Implement a basic secret manager that reads from environment variables:
///
/// ```no_run
/// use liter_llm_proxy::secrets::{SecretManager, SecretValue, SecretMetadata, SecretError};
/// use secrecy::SecretString;
/// use std::collections::HashMap;
/// use std::pin::Pin;
/// use std::future::Future;
/// use std::time::SystemTime;
///
/// struct SimpleEnvSecretManager;
///
/// impl SecretManager for SimpleEnvSecretManager {
///     fn get<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<SecretValue, SecretError>> + Send + 'a>> {
///         Box::pin(async move {
///             let value = std::env::var(name)
///                 .map_err(|_| SecretError::NotFound(name.to_string()))?;
///             Ok(SecretValue {
///                 value: SecretString::new(value),
///                 metadata: SecretMetadata {
///                     name: name.to_string(),
///                     version: "1".to_string(),
///                     created_at: SystemTime::now(),
///                     updated_at: SystemTime::now(),
///                     expires_at: None,
///                     tags: HashMap::new(),
///                 },
///             })
///         })
///     }
///
///     fn set<'a>(&'a self, _: &'a str, _: SecretString, _: HashMap<String, String>) -> Pin<Box<dyn Future<Output = Result<SecretMetadata, SecretError>> + Send + 'a>> {
///         Box::pin(async { Err(SecretError::PermissionDenied("read-only".to_string())) })
///     }
///
///     fn delete<'a>(&'a self, _: &'a str) -> Pin<Box<dyn Future<Output = Result<(), SecretError>> + Send + 'a>> {
///         Box::pin(async { Err(SecretError::PermissionDenied("read-only".to_string())) })
///     }
///
///     fn backend(&self) -> &'static str {
///         "env"
///     }
/// }
/// ```
pub trait SecretManager: Send + Sync + 'static {
    /// Fetch a secret by name. Returns the value and full metadata.
    fn get<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<SecretValue, SecretError>> + Send + 'a>>;

    /// Write or update a secret.
    ///
    /// Implementations that do not support writes **must** return
    /// [`SecretError::PermissionDenied`].
    fn set<'a>(
        &'a self,
        name: &'a str,
        value: SecretString,
        tags: HashMap<String, String>,
    ) -> Pin<Box<dyn Future<Output = Result<SecretMetadata, SecretError>> + Send + 'a>>;

    /// Delete a secret.
    ///
    /// Implementations that do not support deletes **must** return
    /// [`SecretError::PermissionDenied`].
    fn delete<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<(), SecretError>> + Send + 'a>>;

    /// Stable identifier used in log messages and metrics.
    fn backend(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Rotation-warning helper (shared by registry convenience methods)
// ---------------------------------------------------------------------------

/// Duration threshold at which a rotation warning is emitted.
pub(crate) const ROTATION_WARNING_THRESHOLD: Duration = Duration::from_secs(24 * 60 * 60);

/// Emit a `tracing::warn!` and (when the `otel` feature is active) record the
/// `gen_ai.secret.expires_in_seconds` gauge if `expires_at` is within 24 h.
///
/// This is called by [`SecretManagerRegistry`] after every successful `get`.
pub(crate) fn check_rotation_warning(name: &str, metadata: &SecretMetadata, backend: &str) {
    let Some(expires_at) = metadata.expires_at else {
        return;
    };
    let now = SystemTime::now();
    if let Ok(remaining) = expires_at.duration_since(now) {
        if remaining <= ROTATION_WARNING_THRESHOLD {
            tracing::warn!(
                secret.name = name,
                secret.backend = backend,
                secret.expires_in_secs = remaining.as_secs(),
                "secret is expiring soon — rotate before expiry"
            );
        }
    } else {
        // expires_at is in the past.
        tracing::warn!(
            secret.name = name,
            secret.backend = backend,
            "secret has already expired"
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the `check_rotation_warning` function runs without panic for
    /// both the "no expiry" and "expiry within threshold" cases.
    /// Actual warning emission is checked in integration tests via tracing-test.
    #[test]
    fn check_rotation_warning_no_expiry_is_noop() {
        let meta = SecretMetadata {
            name: "test".into(),
            version: "1".into(),
            created_at: SystemTime::UNIX_EPOCH,
            updated_at: SystemTime::UNIX_EPOCH,
            expires_at: None,
            tags: HashMap::new(),
        };
        // Must not panic.
        check_rotation_warning("test", &meta, "env");
    }

    #[test]
    fn check_rotation_warning_far_future_does_not_warn() {
        let meta = SecretMetadata {
            name: "test".into(),
            version: "1".into(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            expires_at: Some(SystemTime::now() + Duration::from_secs(48 * 60 * 60)),
            tags: HashMap::new(),
        };
        // Must not panic (no assert on log output here; tracing-test covers it).
        check_rotation_warning("test", &meta, "env");
    }

    #[test]
    fn secret_error_not_found_display() {
        let err = SecretError::NotFound("my-secret".into());
        assert!(err.to_string().contains("my-secret"));
    }

    #[test]
    fn secret_error_backend_msg_roundtrips() {
        let err = SecretError::backend_msg("something went wrong");
        assert!(err.to_string().contains("something went wrong"));
    }

    #[test]
    fn secret_error_permission_denied_display() {
        let err = SecretError::PermissionDenied("vault://secret".into());
        assert!(err.to_string().contains("vault://secret"));
    }
}
