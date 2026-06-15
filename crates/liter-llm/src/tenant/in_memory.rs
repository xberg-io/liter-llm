use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use dashmap::DashMap;

use super::resolver::{KeyResolver, KeyResolverError, ResolvedKey};

/// Thread-safe in-memory [`KeyResolver`] backed by a [`DashMap`].
///
/// Suitable for tests and single-process deployments where the full key set
/// fits in memory.  For database- or cache-backed production deployments,
/// implement [`KeyResolver`] directly.
///
/// The map is stored behind an [`Arc`] so that the futures returned by
/// [`KeyResolver::resolve`] are `'static` (they capture the `Arc` by clone,
/// not a borrow of `self`).
pub struct InMemoryKeyResolver {
    keys: Arc<DashMap<String, ResolvedKey>>,
}

impl InMemoryKeyResolver {
    /// Create an empty resolver.
    #[must_use]
    pub fn new() -> Self {
        Self { keys: Arc::new(DashMap::new()) }
    }

    /// Create a resolver pre-populated with the given entries.
    #[must_use]
    pub fn with_entries(entries: impl IntoIterator<Item = (String, ResolvedKey)>) -> Self {
        let keys = DashMap::new();
        for (k, v) in entries {
            keys.insert(k, v);
        }
        Self { keys: Arc::new(keys) }
    }

    /// Insert or replace a key record.
    pub fn insert(&self, api_key: impl Into<String>, resolved: ResolvedKey) {
        self.keys.insert(api_key.into(), resolved);
    }

    /// Remove a key record.  Returns the removed record if it existed.
    pub fn remove(&self, api_key: &str) -> Option<ResolvedKey> {
        self.keys.remove(api_key).map(|(_, v)| v)
    }
}

impl Default for InMemoryKeyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyResolver for InMemoryKeyResolver {
    fn resolve(
        &self,
        api_key: String,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'static>> {
        // Clone the Arc-backed DashMap so the future owns it and is 'static.
        let keys = self.keys.clone();
        Box::pin(async move {
            match keys.get(&api_key) {
                None => Err(KeyResolverError::NotFound),
                Some(entry) => {
                    if !entry.active {
                        Err(KeyResolverError::Inactive)
                    } else {
                        Ok(entry.clone())
                    }
                }
            }
        })
    }
}
