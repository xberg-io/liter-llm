use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use super::context::TenantId;

/// A resolved virtual-key record returned by [`KeyResolver::resolve`].
#[derive(Clone, Debug)]
pub struct ResolvedKey {
    /// Tenant this key belongs to.
    pub tenant_id: TenantId,
    /// Model names this key may access.  Empty means unrestricted.
    pub allowed_models: Vec<String>,
    /// Optional per-period spending cap.
    pub monthly_budget: Option<rust_decimal::Decimal>,
    /// ISO-4217 currency code for `monthly_budget`, e.g. `"EUR"`.
    pub currency: Option<String>,
    /// Arbitrary key-value metadata (e.g. `"tier"`, `"label"`).
    pub metadata: HashMap<String, String>,
    /// Whether the key is currently active.
    pub active: bool,
}

/// Errors returned by [`KeyResolver::resolve`].
#[derive(Debug, thiserror::Error)]
pub enum KeyResolverError {
    /// No key matching the supplied token exists.
    #[error("api key not found")]
    NotFound,
    /// The key exists but has been deactivated.
    #[error("api key is inactive")]
    Inactive,
    /// A backend-specific error prevented resolution.
    #[error("key resolver backend error: {0}")]
    Backend(String),
}

/// Resolves a raw API token to a [`ResolvedKey`].
///
/// Implement this trait to plug in a database-backed, remote, or in-process
/// key store.  The built-in implementation is [`super::InMemoryKeyResolver`].
///
/// All methods return `Pin<Box<dyn Future…>>` so the trait is object-safe and
/// can be stored behind `Arc<dyn KeyResolver>`.
pub trait KeyResolver: Send + Sync + 'static {
    /// Resolve `api_key` to its associated [`ResolvedKey`].
    ///
    /// Returns [`KeyResolverError::NotFound`] when no record matches,
    /// [`KeyResolverError::Inactive`] when the record exists but
    /// `active == false`.
    fn resolve<'a>(
        &'a self,
        api_key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'a>>;
}
