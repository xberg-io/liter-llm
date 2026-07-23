//! OpenDAL-backed cache store for the response cache.
//!
//! Implements [`CacheStore`] using an [`opendal::Operator`] for persistence.
//! Supports any OpenDAL backend (S3, Redis, GCS, local filesystem, etc.).

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
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
    /// Unix timestamp (seconds) when this entry expires.
    expires_at: u64,
}

/// Cache store backed by an [`opendal::Operator`].
///
/// Entries are stored as JSON files under `{prefix}/{key}`. TTL is embedded
/// in the stored entry and checked on read. Backend failures are non-fatal:
/// they log a warning and behave as a cache miss / no-op.
pub struct OpenDalCacheStore {
    operator: Operator,
    prefix: String,
    ttl: Duration,
}

impl OpenDalCacheStore {
    /// Create a new OpenDAL cache store.
    ///
    /// `operator` must be a fully configured OpenDAL operator.
    /// `prefix` is prepended to all cache keys (e.g. `"llm-cache/"`).
    /// `ttl` controls how long entries are valid.
    pub fn new(operator: Operator, prefix: impl Into<String>, ttl: Duration) -> Self {
        Self {
            operator,
            prefix: prefix.into(),
            ttl,
        }
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
}

impl CacheStore for OpenDalCacheStore {
    fn get(&self, key: u64, request_body: &str) -> Pin<Box<dyn Future<Output = Option<CachedResponse>> + Send + '_>> {
        let path = self.key_path(key);
        let request_body = request_body.to_owned();
        Box::pin(async move {
            let bytes = match self.operator.read(&path).await {
                Ok(b) => b,
                Err(_) => return None,
            };
            let entry: StoredEntry = match serde_json::from_slice(bytes.to_bytes().as_ref()) {
                Ok(e) => e,
                Err(_) => return None,
            };
            if Self::now_secs() > entry.expires_at {
                let _ = self.operator.delete(&path).await;
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
        let path = self.key_path(key);
        let entry = StoredEntry {
            request_body,
            response,
            expires_at: Self::now_secs() + self.ttl.as_secs(),
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
            }
        })
    }

    fn remove(&self, key: u64) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let path = self.key_path(key);
        Box::pin(async move {
            if let Err(e) = self.operator.delete(&path).await {
                tracing::warn!("OpenDAL cache: failed to delete {path}: {e}");
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
                },
                finish_reason: Some(FinishReason::Stop),
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
}
