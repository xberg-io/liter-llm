//! [`HashCorpVaultProvider`] — HashCorp Vault KV-v2 backend.
//!
//! Uses [`vaultrs`] to talk to the Vault HTTP API.  Secrets are fetched from
//! the KV-v2 secrets engine.  Responses are cached with a configurable TTL
//! (default 60 s).
//!
//! # Feature gate
//!
//! Only compiled when the `secrets-vault` Cargo feature is enabled.
//!
//! # Configuration
//!
//! ```rust,ignore
//! use liter_llm_proxy::secrets::HashCorpVaultProvider;
//!
//! let provider = HashCorpVaultProvider::builder()
//!     .address("https://vault.example.com")
//!     .token("s.xxxx")
//!     .mount("secret")           // KV-v2 mount (default "secret")
//!     .cache_ttl(Duration::from_secs(30))
//!     .build()?;
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use liter_llm::provider::outbound_policy::validate_outbound_url_sync;
use secrecy::{ExposeSecret, SecretString};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

use super::{SecretError, SecretManager, SecretMetadata, SecretValue};

// ---------------------------------------------------------------------------
// Cache (reuses the same pattern as the AWS backend)
// ---------------------------------------------------------------------------

/// A single cached entry.
///
/// `raw` is a [`SecretString`] so the heap memory is zeroed on eviction
/// (when the entry is dropped from the `HashMap`).  Raw `String` caching
/// would leave the plaintext on the heap until the allocator reuses the
/// memory, creating a heap-dump exposure window.
struct CacheEntry {
    /// Secret value, zeroed on drop.
    ///
    /// Use `.expose_secret()` only at the last possible moment — when building
    /// the HTTP `Authorization` header or returning to the caller.  Never
    /// derive `Debug` or `Display` on any type that holds this field.
    raw: SecretString,
    metadata: SecretMetadata,
    cached_at: Instant,
}

/// Simple TTL cache keyed by secret path.
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
    fn get(&self, key: &str) -> Option<(SecretString, SecretMetadata)> {
        let store = self.store.lock().expect("cache mutex poisoned");
        let entry = store.get(key)?;
        if entry.cached_at.elapsed() < self.ttl {
            // Clone exposes the underlying bytes momentarily; the new
            // SecretString takes ownership and will zero them on drop.
            let cloned = SecretString::from(entry.raw.expose_secret().to_owned());
            Some((cloned, entry.metadata.clone()))
        } else {
            None
        }
    }

    /// Insert or overwrite a cache entry.
    ///
    /// The `raw` value is stored as a [`SecretString`].  Any previous entry
    /// for `key` is evicted (and its memory zeroed) by the `HashMap::insert`
    /// replacement.
    fn insert(&self, key: &str, raw: SecretString, metadata: SecretMetadata) {
        let mut store = self.store.lock().expect("cache mutex poisoned");
        store.insert(
            key.to_owned(),
            CacheEntry {
                raw,
                metadata,
                cached_at: Instant::now(),
            },
        );
    }

    /// Evict a cache entry.
    ///
    /// `HashMap::remove` drops the `CacheEntry`, which drops the
    /// `SecretString`, triggering zeroization of the heap allocation.
    fn evict(&self, key: &str) {
        let mut store = self.store.lock().expect("cache mutex poisoned");
        store.remove(key);
    }
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

/// HashCorp Vault KV-v2 secret manager.
pub struct HashCorpVaultProvider {
    client: Arc<VaultClient>,
    mount: String,
    cache: Arc<SecretCache>,
}

impl HashCorpVaultProvider {
    /// Return a builder for this provider.
    pub fn builder() -> HashCorpVaultProviderBuilder {
        HashCorpVaultProviderBuilder::default()
    }

    /// Look up a KV-v2 path in the form `"path/to/secret"` and field
    /// `"field_name"`.  The full cache key is `"mount:path:field"`.
    async fn fetch_from_vault(&self, name: &str) -> Result<SecretValue, SecretError> {
        // Split `path[:field]` — if the caller uses `path#field`, we split on `#`.
        let (path, field) = if let Some(idx) = name.find('#') {
            (&name[..idx], &name[idx + 1..])
        } else {
            (name, "value")
        };

        let data: HashMap<String, String> = vaultrs::kv2::read(self.client.as_ref(), &self.mount, path)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("404") || msg.contains("not found") {
                    SecretError::NotFound(name.to_owned())
                } else if msg.contains("403") || msg.contains("permission denied") {
                    SecretError::PermissionDenied(name.to_owned())
                } else if msg.contains("429") || msg.contains("rate limit") {
                    SecretError::RateLimited
                } else {
                    SecretError::backend(e)
                }
            })?;

        let raw = data
            .get(field)
            .cloned()
            .ok_or_else(|| SecretError::NotFound(format!("{name} (field '{field}'  not present in KV data)")))?;

        // Vault KV-v2 metadata (version, created_time) is returned in a
        // separate API call.  To keep the hot path cheap we read it
        // opportunistically and fall back to defaults on error.
        let (version, created_at, tags) =
            match vaultrs::kv2::read_metadata(self.client.as_ref(), &self.mount, path).await {
                Ok(meta) => {
                    let version = meta.current_version.to_string();
                    let created_at = meta
                        .created_time
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .ok()
                        .map(SystemTime::from)
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    let tags: HashMap<String, String> = meta.custom_metadata.unwrap_or_default();
                    (version, created_at, tags)
                }
                Err(_) => ("unknown".to_owned(), SystemTime::UNIX_EPOCH, HashMap::new()),
            };

        let now = SystemTime::now();
        let metadata = SecretMetadata {
            name: name.to_owned(),
            version,
            created_at,
            updated_at: now,
            expires_at: None,
            tags,
        };

        // Wrap in SecretString before caching so the cache never holds a
        // plain `String`.  Expose the secret only at the very last moment
        // when handing it to the caller.
        let secret_value = SecretString::from(raw);
        // Clone via expose_secret: the cache entry gets its own SecretString.
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

impl SecretManager for HashCorpVaultProvider {
    fn backend(&self) -> &'static str {
        "hashicorp-vault"
    }

    fn get<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<SecretValue, SecretError>> + Send + 'a>> {
        Box::pin(async move {
            // Cache returns a SecretString; no intermediate plain String.
            if let Some((secret, metadata)) = self.cache.get(name) {
                return Ok(SecretValue {
                    value: secret,
                    metadata,
                });
            }
            self.fetch_from_vault(name).await
        })
    }

    fn set<'a>(
        &'a self,
        name: &'a str,
        value: SecretString,
        tags: HashMap<String, String>,
    ) -> Pin<Box<dyn Future<Output = Result<SecretMetadata, SecretError>> + Send + 'a>> {
        Box::pin(async move {
            let (path, field) = if let Some(idx) = name.find('#') {
                (&name[..idx], &name[idx + 1..])
            } else {
                (name, "value")
            };

            let raw = value.expose_secret().to_owned();
            let mut data = HashMap::new();
            data.insert(field.to_owned(), raw.clone());

            vaultrs::kv2::set(self.client.as_ref(), &self.mount, path, &data)
                .await
                .map_err(|e| {
                    if e.to_string().contains("403") {
                        SecretError::PermissionDenied(name.to_owned())
                    } else {
                        SecretError::backend(e)
                    }
                })?;

            let now = SystemTime::now();
            let metadata = SecretMetadata {
                name: name.to_owned(),
                version: "latest".to_owned(),
                created_at: now,
                updated_at: now,
                expires_at: None,
                tags: tags.clone(),
            };
            // Cache as SecretString so the heap bytes are zeroed on eviction.
            self.cache.insert(name, SecretString::from(raw), metadata.clone());
            Ok(metadata)
        })
    }

    fn delete<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<(), SecretError>> + Send + 'a>> {
        Box::pin(async move {
            let path = if let Some(idx) = name.find('#') {
                &name[..idx]
            } else {
                name
            };
            vaultrs::kv2::delete_latest(self.client.as_ref(), &self.mount, path)
                .await
                .map_err(|e| {
                    if e.to_string().contains("403") {
                        SecretError::PermissionDenied(name.to_owned())
                    } else {
                        SecretError::backend(e)
                    }
                })?;
            self.cache.evict(name);
            Ok(())
        })
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`HashCorpVaultProvider`].
#[derive(Default)]
pub struct HashCorpVaultProviderBuilder {
    address: Option<String>,
    token: Option<String>,
    mount: Option<String>,
    cache_ttl: Option<Duration>,
}

impl HashCorpVaultProviderBuilder {
    /// Set the Vault server address (e.g. `"https://vault.example.com"`).
    pub fn address(mut self, address: impl Into<String>) -> Self {
        self.address = Some(address.into());
        self
    }

    /// Set the Vault token (e.g. `"s.xxxx"` or `"hvs.xxxx"`).
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the KV-v2 mount (default `"secret"`).
    pub fn mount(mut self, mount: impl Into<String>) -> Self {
        self.mount = Some(mount.into());
        self
    }

    /// Override the cache TTL (default 60 s).
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }

    /// Build the provider.
    ///
    /// # Errors
    ///
    /// - [`SecretError::Forbidden`] if the Vault address is rejected by the
    ///   active outbound SSRF policy (e.g. targets a loopback or private
    ///   network address when the policy is `DenyPrivate`).
    /// - [`SecretError::Backend`] if the Vault client cannot be constructed
    ///   (e.g. invalid URL format).
    ///
    /// # Security
    ///
    /// The caller-supplied `address` is validated against the global
    /// [`liter_llm::provider::OutboundPolicy`] **before** the `VaultClient`
    /// is constructed.  This prevents SSRF via operator misconfiguration or
    /// config-file injection — an address pointing at `127.0.0.1:8200`,
    /// `169.254.169.254`, or any other private/link-local range is rejected
    /// when the policy is `DenyPrivate` (the proxy default).
    pub fn build(self) -> Result<HashCorpVaultProvider, SecretError> {
        let address = self.address.unwrap_or_else(|| "http://127.0.0.1:8200".to_owned());
        let token = self
            .token
            .unwrap_or_else(|| std::env::var("VAULT_TOKEN").unwrap_or_default());
        let mount = self.mount.unwrap_or_else(|| "secret".to_owned());
        let cache_ttl = self.cache_ttl.unwrap_or(Duration::from_secs(60));

        // ── SSRF guard ────────────────────────────────────────────────────────
        // Validate the address against the active outbound policy.  The sync
        // variant handles literal-IP private ranges (loopback, link-local,
        // CGNAT) without requiring an async context; hostname-based addresses
        // get additional checks at connect time via `GuardedResolver`.
        validate_outbound_url_sync(&address).map_err(|e| {
            SecretError::Forbidden(format!(
                "Vault address '{address}' rejected by outbound policy: {e}"
            ))
        })?;

        let settings = VaultClientSettingsBuilder::default()
            .address(address)
            .token(token)
            .build()
            .map_err(|e| SecretError::backend_msg(format!("invalid Vault client settings: {e}")))?;

        let client = VaultClient::new(settings)
            .map_err(|e| SecretError::backend_msg(format!("failed to create Vault client: {e}")))?;

        Ok(HashCorpVaultProvider {
            client: Arc::new(client),
            mount,
            cache: Arc::new(SecretCache::new(cache_ttl)),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    // ── SecretString cache type verification ──────────────────────────────────

    /// Verify that the cache stores [`SecretString`] values so heap memory is
    /// zeroed on eviction.  This is a type-level assertion: `SecretCache::get`
    /// returns `(SecretString, SecretMetadata)` — if it returned `(String, …)`
    /// the compiler would reject the type annotation below.
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
        // Insert as SecretString (the only accepted type).
        cache.insert("my/secret", SecretString::from("plaintext-value".to_owned()), meta);

        // get() must return SecretString, not String.
        // If this type annotation compiles, the cache uses SecretString in memory.
        let result: Option<(SecretString, SecretMetadata)> = cache.get("my/secret");
        assert!(result.is_some(), "cache hit expected");
        let (secret, _meta) = result.unwrap();
        assert_eq!(
            secret.expose_secret(),
            "plaintext-value",
            "value round-trips through SecretString cache"
        );

        // Evict the entry — the CacheEntry (and its SecretString) is dropped,
        // which triggers zeroization of the heap allocation.
        cache.evict("my/secret");
        assert!(
            cache.get("my/secret").is_none(),
            "evicted entry must not be found"
        );
    }

    // ── SSRF guard tests ──────────────────────────────────────────────────────

    /// The Vault builder must reject private/loopback addresses when the
    /// `DenyPrivate` outbound policy is active.
    ///
    /// The global outbound policy is `Off` by default (library default).
    /// This test explicitly sets `DenyPrivate` for its duration and resets to
    /// `Off` afterwards.  Tests that rely on the default policy run fine
    /// because this test only changes the policy locally within its own serial
    /// execution.
    ///
    /// Note: because the policy is a process-global `OnceLock<RwLock<…>>`,
    /// concurrent tests that set a different policy can interfere.  In
    /// practice, the default `Off` policy makes most tests unaffected.
    #[test]
    fn vault_address_rejects_internal_endpoints() {
        use liter_llm::provider::{OutboundPolicy, set_outbound_policy};

        // Activate the deny-private policy for this test.
        set_outbound_policy(OutboundPolicy::DenyPrivate);

        // Loopback addresses are forbidden under DenyPrivate.
        let result_loopback = HashCorpVaultProvider::builder()
            .address("http://127.0.0.1:8200")
            .token("test-token")
            .build();
        assert!(
            result_loopback.is_err(),
            "loopback address must be rejected: {result_loopback:?}"
        );
        match result_loopback.unwrap_err() {
            SecretError::Forbidden(msg) => {
                assert!(
                    msg.contains("127.0.0.1"),
                    "error message should mention the forbidden address: {msg}"
                );
            }
            other => panic!("expected SecretError::Forbidden, got {other:?}"),
        }

        // Link-local / cloud-metadata addresses are also forbidden.
        let result_metadata = HashCorpVaultProvider::builder()
            .address("http://169.254.169.254/latest/meta-data/")
            .token("test-token")
            .build();
        assert!(
            result_metadata.is_err(),
            "cloud-metadata address must be rejected: {result_metadata:?}"
        );
        match result_metadata.unwrap_err() {
            SecretError::Forbidden(msg) => {
                assert!(
                    msg.contains("169.254.169.254"),
                    "error message should mention the forbidden address: {msg}"
                );
            }
            other => panic!("expected SecretError::Forbidden for link-local, got {other:?}"),
        }

        // Reset to Off so subsequent tests are unaffected.
        set_outbound_policy(OutboundPolicy::Off);
    }

    #[test]
    fn secret_manager_vault_cache_ttl_zero_always_misses() {
        let cache = SecretCache::new(Duration::ZERO);
        let meta = SecretMetadata {
            name: "path/key".to_owned(),
            version: "3".to_owned(),
            created_at: SystemTime::UNIX_EPOCH,
            updated_at: SystemTime::UNIX_EPOCH,
            expires_at: None,
            tags: HashMap::new(),
        };
        cache.insert("path/key", "secret-val".to_owned(), meta);
        // Immediately expired.
        assert!(cache.get("path/key").is_none());
    }

    #[test]
    fn secret_manager_vault_cache_hit_within_ttl() {
        let cache = SecretCache::new(Duration::from_secs(60));
        let meta = SecretMetadata {
            name: "path/key".to_owned(),
            version: "3".to_owned(),
            created_at: SystemTime::UNIX_EPOCH,
            updated_at: SystemTime::UNIX_EPOCH,
            expires_at: None,
            tags: HashMap::new(),
        };
        cache.insert("path/key", "secret-val".to_owned(), meta);
        let hit = cache.get("path/key");
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().0, "secret-val");
    }

    /// Verify that the builder falls back to sane defaults.
    #[test]
    fn builder_defaults_produce_provider() {
        // This test doesn't need a real Vault server — it only checks that the
        // builder constructs the object without panicking.
        let result = HashCorpVaultProvider::builder()
            .address("http://127.0.0.1:8200")
            .token("test-token")
            .build();
        assert!(result.is_ok(), "builder should succeed with minimal config");
        let provider = result.unwrap();
        assert_eq!(provider.backend(), "hashicorp-vault");
        assert_eq!(provider.mount, "secret");
    }

    #[test]
    fn builder_custom_mount() {
        let provider = HashCorpVaultProvider::builder()
            .address("http://127.0.0.1:8200")
            .token("tok")
            .mount("kv")
            .cache_ttl(Duration::from_secs(120))
            .build()
            .expect("should build");
        assert_eq!(provider.mount, "kv");
    }
}
