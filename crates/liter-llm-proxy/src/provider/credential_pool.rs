//! `CredentialPool` trait and supporting types.
//!
//! A `CredentialPool` manages one or more API credentials for a named
//! provider. On a 429 or 5xx rate-limit signal the proxy calls
//! [`CredentialPool::mark_exhausted`] to park the current credential for a
//! cooldown period and advance to the next available one.
//!
//! # Design
//!
//! - Trait-first: the default [`super::credential_pool_memory::InMemoryCredentialPool`]
//!   implementation is interchangeable with any custom backend.
//! - Credentials are identified by an opaque `id` string; the raw API key is
//!   stored behind [`secrecy::SecretString`] and never exposed in logs.
//! - The `model_allowlist` field gates which models a credential may serve.

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, SystemTime};

use secrecy::SecretString;

// ─── CredentialHandle ─────────────────────────────────────────────────────────

/// A credential obtained from a [`CredentialPool`].
///
/// Handles are lightweight clones: `id` is a `String` copy; `api_key` is a
/// [`secrecy::SecretString`] (heap-allocated, zeroised on drop).
#[derive(Clone)]
pub struct CredentialHandle {
    /// Opaque pool-internal identifier (e.g. an index or UUID).
    pub id: String,
    /// The raw API key — never expose in logs or error messages.
    pub api_key: SecretString,
    /// When set, this credential may only be used for the listed models.
    /// `None` means the credential is valid for all models.
    pub model_allowlist: Option<Vec<String>>,
}

impl std::fmt::Debug for CredentialHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialHandle")
            .field("id", &self.id)
            .field("api_key", &"[REDACTED]")
            .field("model_allowlist", &self.model_allowlist)
            .finish()
    }
}

// ─── CredentialError ──────────────────────────────────────────────────────────

/// Errors returned by [`CredentialPool`] operations.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// The pool has no credentials configured.
    #[error("credential pool is empty for this provider")]
    PoolEmpty,
    /// All credentials are currently exhausted (parked for cooldown).
    #[error("all credentials exhausted for this provider")]
    AllExhausted,
    /// A backend or I/O error prevented credential resolution.
    #[error("credential backend error: {0}")]
    BackendError(#[from] Box<dyn std::error::Error + Send + Sync>),
}

// ─── PoolSnapshot ─────────────────────────────────────────────────────────────

/// Observability snapshot of a pool's state at a point in time.
#[derive(Debug, Clone)]
pub struct PoolSnapshot {
    /// Total number of credentials registered in the pool.
    pub total: usize,
    /// Number of credentials currently in the `Active` state.
    pub active: usize,
    /// Number of credentials currently parked (`Exhausted`).
    pub exhausted: usize,
    /// Earliest instant at which any exhausted credential will recover.
    ///
    /// `None` when there are no exhausted credentials or the pool is empty.
    pub next_recovery: Option<SystemTime>,
}

// ─── CredentialPool trait ─────────────────────────────────────────────────────

/// A pool of API credentials for a single provider with automatic rotation.
///
/// # Thread safety
///
/// Implementations must be `Send + Sync + 'static` so they can be stored in
/// shared [`std::sync::Arc`]s and called from multiple Tokio tasks concurrently.
///
/// # Lifetime of returned futures
///
/// Methods return `Pin<Box<dyn Future + Send + 'a>>` — the futures borrow
/// `&'a self` and are `Send`, satisfying Tokio's multi-thread executor.
///
/// # Examples
///
/// Use the default in-memory implementation to rotate through credentials:
///
/// ```no_run
/// use liter_llm_proxy::provider::credential_pool_memory::InMemoryCredentialPool;
/// use liter_llm_proxy::provider::credential_pool::CredentialPool;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let pool = InMemoryCredentialPool::new();
///
///     // Seed with two credentials
///     pool.add_credential("openai", "key-1", "sk-first", None);
///     pool.add_credential("openai", "key-2", "sk-second", None);
///
///     // Get current credential
///     let cred = pool.current("openai").await.unwrap();
///
///     // Mark it exhausted; pool advances to next credential
///     pool.mark_exhausted("openai", &cred, Duration::from_secs(60)).await;
/// }
/// ```
pub trait CredentialPool: Send + Sync + 'static {
    /// Return the current active credential for the pool.
    ///
    /// Returns the first `Active` credential according to the pool's internal
    /// ordering (round-robin for [`super::credential_pool_memory::InMemoryCredentialPool`]).
    ///
    /// # Errors
    ///
    /// - [`CredentialError::PoolEmpty`] — no credentials are registered.
    /// - [`CredentialError::AllExhausted`] — every credential is parked.
    fn current<'a>(
        &'a self,
        provider: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CredentialHandle, CredentialError>> + Send + 'a>>;

    /// Mark a credential as exhausted and park it for `cooldown`.
    ///
    /// The pool advances to the next available credential automatically.
    /// After `cooldown` elapses the credential is reactivated by an internal
    /// background task.
    ///
    /// Calling this with a handle that is already exhausted is a no-op.
    fn mark_exhausted<'a>(
        &'a self,
        provider: &'a str,
        handle: &'a CredentialHandle,
        cooldown: Duration,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

    /// Return a point-in-time snapshot of the pool for observability.
    ///
    /// This is a synchronous call — it must not block or perform I/O.
    fn snapshot(&self, provider: &str) -> PoolSnapshot;
}
