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

use super::cache::{CacheStore, CachedResponse, InMemoryStore, strategy_key};
use super::types::{LlmRequest, LlmResponse};
use crate::cache_key::{CacheKeyStrategy, ExactHashStrategy};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};

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
    // ~keep Must match the CacheKeyStrategy CacheLayer uses, or entries land in a key space CacheLayer never reads.
    key_strategy: Arc<dyn CacheKeyStrategy>,
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
            key_strategy: Arc::new(ExactHashStrategy),
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
    ///
    /// Key derivation defaults to [`ExactHashStrategy`] — the same default
    /// [`crate::tower::cache::CacheLayer::new`] uses. If the paired
    /// `CacheLayer` is customized via `with_key_strategy`, call
    /// [`Self::with_key_strategy`] with the *same* strategy instance so both
    /// layers agree on key derivation (see the [module-level
    /// docs][crate::tower::cache] "must share a key strategy" section).
    #[must_use]
    pub fn new(store: Arc<dyn CacheStore>, policy: Arc<P>) -> Self {
        Self {
            store,
            policy,
            key_strategy: Arc::new(ExactHashStrategy),
        }
    }

    /// Set a custom [`CacheKeyStrategy`].
    ///
    /// Must be the same strategy (or an equivalent one) passed to the paired
    /// [`crate::tower::cache::CacheLayer::with_key_strategy`] — see
    /// [`crate::tower::cache::CacheLayer::key_strategy`] for the recommended
    /// way to share a single instance between both layers.
    #[must_use]
    pub fn with_key_strategy(mut self, strategy: Arc<dyn CacheKeyStrategy>) -> Self {
        self.key_strategy = strategy;
        self
    }
}

impl<P: NegativeCachePolicy, S> Layer<S> for NegativeCacheLayer<P> {
    type Service = NegativeCacheService<P, S>;

    fn layer(&self, inner: S) -> Self::Service {
        NegativeCacheService {
            store: Arc::clone(&self.store),
            policy: Arc::clone(&self.policy),
            key_strategy: Arc::clone(&self.key_strategy),
            inner,
        }
    }
}

/// Tower service produced by [`NegativeCacheLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct NegativeCacheService<P: NegativeCachePolicy, S> {
    store: Arc<dyn CacheStore>,
    policy: Arc<P>,
    key_strategy: Arc<dyn CacheKeyStrategy>,
    inner: S,
}

impl<P: NegativeCachePolicy, S: Clone> Clone for NegativeCacheService<P, S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            policy: Arc::clone(&self.policy),
            key_strategy: Arc::clone(&self.key_strategy),
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
        // ~keep Must derive the identical key CacheService reads with, or this entry is never seen (see cache.rs module docs).
        let key_and_body = strategy_key(self.key_strategy.as_ref(), &req);
        let store = Arc::clone(&self.store);
        let policy = Arc::clone(&self.policy);
        let fut = self.inner.call(req);

        Box::pin(async move {
            let result = fut.await;
            if let Err(ref err) = result
                && let Some(window) = policy.cache_for(err)
                && let Some((key, body, _tenant_id)) = key_and_body
            {
                // ~keep Peek before writing: `inner` is `CacheService`, which shares this
                // ~keep same store, so a replayed cached error re-enters this branch on every
                // ~keep poll (it is still `Err` and still eligible per `policy.cache_for`). If
                // ~keep we unconditionally wrote a new `expires_at` here, a client polling
                // ~keep faster than `window` would keep the entry alive forever, even long
                // ~keep after upstream recovered. Only write when there is no still-live
                // ~keep negative-cache entry for this key, so a replay never extends its own
                // ~keep window — the window is set once, by the call that actually observed
                // ~keep the fresh upstream failure.
                let already_cached = matches!(store.get(key, &body).await, Some(CachedResponse::Error { .. }));
                if !already_cached {
                    let expires_at = Instant::now() + window;
                    // ~keep Preserve the error variant (and fields like `retry_after`) via the
                    // ~keep same owned-conversion `cache_singleflight` uses for the identical
                    // ~keep "LiterLlmError is not Clone" problem, instead of collapsing every
                    // ~keep cached error to `InternalError` and silently downgrading retryability.
                    let cached_err = CachedResponse::Error {
                        error: Arc::new(err.to_singleflight_error()),
                        expires_at,
                    };
                    store.put(key, body, cached_err).await;
                }
            }
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tower::ServiceExt as _;

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
    ) -> (
        Arc<InMemoryStore>,
        impl Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError>,
    ) {
        let store = Arc::new(InMemoryStore::new(&CacheConfig {
            max_entries: 64,
            ttl: Duration::from_secs(60),
            ..Default::default()
        }));
        let cache_layer = CacheLayer::with_store(Arc::clone(&store) as Arc<dyn CacheStore>);
        let neg_layer = NegativeCacheLayer::new(Arc::clone(&store) as Arc<dyn CacheStore>, Arc::new(policy));
        let inner = LlmService::new(client);
        let svc = neg_layer.layer(cache_layer.layer(inner));
        (store, svc)
    }

    /// A `BadRequest` error must not be written to the store (retryable_only = true).
    #[tokio::test]
    async fn negative_cache_skips_non_transient_errors_by_default() {
        let client = MockClient::failing_auth();
        let policy = FixedWindowNegativeCache::default();
        let (store, mut svc) = build_stack(client, policy);

        let req = LlmRequest::Chat(chat_req("gpt-4"));
        let _ = svc.call(req).await;

        let hit = store.get(0, "").await;
        assert!(hit.is_none(), "non-transient error must not be cached");

        // ~keep Must match the key CacheLayer/NegativeCacheLayer's default ExactHashStrategy actually writes.
        let (key, body, _tenant_id) =
            strategy_key(&ExactHashStrategy, &LlmRequest::Chat(chat_req("gpt-4"))).expect("chat request is cacheable");
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

        let first = svc
            .ready()
            .await
            .unwrap()
            .call(LlmRequest::Chat(req_body.clone()))
            .await;
        assert!(first.is_err(), "first call should propagate the upstream error");

        // ~keep Must match the key CacheLayer/NegativeCacheLayer's default ExactHashStrategy actually writes.
        let (key, serialized, _tenant_id) =
            strategy_key(&ExactHashStrategy, &LlmRequest::Chat(req_body.clone())).expect("chat request is cacheable");

        let cached = store.get(key, &serialized).await;
        assert!(cached.is_some(), "RateLimited error must be written to store");
        assert!(
            matches!(cached.unwrap(), CachedResponse::Error { .. }),
            "stored entry must be CachedResponse::Error"
        );

        let second = svc.ready().await.unwrap().call(LlmRequest::Chat(req_body)).await;
        assert!(second.is_err(), "second call must also return an error (cached)");
    }

    /// After the negative-cache window, the cache misses and inner is called again.
    #[tokio::test]
    async fn negative_cache_returns_to_normal_after_window() {
        let client = MockClient::failing_rate_limited();
        let call_count = Arc::clone(&client.call_count);
        let policy = FixedWindowNegativeCache::new(Duration::from_millis(50), true);
        let (_, mut svc) = build_stack(client, policy);

        let req_body = chat_req("gpt-4");

        let _ = svc
            .ready()
            .await
            .unwrap()
            .call(LlmRequest::Chat(req_body.clone()))
            .await;
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        tokio::time::sleep(Duration::from_millis(100)).await;

        let _ = svc.ready().await.unwrap().call(LlmRequest::Chat(req_body)).await;
        assert_eq!(
            call_count.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "after negative-cache window, inner must be called again"
        );
    }

    /// The cached error must preserve its original variant (and fields such as
    /// `retry_after`), not collapse to `InternalError`.
    ///
    /// Before the fix, the write path always wrapped the upstream error in
    /// `LiterLlmError::InternalError { message: err.to_string() }`, discarding
    /// the discriminant. A cached 429 replayed as a 500 with no `retry_after`,
    /// and `LiterLlmError::is_transient()` then returned `false` for the
    /// replayed error — silently disabling retries for a transient failure.
    #[tokio::test]
    async fn negative_cache_replay_preserves_the_error_variant_and_retry_after() {
        let client = MockClient::failing_rate_limited();
        let policy = FixedWindowNegativeCache::new(Duration::from_secs(30), true);
        let (_, mut svc) = build_stack(client, policy);

        let req_body = chat_req("gpt-4");

        let first = svc
            .ready()
            .await
            .unwrap()
            .call(LlmRequest::Chat(req_body.clone()))
            .await;
        assert!(first.is_err(), "first call propagates the upstream error");

        let replayed = svc.ready().await.unwrap().call(LlmRequest::Chat(req_body)).await;
        let err = replayed.expect_err("replay must still be an error");
        assert!(
            matches!(err, LiterLlmError::RateLimited { .. }),
            "replayed error must preserve the RateLimited variant, got: {err:?}"
        );
        assert!(
            err.is_transient(),
            "a replayed RateLimited error must still report as transient so callers keep retrying"
        );
    }

    /// Defect-2 regression ("self-refreshing negative cache"): replaying a
    /// cached error through `NegativeCacheService` must not push its
    /// `expires_at` forward.
    ///
    /// `NegativeCacheService` wraps `CacheService` and shares its store, so a
    /// still-cached error re-emerges as `Err` and re-enters the write branch
    /// on every replay (once the fix above preserves the transient variant,
    /// `policy.cache_for` is `Some` again on the replay, exactly as it was on
    /// the original failure). Before this fix, that branch unconditionally
    /// recomputed `expires_at = Instant::now() + window` and re-wrote it, so a
    /// client polling faster than `window` kept the entry alive forever and
    /// upstream was never contacted again even after it recovered. The fix
    /// only writes when no live entry already exists for the key, so the
    /// window is set exactly once — by the call that observed the genuine
    /// upstream failure — and expires on schedule regardless of how many
    /// replays happen in between.
    #[tokio::test]
    async fn negative_cache_replay_does_not_refresh_the_window() {
        let client = MockClient::failing_rate_limited();
        let call_count = Arc::clone(&client.call_count);
        // ~keep The window is far longer than any plausible test runtime, so no replay can
        // ~keep fall outside it however busy the machine is. The test reads the stored
        // ~keep `expires_at` instead of waiting for the window to end; expiry itself is
        // ~keep covered by `negative_cache_returns_to_normal_after_window`.
        let policy = FixedWindowNegativeCache::new(Duration::from_secs(3600), true);
        let (store, mut svc) = build_stack(client, policy);

        let req_body = chat_req("gpt-4");

        let first = svc
            .ready()
            .await
            .unwrap()
            .call(LlmRequest::Chat(req_body.clone()))
            .await;
        assert!(first.is_err());
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        let (key, serialized, _tenant_id) =
            strategy_key(&ExactHashStrategy, &LlmRequest::Chat(req_body.clone())).expect("chat request is cacheable");
        let expires_at_of = |entry: Option<CachedResponse>| match entry {
            Some(CachedResponse::Error { expires_at, .. }) => expires_at,
            other => panic!("the failure must be stored as a negative-cache entry, got: {other:?}"),
        };
        let original_expires_at = expires_at_of(store.get(key, &serialized).await);

        for replay in 1..=2 {
            // ~keep A refreshing replay computes `Instant::now() + window`, which differs from
            // ~keep the stored deadline only if the clock advanced since the first failure;
            // ~keep this sleep guarantees that, and a longer one on a busy machine only helps.
            tokio::time::sleep(Duration::from_millis(5)).await;
            let replayed = svc
                .ready()
                .await
                .unwrap()
                .call(LlmRequest::Chat(req_body.clone()))
                .await;
            assert!(
                replayed.is_err(),
                "replay {replay} within the window must still be an error"
            );
            assert_eq!(
                call_count.load(std::sync::atomic::Ordering::SeqCst),
                1,
                "replay {replay} must be served from the negative cache, not upstream"
            );
            assert_eq!(
                expires_at_of(store.get(key, &serialized).await),
                original_expires_at,
                "replay {replay} must not push the original deadline forward; if replays refreshed \
                 it, a client polling faster than the window would keep the entry alive forever"
            );
        }
    }

    /// Full round-trip regression for the "dead negative cache" bug: an error
    /// written by `NegativeCacheService` must be visible to `CacheService`'s
    /// read path on the very next call, short-circuiting upstream.
    ///
    /// Before the fix, `NegativeCacheService` wrote using the legacy
    /// `cache::cache_key` (`DefaultHasher` over the full serialized request),
    /// while `CacheService` read using `strategy_key`/`ExactHashStrategy`
    /// (a seeded `ahash` over a curated `model|messages|params|tenant|system`
    /// string). The two hash spaces never agreed, so every write from
    /// `NegativeCacheService` was invisible to `CacheService` and every
    /// repeat call re-hit the (still-failing) upstream — the negative cache
    /// layer was dead code that always incurred the two lookups without ever
    /// paying off.
    #[tokio::test]
    async fn negative_cache_round_trip_short_circuits_upstream_before_window_elapses() {
        let client = MockClient::failing_rate_limited();
        let call_count = Arc::clone(&client.call_count);
        let policy = FixedWindowNegativeCache::new(Duration::from_secs(30), true);
        let (_, mut svc) = build_stack(client, policy);

        let req_body = chat_req("gpt-4");

        let first = svc
            .ready()
            .await
            .unwrap()
            .call(LlmRequest::Chat(req_body.clone()))
            .await;
        assert!(first.is_err(), "first call propagates the upstream error");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        let second = svc.ready().await.unwrap().call(LlmRequest::Chat(req_body)).await;
        assert!(
            second.is_err(),
            "second call must also return an error (served from negative cache)"
        );
        assert_eq!(
            call_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "second call within the negative-cache window must be served from the cache, \
             not re-dispatched to upstream — this fails if the write-path and read-path keys disagree"
        );
    }
}
