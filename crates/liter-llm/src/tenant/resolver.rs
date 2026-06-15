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
/// All methods return `Pin<Box<dyn Future… + 'static>>` so the returned future
/// can be stored or spawned onto a Tokio executor without borrowing `self`.
/// The trait itself requires `'static` ownership; implement via `Arc<Self>` if
/// you need shared ownership across call sites.
pub trait KeyResolver: Send + Sync + 'static {
    /// Resolve `api_key` to its associated [`ResolvedKey`].
    ///
    /// Takes `api_key` by value so the returned future is `'static` and can be
    /// spawned onto a Tokio task or stored without borrowing the resolver.
    ///
    /// Returns [`KeyResolverError::NotFound`] when no record matches,
    /// [`KeyResolverError::Inactive`] when the record exists but
    /// `active == false`.
    fn resolve(
        &self,
        api_key: String,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'static>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tenant::InMemoryKeyResolver;

    /// Compile-time assertion: the future returned by `InMemoryKeyResolver::resolve`
    /// is `Send + 'static` and can be stored or spawned without borrowing the resolver.
    #[allow(dead_code)]
    fn _assert_future_is_static_send<T: Future + Send + 'static>(_: T) {}

    #[allow(dead_code)]
    fn _check_resolve_future_bounds(resolver: &InMemoryKeyResolver) {
        _assert_future_is_static_send(resolver.resolve("sk-test".to_owned()));
    }
}
