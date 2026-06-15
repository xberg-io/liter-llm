//! Response caching middleware.
//!
//! [`CacheLayer`] wraps any [`Service<LlmRequest>`] and caches non-streaming
//! responses keyed by a hash of the serialised request.  Only
//! [`LlmResponse::Chat`] and [`LlmResponse::Embed`] responses are cached;
//! streaming, model-list, and other response variants are passed through
//! uncached.
//!
//! The default backend is an in-memory LRU ([`InMemoryStore`]) with a
//! configurable maximum entry count and TTL.  Implement the [`CacheStore`]
//! trait to plug in Redis, DynamoDB, or any other storage backend.
//!
//! # Recommended layer order
//!
//! When composing the resilience layers, stack them in the following order
//! (outermost to innermost):
//!
//! ```text
//! Singleflight → NegativeCache → Cache → Upstream
//! ```
//!
//! - **`SingleflightLayer`** (outermost): collapses concurrent identical
//!   requests into one upstream call before any cache interaction.
//! - **`NegativeCacheLayer`**: intercepts upstream errors and writes them into
//!   the cache store as [`CachedResponse::Error`] entries so subsequent callers
//!   receive the cached error without hitting upstream again.
//! - **`CacheLayer`**: handles success-path caching.  It sees the result after
//!   `NegativeCacheLayer` has already decided whether to store the error.
//! - **Upstream service**: the actual LLM provider.
//!
//! Using `ServiceBuilder`:
//!
//! ```rust,ignore
//! use tower::ServiceBuilder;
//! use liter_llm::tower::{
//!     CacheConfig, CacheLayer,
//!     NegativeCacheLayer, FixedWindowNegativeCache,
//!     SingleflightLayer, InMemorySingleflight,
//! };
//! use std::sync::Arc;
//!
//! let svc = ServiceBuilder::new()
//!     .layer(SingleflightLayer::new(Arc::new(InMemorySingleflight::default())))
//!     .layer(NegativeCacheLayer::default())
//!     .layer(CacheLayer::new(CacheConfig::default()))
//!     .service(upstream);
//! ```

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tower::{Layer, Service};

use super::types::{LlmRequest, LlmResponse};
use crate::client::BoxFuture;
use crate::error::{LiterLlmError, Result};
use crate::types::{ChatCompletionResponse, EmbeddingResponse};

// ---- Config ----------------------------------------------------------------

/// Storage backend for the response cache.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CacheBackend {
    /// In-memory LRU cache (default). No external dependencies.
    #[default]
    Memory,
    /// OpenDAL-backed storage. Supports 40+ backends (S3, Redis, GCS, local FS, etc.).
    #[cfg(feature = "opendal-cache")]
    OpenDal {
        /// OpenDAL scheme name (e.g. "s3", "redis", "fs", "gcs", "azblob").
        scheme: String,
        /// Backend-specific configuration as key-value pairs passed to OpenDAL.
        config: std::collections::HashMap<String, String>,
    },
}

/// Configuration for the response cache.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheConfig {
    /// Maximum number of cached entries.
    pub max_entries: usize,
    /// Time-to-live for each cached entry.
    pub ttl: Duration,
    /// Storage backend to use.
    pub backend: CacheBackend,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 256,
            ttl: Duration::from_secs(300),
            backend: CacheBackend::Memory,
        }
    }
}

// ---- Cached response -------------------------------------------------------

/// The subset of [`LlmResponse`] variants that can be cached.
///
/// Streaming responses are not cacheable because they are consumed once.
///
/// # `Error` variant
///
/// The [`CachedResponse::Error`] variant stores a transient upstream error
/// together with an expiry instant.  This allows
/// [`crate::tower::cache_negative::NegativeCacheLayer`] to short-circuit
/// repeated calls for the same request key without hitting upstream again while
/// the negative-cache window is open.  The variant is written by
/// `NegativeCacheLayer` and read by `CacheService`; `CacheService` itself never
/// writes it — separation of concerns is maintained by keeping the write path in
/// the negative-cache layer.
///
/// ### Why `Arc<LiterLlmError>` rather than a serialisable form?
///
/// `LiterLlmError` contains a `reqwest::Error` variant gated on `native-http`.
/// That variant is not `Serialize`, so the enum cannot derive `Serialize`
/// unconditionally.  Wrapping in `Arc` lets the in-memory store pass the value
/// around cheaply without serialisation.  External stores (Redis, DynamoDB)
/// that require serialisation should handle the `Error` variant explicitly in
/// their `CacheStore` implementation, converting to/from a serialisable
/// representation of the error.
///
/// ### Serialisation contract
///
/// Custom `Serialize`/`Deserialize` impls are provided.  Only the `Chat` and
/// `Embed` variants are serialisable.  Attempting to serialise an `Error`
/// variant returns an error; this guards against accidentally writing negative-
/// cache entries to external stores without an explicit conversion shim.
///
/// # Performance note
///
/// `CachedResponse` is `Clone`d on every cache hit (to return a value while
/// keeping the cache entry) and when storing (the response inner is cloned to
/// build a `CachedResponse` while the original `LlmResponse` is returned to
/// the caller).  For typical chat/embedding payloads this is inexpensive, but
/// callers caching very large responses should be aware of the allocation
/// cost.  An `Arc<CachedResponse>` wrapper was considered but rejected
/// because it would complicate the [`CacheStore`] trait's serialisation
/// contract (`Serialize`/`Deserialize` on `Arc` requires special handling)
/// and would not benefit external store implementations (Redis, DynamoDB)
/// that must serialise on every read anyway.
#[derive(Clone)]
#[cfg_attr(alef, alef(skip))]
pub enum CachedResponse {
    /// A cached chat completion response.
    Chat(ChatCompletionResponse),
    /// A cached embedding response.
    Embed(EmbeddingResponse),
    /// A cached upstream error, stored by
    /// [`crate::tower::cache_negative::NegativeCacheLayer`].
    ///
    /// The `expires_at` field records the instant at which this negative-cache
    /// entry should be evicted.  Readers that encounter an expired `Error`
    /// entry must treat it as a cache miss.
    Error {
        /// The upstream error, shared cheaply via `Arc`.
        error: Arc<LiterLlmError>,
        /// The wall-clock instant after which this entry must not be served.
        expires_at: Instant,
    },
}

// Manual Serialize/Deserialize for CachedResponse.
//
// The `Error` variant holds `Arc<LiterLlmError>` which is not `Serialize`
// (the `reqwest::Error` inside `LiterLlmError` does not implement `Serialize`).
// We serialise only `Chat` and `Embed` via a repr enum; `Error` entries are
// in-memory only.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CachedResponseRepr {
    Chat(ChatCompletionResponse),
    Embed(EmbeddingResponse),
}

impl Serialize for CachedResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Chat(r) => CachedResponseRepr::Chat(r.clone()).serialize(serializer),
            Self::Embed(r) => CachedResponseRepr::Embed(r.clone()).serialize(serializer),
            Self::Error { .. } => Err(serde::ser::Error::custom(
                "CachedResponse::Error is not serialisable; convert to a serialisable form before writing to an external store",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for CachedResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        match CachedResponseRepr::deserialize(deserializer)? {
            CachedResponseRepr::Chat(r) => Ok(Self::Chat(r)),
            CachedResponseRepr::Embed(r) => Ok(Self::Embed(r)),
        }
    }
}

impl CachedResponse {
    /// Convert this cached response back into the full [`LlmResponse`] enum.
    ///
    /// Returns `Err` when this entry is a [`CachedResponse::Error`] variant.
    /// Callers that only expect success responses should call this method and
    /// propagate the `Err`.
    ///
    /// The in-memory `NegativeCacheLayer` stores errors via `Arc<LiterLlmError>`;
    /// when converting back, `Arc::try_unwrap` is attempted first to avoid an
    /// extra allocation.  If other holders exist (unlikely in normal operation),
    /// the error's `Display` string is re-wrapped in `InternalError`.
    pub fn into_llm_response(self) -> Result<LlmResponse> {
        match self {
            Self::Chat(r) => Ok(LlmResponse::Chat(r)),
            Self::Embed(r) => Ok(LlmResponse::Embed(r)),
            Self::Error { error, .. } => Err(Arc::try_unwrap(error).unwrap_or_else(|arc| {
                LiterLlmError::InternalError {
                    message: arc.to_string(),
                }
            })),
        }
    }

    /// Returns `true` if this entry is an `Error` variant that has passed its expiry.
    #[must_use]
    pub fn is_expired_error(&self) -> bool {
        matches!(self, Self::Error { expires_at, .. } if Instant::now() >= *expires_at)
    }
}

// ---- CacheStore trait ------------------------------------------------------

/// Pluggable cache backend.
///
/// Implement this trait to provide a custom storage layer (Redis, DynamoDB,
/// disk, etc.).  The default in-memory implementation is [`InMemoryStore`].
///
/// All methods return pinned, boxed futures so the trait is object-safe and
/// can be used behind `Arc<dyn CacheStore>`.
#[cfg_attr(alef, alef(skip))]
pub trait CacheStore: Send + Sync + 'static {
    /// Look up a cached response by its hash key.
    ///
    /// `request_body` is the serialized request used to guard against 64-bit
    /// hash collisions — implementations should compare it against the stored
    /// body before returning a hit.
    fn get(&self, key: u64, request_body: &str) -> Pin<Box<dyn Future<Output = Option<CachedResponse>> + Send + '_>>;

    /// Store a response under the given hash key.
    fn put(
        &self,
        key: u64,
        request_body: String,
        response: CachedResponse,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

    /// Remove an entry by key (e.g. on expiry).
    fn remove(&self, key: u64) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

// ---- In-memory store -------------------------------------------------------

/// A cached response with its insertion timestamp and the serialized request
/// body used to verify lookups (guarding against 64-bit hash collisions).
#[derive(Clone)]
struct CacheEntry {
    /// Serialized request body — compared on lookup to avoid collision false positives.
    request_body: String,
    response: CachedResponse,
    inserted_at: Instant,
}

struct InnerCache {
    map: HashMap<u64, CacheEntry>,
    /// Keys in insertion order (front = oldest).
    order: VecDeque<u64>,
    max_entries: usize,
    ttl: Duration,
}

impl InnerCache {
    fn new(config: &CacheConfig) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_entries: config.max_entries,
            ttl: config.ttl,
        }
    }

    /// Try to read a cached entry without needing mutable access.
    ///
    /// Returns `Some(response)` when the entry exists, matches the serialized
    /// request body, and has not expired.  Returns `None` on miss.
    ///
    /// For [`CachedResponse::Error`] entries, expiry is checked against the
    /// per-entry `expires_at` instant (set by `NegativeCacheLayer`) rather than
    /// the global `ttl`, because the negative-cache window is controlled by the
    /// policy, not the success-cache TTL.
    fn get_if_valid(&self, key: u64, request_body: &str) -> Option<CachedResponse> {
        let entry = self.map.get(&key)?;
        if entry.request_body != request_body {
            // Hash collision — different request mapped to the same key.
            return None;
        }
        // Error entries carry their own expiry; success entries use the global TTL.
        let is_expired = match &entry.response {
            CachedResponse::Error { expires_at, .. } => Instant::now() >= *expires_at,
            _ => entry.inserted_at.elapsed() > self.ttl,
        };
        if is_expired {
            return None;
        }
        // Clone is required: the cache retains ownership while the caller
        // receives an independent copy.  See `CachedResponse` doc comment for
        // performance discussion.
        Some(entry.response.clone())
    }

    /// Return `true` if the entry for `key` exists and is expired.
    fn is_expired(&self, key: u64) -> bool {
        self.map.get(&key).is_some_and(|e| {
            match &e.response {
                CachedResponse::Error { expires_at, .. } => Instant::now() >= *expires_at,
                _ => e.inserted_at.elapsed() > self.ttl,
            }
        })
    }

    /// Remove an expired entry (eviction under write lock).
    fn remove_expired(&mut self, key: u64) {
        let expired = self.map.get(&key).is_some_and(|e| {
            match &e.response {
                CachedResponse::Error { expires_at, .. } => Instant::now() >= *expires_at,
                _ => e.inserted_at.elapsed() > self.ttl,
            }
        });
        if expired {
            self.map.remove(&key);
            // Lazily cleaned from `order` during eviction.
        }
    }

    fn insert(&mut self, key: u64, request_body: String, response: CachedResponse) {
        // Remove duplicate from the LRU deque before reinserting so entries
        // are not counted twice toward the capacity limit.
        if self.map.contains_key(&key) {
            self.order.retain(|k| *k != key);
        }

        // Evict oldest entries if at capacity.
        while self.map.len() >= self.max_entries {
            if let Some(oldest_key) = self.order.pop_front() {
                self.map.remove(&oldest_key);
            } else {
                break;
            }
        }

        self.map.insert(
            key,
            CacheEntry {
                request_body,
                response,
                inserted_at: Instant::now(),
            },
        );
        self.order.push_back(key);
    }
}

/// In-memory LRU cache store.
///
/// This is the default [`CacheStore`] backend used by [`CacheLayer::new`].
/// It uses a [`HashMap`] with a [`VecDeque`] for LRU eviction order.
#[cfg_attr(alef, alef(skip))]
pub struct InMemoryStore {
    inner: RwLock<InnerCache>,
}

impl InMemoryStore {
    /// Create a new in-memory store with the given configuration.
    #[must_use]
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            inner: RwLock::new(InnerCache::new(config)),
        }
    }
}

impl CacheStore for InMemoryStore {
    fn get(&self, key: u64, request_body: &str) -> Pin<Box<dyn Future<Output = Option<CachedResponse>> + Send + '_>> {
        // Perform all synchronous work eagerly, then wrap result in a ready
        // future.  This avoids capturing `request_body` in an async block
        // (which would require tying its lifetime to the future).
        let result = self.inner.read().ok().and_then(|cache| {
            let hit = cache.get_if_valid(key, request_body);
            let expired = hit.is_none() && cache.is_expired(key);
            drop(cache);
            if expired && let Ok(mut w) = self.inner.write() {
                w.remove_expired(key);
            }
            hit
        });
        Box::pin(std::future::ready(result))
    }

    fn put(
        &self,
        key: u64,
        request_body: String,
        response: CachedResponse,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        if let Ok(mut cache) = self.inner.write() {
            cache.insert(key, request_body, response);
        }
        Box::pin(std::future::ready(()))
    }

    fn remove(&self, key: u64) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        if let Ok(mut cache) = self.inner.write() {
            cache.map.remove(&key);
        }
        Box::pin(std::future::ready(()))
    }
}

// ---- Layer -----------------------------------------------------------------

/// Tower [`Layer`] that caches non-streaming LLM responses.
#[cfg_attr(alef, alef(skip))]
pub struct CacheLayer {
    store: Arc<dyn CacheStore>,
}

impl CacheLayer {
    /// Create a new cache layer with the given configuration.
    ///
    /// Uses the default [`InMemoryStore`] backend.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            store: Arc::new(InMemoryStore::new(&config)),
        }
    }

    /// Create a new cache layer with a custom [`CacheStore`] backend.
    #[must_use]
    pub fn with_store(store: Arc<dyn CacheStore>) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            store: Arc::clone(&self.store),
        }
    }
}

// ---- Service ---------------------------------------------------------------

/// Tower service produced by [`CacheLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct CacheService<S> {
    inner: S,
    store: Arc<dyn CacheStore>,
}

impl<S: Clone> Clone for CacheService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            store: Arc::clone(&self.store),
        }
    }
}

/// Compute a cache key and serialized body from the request.
///
/// Only `Chat` and `Embed` requests are cacheable.  Returns `None` for all
/// other request variants (streaming, `ListModels`, image, audio, etc.).
///
/// The returned tuple contains the 64-bit hash key and the serialized request
/// body.  The body is stored alongside the cached response so lookups can
/// verify against hash collisions.
pub(crate) fn cache_key(req: &LlmRequest) -> Option<(u64, String)> {
    let json = match req {
        LlmRequest::Chat(r) => serde_json::to_string(r).ok()?,
        LlmRequest::Embed(r) => serde_json::to_string(r).ok()?,
        // Not cacheable.
        _ => return None,
    };

    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    Some((hasher.finish(), json))
}

impl<S> Service<LlmRequest> for CacheService<S>
where
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
        let fut = self.inner.call(req);

        Box::pin(async move {
            // Check cache for a hit.  `into_llm_response` returns `Err` for
            // `CachedResponse::Error` entries written by `NegativeCacheLayer`.
            if let Some((k, ref body)) = key_and_body
                && let Some(cached) = store.get(k, body).await
            {
                return cached.into_llm_response();
            }

            let resp = fut.await?;

            // Store cacheable responses.
            if let Some((k, body)) = key_and_body {
                let cached = match &resp {
                    LlmResponse::Chat(r) => Some(CachedResponse::Chat(r.clone())),
                    LlmResponse::Embed(r) => Some(CachedResponse::Embed(r.clone())),
                    _ => None,
                };
                if let Some(cached) = cached {
                    store.put(k, body, cached).await;
                }
            }

            Ok(resp)
        })
    }
}

// ---- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;

    #[tokio::test]
    async fn cache_returns_cached_response_on_second_call() {
        let config = CacheConfig {
            backend: CacheBackend::default(),
            max_entries: 10,
            ttl: Duration::from_secs(60),
        };
        let layer = CacheLayer::new(config);
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        // First call — cache miss.
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Second call — cache hit.
        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 1, "second call should hit cache");
    }

    #[tokio::test]
    async fn cache_does_not_cache_streaming_requests() {
        let config = CacheConfig {
            backend: CacheBackend::default(),
            max_entries: 10,
            ttl: Duration::from_secs(60),
        };
        let layer = CacheLayer::new(config);
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::ChatStream(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        svc.call(LlmRequest::ChatStream(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 2, "streaming should not be cached");
    }

    #[tokio::test]
    async fn cache_evicts_oldest_when_full() {
        let config = CacheConfig {
            backend: CacheBackend::default(),
            max_entries: 1,
            ttl: Duration::from_secs(60),
        };
        let layer = CacheLayer::new(config);
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        // Fill cache with model-a.
        svc.call(LlmRequest::Chat(chat_req("model-a")))
            .await
            .expect("service call should not fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Insert model-b, evicting model-a.
        svc.call(LlmRequest::Chat(chat_req("model-b")))
            .await
            .expect("service call should not fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        // model-a should be evicted — cache miss.
        svc.call(LlmRequest::Chat(chat_req("model-a")))
            .await
            .expect("service call should not fail");
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            3,
            "evicted entry should be a cache miss"
        );
    }

    #[tokio::test]
    async fn cache_different_requests_have_different_keys() {
        let config = CacheConfig {
            backend: CacheBackend::default(),
            max_entries: 10,
            ttl: Duration::from_secs(60),
        };
        let layer = CacheLayer::new(config);
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        svc.call(LlmRequest::Chat(chat_req("gpt-3.5-turbo")))
            .await
            .expect("service call should not fail");
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "different models should be cache misses"
        );
    }
}
