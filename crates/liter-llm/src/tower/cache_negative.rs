//! Negative-cache Tower middleware.
//!
//! When an upstream LLM provider returns a transient error, repeated retries
//! from all callers amplify the load on an already-stressed service.  The
//! negative-cache layer intercepts those errors and writes a
//! [`CachedResponse::Error`] entry into the cache store.  Subsequent callers
//! for the same request key receive the cached error immediately, without
//! hitting upstream, until the negative-cache window elapses.
//!
//! # Design
//!
//! The [`NegativeCachePolicy`] trait decides *whether* and *for how long* to
//! cache a given error.  The default implementation
//! ([`FixedWindowNegativeCache`]) caches only transient errors
//! (`RateLimited`, `ServiceUnavailable`, `Timeout`) for a fixed duration.
//!
//! Errors are stored as [`CachedResponse::Error`] entries in the *same*
//! `CacheStore` used by [`crate::tower::cache::CacheLayer`].  This avoids
//! maintaining a separate `NegativeStore` trait and lets the existing
//! `CacheService` serve cached errors naturally — it calls `into_llm_response()`
//! which converts the `Error` variant back into `Err(LiterLlmError)`.
//!
//! # Why `CachedResponse::Error` rather than a sibling `NegativeStore`?
//!
//! A sibling `NegativeStore` trait would require:
//! 1. A second `Arc<dyn NegativeStore>` on every service in the stack.
//! 2. Two cache lookups on every request (success store + negative store).
//! 3. Coordination between the two stores on eviction (TTL, capacity).
//!
//! By reusing the existing `CacheStore` with the `Error` variant we get:
//! - Single lookup per request — `CacheService::get` returns either a success
//!   entry, an error entry, or a miss.
//! - Unified capacity and TTL management in `InMemoryStore`.
//! - A single trait surface that external-store implementors need to satisfy.
//!
//! The trade-off: `Error` entries are not serialisable (see
//! [`crate::tower::cache::CachedResponse`] doc comment).  External stores that
//! need to replicate negative-cache state across processes must implement their
//! own serialisation shim in `CacheStore::put`.
//!
//! # Recommended layer order
//!
//! See [`crate::tower::cache`] module documentation for the full recommended
//! composition order.

use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tower::{Layer, Service};

use super::cache::{CacheStore, CachedResponse, InMemoryStore, cache_key};
use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

// ─── NegativeCachePolicy trait ────────────────────────────────────────────────

/// Decides whether and for how long to cache an upstream error.
///
/// Implement this trait to provide custom negative-cache strategies (e.g.
/// model-specific windows, exponential back-off, per-tenant policies).
///
/// The default in-tree implementation is [`FixedWindowNegativeCache`].
#[cfg_attr(alef, alef(skip))]
pub trait NegativeCachePolicy: Send + Sync + 'static {
    /// Inspect `error` and return how long it should be cached.
    ///
    /// - `Some(duration)` — cache the error for `duration`.
    /// - `None` — do not cache this error (let callers retry immediately).
    fn cache_for(&self, error: &LiterLlmError) -> Option<Duration>;
}

// ─── FixedWindowNegativeCache ─────────────────────────────────────────────────

/// Default [`NegativeCachePolicy`]: cache transient errors for a fixed window.
///
/// When `retryable_only` is `true` (the default), only transient errors are
/// cached:
/// - [`LiterLlmError::RateLimited`]
/// - [`LiterLlmError::ServiceUnavailable`]
/// - [`LiterLlmError::Timeout`]
///
/// Non-transient errors (`BadRequest`, `Authentication`, `NotFound`, etc.) are
/// not cached because they indicate a client-side problem that will not resolve
/// by waiting.
///
/// When `retryable_only` is `false`, every error variant is cached for `window`.
///
/// # Example
///
/// ```rust,ignore
/// use liter_llm::tower::FixedWindowNegativeCache;
/// use std::time::Duration;
///
/// // Cache only transient errors for 30 seconds (default behaviour).
/// let policy = FixedWindowNegativeCache::default();
///
/// // Cache all errors for 5 seconds.
/// let policy = FixedWindowNegativeCache::new(Duration::from_secs(5), false);
/// ```
#[cfg_attr(alef, alef(skip))]
pub struct FixedWindowNegativeCache {
    /// How long to cache an eligible error.
    window: Duration,
    /// When `true`, only transient errors are cached (see [`LiterLlmError::is_transient`]).
    retryable_only: bool,
}

impl FixedWindowNegativeCache {
    /// Create a new policy with a custom window and filter.
    #[must_use]
    pub fn new(window: Duration, retryable_only: bool) -> Self {
        Self { window, retryable_only }
    }
}

impl Default for FixedWindowNegativeCache {
    /// Cache only transient errors for 5 seconds.
    fn default() -> Self {
        Self {
            window: Duration::from_secs(5),
            retryable_only: true,
        }
    }
}

impl NegativeCachePolicy for FixedWindowNegativeCache {
    fn cache_for(&self, error: &LiterLlmError) -> Option<Duration> {
        let eligible = if self.retryable_only {
            error.is_transient()
        } else {
            true
        };
        eligible.then_some(self.window)
    }
}

// ─── NegativeCacheLayer ───────────────────────────────────────────────────────

/// Tower [`Layer`] that intercepts upstream errors and caches them.
///
/// This layer wraps any inner service and a [`CacheStore`].  On an upstream
/// error:
/// 1. Consults the [`NegativeCachePolicy`] to decide whether and for how long
///    to cache the error.
/// 2. If the policy returns `Some(duration)`, writes a [`CachedResponse::Error`]
///    entry into the store.
/// 3. Returns the error to the caller (the error is never swallowed).
///
/// Subsequent calls for the same key hit the store directly (via the upstream
/// `CacheLayer`) and receive the cached error.
///
/// # Positioning
///
/// `NegativeCacheLayer` must wrap the inner `CacheLayer` — it is between the
/// singleflight layer and the success-path cache.  See
/// [`crate::tower::cache`] for the full recommended composition order.
#[cfg_attr(alef, alef(skip))]
pub struct NegativeCacheLayer<P: NegativeCachePolicy = FixedWindowNegativeCache> {
    store: Arc<dyn CacheStore>,
    policy: Arc<P>,
}

impl NegativeCacheLayer<FixedWindowNegativeCache> {
    /// Create a new layer using the default [`FixedWindowNegativeCache`] policy
    /// and an in-memory store.
    #[must_use]
    pub fn default_in_memory() -> Self {
        use crate::tower::cache::CacheConfig;
        Self {
            store: Arc::new(InMemoryStore::new(&CacheConfig::default())),
            policy: Arc::new(FixedWindowNegativeCache::default()),
        }
    }
}

impl Default for NegativeCacheLayer<FixedWindowNegativeCache> {
    fn default() -> Self {
        Self::default_in_memory()
    }
}

impl<P: NegativeCachePolicy> NegativeCacheLayer<P> {
    /// Create a new layer with a custom store and policy.
    #[must_use]
    pub fn new(store: Arc<dyn CacheStore>, policy: Arc<P>) -> Self {
        Self { store, policy }
    }
}

impl<P: NegativeCachePolicy, S> Layer<S> for NegativeCacheLayer<P> {
    type Service = NegativeCacheService<P, S>;

    fn layer(&self, inner: S) -> Self::Service {
        NegativeCacheService {
            store: Arc::clone(&self.store),
            policy: Arc::clone(&self.policy),
            inner,
        }
    }
}

// ─── NegativeCacheService ─────────────────────────────────────────────────────

/// Tower service produced by [`NegativeCacheLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct NegativeCacheService<P: NegativeCachePolicy, S> {
    store: Arc<dyn CacheStore>,
    policy: Arc<P>,
    inner: S,
}

impl<P: NegativeCachePolicy, S: Clone> Clone for NegativeCacheService<P, S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            policy: Arc::clone(&self.policy),
            inner: self.inner.clone(),
        }
    }
}

impl<P, S> Service<LlmRequest> for NegativeCacheService<P, S>
where
    P: NegativeCachePolicy,
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        let key_and_body = cache_key(&req);
        let store = Arc::clone(&self.store);
        let policy = Arc::clone(&self.policy);
        let fut = self.inner.call(req);

        Box::pin(async move {
            let result = fut.await;
            if let Err(ref err) = result
                && let Some(window) = policy.cache_for(err)
                && let Some((key, body)) = key_and_body
            {
                let expires_at = Instant::now() + window;
                // We cannot clone `LiterLlmError` (`reqwest::Error` is not `Clone`),
                // so we store the error's `Display` string as an `InternalError`.
                // The caller already holds the real error via `result`; the cached
                // version is served only to *subsequent* callers.
                let cached_err = CachedResponse::Error {
                    error: Arc::new(LiterLlmError::InternalError {
                        message: err.to_string(),
                    }),
                    expires_at,
                };
                store.put(key, body, cached_err).await;
            }
            result
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tower::{Layer as _, Service as _, ServiceExt as _};

    use super::*;
    use crate::tower::cache::{CacheConfig, CacheLayer, InMemoryStore};
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    /// Build a shared `InMemoryStore` and compose:
    /// `NegativeCacheLayer → CacheLayer → upstream`
    ///
    /// Returns `(store, service)`.
    fn build_stack(
        client: MockClient,
        policy: FixedWindowNegativeCache,
    ) -> (Arc<InMemoryStore>, impl Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError>) {
        let store = Arc::new(InMemoryStore::new(&CacheConfig {
            max_entries: 64,
            ttl: Duration::from_secs(60),
            ..Default::default()
        }));
        let cache_layer = CacheLayer::with_store(Arc::clone(&store) as Arc<dyn CacheStore>);
        let neg_layer = NegativeCacheLayer::new(
            Arc::clone(&store) as Arc<dyn CacheStore>,
            Arc::new(policy),
        );
        let inner = LlmService::new(client);
        let svc = neg_layer.layer(cache_layer.layer(inner));
        (store, svc)
    }

    /// A `BadRequest` error must not be written to the store (retryable_only = true).
    #[tokio::test]
    async fn negative_cache_skips_non_transient_errors_by_default() {
        let client = MockClient::failing_auth(); // returns BadRequest
        let policy = FixedWindowNegativeCache::default(); // retryable_only = true
        let (store, mut svc) = build_stack(client, policy);

        let req = LlmRequest::Chat(chat_req("gpt-4"));
        let _ = svc.call(req).await;

        // Nothing should be in the store.
        let hit = store
            .get(0, "") // any key — store was never written
            .await;
        assert!(hit.is_none(), "non-transient error must not be cached");

        // Verify by checking a realistic key.
        let body = serde_json::to_string(&chat_req("gpt-4")).unwrap();
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        body.hash(&mut hasher);
        let key = hasher.finish();
        let hit = store.get(key, &body).await;
        assert!(hit.is_none(), "non-transient error must not be stored");
    }

    /// A `RateLimited` error must be stored and served on the next call.
    #[tokio::test]
    async fn negative_cache_stores_rate_limited_for_window() {
        let client = MockClient::failing_rate_limited();
        let policy = FixedWindowNegativeCache::new(Duration::from_secs(30), true);
        let (store, mut svc) = build_stack(client, policy);

        let req_body = chat_req("gpt-4");

        // First call — cache miss, upstream returns RateLimited.
        let first = svc.ready().await.unwrap().call(LlmRequest::Chat(req_body.clone())).await;
        assert!(first.is_err(), "first call should propagate the upstream error");

        // Verify the error was written to the store.
        let serialized = serde_json::to_string(&req_body).unwrap();
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        serialized.hash(&mut hasher);
        let key = hasher.finish();

        let cached = store.get(key, &serialized).await;
        assert!(cached.is_some(), "RateLimited error must be written to store");
        assert!(
            matches!(cached.unwrap(), CachedResponse::Error { .. }),
            "stored entry must be CachedResponse::Error"
        );

        // Second call — should return from cache (the inner service won't be called
        // again, but CacheLayer returns the cached Error entry).
        let second = svc.ready().await.unwrap().call(LlmRequest::Chat(req_body)).await;
        assert!(second.is_err(), "second call must also return an error (cached)");
    }

    /// After the negative-cache window, the cache misses and inner is called again.
    #[tokio::test]
    async fn negative_cache_returns_to_normal_after_window() {
        // Use a very short window.
        let client = MockClient::failing_rate_limited();
        let call_count = Arc::clone(&client.call_count);
        let policy = FixedWindowNegativeCache::new(Duration::from_millis(50), true);
        let (_, mut svc) = build_stack(client, policy);

        let req_body = chat_req("gpt-4");

        // First call — hits upstream, gets RateLimited, caches it.
        let _ = svc.ready().await.unwrap().call(LlmRequest::Chat(req_body.clone())).await;
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Wait for the window to elapse.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // After window — cache miss, inner is called again.
        let _ = svc.ready().await.unwrap().call(LlmRequest::Chat(req_body)).await;
        assert_eq!(
            call_count.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "after negative-cache window, inner must be called again"
        );
    }
}
