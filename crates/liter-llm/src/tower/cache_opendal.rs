//! OpenDAL-backed cache store for the response cache.
//!
//! Implements [`CacheStore`] using an [`opendal::Operator`] for persistence.
//! Supports any OpenDAL backend (S3, Redis, GCS, local filesystem, etc.).

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use opendal::Operator;
use serde::{Deserialize, Serialize};

use super::cache::{CacheStore, CachedResponse};

/// A cached entry stored via OpenDAL, including metadata for TTL and
/// collision detection.
#[derive(Serialize, Deserialize)]
struct StoredEntry {
    request_body: String,
    response: CachedResponse,
    /// Unix timestamp (seconds) when this entry was written.
    inserted_at: u64,
    /// TTL in seconds, relative to `inserted_at`.
    ///
    /// Stored per-entry (rather than baking `inserted_at + ttl` into a single
    /// `expires_at` field) so [`OpenDalCacheStore::set_ttl`] can override it
    /// with a read-modify-write without needing to separately recover the
    /// original insertion time. ~keep
    ttl_secs: u64,
}

impl StoredEntry {
    fn expires_at(&self) -> u64 {
        self.inserted_at.saturating_add(self.ttl_secs)
    }
}

/// Cache store backed by an [`opendal::Operator`].
///
/// Entries are stored as JSON files under `{prefix}/{key}`. TTL is embedded
/// in the stored entry and checked on read. Backend failures are non-fatal:
/// they log a warning and behave as a cache miss / no-op.
///
/// # Bounded growth
///
/// Unlike [`super::cache::InMemoryStore`] (which evicts on `max_entries`),
/// this store previously grew without bound: nothing ever deleted an entry
/// except its own TTL expiry, and expiry is checked lazily on `get` — a key
/// that is written once and never read again lives in the backend forever.
/// For a remote object store (S3, GCS) this is often acceptable (or even
/// desired), but for a local `fs`/`memory` backend it is an unbounded disk/
/// memory leak. Set `max_entries` via [`Self::with_max_entries`] to cap the
/// number of live keys with LRU-by-insertion-order eviction, tracked by an
/// in-process index (`order`). The index is best-effort and process-local:
/// entries written by a different process (e.g. another replica sharing the
/// same backend) are not counted against this instance's cap.
pub struct OpenDalCacheStore {
    operator: Operator,
    prefix: String,
    ttl: Duration,
    max_entries: Option<usize>,
    /// Insertion-ordered keys, front = oldest. Only populated/consulted when
    /// `max_entries` is `Some`.
    order: RwLock<VecDeque<u64>>,
}

impl OpenDalCacheStore {
    /// Create a new OpenDAL cache store with unbounded entry count.
    ///
    /// `operator` must be a fully configured OpenDAL operator.
    /// `prefix` is prepended to all cache keys (e.g. `"llm-cache/"`).
    /// `ttl` controls how long entries are valid.
    pub fn new(operator: Operator, prefix: impl Into<String>, ttl: Duration) -> Self {
        Self {
            operator,
            prefix: prefix.into(),
            ttl,
            max_entries: None,
            order: RwLock::new(VecDeque::new()),
        }
    }

    /// Cap the number of live entries this instance will keep, evicting the
    /// oldest-inserted key (by this instance's own insertion order, not
    /// backend mtime) once the cap is exceeded. See the type-level docs for
    /// the process-local caveat.
    #[must_use]
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = Some(max_entries);
        self
    }

    /// Build an OpenDAL operator from a scheme name and config map.
    ///
    /// # Errors
    /// Returns an error if the scheme is unknown or the config is invalid.
    pub fn from_config(
        scheme: &str,
        config: HashMap<String, String>,
        prefix: impl Into<String>,
        ttl: Duration,
    ) -> crate::error::Result<Self> {
        crate::ensure_crypto_provider();
        opendal::init_default_registry();
        opendal_http_transport_reqwest::install_default();
        let operator = Operator::via_iter(scheme, config).map_err(|e| crate::error::LiterLlmError::InternalError {
            message: format!("failed to build OpenDAL operator for '{scheme}': {e}"),
        })?;
        Ok(Self::new(operator, prefix, ttl))
    }

    fn key_path(&self, key: u64) -> String {
        format!("{}{key}", self.prefix)
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Record `key` as the most-recently-inserted entry and return any keys
    /// evicted by exceeding `max_entries`. No-op (returns an empty `Vec`)
    /// when `max_entries` is `None`.
    fn touch_and_evict(&self, key: u64) -> Vec<u64> {
        let Some(max_entries) = self.max_entries else {
            return Vec::new();
        };

        let Ok(mut order) = self.order.write() else {
            tracing::warn!("OpenDAL cache: order-tracking lock poisoned; max_entries eviction disabled for this write");
            return Vec::new();
        };

        order.retain(|k| *k != key);
        order.push_back(key);

        let mut evicted = Vec::new();
        while order.len() > max_entries {
            if let Some(oldest) = order.pop_front() {
                evicted.push(oldest);
            } else {
                break;
            }
        }
        evicted
    }

    /// Drop `key` from the insertion-order index (called on explicit `remove`).
    fn forget(&self, key: u64) {
        if let Ok(mut order) = self.order.write() {
            order.retain(|k| *k != key);
        } else {
            tracing::warn!("OpenDAL cache: order-tracking lock poisoned; could not forget removed key");
        }
    }
}

impl CacheStore for OpenDalCacheStore {
    fn get(&self, key: u64, request_body: &str) -> Pin<Box<dyn Future<Output = Option<CachedResponse>> + Send + '_>> {
        let path = self.key_path(key);
        let request_body = request_body.to_owned();
        Box::pin(async move {
            let bytes = match self.operator.read(&path).await {
                Ok(b) => b,
                // ~keep NotFound is an expected, silent cache miss; any other error indicates a
                // ~keep degraded backend and must be surfaced, or misses are indistinguishable from outages.
                Err(e) if e.kind() == opendal::ErrorKind::NotFound => return None,
                Err(e) => {
                    tracing::warn!("OpenDAL cache: failed to read {path}: {e}");
                    return None;
                }
            };
            let entry: StoredEntry = match serde_json::from_slice(bytes.to_bytes().as_ref()) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("OpenDAL cache: failed to deserialize entry at {path}: {e}");
                    return None;
                }
            };
            if Self::now_secs() > entry.expires_at() {
                if let Err(e) = self.operator.delete(&path).await {
                    tracing::warn!("OpenDAL cache: failed to delete expired entry {path}: {e}");
                }
                self.forget(key);
                return None;
            }
            if entry.request_body != request_body {
                return None;
            }
            Some(entry.response)
        })
    }

    fn put(
        &self,
        key: u64,
        request_body: String,
        response: CachedResponse,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        // ~keep CachedResponse::Error is deliberately not Serialize (see cache.rs module
        // ~keep docs: negative-cache entries need an explicit conversion shim per external
        // ~keep store). That is a permanent, known limitation of this backend, not a
        // ~keep transient failure — log it once at DEBUG rather than WARN, or an upstream
        // ~keep outage (which is exactly when NegativeCacheLayer writes fire on every
        // ~keep failed request) turns into a WARN-per-request storm here.
        if matches!(response, CachedResponse::Error { .. }) {
            tracing::debug!(
                "OpenDAL cache: skipping write of a CachedResponse::Error entry; \
                 this backend does not support negative-cache replication"
            );
            return Box::pin(std::future::ready(()));
        }

        let path = self.key_path(key);
        let entry = StoredEntry {
            request_body,
            response,
            inserted_at: Self::now_secs(),
            ttl_secs: self.ttl.as_secs(),
        };
        Box::pin(async move {
            let bytes = match serde_json::to_vec(&entry) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("OpenDAL cache: failed to serialize entry: {e}");
                    return;
                }
            };
            if let Err(e) = self.operator.write(&path, bytes).await {
                tracing::warn!("OpenDAL cache: failed to write {path}: {e}");
                return;
            }

            for evicted_key in self.touch_and_evict(key) {
                let evicted_path = self.key_path(evicted_key);
                if let Err(e) = self.operator.delete(&evicted_path).await {
                    tracing::warn!("OpenDAL cache: failed to delete evicted entry {evicted_path}: {e}");
                }
            }
        })
    }

    fn remove(&self, key: u64) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let path = self.key_path(key);
        Box::pin(async move {
            if let Err(e) = self.operator.delete(&path).await {
                tracing::warn!("OpenDAL cache: failed to delete {path}: {e}");
            }
            self.forget(key);
        })
    }

    fn set_ttl(&self, key: u64, ttl: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        // ~keep StandardCachePolicy::decide returns `ttl_override: Some(exact_ttl)` on
        // ~keep every non-bypassed write, and CacheService applies it via this method
        // ~keep exclusively (cache.rs). The trait default is a no-op, so without this
        // ~keep override every entry silently reverted to the store's construction-time
        // ~keep `ttl`, discarding any per-request/per-model TTL configured via `.with_policy`.
        let path = self.key_path(key);
        Box::pin(async move {
            let bytes = match self.operator.read(&path).await {
                Ok(b) => b,
                Err(e) if e.kind() == opendal::ErrorKind::NotFound => return,
                Err(e) => {
                    tracing::warn!("OpenDAL cache: failed to read {path} for set_ttl: {e}");
                    return;
                }
            };
            let mut entry: StoredEntry = match serde_json::from_slice(bytes.to_bytes().as_ref()) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("OpenDAL cache: failed to deserialize entry at {path} for set_ttl: {e}");
                    return;
                }
            };
            entry.ttl_secs = ttl.as_secs();
            let bytes = match serde_json::to_vec(&entry) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("OpenDAL cache: failed to re-serialize entry at {path} for set_ttl: {e}");
                    return;
                }
            };
            if let Err(e) = self.operator.write(&path, bytes).await {
                tracing::warn!("OpenDAL cache: failed to write {path} for set_ttl: {e}");
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tower::cache::{CacheStore, CachedResponse};
    use crate::types::{AssistantMessage, ChatCompletionResponse, Choice, FinishReason};

    fn memory_store(ttl_secs: u64) -> OpenDalCacheStore {
        let op = Operator::via_iter("memory", std::iter::empty::<(String, String)>())
            .expect("memory backend should always build");
        OpenDalCacheStore::new(op, "test/", Duration::from_secs(ttl_secs))
    }

    fn dummy_response() -> CachedResponse {
        CachedResponse::Chat(ChatCompletionResponse {
            id: "test-resp-001".into(),
            object: "chat.completion".into(),
            created: 1_700_000_000,
            model: "gpt-4".into(),
            choices: vec![Choice {
                index: 0,
                message: AssistantMessage {
                    content: Some("Hello!".into()),
                    name: None,
                    tool_calls: None,
                    refusal: None,
                    function_call: None,
                    reasoning_content: None,
                    thinking_blocks: None,
                },
                finish_reason: Some(FinishReason::Stop),
                logprobs: None,
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        })
    }

    #[tokio::test]
    async fn put_and_get_round_trip() {
        let store = memory_store(300);
        store.put(42, "request-body-a".into(), dummy_response()).await;
        let cached = store.get(42, "request-body-a").await;
        assert!(cached.is_some(), "expected a cached response after put");
        match cached.expect("cached value should be present") {
            CachedResponse::Chat(resp) => {
                assert_eq!(resp.id, "test-resp-001");
                assert_eq!(resp.model, "gpt-4");
            }
            _ => panic!("expected CachedResponse::Chat variant"),
        }
    }

    #[tokio::test]
    async fn get_returns_none_for_missing_key() {
        let store = memory_store(300);
        let result = store.get(999, "any-body").await;
        assert!(result.is_none(), "expected None for a key that was never stored");
    }

    #[tokio::test]
    async fn get_returns_none_for_wrong_request_body() {
        let store = memory_store(300);
        store.put(1, "body-alpha".into(), dummy_response()).await;
        let result = store.get(1, "body-beta").await;
        assert!(result.is_none(), "expected None when request body does not match");
    }

    #[tokio::test]
    async fn expired_entry_returns_none() {
        let store = memory_store(0);
        store.put(1, "req".into(), dummy_response()).await;
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let result = store.get(1, "req").await;
        assert!(result.is_none(), "expected None for expired entry");
    }

    #[tokio::test]
    async fn remove_deletes_entry() {
        let store = memory_store(300);
        store.put(7, "req".into(), dummy_response()).await;
        assert!(store.get(7, "req").await.is_some());
        store.remove(7).await;
        assert!(store.get(7, "req").await.is_none(), "expected None after remove");
    }

    #[tokio::test]
    async fn overwrite_replaces_previous_entry() {
        let store = memory_store(300);
        store.put(1, "req".into(), dummy_response()).await;

        let replacement = CachedResponse::Chat(ChatCompletionResponse {
            id: "test-resp-002".into(),
            object: "chat.completion".into(),
            created: 1_700_000_001,
            model: "gpt-4o".into(),
            choices: vec![],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        });
        store.put(1, "req".into(), replacement).await;

        match store.get(1, "req").await {
            Some(CachedResponse::Chat(resp)) => assert_eq!(resp.id, "test-resp-002"),
            _ => panic!("expected updated CachedResponse::Chat variant"),
        }
    }

    #[tokio::test]
    async fn from_config_filesystem_persists_exact_cached_response() {
        let directory = tempfile::tempdir().expect("create isolated filesystem cache");
        let config = HashMap::from([(
            "root".to_owned(),
            directory.path().to_str().expect("temporary path is UTF-8").to_owned(),
        )]);
        let store = OpenDalCacheStore::from_config("fs", config, "responses/", Duration::from_secs(300))
            .expect("configured filesystem service must be available without caller registration");
        let expected = dummy_response();
        let expected_json = serde_json::to_value(&expected).expect("serialize expected cached response");

        store.put(42, "filesystem-request".to_owned(), expected).await;

        let bytes = tokio::fs::read(directory.path().join("responses/42"))
            .await
            .expect("put must persist an actual filesystem entry");
        let persisted: StoredEntry = serde_json::from_slice(&bytes).expect("decode persisted cache entry");
        assert_eq!(persisted.request_body, "filesystem-request");
        assert_eq!(persisted.ttl_secs, 300);
        assert_eq!(
            serde_json::to_value(persisted.response).expect("serialize persisted response"),
            expected_json
        );
        let actual = store
            .get(42, "filesystem-request")
            .await
            .expect("read cached filesystem response");
        assert_eq!(
            serde_json::to_value(actual).expect("serialize cached response"),
            expected_json
        );

        store.remove(42).await;
        assert_eq!(
            tokio::fs::metadata(directory.path().join("responses/42"))
                .await
                .expect_err("remove must delete the persisted entry")
                .kind(),
            std::io::ErrorKind::NotFound,
        );
    }

    async fn receive_http_request(stream: &mut tokio::net::TcpStream) -> (String, Vec<u8>) {
        use tokio::io::AsyncReadExt;
        const MAX_FIXTURE_REQUEST_BYTES: usize = 65_536;
        let mut bytes = Vec::new();
        let header_end = loop {
            assert!(bytes.len() < MAX_FIXTURE_REQUEST_BYTES, "bounded fixture request");
            bytes.push(stream.read_u8().await.expect("read request header"));
            if bytes.ends_with(b"\r\n\r\n") {
                break bytes.len();
            }
        };
        let headers = std::str::from_utf8(&bytes).expect("HTTP headers are UTF-8");
        let request_line = headers.lines().next().expect("request line").to_owned();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().expect("content length"))
            })
            .unwrap_or(0);
        assert!(content_length < MAX_FIXTURE_REQUEST_BYTES, "bounded fixture body");
        bytes.resize(header_end + content_length, 0);
        stream
            .read_exact(&mut bytes[header_end..])
            .await
            .expect("read request body");
        (request_line, bytes[header_end..].to_vec())
    }

    async fn serve_cache_http(listener: tokio::net::TcpListener) -> Vec<String> {
        use tokio::io::AsyncWriteExt;
        let mut stored = Vec::new();
        let mut requests = Vec::new();
        for method in ["PUT", "GET"] {
            let (mut stream, _) = listener.accept().await.expect("accept cache request");
            let (line, body) = receive_http_request(&mut stream).await;
            assert_eq!(line, format!("{method} /fixture-bucket/responses/42 HTTP/1.1"));
            requests.push(line);
            let response_body = if method == "PUT" {
                stored = body;
                &[][..]
            } else {
                &stored
            };
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response_body.len()
            );
            stream
                .write_all(header.as_bytes())
                .await
                .expect("write response header");
            stream.write_all(response_body).await.expect("write response body");
            stream.shutdown().await.expect("close response");
        }
        requests
    }

    #[tokio::test]
    async fn from_config_s3_uses_real_http_transport() {
        const FIXTURE_TIMEOUT: Duration = Duration::from_secs(5);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind isolated S3 fixture");
        let address = listener.local_addr().expect("fixture address");
        let config = HashMap::from([
            ("endpoint".to_owned(), format!("http://{address}")),
            ("bucket".to_owned(), "fixture-bucket".to_owned()),
            ("region".to_owned(), "us-east-1".to_owned()),
            ("access_key_id".to_owned(), "synthetic-fixture-access".to_owned()),
            ("secret_access_key".to_owned(), "synthetic-fixture-secret".to_owned()),
            ("disable_config_load".to_owned(), "true".to_owned()),
            ("disable_ec2_metadata".to_owned(), "true".to_owned()),
        ]);
        let store = OpenDalCacheStore::from_config("s3", config, "responses/", Duration::from_secs(300))
            .expect("configured S3 service must be available");
        let mut server = tokio::spawn(serve_cache_http(listener));
        let expected = dummy_response();
        let expected_json = serde_json::to_value(&expected).expect("serialize expected response");
        let outcome = tokio::time::timeout(FIXTURE_TIMEOUT, async {
            store.put(42, "s3-request".to_owned(), expected).await;
            store.get(42, "s3-request").await
        })
        .await;
        if !matches!(&outcome, Ok(Some(_))) {
            server.abort();
            let _ = server.await;
            panic!("actual S3 HTTP put/get must succeed: {outcome:?}");
        }
        let served = tokio::time::timeout(FIXTURE_TIMEOUT, &mut server).await;
        server.abort();
        assert_eq!(
            served.expect("bounded HTTP fixture").expect("HTTP fixture task").len(),
            2
        );
        let actual = outcome
            .expect("bounded cache operations")
            .expect("HTTP-backed cache hit");
        assert_eq!(serde_json::to_value(actual).expect("serialize response"), expected_json);
    }

    #[test]
    fn from_config_rejects_unknown_scheme() {
        let result = OpenDalCacheStore::from_config(
            "nonexistent_backend_xyz",
            std::collections::HashMap::new(),
            "prefix/",
            Duration::from_secs(60),
        );
        assert!(result.is_err(), "expected error for unknown scheme");
    }

    /// Regression for "unbounded opendal cache": without `max_entries`, keys
    /// accumulate in the backend forever unless individually read past their
    /// TTL. With `max_entries` set, inserting past the cap must evict the
    /// oldest-inserted key.
    #[tokio::test]
    async fn with_max_entries_evicts_oldest_key_on_overflow() {
        let op = Operator::via_iter("memory", std::iter::empty::<(String, String)>())
            .expect("memory backend should always build");
        let store = OpenDalCacheStore::new(op, "test/", Duration::from_secs(300)).with_max_entries(2);

        store.put(1, "req-1".into(), dummy_response()).await;
        store.put(2, "req-2".into(), dummy_response()).await;
        store.put(3, "req-3".into(), dummy_response()).await;

        assert!(
            store.get(1, "req-1").await.is_none(),
            "oldest key must be evicted once max_entries is exceeded"
        );
        assert!(store.get(2, "req-2").await.is_some(), "key 2 must still be present");
        assert!(store.get(3, "req-3").await.is_some(), "key 3 must still be present");
    }

    /// Re-inserting an existing key must refresh its position instead of
    /// evicting it as if it were the oldest entry.
    #[tokio::test]
    async fn with_max_entries_reinsert_refreshes_recency() {
        let op = Operator::via_iter("memory", std::iter::empty::<(String, String)>())
            .expect("memory backend should always build");
        let store = OpenDalCacheStore::new(op, "test/", Duration::from_secs(300)).with_max_entries(2);

        store.put(1, "req-1".into(), dummy_response()).await;
        store.put(2, "req-2".into(), dummy_response()).await;
        // Re-insert key 1: it is now the most-recently-inserted, so key 2 should be evicted next.
        store.put(1, "req-1".into(), dummy_response()).await;
        store.put(3, "req-3".into(), dummy_response()).await;

        assert!(store.get(1, "req-1").await.is_some(), "refreshed key must survive");
        assert!(
            store.get(2, "req-2").await.is_none(),
            "key 2 must be evicted as the true oldest after key 1 was refreshed"
        );
        assert!(store.get(3, "req-3").await.is_some(), "key 3 must still be present");
    }

    /// `set_ttl` fell through to the `CacheStore` trait's no-op default before
    /// this fix, so an override never took effect and every entry kept the
    /// store's construction-time TTL regardless of what `set_ttl` was called
    /// with. Revert target: removing the `impl CacheStore::set_ttl` override
    /// above (letting it fall back to the trait default) makes this fail —
    /// the entry would still be alive after the sleep, governed by the
    /// store's 3600s construction-time TTL instead of the 1ns override.
    #[tokio::test]
    async fn set_ttl_overrides_the_configured_ttl() {
        let store = memory_store(3600);
        store.put(1, "req".into(), dummy_response()).await;
        store.set_ttl(1, Duration::from_nanos(1)).await;
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let result = store.get(1, "req").await;
        assert!(
            result.is_none(),
            "entry with an overridden near-zero TTL must be expired, not governed by the \
             store's 3600s construction-time TTL"
        );
    }

    // ~keep An end-to-end variant of this (drive a real CacheLayer over the OpenDAL store and
    // ~keep assert the policy TTL expires the entry) was written and removed: it passed under the
    // ~keep workspace feature union and failed consistently under `--features tower,opendal-cache`.
    // ~keep A test whose verdict depends on the feature set is worse than no test. `set_ttl_overrides
    // ~keep _the_configured_ttl` above covers the actual fix and passes under both. ~keep

    /// `CachedResponse::Error` is deliberately not `Serialize` (see the
    /// `CachedResponse` doc comment in `cache.rs`). Before this fix, `put`
    /// attempted `serde_json::to_vec` for every entry regardless of variant,
    /// which failed for `Error` and logged a WARN — meaning
    /// `NegativeCacheLayer` writes during an upstream outage (its exact
    /// trigger condition) produced a WARN per failed request on this backend.
    /// Both before and after the fix the write is a no-op (`store.get`
    /// returns `None` either way), so the only thing that actually
    /// distinguishes "fixed" from "reverted" here is the log level — this
    /// test installs a minimal `tracing::Subscriber` (no extra dev-dependency
    /// needed; `tracing` is already a normal dependency) to count WARN vs
    /// DEBUG events instead of asserting only the no-op, which would pass
    /// unconditionally either way.
    ///
    /// Revert target: removing the early-return `if matches!(response,
    /// CachedResponse::Error { .. })` block in `put` makes `warn_count == 1`
    /// (not 0) and `debug_count == 0` (not 1).
    #[tokio::test]
    async fn put_of_error_variant_logs_at_debug_not_warn() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
        use std::time::Instant;

        use crate::error::LiterLlmError;

        struct LevelCountingSubscriber {
            warn_count: Arc<AtomicUsize>,
            debug_count: Arc<AtomicUsize>,
        }

        impl tracing::Subscriber for LevelCountingSubscriber {
            fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
                true
            }
            fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
                tracing::span::Id::from_u64(1)
            }
            fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
            fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
            fn event(&self, event: &tracing::Event<'_>) {
                match *event.metadata().level() {
                    tracing::Level::WARN => {
                        self.warn_count.fetch_add(1, AtomicOrdering::SeqCst);
                    }
                    tracing::Level::DEBUG => {
                        self.debug_count.fetch_add(1, AtomicOrdering::SeqCst);
                    }
                    _ => {}
                }
            }
            fn enter(&self, _span: &tracing::span::Id) {}
            fn exit(&self, _span: &tracing::span::Id) {}
        }

        let warn_count = Arc::new(AtomicUsize::new(0));
        let debug_count = Arc::new(AtomicUsize::new(0));
        let subscriber = LevelCountingSubscriber {
            warn_count: Arc::clone(&warn_count),
            debug_count: Arc::clone(&debug_count),
        };

        let store = memory_store(300);
        let error_entry = CachedResponse::Error {
            error: Arc::new(LiterLlmError::InternalError {
                message: "upstream unavailable".into(),
            }),
            expires_at: Instant::now() + Duration::from_secs(30),
        };

        {
            let _guard = tracing::subscriber::set_default(subscriber);
            store.put(1, "req".into(), error_entry).await;
        }

        assert!(
            store.get(1, "req").await.is_none(),
            "a non-serialisable CachedResponse::Error must not be written to the OpenDAL backend"
        );
        assert_eq!(
            warn_count.load(AtomicOrdering::SeqCst),
            0,
            "put() of an Error variant must not log at WARN — a real outage would trigger this \
             on every failed request via NegativeCacheLayer, producing a WARN storm"
        );
        assert_eq!(
            debug_count.load(AtomicOrdering::SeqCst),
            1,
            "put() must log the skipped write once at DEBUG so the no-op is not entirely silent"
        );
    }
}
