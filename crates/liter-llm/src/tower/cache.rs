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
//! # `NegativeCacheLayer` and `CacheLayer` must share a key strategy
//!
//! Both layers read and write the same [`CacheStore`], so they must derive
//! identical `(u64, String)` keys for the same request — otherwise an error
//! written by `NegativeCacheLayer` lands in a key space `CacheLayer` never
//! looks up, making the negative-cache entry permanently unreachable. The
//! default constructors on both layers agree (both default to
//! [`ExactHashStrategy`]); if you call
//! [`CacheLayer::with_key_strategy`], pass the *same* strategy instance (via
//! [`CacheLayer::key_strategy`]) into
//! [`crate::tower::cache_negative::NegativeCacheLayer::with_key_strategy`].
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

use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock, RwLock};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use crate::observability::usage::CacheState;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tower::{Layer, Service};

use super::types::{LlmRequest, LlmRequestKind, LlmResponse};
use crate::cache_key::{CacheKeyInput, CacheKeyStrategy, ExactHashStrategy};
use crate::client::BoxFuture;
use crate::embedding::EmbeddingProvider;
use crate::error::{LiterLlmError, Result};
use crate::tower::cache_policy::{CacheDecision, CachePolicy, CachePolicyContext, StandardCachePolicy};
use crate::types::{ChatCompletionResponse, EmbeddingResponse};
use crate::vectorstore::VectorStore;

// ~keep task_local! values must be 'static; Cell gives per-task mutability without Sync.
tokio::task_local! {
    /// Records the cache outcome for the current request task.
    ///
    /// Initialized by [`crate::tower::hooks::HooksService`] via
    /// `CACHE_STATE_CELL.scope(Cell::new(CacheState::Bypass), fut)` before
    /// the inner service stack runs. `CacheService` and
    /// `SingleflightService` update the cell via [`record_cache_state`].
    pub static CACHE_STATE_CELL: Cell<CacheState>;
}

/// Set the cache outcome for the current task.
///
/// Uses `try_with` so that callers that run outside a `CACHE_STATE_CELL.scope`
/// (e.g. in tests that do not involve `HooksLayer`) are silently ignored rather
/// than panicking.
#[cfg_attr(alef, alef(skip))]
pub fn record_cache_state(state: CacheState) {
    let _ = CACHE_STATE_CELL.try_with(|c| c.set(state));
}

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
/// ### Why a shared error value rather than a serialisable form?
///
/// `LiterLlmError` contains a `reqwest::Error` variant gated on `native-http`.
/// That variant is not `Serialize`, so the enum cannot derive `Serialize`
/// unconditionally.  Wrapping in `Arc` lets the in-memory store pass the value
/// around cheaply without serialisation.  External stores (Redis, DynamoDB)
/// that require serialisation should handle the `Error` variant explicitly in
/// their `CacheStore` implementation, converting to and from a serialisable
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
#[derive(Clone, Debug)]
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
    /// The in-memory `NegativeCacheLayer` stores shared error values behind an
    /// `Arc`, so `self` is never the sole owner here — `InnerCache::get_if_valid`
    /// hands back a `.clone()` of the stored entry, which bumps the `Arc`'s
    /// strong count to at least 2 before this method ever runs. An
    /// `Arc::try_unwrap` would therefore always fail; `to_singleflight_error`
    /// reconstructs an owned, semantically equivalent error (preserving the
    /// variant and fields like `retry_after`) from the shared reference instead
    /// of falling back to a variant-discarding `InternalError`. ~keep
    pub fn into_llm_response(self) -> Result<LlmResponse> {
        match self {
            Self::Chat(r) => Ok(LlmResponse::Chat(r)),
            Self::Embed(r) => Ok(LlmResponse::Embed(r)),
            Self::Error { error, .. } => Err(error.to_singleflight_error()),
        }
    }

    /// Returns `true` if this entry is an `Error` variant that has passed its expiry.
    #[must_use]
    pub fn is_expired_error(&self) -> bool {
        matches!(self, Self::Error { expires_at, .. } if Instant::now() >= *expires_at)
    }
}

/// Metadata about a cached entry.
///
/// Returned by [`CacheStore::metadata`].  Implementations that cannot track
/// all fields (e.g. because the backing store does not expose TTL or hit
/// counts) may return approximate values.
#[derive(Debug, Clone)]
pub struct CacheMetadata {
    /// When the entry was written into the cache.
    pub inserted_at: Instant,
    /// Effective TTL at insertion time.
    pub ttl: Duration,
    /// Approximate serialized size of the stored response in bytes.
    pub size_bytes: usize,
    /// Number of times this entry has been served since insertion.
    pub hit_count: u64,
}

/// Pluggable cache backend.
///
/// Implement this trait to provide a custom storage layer (Redis, DynamoDB,
/// disk, etc.).  The default in-memory implementation is [`InMemoryStore`].
///
/// All methods return pinned, boxed futures so the trait is object-safe and
/// can be used behind `Arc<dyn CacheStore>`.
///
/// # Extension methods
///
/// The trait provides three extension methods with default no-op
/// implementations so that existing `CacheStore` implementations do not need
/// to be updated:
///
/// - [`set_ttl`][CacheStore::set_ttl] — per-entry TTL override.
/// - [`iter_keys`][CacheStore::iter_keys] — enumerate all stored keys (for cache warming).
/// - [`metadata`][CacheStore::metadata] — return metadata for a single entry.
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

    /// Override the TTL for an existing entry.
    ///
    /// Has no effect if the entry does not exist.
    /// Default implementation is a no-op.
    fn set_ttl(&self, _key: u64, _ttl: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(std::future::ready(()))
    }

    /// Enumerate all stored cache keys.
    ///
    /// Used by cache-warming utilities to pre-populate the store.
    /// Default implementation returns an empty list.
    fn iter_keys(&self) -> Pin<Box<dyn Future<Output = Vec<u64>> + Send + '_>> {
        Box::pin(std::future::ready(Vec::new()))
    }

    /// Return metadata for the entry with the given key.
    ///
    /// Returns `None` if the key does not exist.
    /// Default implementation returns `None`.
    fn metadata(&self, _key: u64) -> Pin<Box<dyn Future<Output = Option<CacheMetadata>> + Send + '_>> {
        Box::pin(std::future::ready(None))
    }
}

/// A cached response with its insertion timestamp and the serialized request
/// body used to verify lookups (guarding against 64-bit hash collisions).
#[derive(Clone)]
struct CacheEntry {
    /// Serialized request body — compared on lookup to avoid collision false positives.
    request_body: String,
    response: CachedResponse,
    inserted_at: Instant,
    /// Per-entry TTL override. `None` falls back to `InnerCache::ttl`.
    ttl_override: Option<Duration>,
    /// Number of times this entry has been served since insertion.
    hit_count: u64,
    /// Approximate size of the serialized response body in bytes.
    size_bytes: usize,
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

    /// Effective TTL for an entry (per-entry override wins over global TTL).
    fn effective_ttl(&self, entry: &CacheEntry) -> Duration {
        entry.ttl_override.unwrap_or(self.ttl)
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
            return None;
        }
        let is_expired = match &entry.response {
            CachedResponse::Error { expires_at, .. } => Instant::now() >= *expires_at,
            _ => entry.inserted_at.elapsed() > self.effective_ttl(entry),
        };
        if is_expired {
            return None;
        }
        Some(entry.response.clone())
    }

    /// Remove an expired entry (eviction under write lock).
    ///
    /// Must also drop `key` from `order`, or `order` desyncs from `map`: a key
    /// that repeatedly expires and gets re-inserted (one write per TTL period)
    /// would otherwise leave one stale duplicate behind in `order` per cycle
    /// forever, growing it without bound while `map` stays at a single entry. ~keep
    fn remove_expired(&mut self, key: u64) {
        let ttl = self.ttl;
        let expired = self.map.get(&key).is_some_and(|e| {
            let eff = e.ttl_override.unwrap_or(ttl);
            match &e.response {
                CachedResponse::Error { expires_at, .. } => Instant::now() >= *expires_at,
                _ => e.inserted_at.elapsed() > eff,
            }
        });
        if expired {
            self.map.remove(&key);
            self.order.retain(|k| *k != key);
        }
    }

    fn insert(&mut self, key: u64, request_body: String, response: CachedResponse) {
        let is_new = !self.map.contains_key(&key);
        if !is_new {
            self.order.retain(|k| *k != key);
        }

        // ~keep Only run eviction for a genuinely new key. Re-inserting an existing
        // ~keep key does not grow `map`, so gating this on `is_new` prevents evicting
        // ~keep an unrelated live entry: without the gate, `map.len()` still counts the
        // ~keep about-to-be-overwritten key's old entry (not yet removed at this point),
        // ~keep so a re-insert at exact capacity looked indistinguishable from "full"
        // ~keep and evicted the true-oldest entry despite there being no real pressure.
        if is_new {
            while self.map.len() >= self.max_entries {
                if let Some(oldest_key) = self.order.pop_front() {
                    self.map.remove(&oldest_key);
                } else {
                    break;
                }
            }
        }

        let size_bytes = serde_json::to_string(&response).map(|s| s.len()).unwrap_or(0);
        self.map.insert(
            key,
            CacheEntry {
                request_body,
                response,
                inserted_at: Instant::now(),
                ttl_override: None,
                hit_count: 0,
                size_bytes,
            },
        );
        self.order.push_back(key);
    }

    /// Bump the hit counter for an entry.  No-op if the key does not exist.
    fn record_hit(&mut self, key: u64) {
        if let Some(entry) = self.map.get_mut(&key) {
            entry.hit_count = entry.hit_count.saturating_add(1);
        }
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
        // ~keep Perform synchronous cache read/expiry/hit-count work under one write lock to avoid TOCTOU.
        // ~keep Sharded locks are the upgrade path if this single-lock path becomes contentious.
        let hit = match self.inner.write() {
            Ok(mut cache) => {
                // ~keep Check validity first; `get_if_valid` handles expiry inline.
                let hit = cache.get_if_valid(key, request_body);
                if hit.is_none() {
                    cache.remove_expired(key);
                } else {
                    cache.record_hit(key);
                }
                hit
            }
            Err(_) => {
                warn_lock_poisoned("get");
                None
            }
        };
        Box::pin(std::future::ready(hit))
    }

    fn put(
        &self,
        key: u64,
        request_body: String,
        response: CachedResponse,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        match self.inner.write() {
            Ok(mut cache) => cache.insert(key, request_body, response),
            Err(_) => warn_lock_poisoned("put"),
        }
        Box::pin(std::future::ready(()))
    }

    fn remove(&self, key: u64) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        match self.inner.write() {
            Ok(mut cache) => {
                cache.map.remove(&key);
                // ~keep Must stay in sync with `map`, or `order` accumulates a stale
                // ~keep duplicate for every explicitly removed key (see `remove_expired`).
                cache.order.retain(|k| *k != key);
            }
            Err(_) => warn_lock_poisoned("remove"),
        }
        Box::pin(std::future::ready(()))
    }

    fn set_ttl(&self, key: u64, ttl: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        match self.inner.write() {
            Ok(mut cache) => {
                if let Some(entry) = cache.map.get_mut(&key) {
                    entry.ttl_override = Some(ttl);
                }
            }
            Err(_) => warn_lock_poisoned("set_ttl"),
        }
        Box::pin(std::future::ready(()))
    }

    fn iter_keys(&self) -> Pin<Box<dyn Future<Output = Vec<u64>> + Send + '_>> {
        let keys = match self.inner.read() {
            Ok(cache) => cache.map.keys().copied().collect(),
            Err(_) => {
                warn_lock_poisoned("iter_keys");
                Vec::new()
            }
        };
        Box::pin(std::future::ready(keys))
    }

    fn metadata(&self, key: u64) -> Pin<Box<dyn Future<Output = Option<CacheMetadata>> + Send + '_>> {
        let result = match self.inner.read() {
            Ok(cache) => cache.map.get(&key).map(|entry| CacheMetadata {
                inserted_at: entry.inserted_at,
                ttl: cache.effective_ttl(entry),
                size_bytes: entry.size_bytes,
                hit_count: entry.hit_count,
            }),
            Err(_) => {
                warn_lock_poisoned("metadata");
                None
            }
        };
        Box::pin(std::future::ready(result))
    }
}

/// Log a poisoned in-memory cache lock instead of silently treating every
/// subsequent operation as a cache miss / no-op.
///
/// A poisoned `RwLock` means a previous holder panicked while mutating the
/// cache; the data structure itself is still intact (`std::sync::RwLock`
/// does not discard state on poisoning), so falling back to "miss" is safe,
/// but doing so silently makes a degraded cache indistinguishable from a
/// merely cold one. Emitting a `WARN` here lets operators notice and act.
fn warn_lock_poisoned(op: &'static str) {
    tracing::warn!(operation = op, "in-memory cache lock poisoned; treating as no-op/miss");
}

/// Tower [`Layer`] that caches non-streaming LLM responses.
///
/// Supports two tiers (configured via [`CachePolicy`]):
///
/// 1. **Exact hash** — fast O(1) lookup keyed by the full serialized request.
/// 2. **Semantic** — embedding-similarity lookup via [`EmbeddingProvider`] +
///    [`VectorStore`] (opt-in via policy).
///
/// Joining an in-progress request as a streaming-replay follower is handled by
/// [`crate::tower::cache_singleflight::SingleflightLayer`], not by this layer;
/// `CacheDecision::use_streaming_replay` (removed) never had a reader here.
#[cfg_attr(alef, alef(skip))]
pub struct CacheLayer {
    store: Arc<dyn CacheStore>,
    key_strategy: Arc<dyn CacheKeyStrategy>,
    cache_policy: Arc<dyn CachePolicy>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    vector_store: Option<Arc<dyn VectorStore>>,
}

impl CacheLayer {
    /// Create a new cache layer with the given configuration.
    ///
    /// Uses the default [`InMemoryStore`] backend and [`ExactHashStrategy`]
    /// key strategy with the [`StandardCachePolicy`].
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        // ~keep The policy's exact_ttl must come from the config. `decide` returns
        // ~keep `ttl_override: Some(exact_ttl)` on every non-bypassed write, and that
        // ~keep override wins over the store's own TTL — so leaving it at the policy
        // ~keep default silently discarded `config.ttl` entirely. It went unnoticed
        // ~keep because both defaults are 300s, written independently in two places.
        let cache_policy = StandardCachePolicy {
            exact_ttl: config.ttl,
            ..StandardCachePolicy::default()
        };

        Self {
            store: Arc::new(InMemoryStore::new(&config)),
            key_strategy: Arc::new(ExactHashStrategy),
            cache_policy: Arc::new(cache_policy),
            embedding_provider: None,
            vector_store: None,
        }
    }

    /// Create a new cache layer with a custom [`CacheStore`] backend, using the
    /// default cache policy.
    ///
    /// This constructor takes no [`CacheConfig`], so it cannot honour a
    /// configured TTL — entries expire at [`StandardCachePolicy`]'s default.
    /// Prefer [`Self::with_store_and_config`] whenever a config is available. ~keep
    #[must_use]
    pub fn with_store(store: Arc<dyn CacheStore>) -> Self {
        Self {
            store,
            key_strategy: Arc::new(ExactHashStrategy),
            cache_policy: Arc::new(StandardCachePolicy::default()),
            embedding_provider: None,
            vector_store: None,
        }
    }

    /// Create a new cache layer with a custom [`CacheStore`] backend that
    /// honours `config`'s TTL.
    ///
    /// `decide` returns `ttl_override: Some(exact_ttl)` on every non-bypassed
    /// write and that override wins over the store's own TTL, so a policy left
    /// at its default silently discards `config.ttl` — the same defect
    /// [`Self::new`] carried. A custom store that implements `set_ttl` is
    /// overridden by the policy regardless of what TTL it was built with. ~keep
    #[must_use]
    pub fn with_store_and_config(store: Arc<dyn CacheStore>, config: &CacheConfig) -> Self {
        Self {
            store,
            key_strategy: Arc::new(ExactHashStrategy),
            cache_policy: Arc::new(StandardCachePolicy {
                exact_ttl: config.ttl,
                ..StandardCachePolicy::default()
            }),
            embedding_provider: None,
            vector_store: None,
        }
    }

    /// Set a custom [`CacheKeyStrategy`].
    #[must_use]
    pub fn with_key_strategy(mut self, strategy: Arc<dyn CacheKeyStrategy>) -> Self {
        self.key_strategy = strategy;
        self
    }

    /// Return the configured [`CacheKeyStrategy`].
    ///
    /// Use this to wire the identical strategy into
    /// [`crate::tower::cache_negative::NegativeCacheLayer::with_key_strategy`]
    /// when customizing key derivation — both layers must agree on key
    /// derivation for negative-cache entries to be visible to this layer's
    /// read path.
    #[must_use]
    pub fn key_strategy(&self) -> Arc<dyn CacheKeyStrategy> {
        Arc::clone(&self.key_strategy)
    }

    /// Set a custom [`CachePolicy`].
    #[must_use]
    pub fn with_policy(mut self, policy: Arc<dyn CachePolicy>) -> Self {
        self.cache_policy = policy;
        self
    }

    /// Enable the semantic cache tier by providing an [`EmbeddingProvider`]
    /// and a [`VectorStore`].
    #[must_use]
    pub fn with_semantic_cache(
        mut self,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Self {
        self.embedding_provider = Some(embedding_provider);
        self.vector_store = Some(vector_store);
        self
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            store: Arc::clone(&self.store),
            key_strategy: Arc::clone(&self.key_strategy),
            cache_policy: Arc::clone(&self.cache_policy),
            embedding_provider: self.embedding_provider.clone(),
            vector_store: self.vector_store.clone(),
        }
    }
}

/// Tower service produced by [`CacheLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct CacheService<S> {
    inner: S,
    store: Arc<dyn CacheStore>,
    key_strategy: Arc<dyn CacheKeyStrategy>,
    cache_policy: Arc<dyn CachePolicy>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    vector_store: Option<Arc<dyn VectorStore>>,
}

impl<S: Clone> Clone for CacheService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            store: Arc::clone(&self.store),
            key_strategy: Arc::clone(&self.key_strategy),
            cache_policy: Arc::clone(&self.cache_policy),
            embedding_provider: self.embedding_provider.clone(),
            vector_store: self.vector_store.clone(),
        }
    }
}

impl<S> CacheService<S> {
    /// Pre-populate the cache by hashing each [`CacheKeyInput`].
    ///
    /// This allocates cache slots without making any upstream calls.  Subsequent
    /// writes for the same keys will replace the warm slot with real responses.
    ///
    /// Useful for warming the exact-hash index before deploying a new version
    /// so the first real traffic wave sees pre-allocated entries.
    pub async fn warm<'a>(&self, requests: impl Iterator<Item = CacheKeyInput<'a>>) {
        for input in requests {
            let (key, body) = self.key_strategy.key_for(&input);
            if self.store.get(key, &body).await.is_none() {
                // ~keep Previously this branch was `let _ = (key, body);` — a complete
                // ~keep no-op that never wrote anything, contradicting this method's own
                // ~keep "allocates cache slots" doc contract. `expires_at: Instant::now()`
                // ~keep makes the placeholder already-expired at the moment it is written:
                // ~keep any real `get()` for this key still treats it as a miss (never
                // ~keep serves fabricated content to a real caller), but it occupies a slot
                // ~keep in `InnerCache::map`/`order` — competing for `max_entries` capacity,
                // ~keep exactly as "allocates cache slots" promises — until either a real
                // ~keep `put()` overwrites it or a `get()` on it triggers `remove_expired`.
                let placeholder = CachedResponse::Error {
                    error: Arc::new(LiterLlmError::InternalError {
                        message: "cache slot pre-warmed by CacheService::warm; not yet populated".into(),
                    }),
                    expires_at: Instant::now(),
                };
                self.store.put(key, body, placeholder).await;
            }
        }
    }
}

/// Derive a [`CacheKeyInput`] from an [`LlmRequest`] suitable for the
/// configured [`CacheKeyStrategy`].
///
/// Returns `None` for non-cacheable request variants.
///
/// # Tenant and system-prompt extraction
///
/// `tenant_id` is sourced from the `user` field of a `Chat` request using the
/// convention `"tenant:<id>"` (e.g. `"tenant:acme"`).  If the field is absent
/// or does not start with `"tenant:"`, `tenant_id` is `None`.
///
/// `system_prompt` is extracted from the first `Message::System` message in
/// the conversation, if one is present.  This ensures that
/// [`SystemPromptAwareStrategy`][crate::cache_key::SystemPromptAwareStrategy]
/// and [`TenantScopedStrategy`][crate::cache_key::TenantScopedStrategy]
/// produce isolation keys correctly; previously both fields were hard-coded to
/// `None`, which meant tenant A's response could be served to tenant B.
///
/// # Field coverage (must stay in sync with [`crate::types::ChatCompletionRequest`])
///
/// `params_json` must include every request field that changes the semantics
/// of the response, or two requests that the caller considers different will
/// collide on the same cache entry and one tenant's request could return
/// another's response.  `tools`, `response_format`, and `seed` were
/// previously omitted here; a request with `tools` attached could receive a
/// cached response generated without any tools available. `stream` and
/// `stream_options` are intentionally omitted for `Chat` — this function is
/// only reached for the non-streaming `LlmRequestKind::Chat` variant, so those
/// fields do not affect the cached payload.
///
/// This is `pub(crate)` (not private) so that
/// [`crate::tower::cache_negative::NegativeCacheService`] can derive the exact
/// same key when writing negative-cache entries; the two services MUST agree
/// on key derivation or an error cached by the negative-cache layer becomes
/// permanently unreadable by the success-path reader (a different hash space
/// is equivalent to a different, disjoint cache).
///
/// ~keep The returned `tenant_id` is the third tuple element so that the
/// ~keep exact-hash tier (which folds it into the returned key) and the
/// ~keep semantic tier (which uses it to scope `VectorStore::search` /
/// ~keep `VectorMetadata::tenant_id`) read tenant identity from one place.
/// ~keep Previously the semantic tier derived nothing at all here and wrote
/// ~keep `tenant_id: None` unconditionally, letting one tenant's semantically
/// ~keep similar prompt be served another tenant's cached response even
/// ~keep though the exact tier was correctly isolated.
pub(crate) fn strategy_key(strategy: &dyn CacheKeyStrategy, req: &LlmRequest) -> Option<(u64, String, Option<String>)> {
    let req_tenant = req.tenant_id().map(|t| t.as_ref().to_owned());
    let (model, messages_json, params_json, tenant_id, system_prompt) = match &req.kind {
        LlmRequestKind::Chat(r) => {
            let msgs = serde_json::to_string(&r.messages).ok()?;
            let params = serde_json::json!({
                "temperature": r.temperature,
                "top_p": r.top_p,
                "max_tokens": r.max_tokens,
                "n": r.n,
                "stop": r.stop,
                "presence_penalty": r.presence_penalty,
                "frequency_penalty": r.frequency_penalty,
                "logit_bias": r.logit_bias,
                "tools": r.tools,
                "tool_choice": r.tool_choice,
                "parallel_tool_calls": r.parallel_tool_calls,
                "response_format": r.response_format,
                "seed": r.seed,
                "reasoning_effort": r.reasoning_effort,
                "modalities": r.modalities,
                "extra_body": r.extra_body,
            });
            let tenant_id: Option<String> = req_tenant.or_else(|| {
                r.user
                    .as_deref()
                    .and_then(|u| u.strip_prefix("tenant:"))
                    .map(str::to_owned)
            });
            let system_prompt: Option<String> = r.messages.iter().find_map(|m| {
                if let crate::types::Message::System(s) = m {
                    s.content.as_text()
                } else {
                    None
                }
            });
            (
                r.model.as_str().to_owned(),
                msgs,
                params.to_string(),
                tenant_id,
                system_prompt,
            )
        }
        LlmRequestKind::Embed(r) => {
            let input = serde_json::to_string(&r.input).ok()?;
            let mut params = serde_json::json!({
                "dimensions": r.dimensions,
                "encoding_format": r.encoding_format,
            });
            // ~keep `extra_body` can change the vectors (for example `input_type`), so it is part
            // ~keep of the key. It is added only when set, which keeps the key of a request
            // ~keep without it identical to the key before the field existed.
            if let Some(extra_body) = &r.extra_body {
                params["extra_body"] = extra_body.clone();
            }
            (r.model.as_str().to_owned(), input, params.to_string(), req_tenant, None)
        }
        _ => return None,
    };

    let input = CacheKeyInput {
        model: &model,
        messages_json: &messages_json,
        params_json: &params_json,
        tenant_id: tenant_id.as_deref(),
        system_prompt: system_prompt.as_deref(),
    };
    let (key, body) = strategy.key_for(&input);
    Some((key, body, tenant_id))
}

impl<S> Service<LlmRequest> for CacheService<S>
where
    S: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, Result<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LlmRequest) -> Self::Future {
        // ~keep SAFETY: StandardCachePolicy only reads metadata, so the static empty map is shared safely.
        static EMPTY_METADATA: OnceLock<HashMap<String, String>> = OnceLock::new();
        let empty_meta = EMPTY_METADATA.get_or_init(HashMap::new);

        let stream = matches!(req.kind, LlmRequestKind::ChatStream(_));
        let model = req.model().unwrap_or("").to_owned();
        let tenant_id_str: Option<String> = req.tenant_id().map(|t| t.as_ref().to_owned());
        let ctx = CachePolicyContext {
            model: &model,
            tenant_id: tenant_id_str.as_deref(),
            stream,
            metadata: empty_meta,
        };
        let decision: CacheDecision = self.cache_policy.decide(&ctx);

        let key_and_body = if decision.bypass {
            None
        } else {
            strategy_key(self.key_strategy.as_ref(), &req)
        };

        let store = Arc::clone(&self.store);
        let embedding_provider = self.embedding_provider.clone();
        let vector_store = self.vector_store.clone();

        // ~keep Consume the poll-ready service instance and leave a fresh standby clone.
        let standby = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, standby);
        let fut = inner.call(req);

        Box::pin(async move {
            if decision.use_exact
                && let Some((k, ref body, _)) = key_and_body
                && let Some(cached) = store.get(k, body).await
            {
                #[cfg(feature = "otel")]
                crate::tower::metrics::record_cache_tier_hit("", &model, "exact");
                record_cache_state(CacheState::ExactHit);
                return cached.into_llm_response();
            }
            #[cfg(feature = "otel")]
            if decision.use_exact && key_and_body.is_some() {
                crate::tower::metrics::record_cache_tier_miss("", &model, "exact");
            }

            if decision.use_semantic
                && let (Some(ep), Some(vs)) = (&embedding_provider, &vector_store)
                && let Some((_, ref body, ref tenant_id)) = key_and_body
            {
                let maybe_cached = async {
                    let input = crate::types::EmbeddingInput::Single(body.clone());
                    let query_vec = ep.embed(&input).await.ok()?;
                    let best = vs
                        .search(&query_vec, 1, decision.similarity_threshold, tenant_id.as_deref())
                        .await
                        .into_iter()
                        .next()?;
                    // ~keep Semantic hits must use the stored original body for collision checking.
                    store
                        .get(best.metadata.cache_key, &best.metadata.original_request_body)
                        .await
                }
                .await;
                if let Some(cached) = maybe_cached {
                    #[cfg(feature = "otel")]
                    crate::tower::metrics::record_cache_tier_hit("", &model, "semantic");
                    record_cache_state(CacheState::SemanticHit);
                    return cached.into_llm_response();
                }
                #[cfg(feature = "otel")]
                crate::tower::metrics::record_cache_tier_miss("", &model, "semantic");
            }

            // ~keep `decision.bypass` means `key_and_body` is always `None`, so the tier
            // ~keep checks above never touch it; report the correct outcome instead of
            // ~keep letting a bypassed request fall through and be misreported as a Miss.
            record_cache_state(if decision.bypass {
                CacheState::Bypass
            } else {
                CacheState::Miss
            });
            let resp = fut.await?;

            if let Some((k, body, tenant_id)) = key_and_body {
                let cached = match &resp {
                    LlmResponse::Chat(r) => Some(CachedResponse::Chat(r.clone())),
                    LlmResponse::Embed(r) => Some(CachedResponse::Embed(r.clone())),
                    _ => None,
                };
                if let Some(cached_resp) = cached {
                    store.put(k, body.clone(), cached_resp).await;
                    if let Some(ttl) = decision.ttl_override {
                        store.set_ttl(k, ttl).await;
                    }

                    if decision.use_semantic
                        && let (Some(ep), Some(vs)) = (&embedding_provider, &vector_store)
                        && let Ok(vec) = ep.embed(&crate::types::EmbeddingInput::Single(body.clone())).await
                    {
                        let metadata = crate::vectorstore::VectorMetadata {
                            cache_key: k,
                            original_request_body: body.clone(),
                            image_url: None,
                            tenant_id,
                            inserted_at: std::time::SystemTime::now(),
                            extra: HashMap::new(),
                        };
                        // ~keep A failed upsert leaves the semantic tier silently blind to this
                        // ~keep entry (the exact tier still has it) rather than corrupting state,
                        // ~keep so it is safe to continue — but it must not be silent, or a
                        // ~keep completely broken vector store looks identical to a working one.
                        if let Err(error) = vs.upsert(format!("{k}"), vec, metadata).await {
                            tracing::warn!(
                                cache_key = k,
                                %error,
                                "semantic cache: vector store upsert failed; entry will not be searchable"
                            );
                        }
                    }
                }
            }

            Ok(resp)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

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

        svc.call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect("service call should not fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

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

        svc.call(LlmRequest::Chat(chat_req("model-a")))
            .await
            .expect("service call should not fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        svc.call(LlmRequest::Chat(chat_req("model-b")))
            .await
            .expect("service call should not fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

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

    #[tokio::test]
    async fn in_memory_store_set_ttl_overrides_default_ttl() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(3600),
            backend: CacheBackend::default(),
        };
        let store = InMemoryStore::new(&config);
        store
            .put(
                1,
                "body".into(),
                CachedResponse::Chat(crate::tower::tests_common::make_chat_response("gpt-4")),
            )
            .await;
        store.set_ttl(1, Duration::from_nanos(1)).await;
        tokio::time::sleep(Duration::from_millis(2)).await;
        let result = store.get(1, "body").await;
        assert!(result.is_none(), "entry with overridden near-zero TTL must be expired");
    }

    #[tokio::test]
    async fn in_memory_store_iter_keys_lists_all_keys() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(3600),
            backend: CacheBackend::default(),
        };
        let store = InMemoryStore::new(&config);
        store
            .put(
                10,
                "b1".into(),
                CachedResponse::Chat(crate::tower::tests_common::make_chat_response("m")),
            )
            .await;
        store
            .put(
                20,
                "b2".into(),
                CachedResponse::Chat(crate::tower::tests_common::make_chat_response("m")),
            )
            .await;
        let mut keys = store.iter_keys().await;
        keys.sort_unstable();
        assert_eq!(keys, vec![10, 20]);
    }

    #[tokio::test]
    async fn in_memory_store_metadata_tracks_hit_count() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(3600),
            backend: CacheBackend::default(),
        };
        let store = InMemoryStore::new(&config);
        store
            .put(
                42,
                "req".into(),
                CachedResponse::Chat(crate::tower::tests_common::make_chat_response("gpt-4")),
            )
            .await;
        let _ = store.get(42, "req").await;
        let _ = store.get(42, "req").await;
        let meta = store.metadata(42).await.expect("metadata must be present");
        assert_eq!(meta.hit_count, 2, "hit_count must reflect both cache hits");
        assert!(meta.size_bytes > 0, "size_bytes must be non-zero");
    }

    /// `InnerCache::remove_expired` must drop the key from `order`, not just
    /// `map`, or the two desync: repeatedly writing and expiring the SAME key
    /// (one write/expiry cycle per period) removes it from `map` each time but
    /// leaves a stale duplicate behind in `order` forever, growing `order`
    /// without bound while `map` stays at a single entry.
    ///
    /// Revert target: removing the `self.order.retain(|k| *k != key);` line
    /// added to `InnerCache::remove_expired` makes `order` grow by one entry
    /// per cycle, so after 5 cycles `order.len()` would be 5, not 0.
    #[tokio::test]
    async fn remove_expired_also_removes_from_order_index() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_millis(20),
            backend: CacheBackend::default(),
        };
        let store = InMemoryStore::new(&config);

        for _ in 0..5 {
            store
                .put(
                    1,
                    "body".into(),
                    CachedResponse::Chat(crate::tower::tests_common::make_chat_response("m")),
                )
                .await;
            tokio::time::sleep(Duration::from_millis(30)).await;
            // ~keep This `get` on an already-expired entry is what triggers `remove_expired`.
            assert!(store.get(1, "body").await.is_none(), "entry must have expired by now");
        }

        let order_len = store.inner.read().unwrap().order.len();
        assert_eq!(
            order_len, 0,
            "order index must not accumulate a stale duplicate per expiry cycle; got {order_len} stale entries"
        );
    }

    /// Re-inserting a key that is already present must not trigger eviction
    /// of an unrelated, live entry just because `map.len()` happens to equal
    /// `max_entries` at that moment (the re-inserted key's OLD entry is still
    /// counted at that point, making an update look like growth).
    ///
    /// Revert target: changing `insert`'s eviction loop back to unconditional
    /// (`while self.map.len() >= self.max_entries`, without the `is_new`
    /// gate) makes this fail — key 2 gets evicted when key 1 is re-inserted.
    #[tokio::test]
    async fn reinsert_existing_key_does_not_evict_unrelated_entry_at_capacity() {
        let config = CacheConfig {
            max_entries: 2,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let store = InMemoryStore::new(&config);

        store
            .put(
                1,
                "a".into(),
                CachedResponse::Chat(crate::tower::tests_common::make_chat_response("m")),
            )
            .await;
        store
            .put(
                2,
                "b".into(),
                CachedResponse::Chat(crate::tower::tests_common::make_chat_response("m")),
            )
            .await;

        // At capacity (2/2). Re-insert key 1 — an update, not a new key.
        store
            .put(
                1,
                "a".into(),
                CachedResponse::Chat(crate::tower::tests_common::make_chat_response("m2")),
            )
            .await;

        assert!(
            store.get(2, "b").await.is_some(),
            "re-inserting an existing key must not evict an unrelated live entry when at capacity"
        );
    }

    #[tokio::test]
    async fn three_tier_exact_hit_short_circuits_upstream() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let layer = CacheLayer::new(config);
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "exact hit must short-circuit upstream"
        );
    }

    /// Verify that the semantic cache tier returns a stored response when the
    /// vector store reports a similarity match above the threshold.
    ///
    /// Previously, the tier called `store.get(key, current_body)` where
    /// `current_body` is the incoming request's serialised form.  The
    /// collision-guard comparison always failed because the stored body (from
    /// the original request) differs from `current_body` by definition.
    ///
    /// The fix: `VectorMetadata` now carries `original_request_body`, and the
    /// semantic tier passes that to `store.get` instead of the current body.
    #[tokio::test]
    async fn semantic_cache_tier_returns_hit_when_vector_match_above_threshold() {
        use std::collections::HashMap;
        use std::sync::Arc;
        use std::time::SystemTime;

        use crate::cache_key::ExactHashStrategy;
        use crate::embedding::NoOpEmbeddingProvider;
        use crate::tower::cache_policy::StandardCachePolicy;
        use crate::vectorstore::{InMemoryVectorStore, VectorMetadata, VectorStore};

        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let store: Arc<dyn CacheStore> = Arc::new(InMemoryStore::new(&config));

        let cached = CachedResponse::Chat(crate::tower::tests_common::make_chat_response("gpt-4"));
        let exact_key: u64 = 9999;
        let sentinel_body = "sentinel-body";
        store.put(exact_key, sentinel_body.into(), cached).await;

        let vs: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new(1));
        vs.upsert(
            "sentinel".into(),
            vec![0.0],
            VectorMetadata {
                cache_key: exact_key,
                original_request_body: sentinel_body.into(),
                image_url: None,
                tenant_id: None,
                inserted_at: SystemTime::now(),
                extra: HashMap::new(),
            },
        )
        .await
        .unwrap();

        let ep: Arc<dyn crate::embedding::EmbeddingProvider> = Arc::new(NoOpEmbeddingProvider { dim: 1 });

        let policy = Arc::new(StandardCachePolicy {
            semantic_ttl: Some(Duration::from_secs(60)),
            similarity_threshold: 0.0,
            ..Default::default()
        });

        let layer = CacheLayer::with_store(Arc::clone(&store))
            .with_key_strategy(Arc::new(ExactHashStrategy))
            .with_policy(policy)
            .with_semantic_cache(ep, vs);

        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            0,
            "semantic hit must short-circuit upstream without calling it"
        );
    }

    /// Regression test for the semantic-tier tenant-isolation bug: the
    /// semantic tier used to write every `VectorMetadata` with
    /// `tenant_id: None` regardless of the request's actual tenant, and
    /// `VectorStore::search` had no tenant parameter to filter on even if the
    /// metadata had been populated correctly. Since `TenantScopedStrategy`
    /// already gives tenant A and tenant B disjoint *exact*-key hashes for an
    /// otherwise identical request, an exact-tier miss for tenant B's request
    /// must fall through to a semantic-tier *miss* too — it must not surface
    /// tenant A's entry just because the (exact-tier-agnostic) embedding
    /// vector matches. `NoOpEmbeddingProvider` returns the zero vector for
    /// every input, so with `similarity_threshold: 0.0` every entry matches
    /// on vector similarity alone — only the tenant filter can prevent the
    /// cross-tenant hit here, which is exactly what this test isolates.
    #[tokio::test]
    async fn semantic_cache_tier_does_not_leak_across_tenants() {
        use crate::cache_key::TenantScopedStrategy;
        use crate::embedding::NoOpEmbeddingProvider;
        use crate::tower::cache_policy::StandardCachePolicy;
        use crate::vectorstore::InMemoryVectorStore;

        let config = CacheConfig {
            max_entries: 20,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };

        let vs: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new(1));
        let ep: Arc<dyn crate::embedding::EmbeddingProvider> = Arc::new(NoOpEmbeddingProvider { dim: 1 });
        let policy = Arc::new(StandardCachePolicy {
            semantic_ttl: Some(Duration::from_secs(60)),
            similarity_threshold: 0.0,
            ..Default::default()
        });

        let layer = CacheLayer::new(config)
            .with_key_strategy(Arc::new(TenantScopedStrategy))
            .with_policy(policy)
            .with_semantic_cache(ep, vs);

        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        let mut req_a = chat_req("gpt-4");
        req_a.user = Some("tenant:acme".into());

        let mut req_b = chat_req("gpt-4");
        req_b.user = Some("tenant:globex".into());

        svc.call(LlmRequest::Chat(req_a)).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "tenant A's first call must miss and populate both cache tiers"
        );

        svc.call(LlmRequest::Chat(req_b)).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "tenant B must not receive tenant A's semantically-matched cached response"
        );
    }

    /// Verify that `TenantScopedStrategy` produces different cache keys for
    /// different tenants so that tenant A's response is never served to tenant B.
    ///
    /// Previously `strategy_key` hard-coded `tenant_id: None`, meaning the
    /// strategy's tenant prefix was never applied and all tenants shared the
    /// same cache slot — a data-leakage bug.
    #[tokio::test]
    async fn tenant_scoped_strategy_isolates_tenants_via_cache_service() {
        use crate::cache_key::TenantScopedStrategy;

        let config = CacheConfig {
            max_entries: 20,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let layer = CacheLayer::new(config).with_key_strategy(Arc::new(TenantScopedStrategy));
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        let mut req_a = chat_req("gpt-4");
        req_a.user = Some("tenant:acme".into());

        let mut req_b = chat_req("gpt-4");
        req_b.user = Some("tenant:globex".into());

        svc.call(LlmRequest::Chat(req_a.clone())).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1, "first call must miss");

        svc.call(LlmRequest::Chat(req_b)).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "tenant-b must not receive tenant-a cached response"
        );

        svc.call(LlmRequest::Chat(req_a)).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "tenant-a second call must hit cache"
        );
    }

    /// Verify that `SystemPromptAwareStrategy` isolates responses by system
    /// prompt, so different system prompts produce different cache keys.
    ///
    /// Previously `strategy_key` hard-coded `system_prompt: None`, so the
    /// system prompt was never factored into the cache key.
    #[tokio::test]
    async fn system_prompt_aware_strategy_isolates_via_cache_service() {
        use crate::cache_key::SystemPromptAwareStrategy;
        use crate::types::{Message, SystemMessage, UserContent, UserMessage};

        let config = CacheConfig {
            max_entries: 20,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let layer = CacheLayer::new(config).with_key_strategy(Arc::new(SystemPromptAwareStrategy));
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        let mut req_a = chat_req("gpt-4");
        req_a.messages = vec![
            Message::System(SystemMessage {
                content: "You are a helpful assistant.".into(),
                name: None,
            }),
            Message::User(UserMessage {
                content: UserContent::Text("Hello".into()),
                name: None,
            }),
        ];

        let mut req_b = chat_req("gpt-4");
        req_b.messages = vec![
            Message::System(SystemMessage {
                content: "You are a pirate.".into(),
                name: None,
            }),
            Message::User(UserMessage {
                content: UserContent::Text("Hello".into()),
                name: None,
            }),
        ];

        svc.call(LlmRequest::Chat(req_a.clone())).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1, "first call must miss");

        svc.call(LlmRequest::Chat(req_b)).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "different system prompt must produce a cache miss"
        );

        svc.call(LlmRequest::Chat(req_a)).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "same system prompt must hit cache"
        );
    }

    /// Stress-test `InMemoryStore::get` under concurrent get/put pairs.
    ///
    /// Verifies that the single-lock refactor eliminates TOCTOU races: one
    /// hundred tasks each write a unique key then immediately read it back.
    /// No panics, no data corruption, and every write is immediately readable.
    #[tokio::test]
    async fn in_memory_store_get_single_lock_acquisition() {
        let config = CacheConfig {
            max_entries: 1000,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let store = Arc::new(InMemoryStore::new(&config));
        const TASKS: u64 = 100;

        let handles: Vec<_> = (0..TASKS)
            .map(|i| {
                let store = Arc::clone(&store);
                tokio::spawn(async move {
                    let key = i;
                    let body = format!("body-{i}");
                    let response = CachedResponse::Chat(crate::tower::tests_common::make_chat_response("m"));
                    store.put(key, body.clone(), response).await;
                    let result = store.get(key, &body).await;
                    assert!(
                        result.is_some(),
                        "key {key} written by task {i} must be immediately readable"
                    );
                })
            })
            .collect();

        for h in handles {
            h.await.expect("task must not panic");
        }
    }

    /// A configured `CacheConfig.ttl` must actually govern entry lifetime.
    ///
    /// `StandardCachePolicy::decide` returns `ttl_override: Some(exact_ttl)` on
    /// every non-bypassed write, and that override wins over the store's own
    /// TTL — so building the layer with the policy default silently discarded
    /// `config.ttl`. The bug was invisible because `CacheConfig::default().ttl`
    /// and `StandardCachePolicy::default().exact_ttl` are both 300s, defined
    /// independently; only a non-default value exposes it.
    #[tokio::test]
    async fn configured_ttl_expires_the_entry() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_millis(60),
            backend: CacheBackend::default(),
        };
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let mut svc = CacheLayer::new(config).layer(LlmService::new(client));

        svc.call(LlmRequest::Chat(chat_req("ttl-model"))).await.unwrap();
        svc.call(LlmRequest::Chat(chat_req("ttl-model"))).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "second call within the TTL must be served from cache"
        );

        tokio::time::sleep(Duration::from_millis(150)).await;

        svc.call(LlmRequest::Chat(chat_req("ttl-model"))).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "entry must expire after the CONFIGURED 60ms, not the 300s policy default"
        );
    }

    /// The same TTL defect survived on the custom-store path after `CacheLayer::new`
    /// was fixed. `ManagedClient` routes every user-supplied store and every OpenDAL
    /// backend through `with_store`, which takes no config at all — so a configured
    /// TTL was still discarded there in favour of the 300s policy default.
    #[tokio::test]
    async fn configured_ttl_expires_the_entry_on_the_custom_store_path() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_millis(60),
            backend: CacheBackend::default(),
        };
        let store: Arc<dyn CacheStore> = Arc::new(InMemoryStore::new(&config));
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let mut svc = CacheLayer::with_store_and_config(store, &config).layer(LlmService::new(client));

        svc.call(LlmRequest::Chat(chat_req("ttl-store-model"))).await.unwrap();
        svc.call(LlmRequest::Chat(chat_req("ttl-store-model"))).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "second call within the TTL must be served from cache"
        );

        tokio::time::sleep(Duration::from_millis(150)).await;

        svc.call(LlmRequest::Chat(chat_req("ttl-store-model"))).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "entry must expire after the CONFIGURED 60ms, not the 300s policy default"
        );
    }

    #[tokio::test]
    async fn three_tier_full_miss_calls_upstream() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let layer = CacheLayer::new(config);
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("new-model"))).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1, "full miss must call upstream");
    }

    #[tokio::test]
    async fn warm_does_not_call_inner_service() {
        use crate::cache_key::CacheKeyInput;

        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let layer = CacheLayer::new(config);
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let svc = layer.layer(inner);

        let inputs: Vec<CacheKeyInput<'_>> = vec![
            CacheKeyInput {
                model: "gpt-4",
                messages_json: r#"[{"role":"user","content":"hi"}]"#,
                params_json: "{}",
                tenant_id: None,
                system_prompt: None,
            },
            CacheKeyInput {
                model: "gpt-4o",
                messages_json: r#"[{"role":"user","content":"hi"}]"#,
                params_json: "{}",
                tenant_id: None,
                system_prompt: None,
            },
        ];

        svc.warm(inputs.into_iter()).await;

        assert_eq!(call_count.load(Ordering::SeqCst), 0, "warm must not call inner service");
    }

    /// `warm`'s doc contract says it "allocates cache slots without making any
    /// upstream calls" — before the fix, the implementation was a pure no-op
    /// (`if store.get(...).is_none() { let _ = (key, body); }`), so this test
    /// used to pass trivially regardless of whether any slot was actually
    /// allocated: `call_count == 0` holds whether or not `warm` does anything
    /// at all. This test constrains the OTHER half of the doc contract — that
    /// a slot is actually occupied — via `CacheStore::metadata`, which reports
    /// a key as present regardless of whether its content has since expired.
    #[tokio::test]
    async fn warm_allocates_a_cache_slot_for_each_key() {
        use crate::cache_key::CacheKeyInput;

        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let store = Arc::new(InMemoryStore::new(&config));
        let layer = CacheLayer::with_store(Arc::clone(&store) as Arc<dyn CacheStore>);
        let client = MockClient::ok();
        let inner = LlmService::new(client);
        let svc = layer.layer(inner);

        let input = CacheKeyInput {
            model: "gpt-4",
            messages_json: r#"[{"role":"user","content":"hi"}]"#,
            params_json: "{}",
            tenant_id: None,
            system_prompt: None,
        };
        let (key, _body) = ExactHashStrategy.key_for(&input);

        svc.warm(std::iter::once(input)).await;

        assert!(
            store.metadata(key).await.is_some(),
            "warm must allocate a cache slot for each probed key, per its own doc contract — \
             this fails if warm() reverts to discarding (key, body) instead of writing a placeholder"
        );
    }

    #[tokio::test]
    async fn cache_bypassed_when_policy_returns_bypass() {
        use crate::tower::cache_policy::{CacheDecision, CachePolicy, CachePolicyContext};

        struct AlwaysBypassPolicy;
        impl CachePolicy for AlwaysBypassPolicy {
            fn decide(&self, _ctx: &CachePolicyContext<'_>) -> CacheDecision {
                CacheDecision {
                    bypass: true,
                    ..Default::default()
                }
            }
        }

        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let layer = CacheLayer::new(config).with_policy(Arc::new(AlwaysBypassPolicy));
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.unwrap();
        svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "bypassed calls must all hit upstream"
        );
    }

    /// A policy-bypassed request must record `CacheState::Bypass`, not
    /// `CacheState::Miss`. Before the fix, `decision.bypass` made
    /// `key_and_body` `None`, so the exact/semantic tier checks (both gated on
    /// `key_and_body.is_some()`) were skipped entirely and control fell
    /// straight through to an unconditional `record_cache_state(CacheState::Miss)`
    /// — misreporting every bypassed request as a genuine cache miss.
    ///
    /// Revert target: replacing `record_cache_state(if decision.bypass {
    /// CacheState::Bypass } else { CacheState::Miss })` with the old
    /// unconditional `record_cache_state(CacheState::Miss)` makes this fail —
    /// `state` would read back `Miss`.
    #[tokio::test]
    async fn bypassed_request_records_bypass_not_miss() {
        use std::cell::Cell;

        use crate::tower::cache_policy::{CacheDecision, CachePolicy, CachePolicyContext};

        struct AlwaysBypassPolicy;
        impl CachePolicy for AlwaysBypassPolicy {
            fn decide(&self, _ctx: &CachePolicyContext<'_>) -> CacheDecision {
                CacheDecision {
                    bypass: true,
                    ..Default::default()
                }
            }
        }

        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let layer = CacheLayer::new(config).with_policy(Arc::new(AlwaysBypassPolicy));
        let client = MockClient::ok();
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        // ~keep Same task-local scoping pattern HooksService uses: read the cell
        // ~keep before leaving the scope, or the recorded state does not survive.
        let (result, state) = CACHE_STATE_CELL
            .scope(Cell::new(CacheState::Miss), async {
                let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
                let state = CACHE_STATE_CELL.with(|c| c.get());
                (result, state)
            })
            .await;

        result.expect("bypassed call should still succeed");
        assert_eq!(
            state,
            CacheState::Bypass,
            "a policy-bypassed request must record CacheState::Bypass, not Miss"
        );
    }

    /// Verify that `CacheService::call` uses the `mem::replace` Tower swap
    /// (`let standby = self.inner.clone(); let mut inner = mem::replace(&mut
    /// self.inner, standby); let fut = inner.call(req);`) rather than calling
    /// `self.inner.call(req)` directly.
    ///
    /// The previous version of this test used an inner `poll_ready` that
    /// always returned `Ready` and two calls with different models, so it
    /// passed identically whether or not the swap existed — deleting the swap
    /// did not fail it. This version distinguishes the two implementations
    /// directly: each `Clone` of `IdentityService` gets a fresh, unique `id`
    /// (assigned from a shared counter), and `call()` records which `id`
    /// handled the request. With the swap in place, `self.inner`'s `id`
    /// changes after every `call()` (a fresh standby was swapped in) while the
    /// id recorded by `call()` matches the id `self.inner` held *before* the
    /// swap. Deleting the swap makes `self.inner`'s `id` stay constant across
    /// calls, failing the `assert_ne!` below.
    #[tokio::test]
    async fn cache_call_swaps_a_fresh_standby_into_inner() {
        use std::sync::Mutex;
        use std::sync::atomic::AtomicUsize;
        use std::task::Poll;

        struct IdentityService {
            id: usize,
            next_id: Arc<AtomicUsize>,
            call_ids: Arc<Mutex<Vec<usize>>>,
        }

        impl Clone for IdentityService {
            fn clone(&self) -> Self {
                Self {
                    id: self.next_id.fetch_add(1, Ordering::SeqCst),
                    next_id: Arc::clone(&self.next_id),
                    call_ids: Arc::clone(&self.call_ids),
                }
            }
        }

        impl Service<LlmRequest> for IdentityService {
            type Response = LlmResponse;
            type Error = LiterLlmError;
            type Future = crate::client::BoxFuture<'static, crate::error::Result<LlmResponse>>;

            fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<crate::error::Result<()>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, _req: LlmRequest) -> Self::Future {
                self.call_ids.lock().unwrap().push(self.id);
                Box::pin(async {
                    Ok(LlmResponse::Chat(crate::tower::tests_common::make_chat_response(
                        "gpt-4",
                    )))
                })
            }
        }

        let next_id = Arc::new(AtomicUsize::new(1));
        let call_ids = Arc::new(Mutex::new(Vec::new()));
        let inner = IdentityService {
            id: 0,
            next_id: Arc::clone(&next_id),
            call_ids: Arc::clone(&call_ids),
        };

        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let mut svc = CacheLayer::new(config).layer(inner);

        let id_before_call = svc.inner.id;
        svc.call(LlmRequest::Chat(chat_req("gpt-4-v1"))).await.unwrap();
        let id_after_call = svc.inner.id;

        assert_eq!(
            *call_ids.lock().unwrap(),
            vec![id_before_call],
            "call() must run on the instance whose readiness was current before the swap"
        );
        assert_ne!(
            id_after_call, id_before_call,
            "self.inner must be left holding a freshly cloned standby after call(), not the \
             consumed instance — this fails if the mem::replace swap in CacheService::call is removed"
        );

        let id_before_second_call = svc.inner.id;
        assert_eq!(
            id_before_second_call, id_after_call,
            "sanity check: no swap happens between calls, only during one"
        );
        svc.call(LlmRequest::Chat(chat_req("gpt-4-v2"))).await.unwrap();
        assert_eq!(
            *call_ids.lock().unwrap(),
            vec![id_before_call, id_before_second_call],
            "second call() must run on the standby swapped in after the first call, not a \
             further, un-polled clone of it"
        );
        assert_ne!(
            svc.inner.id, id_before_second_call,
            "self.inner must again be left holding a fresh standby after the second call"
        );
    }

    /// The previous version of this test looped 1000 times with zero
    /// assertions — it could not fail regardless of what the code did.
    ///
    /// The claim it was meant to guard ("`CacheService::call` reuses a single
    /// static empty metadata map instead of allocating one per call") is not
    /// safely testable from ordinary async test code: comparing the address of
    /// a per-call local `HashMap` versus a `'static` one is unreliable, since
    /// an empty `HashMap::new()` never allocates on the heap and its stack
    /// slot is likely to be reused at the same address on every loop
    /// iteration regardless of whether it is actually re-created — a false
    /// pass waiting to happen. Proving "no allocation" rigorously would need a
    /// custom `#[global_allocator]`, which is process-global and cannot be
    /// safely scoped to one test in a shared test binary.
    ///
    /// This test instead pins the actual, safely observable contract of
    /// passing `CachePolicyContext` through many repeated calls: identical
    /// requests must consistently resolve to a cache hit under sustained
    /// load. A regression that made the per-call policy metadata unstable
    /// (e.g. a bypass flag that flips unpredictably) would surface here as
    /// extra upstream calls.
    #[tokio::test]
    async fn cache_policy_meta_stable_across_many_repeated_calls() {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::from_secs(60),
            backend: CacheBackend::default(),
        };
        let layer = CacheLayer::new(config);
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let inner = LlmService::new(client);
        let mut svc = layer.layer(inner);

        for _ in 0..1000 {
            svc.call(LlmRequest::Chat(chat_req("gpt-4")))
                .await
                .expect("cached call should not fail");
        }

        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "1000 identical calls must all be served from the cache after the first upstream call"
        );
    }

    /// Bug fix regression: `strategy_key` must fold `tools`, `response_format`,
    /// and `seed` into the cache key. Before the fix, `params_json` only
    /// covered `temperature`/`top_p`/`max_tokens`/`n`/`stop`, so two requests
    /// that differ only in tools, response format, or seed produced the SAME
    /// cache key and one caller could receive another's response — the
    /// wrong-tools case is a correctness *and* security issue (a response
    /// generated with a different tool surface, served to a caller who
    /// expected its own tools to be honoured).
    #[tokio::test]
    async fn cache_key_differs_when_tools_response_format_or_seed_differ() {
        use crate::types::{ChatCompletionTool, FunctionDefinition, ResponseFormat, ToolType};

        let strategy = ExactHashStrategy;

        let base = chat_req("gpt-4");

        let mut with_tools = base.clone();
        with_tools.tools = Some(vec![ChatCompletionTool {
            tool_type: ToolType::Function,
            function: FunctionDefinition {
                name: "get_weather".into(),
                description: None,
                parameters: None,
                strict: None,
            },
        }]);

        let mut with_response_format = base.clone();
        with_response_format.response_format = Some(ResponseFormat::JsonObject);

        let mut with_seed = base.clone();
        with_seed.seed = Some(42);

        let base_key = strategy_key(&strategy, &LlmRequest::Chat(base)).expect("base request is cacheable");
        let tools_key = strategy_key(&strategy, &LlmRequest::Chat(with_tools)).expect("tools request is cacheable");
        let format_key = strategy_key(&strategy, &LlmRequest::Chat(with_response_format))
            .expect("response_format request is cacheable");
        let seed_key = strategy_key(&strategy, &LlmRequest::Chat(with_seed)).expect("seed request is cacheable");

        assert_ne!(base_key.0, tools_key.0, "adding `tools` must change the cache key");
        assert_ne!(
            base_key.0, format_key.0,
            "changing `response_format` must change the cache key"
        );
        assert_ne!(base_key.0, seed_key.0, "changing `seed` must change the cache key");
    }

    /// `extra_body` can change an embedding (for example `input_type`), so two
    /// requests that differ only there must not share a cache entry.
    #[test]
    fn embed_cache_key_changes_with_extra_body() {
        use crate::types::{EmbeddingInput, EmbeddingRequest};

        let embed = |extra_body: Option<serde_json::Value>| {
            LlmRequest::Embed(EmbeddingRequest {
                model: "bedrock/cohere.embed-english-v3".into(),
                input: EmbeddingInput::Single("what is rust?".into()),
                extra_body,
                ..Default::default()
            })
        };
        let strategy = ExactHashStrategy;

        let none = strategy_key(&strategy, &embed(None)).expect("embed request is cacheable");
        let none_again = strategy_key(&strategy, &embed(None)).expect("embed request is cacheable");
        let query = strategy_key(
            &strategy,
            &embed(Some(serde_json::json!({"input_type": "search_query"}))),
        )
        .expect("embed request is cacheable");
        let document = strategy_key(
            &strategy,
            &embed(Some(serde_json::json!({"input_type": "search_document"}))),
        )
        .expect("embed request is cacheable");

        assert_eq!(none.0, none_again.0, "equal requests share a key");
        assert_ne!(none.0, query.0, "setting `extra_body` must change the cache key");
        assert_ne!(
            query.0, document.0,
            "a different `input_type` must change the cache key"
        );
    }
}
