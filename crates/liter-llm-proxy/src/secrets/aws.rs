//! [`AwsSecretsManagerProvider`] — AWS Secrets Manager backend.
//!
//! Fetches secrets via `GetSecretValue`.  Responses are cached with a
//! configurable TTL (default 60 s) to avoid hammering the AWS API.
//!
//! # Feature gate
//!
//! Only compiled when the `secrets-aws` Cargo feature is enabled.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use aws_sdk_secretsmanager::Client;
use secrecy::{ExposeSecret, SecretString};

use super::{SecretError, SecretManager, SecretMetadata, SecretValue};

/// A single cached entry.
///
/// `value` is a [`SecretString`] so the heap memory is zeroed on eviction
/// (when the entry is dropped from the `HashMap`).  Raw `String` caching
/// would leave the plaintext on the heap until the allocator reuses the
/// memory, creating a heap-dump exposure window.
struct CacheEntry {
    /// Secret value, zeroed on drop.
    ///
    /// Use `.expose_secret()` only at the last possible moment — when building
    /// the HTTP `Authorization` header or returning to the caller.  Never
    /// derive `Debug` or `Display` on any type that holds this field.
    value: SecretString,
    metadata: SecretMetadata,
    cached_at: Instant,
}

/// Simple TTL cache keyed by secret name.
///
/// Uses a `Mutex<HashMap>` — cache hits are fast (no network I/O), so the
/// lock contention overhead is negligible compared to an AWS API call.
///
/// Secret values are stored as [`SecretString`] to ensure heap memory is
/// zeroed when entries are evicted.
struct SecretCache {
    store: Mutex<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl SecretCache {
    fn new(ttl: Duration) -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Return a cached entry if it exists and has not expired.
    ///
    /// The returned [`SecretString`] wraps a clone of the cached bytes.
    /// Memory zeroing occurs when the returned value is dropped.
    fn get(&self, name: &str) -> Option<(SecretString, SecretMetadata)> {
        let store = self.store.lock().expect("cache mutex poisoned");
        let entry = store.get(name)?;
        if entry.cached_at.elapsed() < self.ttl {
            // ~keep Clone through a fresh SecretString so returned heap bytes are zeroed on drop.
            let cloned = SecretString::from(entry.value.expose_secret().to_owned());
            Some((cloned, entry.metadata.clone()))
        } else {
            None
        }
    }

    /// Insert or overwrite a cache entry.
    ///
    /// The `value` is stored as-is (already a [`SecretString`]).  Any
    /// previous entry for `name` is evicted (and its memory zeroed) by the
    /// `HashMap::insert` replacement.
    fn insert(&self, name: &str, value: SecretString, metadata: SecretMetadata) {
        let mut store = self.store.lock().expect("cache mutex poisoned");
        store.insert(
            name.to_owned(),
            CacheEntry {
                value,
                metadata,
                cached_at: Instant::now(),
            },
        );
    }

    /// Evict a cache entry (used after a successful `delete`).
    ///
    /// `HashMap::remove` drops the `CacheEntry`, which drops the
    /// `SecretString`, triggering zeroization of the heap allocation.
    #[allow(dead_code)]
    fn evict(&self, name: &str) {
        let mut store = self.store.lock().expect("cache mutex poisoned");
        store.remove(name);
    }
}

/// AWS Secrets Manager secret manager.
///
/// Fetches secrets via [`GetSecretValue`](aws_sdk_secretsmanager::operation::get_secret_value).
/// Results are cached for `cache_ttl` (default 60 s).
///
/// # Construction
///
/// ```rust,ignore
/// use aws_config::BehaviorVersion;
/// use liter_llm_proxy::secrets::AwsSecretsManagerProvider;
///
/// let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
/// let provider = AwsSecretsManagerProvider::from_aws_config(&config);
/// ```
pub struct AwsSecretsManagerProvider {
    client: Arc<Client>,
    cache: Arc<SecretCache>,
}

impl AwsSecretsManagerProvider {
    /// Construct from an already-loaded [`aws_config::SdkConfig`].
    pub fn from_aws_config(config: &aws_config::SdkConfig) -> Self {
        let client = Client::new(config);
        Self::from_client(client, Duration::from_secs(60))
    }

    /// Construct from a pre-built [`Client`] with a custom cache TTL.
    pub fn from_client(client: Client, cache_ttl: Duration) -> Self {
        Self {
            client: Arc::new(client),
            cache: Arc::new(SecretCache::new(cache_ttl)),
        }
    }

    /// Fetch from the AWS API and populate the cache.
    async fn fetch_from_aws(&self, name: &str) -> Result<SecretValue, SecretError> {
        let resp = self
            .client
            .get_secret_value()
            .secret_id(name)
            .send()
            .await
            .map_err(|sdk_err| {
                use aws_sdk_secretsmanager::error::SdkError;
                use aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueError;

                match sdk_err {
                    SdkError::ServiceError(ref svc) => match svc.err() {
                        GetSecretValueError::ResourceNotFoundException(_) => SecretError::NotFound(name.to_owned()),
                        other => {
                            let msg = other.to_string();
                            if msg.contains("AccessDenied") {
                                SecretError::PermissionDenied(name.to_owned())
                            } else {
                                SecretError::backend_msg(format!("AWS error: {other}"))
                            }
                        }
                    },
                    other => SecretError::backend(other),
                }
            })?;

        let raw_value = resp
            .secret_string()
            .map(|s| s.to_owned())
            .or_else(|| {
                resp.secret_binary()
                    .map(|b| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b.as_ref()))
            })
            .ok_or_else(|| SecretError::backend_msg("AWS returned a secret with no string or binary value"))?;

        let created_at = resp
            .created_date()
            .and_then(|dt| UNIX_EPOCH.checked_add(Duration::from_secs_f64(dt.as_secs_f64())))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let updated_at = created_at;

        let tags: HashMap<String, String> = resp
            .version_stages()
            .iter()
            .enumerate()
            .map(|(i, s)| (format!("stage_{i}"), s.to_owned()))
            .collect();

        let version = resp.version_id().unwrap_or("unknown").to_owned();

        let metadata = SecretMetadata {
            name: name.to_owned(),
            version,
            created_at,
            updated_at,
            expires_at: None,
            tags,
        };

        // ~keep Cache AWS secrets as SecretString, never plain String.
        let secret_value = SecretString::from(raw_value);
        // ~keep Cache entry owns a separate SecretString allocation.
        self.cache.insert(
            name,
            SecretString::from(secret_value.expose_secret().to_owned()),
            metadata.clone(),
        );

        Ok(SecretValue {
            value: secret_value,
            metadata,
        })
    }
}

impl SecretManager for AwsSecretsManagerProvider {
    fn backend(&self) -> &'static str {
        "aws-secrets-manager"
    }

    fn get<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<SecretValue, SecretError>> + Send + 'a>> {
        Box::pin(async move {
            if let Some((secret, metadata)) = self.cache.get(name) {
                return Ok(SecretValue {
                    value: secret,
                    metadata,
                });
            }
            self.fetch_from_aws(name).await
        })
    }

    fn set<'a>(
        &'a self,
        name: &'a str,
        value: SecretString,
        tags: HashMap<String, String>,
    ) -> Pin<Box<dyn Future<Output = Result<SecretMetadata, SecretError>> + Send + 'a>> {
        Box::pin(async move {
            let aws_tags: Vec<aws_sdk_secretsmanager::types::Tag> = tags
                .iter()
                .map(|(k, v)| aws_sdk_secretsmanager::types::Tag::builder().key(k).value(v).build())
                .collect();

            let raw = value.expose_secret().to_owned();
            let put_result = self
                .client
                .put_secret_value()
                .secret_id(name)
                .secret_string(&raw)
                .send()
                .await;

            let version = match put_result {
                Ok(resp) => resp.version_id().unwrap_or("unknown").to_owned(),
                Err(sdk_err) => {
                    use aws_sdk_secretsmanager::error::SdkError;
                    use aws_sdk_secretsmanager::operation::put_secret_value::PutSecretValueError;

                    let is_not_found = matches!(
                        &sdk_err,
                        SdkError::ServiceError(svc)
                            if matches!(svc.err(), PutSecretValueError::ResourceNotFoundException(_))
                    );
                    if is_not_found {
                        let mut req = self.client.create_secret().name(name).secret_string(&raw);
                        for tag in &aws_tags {
                            if let (Some(k), Some(v)) = (tag.key(), tag.value()) {
                                req = req.tags(aws_sdk_secretsmanager::types::Tag::builder().key(k).value(v).build());
                            }
                        }
                        req.send()
                            .await
                            .map(|r| r.version_id().unwrap_or("unknown").to_owned())
                            .map_err(SecretError::backend)?
                    } else {
                        return Err(SecretError::backend(sdk_err));
                    }
                }
            };

            let now = SystemTime::now();
            let metadata = SecretMetadata {
                name: name.to_owned(),
                version,
                created_at: now,
                updated_at: now,
                expires_at: None,
                tags: tags.clone(),
            };
            // ~keep Cache the value as SecretString so heap bytes are zeroed on eviction.
            self.cache.insert(name, SecretString::from(raw), metadata.clone());
            Ok(metadata)
        })
    }

    fn delete<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<(), SecretError>> + Send + 'a>> {
        Box::pin(async move {
            self.client
                .delete_secret()
                .secret_id(name)
                .send()
                .await
                .map_err(SecretError::backend)?;
            self.cache.evict(name);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn cache_hit_returns_cached_value() {
        let cache = SecretCache::new(Duration::from_secs(60));
        let meta = SecretMetadata {
            name: "prod/api-key".to_owned(),
            version: "abc123".to_owned(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            expires_at: None,
            tags: HashMap::new(),
        };
        cache.insert("prod/api-key", SecretString::from("super-secret".to_owned()), meta);
        let hit = cache.get("prod/api-key");
        assert!(hit.is_some());
        let (val, _meta) = hit.unwrap();
        assert_eq!(val.expose_secret(), "super-secret");
    }

    #[test]
    fn secret_manager_aws_cache_hit_avoids_second_fetch() {
        let cache = Arc::new(SecretCache::new(Duration::from_secs(60)));
        let meta = SecretMetadata {
            name: "my/secret".to_owned(),
            version: "v1".to_owned(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            expires_at: None,
            tags: HashMap::new(),
        };
        cache.insert("my/secret", SecretString::from("value-one".to_owned()), meta.clone());

        let hit1 = cache.get("my/secret");
        let hit2 = cache.get("my/secret");

        assert!(hit1.is_some());
        assert!(hit2.is_some());
        assert_eq!(hit1.unwrap().0.expose_secret(), "value-one");
        assert_eq!(hit2.unwrap().0.expose_secret(), "value-one");
    }

    #[test]
    fn secret_manager_aws_cache_miss_after_ttl() {
        let cache = SecretCache::new(Duration::ZERO);
        let meta = SecretMetadata {
            name: "key".to_owned(),
            version: "1".to_owned(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            expires_at: None,
            tags: HashMap::new(),
        };
        cache.insert("key", SecretString::from("val".to_owned()), meta);
        assert!(cache.get("key").is_none(), "zero-TTL cache should always miss");
    }

    #[test]
    fn cache_miss_on_unknown_key() {
        let cache = SecretCache::new(Duration::from_secs(60));
        assert!(cache.get("nonexistent").is_none());
    }

    /// Verify that the AWS cache uses [`SecretString`] values so heap memory
    /// is zeroed on eviction.  This is a type-level assertion enforced at
    /// compile time: `SecretCache::get` returns `(SecretString, SecretMetadata)`
    /// — if it returned `(String, …)` the type annotation below would not compile.
    #[test]
    fn aws_cache_zeroizes_on_eviction_type_check() {
        let cache = SecretCache::new(Duration::from_secs(60));
        let meta = SecretMetadata {
            name: "my/secret".to_owned(),
            version: "v1".to_owned(),
            created_at: SystemTime::UNIX_EPOCH,
            updated_at: SystemTime::UNIX_EPOCH,
            expires_at: None,
            tags: HashMap::new(),
        };
        cache.insert("my/secret", SecretString::from("plaintext-value".to_owned()), meta);

        let result: Option<(SecretString, SecretMetadata)> = cache.get("my/secret");
        assert!(result.is_some(), "cache hit expected");
        let (secret, _meta) = result.unwrap();
        assert_eq!(
            secret.expose_secret(),
            "plaintext-value",
            "value round-trips through SecretString cache"
        );

        cache.evict("my/secret");
        assert!(cache.get("my/secret").is_none(), "evicted entry must not be found");
    }
}
