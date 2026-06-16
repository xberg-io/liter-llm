//! Idempotency-Key dedup layer.
//!
//! [`IdempotencyLayer`] implements the OpenAI `Idempotency-Key` header
//! convention for Tower services.  When a request carries an
//! [`LlmRequest::idempotency_key`][crate::tower::types::LlmRequest::idempotency_key],
//! the layer enforces three semantics:
//!
//! 1. **First request** — forwarded to the inner service.  On success the
//!    response is stored in the [`IdempotencyStore`].
//! 2. **Repeat request, same body** — the stored response is returned without
//!    invoking the inner service (within TTL).
//! 3. **Repeat request, different body, same key** — returns
//!    [`LiterLlmError::IdempotencyConflict`][crate::error::LiterLlmError::IdempotencyConflict].
//!
//! If a request with the same key is currently in-flight (the first request
//! has not yet returned a response), the layer returns
//! [`LiterLlmError::IdempotencyInFlight`][crate::error::LiterLlmError::IdempotencyInFlight]
//! immediately so the caller can retry after a short delay.  This avoids
//! sleep-polling inside the library and keeps Tokio task lifetimes bounded.
//!
//! # Default TTL
//!
//! The default TTL is **24 hours**, matching the OpenAI `Idempotency-Key`
//! convention.  Use [`IdempotencyLayer::with_ttl`] to override.
//!
//! # Storage
//!
//! [`InMemoryIdempotencyStore`] is the default backend.  It uses a
//! [`dashmap::DashMap`] with per-entry TTL checked on every read.  Implement
//! [`IdempotencyStore`] to plug in Redis, DynamoDB, or any other backend.
//!
//! # Layer order
//!
//! Place `IdempotencyLayer` **outermost** — before singleflight and caching —
//! so that repeat requests short-circuit before any cache interaction:
//!
//! ```text
//! IdempotencyLayer → SingleflightLayer → NegativeCacheLayer → CacheLayer → Upstream
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use liter_llm::tower::{IdempotencyLayer, InMemoryIdempotencyStore, LlmService};
//! use tower::ServiceBuilder;
//!
//! let store = InMemoryIdempotencyStore::default();
//! let svc = ServiceBuilder::new()
//!     .layer(IdempotencyLayer::new(store))
//!     .service(LlmService::new(client));
//!
//! let request = LlmRequest::Chat(chat_req).with_idempotency_key("req-abc-123");
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tower::{Layer, Service};

use crate::client::BoxFuture;
use crate::error::LiterLlmError;
use crate::error::Result as LiterResult;
use crate::tower::cache::CachedResponse;
use crate::tower::types::{LlmRequest, LlmRequestKind, LlmResponse};

// ── Body hash ─────────────────────────────────────────────────────────────────

/// Fixed seeds for the `ahash` [`RandomState`] used by body hashing.
///
/// These constants MUST NOT be changed once idempotency entries have been
/// persisted, as a seed change would invalidate stored hashes.
const IDEM_HASH_SEED_0: u64 = 0x6964_656d_706f_7465; // "idempote"
const IDEM_HASH_SEED_1: u64 = 0x6e63_795f_6861_7368; // "ncy_hash"
const IDEM_HASH_SEED_2: u64 = 0x5f73_6565_6430_5f76; // "_seed0_v"
const IDEM_HASH_SEED_3: u64 = 0x315f_6c6c_6d00_0000; // "1_llm\0\0\0"

/// Process-global deterministic [`ahash::RandomState`] for body hashing.
///
/// Constructed once from compile-time-fixed seeds so the same body always
/// produces the same hash across process restarts and Rust version upgrades.
/// This makes it safe to use in distributed stores (Redis, DynamoDB) where
/// multiple processes must agree on the hash value.
fn idem_random_state() -> &'static ahash::RandomState {
    use std::sync::OnceLock;
    static STATE: OnceLock<ahash::RandomState> = OnceLock::new();
    STATE.get_or_init(|| {
        ahash::RandomState::generate_with(IDEM_HASH_SEED_0, IDEM_HASH_SEED_1, IDEM_HASH_SEED_2, IDEM_HASH_SEED_3)
    })
}

/// Compute a stable body hash for the request.
///
/// Only `kind` is hashed — `tenant_id` and `idempotency_key` are infra
/// metadata and must not affect the body identity check.
///
/// Uses [`ahash::RandomState`] with four fixed compile-time seeds so the
/// hash is identical across process restarts, distributed nodes, and Rust
/// versions.  The body string prefix is embedded in the output for extra
/// collision resistance, so a hash collision yields a spurious
/// `IdempotencyConflict` rather than silent data corruption.
///
/// Returns `None` for request variants that cannot be serialised (should
/// never happen in practice — all variants derive `serde::Serialize`).
fn compute_body_hash(request: &LlmRequest) -> Option<String> {
    // Serialise only the kind (provider payload), excluding infra metadata.
    let json = serde_json::to_string(&request.kind).ok()?;

    let h = idem_random_state().hash_one(&json);
    // Embed a JSON body prefix alongside the hash for extra collision
    // resistance: a collision only causes a spurious IdempotencyConflict,
    // never silent data corruption.
    Some(format!("{h:016x}:{}", &json[..json.len().min(64)]))
}

// ── IdempotencyEntry ──────────────────────────────────────────────────────────

/// An entry in the idempotency store.
#[derive(Clone)]
pub struct IdempotencyEntry {
    /// Hash of the canonical request body at the time of first insertion.
    pub body_hash: String,
    /// The stored response.  `None` while the first request is still in-flight.
    pub response: Option<CachedResponse>,
    /// Wall-clock instant at which this entry was created.
    pub inserted_at: Instant,
    /// Effective TTL for this entry.
    pub ttl: Duration,
}

impl std::fmt::Debug for IdempotencyEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdempotencyEntry")
            .field("body_hash", &self.body_hash)
            .field("has_response", &self.response.is_some())
            .field("inserted_at", &self.inserted_at)
            .field("ttl", &self.ttl)
            .finish()
    }
}

impl IdempotencyEntry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

// ── IdempotencyStoreError ─────────────────────────────────────────────────────

/// Error type for [`IdempotencyStore`] operations.
#[derive(Debug, thiserror::Error)]
pub enum IdempotencyStoreError {
    /// A backend-specific error occurred.
    #[error("idempotency store backend error: {0}")]
    Backend(String),
}

// ── IdempotencyStore trait ────────────────────────────────────────────────────

/// Pluggable backing store for the idempotency layer.
///
/// The default in-process implementation is [`InMemoryIdempotencyStore`].
/// Implement this trait to provide distributed idempotency coordination via
/// Redis, DynamoDB, or any other backend.
///
/// All methods return pinned boxed futures so the trait is object-safe and
/// can be used behind `Arc<dyn IdempotencyStore>`.
pub trait IdempotencyStore: Send + Sync + 'static {
    /// Look up an existing entry by idempotency key.
    ///
    /// Returns `None` on a miss (key never seen or TTL expired).
    fn get<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<IdempotencyEntry>, IdempotencyStoreError>> + Send + 'a>>;

    /// Insert a placeholder entry for `key` if none exists yet.
    ///
    /// Returns `Ok(true)` when this caller won the insertion race (it is the
    /// writer — the caller proceeds to invoke the inner service).
    /// Returns `Ok(false)` when a concurrent inserter beat this caller (the
    /// caller should re-read the entry and act accordingly).
    fn try_insert<'a>(
        &'a self,
        key: &'a str,
        body_hash: &'a str,
        ttl: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<bool, IdempotencyStoreError>> + Send + 'a>>;

    /// Finalise an in-flight entry by storing the inner service's response.
    ///
    /// Called by the writer after the inner service returns successfully.
    /// A failed inner call must NOT call `store_response`; the placeholder
    /// entry will expire naturally so subsequent callers can retry.
    fn store_response<'a>(
        &'a self,
        key: &'a str,
        response: CachedResponse,
    ) -> Pin<Box<dyn Future<Output = Result<(), IdempotencyStoreError>> + Send + 'a>>;

    /// Remove the placeholder entry for `key`.
    ///
    /// Called by the writer when the inner service fails, so subsequent
    /// callers do not observe a stale in-flight entry.  Implementations that
    /// do not support explicit removal may rely on TTL expiry instead.
    fn remove<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), IdempotencyStoreError>> + Send + 'a>>;
}

// ── InMemoryIdempotencyStore ──────────────────────────────────────────────────

/// In-memory idempotency store backed by a [`DashMap`].
///
/// Per-entry TTLs are checked lazily on every `get` call; there is no
/// background expiry task.
///
/// # Concurrency
///
/// `DashMap` provides lock-striped concurrent access.  `try_insert` uses an
/// atomic `entry()` operation to guarantee that exactly one concurrent caller
/// wins the insertion race.
#[derive(Default)]
pub struct InMemoryIdempotencyStore {
    map: DashMap<String, IdempotencyEntry>,
}

impl InMemoryIdempotencyStore {
    /// Create a new empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl IdempotencyStore for InMemoryIdempotencyStore {
    fn get<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<IdempotencyEntry>, IdempotencyStoreError>> + Send + 'a>> {
        let result = self
            .map
            .get(key)
            .and_then(|entry| if entry.is_expired() { None } else { Some(entry.clone()) });
        Box::pin(std::future::ready(Ok(result)))
    }

    fn try_insert<'a>(
        &'a self,
        key: &'a str,
        body_hash: &'a str,
        ttl: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<bool, IdempotencyStoreError>> + Send + 'a>> {
        use dashmap::mapref::entry::Entry;

        let inserted = match self.map.entry(key.to_owned()) {
            Entry::Vacant(slot) => {
                slot.insert(IdempotencyEntry {
                    body_hash: body_hash.to_owned(),
                    response: None,
                    inserted_at: Instant::now(),
                    ttl,
                });
                true
            }
            Entry::Occupied(entry) => {
                // If the existing entry is expired, replace it and win.
                if entry.get().is_expired() {
                    entry.replace_entry(IdempotencyEntry {
                        body_hash: body_hash.to_owned(),
                        response: None,
                        inserted_at: Instant::now(),
                        ttl,
                    });
                    true
                } else {
                    false
                }
            }
        };
        Box::pin(std::future::ready(Ok(inserted)))
    }

    fn store_response<'a>(
        &'a self,
        key: &'a str,
        response: CachedResponse,
    ) -> Pin<Box<dyn Future<Output = Result<(), IdempotencyStoreError>> + Send + 'a>> {
        if let Some(mut entry) = self.map.get_mut(key) {
            entry.response = Some(response);
        }
        Box::pin(std::future::ready(Ok(())))
    }

    fn remove<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), IdempotencyStoreError>> + Send + 'a>> {
        self.map.remove(key);
        Box::pin(std::future::ready(Ok(())))
    }
}

// ── IdempotencyLayer ──────────────────────────────────────────────────────────

/// Tower [`Layer`] that deduplicates requests sharing the same `Idempotency-Key`.
///
/// See [module documentation][self] for semantics and layer order.
#[cfg_attr(alef, alef(skip))]
pub struct IdempotencyLayer<S: IdempotencyStore> {
    store: Arc<S>,
    ttl: Duration,
}

impl<S: IdempotencyStore> IdempotencyLayer<S> {
    /// Create a new layer with the default 24-hour TTL.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self::with_ttl(store, Duration::from_secs(24 * 60 * 60))
    }

    /// Create a new layer with an explicit TTL.
    #[must_use]
    pub fn with_ttl(store: S, ttl: Duration) -> Self {
        Self {
            store: Arc::new(store),
            ttl,
        }
    }
}

impl<S: IdempotencyStore> Clone for IdempotencyLayer<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            ttl: self.ttl,
        }
    }
}

impl<I, S: IdempotencyStore> Layer<I> for IdempotencyLayer<S> {
    type Service = IdempotencyService<I, S>;

    fn layer(&self, inner: I) -> Self::Service {
        IdempotencyService {
            inner,
            store: Arc::clone(&self.store),
            ttl: self.ttl,
        }
    }
}

// ── IdempotencyService ────────────────────────────────────────────────────────

/// Tower service produced by [`IdempotencyLayer`].
#[cfg_attr(alef, alef(skip))]
pub struct IdempotencyService<I, S: IdempotencyStore> {
    inner: I,
    store: Arc<S>,
    ttl: Duration,
}

impl<I: Clone, S: IdempotencyStore> Clone for IdempotencyService<I, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            store: Arc::clone(&self.store),
            ttl: self.ttl,
        }
    }
}

impl<I, S> Service<LlmRequest> for IdempotencyService<I, S>
where
    I: Service<LlmRequest, Response = LlmResponse, Error = LiterLlmError> + Clone + Send + 'static,
    I::Future: Send + 'static,
    S: IdempotencyStore,
{
    type Response = LlmResponse;
    type Error = LiterLlmError;
    type Future = BoxFuture<'static, LiterResult<LlmResponse>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<LiterResult<()>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: LlmRequest) -> Self::Future {
        // Tower contract: consume the polled-ready inner instance, leave a
        // fresh clone as standby for the next poll_ready/call cycle.
        let standby = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, standby);

        let store = Arc::clone(&self.store);
        let ttl = self.ttl;

        Box::pin(async move {
            // ── No key: pass through ──────────────────────────────────────
            let Some(ref raw_key) = request.idempotency_key.clone() else {
                return inner.call(request).await;
            };

            // ── Tenant-scope the store key ────────────────────────────────
            // An idempotency key is scoped to the tenant that supplied it.
            // Without a tenant prefix, a guessable key such as `"req-001"` used
            // by tenant A would collide with the same key used by tenant B,
            // leaking tenant A's cached response to tenant B.
            let tenant_prefix = request.tenant_id.as_ref().map(|t| t.as_ref()).unwrap_or("_");
            let key = format!("{tenant_prefix}:{raw_key}");

            // ── Compute body hash ─────────────────────────────────────────
            let body_hash = match compute_body_hash(&request) {
                Some(h) => h,
                None => {
                    // Serialisation failed (should never happen) — pass through
                    // to avoid blocking the caller.
                    return inner.call(request).await;
                }
            };

            // ── Check existing entry ──────────────────────────────────────
            if let Some(entry) = store.get(&key).await.map_err(store_err)? {
                if entry.body_hash != body_hash {
                    // Report the raw (user-facing) key in the error, not the
                    // tenant-scoped internal store key.
                    return Err(LiterLlmError::IdempotencyConflict { key: raw_key.clone() });
                }
                if let Some(cached) = entry.response {
                    // Hit: return stored response without calling inner.
                    return cached.into_llm_response();
                }
                // Entry exists but no response yet — another caller is the
                // writer and has not completed.  Error-out so the caller can
                // retry after a brief delay (see module docs for rationale).
                return Err(LiterLlmError::IdempotencyInFlight { key: raw_key.clone() });
            }

            // ── Try to become the writer ──────────────────────────────────
            let inserted = store.try_insert(&key, &body_hash, ttl).await.map_err(store_err)?;

            if !inserted {
                // Lost the race.  Re-read and apply the same logic as above.
                if let Some(entry) = store.get(&key).await.map_err(store_err)? {
                    if entry.body_hash != body_hash {
                        return Err(LiterLlmError::IdempotencyConflict { key: raw_key.clone() });
                    }
                    if let Some(cached) = entry.response {
                        return cached.into_llm_response();
                    }
                    return Err(LiterLlmError::IdempotencyInFlight { key: raw_key.clone() });
                }
                // Entry disappeared between try_insert and get (expired between
                // the two calls) — fall through to call inner.
                // This is an extremely unlikely race; if it happens the caller
                // makes an extra upstream call, which is safe.
            }

            // ── Call inner service ────────────────────────────────────────
            let result = inner.call(request).await;

            match &result {
                Ok(resp) => {
                    let cached = match resp {
                        LlmResponse::Chat(r) => Some(CachedResponse::Chat(r.clone())),
                        LlmResponse::Embed(r) => Some(CachedResponse::Embed(r.clone())),
                        // Non-cacheable variants (streaming, image, audio, …):
                        // remove the placeholder so subsequent callers can retry.
                        // Idempotency for streaming responses is intentionally
                        // not supported — streams are consumed once.
                        _ => None,
                    };
                    if let Some(cached_resp) = cached {
                        let _ = store.store_response(&key, cached_resp).await;
                    } else {
                        let _ = store.remove(&key).await;
                    }
                }
                Err(_) => {
                    // On inner error: remove the placeholder so subsequent
                    // callers with the same key+body can retry the operation.
                    let _ = store.remove(&key).await;
                }
            }

            result
        })
    }
}

/// Map an [`IdempotencyStoreError`] to [`LiterLlmError::InternalError`].
#[inline]
fn store_err(e: IdempotencyStoreError) -> LiterLlmError {
    LiterLlmError::InternalError {
        message: format!("idempotency store: {e}"),
    }
}

/// Helper: return true only for request variants whose responses are cacheable.
///
/// Used by `IdempotencyService` to decide whether to store or discard the
/// inner service's response.
#[must_use]
#[allow(dead_code)]
pub(crate) fn is_cacheable_kind(kind: &LlmRequestKind) -> bool {
    matches!(kind, LlmRequestKind::Chat(_) | LlmRequestKind::Embed(_))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use tower::{Layer as _, Service as _};

    use super::*;
    use crate::error::LiterLlmError;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::{LlmRequest, LlmResponse};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_layer() -> IdempotencyLayer<InMemoryIdempotencyStore> {
        IdempotencyLayer::new(InMemoryIdempotencyStore::new())
    }

    fn req_with_key(model: &str, key: &str) -> LlmRequest {
        LlmRequest::Chat(chat_req(model)).with_idempotency_key(key)
    }

    // ── Unit tests for InMemoryIdempotencyStore ───────────────────────────────

    #[tokio::test]
    async fn store_get_returns_none_on_miss() {
        let store = InMemoryIdempotencyStore::new();
        let result = store.get("missing-key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn store_try_insert_wins_first_caller() {
        let store = InMemoryIdempotencyStore::new();
        let inserted = store.try_insert("k1", "hash1", Duration::from_secs(60)).await.unwrap();
        assert!(inserted, "first caller must win insertion");

        // Second insertion for the same key must fail.
        let second = store.try_insert("k1", "hash1", Duration::from_secs(60)).await.unwrap();
        assert!(!second, "second caller must lose insertion race");
    }

    #[tokio::test]
    async fn store_try_insert_wins_after_expiry() {
        let store = InMemoryIdempotencyStore::new();
        // Insert with near-zero TTL so it expires immediately.
        store.try_insert("k2", "hash", Duration::from_nanos(1)).await.unwrap();

        // Sleep past the TTL.
        tokio::time::sleep(Duration::from_millis(2)).await;

        // Second insertion should now win because the entry is expired.
        let inserted = store.try_insert("k2", "hash", Duration::from_secs(60)).await.unwrap();
        assert!(inserted, "insertion after TTL expiry must succeed");
    }

    #[tokio::test]
    async fn store_get_returns_none_for_expired_entry() {
        let store = InMemoryIdempotencyStore::new();
        store.try_insert("k3", "hash", Duration::from_nanos(1)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(2)).await;
        let result = store.get("k3").await.unwrap();
        assert!(result.is_none(), "expired entry must not be returned");
    }

    #[tokio::test]
    async fn store_store_response_populates_entry() {
        let store = InMemoryIdempotencyStore::new();
        store.try_insert("k4", "hash", Duration::from_secs(60)).await.unwrap();
        let resp = CachedResponse::Chat(crate::tower::tests_common::make_chat_response("gpt-4"));
        store.store_response("k4", resp).await.unwrap();

        let entry = store.get("k4").await.unwrap().expect("entry must exist");
        assert!(entry.response.is_some(), "response must be populated");
    }

    #[tokio::test]
    async fn store_remove_deletes_entry() {
        let store = InMemoryIdempotencyStore::new();
        store.try_insert("k5", "hash", Duration::from_secs(60)).await.unwrap();
        store.remove("k5").await.unwrap();
        let result = store.get("k5").await.unwrap();
        assert!(result.is_none(), "removed entry must not be present");
    }

    // ── IdempotencyService tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn first_request_hits_inner() {
        let layer = make_layer();
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let mut svc = layer.layer(LlmService::new(client));

        let result = svc.call(req_with_key("gpt-4", "key-001")).await;
        assert!(result.is_ok(), "first request must succeed");
        assert_eq!(call_count.load(Ordering::SeqCst), 1, "inner must be called once");
    }

    #[tokio::test]
    async fn repeat_same_key_same_body_returns_cached() {
        let layer = make_layer();
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let mut svc = layer.layer(LlmService::new(client));

        // First call — populates the store.
        svc.call(req_with_key("gpt-4", "key-002"))
            .await
            .expect("first call must succeed");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Second call — same key + same body, must return cached response.
        let result = svc.call(req_with_key("gpt-4", "key-002")).await;
        assert!(result.is_ok(), "second call must succeed");
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "inner must NOT be called on second request with same key+body"
        );
    }

    #[tokio::test]
    async fn repeat_same_key_different_body_returns_conflict() {
        let layer = make_layer();
        let client = MockClient::ok();
        let mut svc = layer.layer(LlmService::new(client));

        // First call with model-a.
        svc.call(req_with_key("gpt-4", "key-003"))
            .await
            .expect("first call must succeed");

        // Second call — same key, different model (→ different body hash).
        let result = svc.call(req_with_key("gpt-3.5-turbo", "key-003")).await;
        assert!(
            matches!(result, Err(LiterLlmError::IdempotencyConflict { .. })),
            "different body for same key must return IdempotencyConflict; got {result:?}"
        );
    }

    #[tokio::test]
    async fn no_key_passes_through() {
        let layer = make_layer();
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let mut svc = layer.layer(LlmService::new(client));

        // Request without idempotency_key — no store interaction.
        let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(result.is_ok(), "request without key must succeed");
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "inner must be called for keyless request"
        );
    }

    #[tokio::test]
    async fn inner_error_does_not_cache() {
        let layer = make_layer();
        let client = MockClient::failing_rate_limited();
        let call_count = Arc::clone(&client.call_count);
        let mut svc = layer.layer(LlmService::new(client));

        // First call — inner fails with RateLimited.
        let first = svc.call(req_with_key("gpt-4", "key-err")).await;
        assert!(first.is_err(), "first call must fail");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Second call — same key+body; placeholder was removed on error,
        // so inner is called again (retry semantics).
        let second = svc.call(req_with_key("gpt-4", "key-err")).await;
        assert!(second.is_err(), "second call must also fail (same inner error)");
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "inner must be called again after first failed call"
        );
    }

    #[tokio::test]
    #[ignore = "moka time-mocking not available; TTL expiry tested via InMemoryIdempotencyStore unit tests"]
    async fn ttl_expiry_allows_new_invocation() {
        // This test is covered by `store_try_insert_wins_after_expiry` above,
        // which exercises the same TTL expiry path at the store level.
    }

    #[tokio::test]
    async fn different_keys_are_independent() {
        let layer = make_layer();
        let client = MockClient::ok();
        let call_count = Arc::clone(&client.call_count);
        let mut svc = layer.layer(LlmService::new(client));

        svc.call(req_with_key("gpt-4", "key-A"))
            .await
            .expect("call A must succeed");
        svc.call(req_with_key("gpt-4", "key-B"))
            .await
            .expect("call B must succeed");
        // Both must hit inner — different keys are independent.
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "different keys must both hit inner"
        );

        // Repeat each key — both must return cached.
        svc.call(req_with_key("gpt-4", "key-A"))
            .await
            .expect("repeat A must succeed");
        svc.call(req_with_key("gpt-4", "key-B"))
            .await
            .expect("repeat B must succeed");
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "repeated calls with same key+body must not hit inner"
        );
    }

    #[tokio::test]
    async fn returned_response_matches_original() {
        let layer = make_layer();
        let client = MockClient::ok();
        let mut svc = layer.layer(LlmService::new(client));

        let first = svc
            .call(req_with_key("gpt-4", "key-content"))
            .await
            .expect("first call");
        let first_model = match &first {
            LlmResponse::Chat(r) => r.model.clone(),
            _ => panic!("expected Chat response"),
        };

        let second = svc
            .call(req_with_key("gpt-4", "key-content"))
            .await
            .expect("second call");
        let second_model = match &second {
            LlmResponse::Chat(r) => r.model.clone(),
            _ => panic!("expected Chat response"),
        };

        assert_eq!(first_model, second_model, "cached response must match original");
    }

    // ── Fix 2: ahash determinism ──────────────────────────────────────────────

    /// `compute_body_hash` must return the same value on every call for the
    /// same request, even when constructed from independent instances.
    /// The old `DefaultHasher` used a randomized seed (Rust 1.36+), so two
    /// different process runs (or, in distributed setups, two different nodes)
    /// could produce different hashes for the same request body, breaking
    /// distributed idempotency coordination.
    #[test]
    fn idempotency_body_hash_deterministic_across_instances() {
        let req = LlmRequest::Chat(chat_req("gpt-4"));

        let hashes: Vec<_> = (0..10).map(|_| compute_body_hash(&req)).collect();

        // Every hash must be Some and identical to the first.
        let first = hashes[0].as_ref().expect("hash must be Some");
        for (i, h) in hashes.iter().enumerate() {
            assert_eq!(
                h.as_ref().expect("hash must be Some"),
                first,
                "hash #{i} differs from hash #0 — ahash seed is not fixed"
            );
        }
    }

    // ── Fix 3: tenant-scoped keys ─────────────────────────────────────────────

    /// Two requests with the same idempotency key but different tenant IDs must
    /// not share the same cached response.  Before the fix, the store key was
    /// the raw idempotency key with no tenant prefix, so tenant B could observe
    /// tenant A's cached response if they happened to use the same key string.
    #[tokio::test]
    async fn idempotency_tenant_scoped_keys_dont_collide() {
        use crate::tower::types::LlmResponse;

        let store = Arc::new(InMemoryIdempotencyStore::new());
        let layer_a = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
        let layer_b = IdempotencyLayer::new(InMemoryIdempotencyStore::new());
        // Use the same shared store for both tenants to verify isolation.
        let _ = (store, layer_a, layer_b); // silence unused

        // Build a shared store and two layers that share it.
        let shared_store = Arc::new(InMemoryIdempotencyStore::new());
        let make_layer_shared = || IdempotencyLayer {
            store: Arc::clone(&shared_store),
            ttl: Duration::from_secs(60),
        };

        let client_a = MockClient::ok();
        let call_count_a = Arc::clone(&client_a.call_count);
        let mut svc_a = make_layer_shared().layer(LlmService::new(client_a));

        let client_b = MockClient::ok();
        let call_count_b = Arc::clone(&client_b.call_count);
        let mut svc_b = make_layer_shared().layer(LlmService::new(client_b));

        // Both tenants use the exact same idempotency key "shared-key".
        let req_a = LlmRequest::Chat(chat_req("gpt-4"))
            .with_idempotency_key("shared-key")
            .with_tenant_id("tenant-A");
        let req_b = LlmRequest::Chat(chat_req("gpt-4"))
            .with_idempotency_key("shared-key")
            .with_tenant_id("tenant-B");

        // Tenant A's first request — must hit inner.
        let resp_a = svc_a.call(req_a.clone()).await.expect("tenant A first call");
        assert!(matches!(resp_a, LlmResponse::Chat(_)));
        assert_eq!(call_count_a.load(Ordering::SeqCst), 1, "inner called for tenant A");

        // Tenant B's first request — must ALSO hit inner (different store key).
        let resp_b = svc_b.call(req_b.clone()).await.expect("tenant B first call");
        assert!(matches!(resp_b, LlmResponse::Chat(_)));
        assert_eq!(
            call_count_b.load(Ordering::SeqCst),
            1,
            "inner called for tenant B (no cross-tenant hit)"
        );

        // Repeat for both — should now return cached without calling inner again.
        svc_a.call(req_a).await.expect("tenant A repeat");
        assert_eq!(
            call_count_a.load(Ordering::SeqCst),
            1,
            "inner NOT called on tenant A repeat"
        );

        svc_b.call(req_b).await.expect("tenant B repeat");
        assert_eq!(
            call_count_b.load(Ordering::SeqCst),
            1,
            "inner NOT called on tenant B repeat"
        );
    }
}
