//! [`HashiCorpVaultProvider`] — HashiCorp Vault KV-v2 backend.
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
//! use liter_llm_proxy::secrets::HashiCorpVaultProvider;
//!
//! let provider = HashiCorpVaultProvider::builder()
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

use secrecy::{ExposeSecret, SecretString};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

use super::{SecretError, SecretMetadata, SecretManager, SecretValue};

// ---------------------------------------------------------------------------
// Cache (reuses the same pattern as the AWS backend)
// ---------------------------------------------------------------------------

struct CacheEntry {
    raw: String,
    metadata: SecretMetadata,
    cached_at: Instant,
}

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

    fn get(&self, key: &str) -> Option<(String, SecretMetadata)> {
        let store = self.store.lock().expect("cache mutex poisoned");
        let entry = store.get(key)?;
        if entry.cached_at.elapsed() < self.ttl {
            Some((entry.raw.clone(), entry.metadata.clone()))
        } else {
            None
        }
    }

    fn insert(&self, key: &str, raw: String, metadata: SecretMetadata) {
        let mut store = self.store.lock().expect("cache mutex poisoned");
        store.insert(key.to_owned(), CacheEntry { raw, metadata, cached_at: Instant::now() });
    }

    fn evict(&self, key: &str) {
        let mut store = self.store.lock().expect("cache mutex poisoned");
        store.remove(key);
    }
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

/// HashiCorp Vault KV-v2 secret manager.
pub struct HashiCorpVaultProvider {
    client: Arc<VaultClient>,
    mount: String,
    cache: Arc<SecretCache>,
}

impl HashiCorpVaultProvider {
    /// Return a builder for this provider.
    pub fn builder() -> HashiCorpVaultProviderBuilder {
        HashiCorpVaultProviderBuilder::default()
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

        let data: HashMap<String, String> = vaultrs::kv2::read(
            self.client.as_ref(),
            &self.mount,
            path,
        )
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
        let (version, created_at, tags) = match vaultrs::kv2::read_metadata(
            self.client.as_ref(),
            &self.mount,
            path,
        )
        .await
        {
            Ok(meta) => {
                let version = meta.current_version.to_string();
                let created_at = meta
                    .created_time
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .ok()
                    .map(SystemTime::from)
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let tags: HashMap<String, String> = meta
                    .custom_metadata
                    .unwrap_or_default();
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

        self.cache.insert(name, raw.clone(), metadata.clone());

        Ok(SecretValue {
            value: SecretString::from(raw),
            metadata,
        })
    }
}

impl SecretManager for HashiCorpVaultProvider {
    fn backend(&self) -> &'static str {
        "hashicorp-vault"
    }

    fn get<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<SecretValue, SecretError>> + Send + 'a>> {
        Box::pin(async move {
            if let Some((raw, metadata)) = self.cache.get(name) {
                return Ok(SecretValue {
                    value: SecretString::from(raw),
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

            vaultrs::kv2::set(
                self.client.as_ref(),
                &self.mount,
                path,
                &data,
            )
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
            self.cache.insert(name, raw, metadata.clone());
            Ok(metadata)
        })
    }

    fn delete<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), SecretError>> + Send + 'a>> {
        Box::pin(async move {
            let path = if let Some(idx) = name.find('#') { &name[..idx] } else { name };
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

/// Builder for [`HashiCorpVaultProvider`].
#[derive(Default)]
pub struct HashiCorpVaultProviderBuilder {
    address: Option<String>,
    token: Option<String>,
    mount: Option<String>,
    cache_ttl: Option<Duration>,
}

impl HashiCorpVaultProviderBuilder {
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
    /// Returns a [`SecretError::Backend`] if the Vault client cannot be
    /// constructed (e.g. invalid URL).
    pub fn build(self) -> Result<HashiCorpVaultProvider, SecretError> {
        let address = self
            .address
            .unwrap_or_else(|| "http://127.0.0.1:8200".to_owned());
        let token = self
            .token
            .unwrap_or_else(|| std::env::var("VAULT_TOKEN").unwrap_or_default());
        let mount = self.mount.unwrap_or_else(|| "secret".to_owned());
        let cache_ttl = self.cache_ttl.unwrap_or(Duration::from_secs(60));

        let settings = VaultClientSettingsBuilder::default()
            .address(address)
            .token(token)
            .build()
            .map_err(|e| SecretError::backend_msg(format!("invalid Vault client settings: {e}")))?;

        let client = VaultClient::new(settings)
            .map_err(|e| SecretError::backend_msg(format!("failed to create Vault client: {e}")))?;

        Ok(HashiCorpVaultProvider {
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
        let result = HashiCorpVaultProvider::builder()
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
        let provider = HashiCorpVaultProvider::builder()
            .address("http://127.0.0.1:8200")
            .token("tok")
            .mount("kv")
            .cache_ttl(Duration::from_secs(120))
            .build()
            .expect("should build");
        assert_eq!(provider.mount, "kv");
    }
}
