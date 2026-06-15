//! etcd-backed [`KeyResolver`] implementation.
//!
//! Keys are stored in etcd as JSON-serialised [`ResolvedKey`] values at the
//! path `{prefix}/{sha256(api_key)}`.  Hashing before lookup means raw API
//! keys are never written to the etcd key space — only their digests appear
//! as path components.
//!
//! # Example
//!
//! ```no_run
//! use liter_llm::tenant::{EtcdKeyResolver, EtcdKeyResolverConfig, KeyResolver};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let resolver = EtcdKeyResolver::connect(EtcdKeyResolverConfig::default()).await?;
//! let key = resolver.resolve("sk-my-api-key".to_owned()).await?;
//! println!("tenant: {}", key.tenant_id);
//! # Ok(())
//! # }
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use etcd_client::{Client, ConnectOptions};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use super::resolver::{KeyResolver, KeyResolverError, ResolvedKey};

/// Configuration for [`EtcdKeyResolver`].
#[derive(Clone, Debug)]
pub struct EtcdKeyResolverConfig {
    /// etcd endpoint URLs, e.g. `["http://127.0.0.1:2379"]`.
    pub endpoints: Vec<String>,
    /// Prefix under which resolved-key JSON records are stored.
    /// Records live at `{prefix}/{sha256(api_key)}`.
    pub prefix: String,
    /// Timeout for the initial TCP connection to etcd.
    pub connect_timeout: Duration,
    /// Per-request timeout for etcd RPCs.
    pub request_timeout: Duration,
    /// Optional etcd username for authentication.
    pub username: Option<String>,
    /// Optional etcd password for authentication.
    pub password: Option<String>,
}

impl Default for EtcdKeyResolverConfig {
    fn default() -> Self {
        Self {
            endpoints: vec!["http://localhost:2379".into()],
            prefix: "liter-llm/keys".into(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(2),
            username: None,
            password: None,
        }
    }
}

/// Distributed [`KeyResolver`] backed by an etcd cluster.
///
/// Looks up API keys stored as JSON-serialised [`ResolvedKey`] records at
/// `{prefix}/{sha256(api_key)}` in etcd.  Each lookup is a single point `GET`
/// RPC; there is no local cache or watch stream.
///
/// The underlying `etcd_client::Client` is `Clone`; cloning is cheap (the
/// connection pool is shared via `Arc` inside the etcd client crate).
/// `EtcdKeyResolver` wraps the client in `Arc<Mutex<…>>` to allow concurrent
/// use from the `'static` futures required by the [`KeyResolver`] contract.
#[derive(Clone)]
pub struct EtcdKeyResolver {
    client: Arc<Mutex<Client>>,
    prefix: String,
}

impl EtcdKeyResolver {
    /// Connect to an etcd cluster and return a ready resolver.
    ///
    /// Returns [`KeyResolverError::Backend`] when the connection attempt fails.
    pub async fn connect(config: EtcdKeyResolverConfig) -> Result<Self, KeyResolverError> {
        let mut options = ConnectOptions::new()
            .with_connect_timeout(config.connect_timeout)
            .with_timeout(config.request_timeout);
        if let (Some(username), Some(password)) = (config.username.as_deref(), config.password.as_deref()) {
            options = options.with_user(username, password);
        }
        let client = Client::connect(config.endpoints, Some(options))
            .await
            .map_err(|e| KeyResolverError::Backend(format!("etcd connect failed: {e}")))?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            prefix: config.prefix,
        })
    }

    /// Compute the hex-encoded SHA-256 digest of `api_key`.
    ///
    /// We hash before lookup so raw key material is never written to the etcd
    /// key space — only the digest appears as a path component.
    pub fn hash_api_key(api_key: &str) -> String {
        let digest = Sha256::digest(api_key.as_bytes());
        hex::encode(digest)
    }

    /// Return the full etcd key path for a given prefix and SHA-256 hex digest.
    pub fn key_path(prefix: &str, api_key_hash: &str) -> String {
        format!("{}/{}", prefix.trim_end_matches('/'), api_key_hash)
    }
}

impl KeyResolver for EtcdKeyResolver {
    fn resolve(
        &self,
        api_key: String,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'static>> {
        let client = Arc::clone(&self.client);
        let etcd_key = Self::key_path(&self.prefix, &Self::hash_api_key(&api_key));
        Box::pin(async move {
            let mut guard = client.lock().await;
            let response = guard
                .get(etcd_key.as_bytes(), None)
                .await
                .map_err(|e| KeyResolverError::Backend(format!("etcd get failed: {e}")))?;
            let kv = response.kvs().first().ok_or(KeyResolverError::NotFound)?;
            let resolved: ResolvedKey = serde_json::from_slice(kv.value())
                .map_err(|e| KeyResolverError::Backend(format!("invalid resolved key json: {e}")))?;
            if !resolved.active {
                return Err(KeyResolverError::Inactive);
            }
            Ok(resolved)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_api_key_is_deterministic() {
        assert_eq!(
            EtcdKeyResolver::hash_api_key("sk-abc123"),
            EtcdKeyResolver::hash_api_key("sk-abc123"),
        );
    }

    #[test]
    fn hash_api_key_differs_for_different_inputs() {
        assert_ne!(
            EtcdKeyResolver::hash_api_key("sk-abc123"),
            EtcdKeyResolver::hash_api_key("sk-abc456"),
        );
    }

    #[test]
    fn hash_api_key_produces_64_char_lowercase_hex() {
        let hash = EtcdKeyResolver::hash_api_key("sk-abc123");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn key_path_joins_prefix_and_hash() {
        assert_eq!(
            EtcdKeyResolver::key_path("liter-llm/keys", "abc123"),
            "liter-llm/keys/abc123"
        );
    }

    #[test]
    fn key_path_trims_trailing_slash_from_prefix() {
        assert_eq!(
            EtcdKeyResolver::key_path("liter-llm/keys/", "abc123"),
            "liter-llm/keys/abc123"
        );
    }

    #[test]
    fn key_path_round_trip_with_real_hash() {
        let hash = EtcdKeyResolver::hash_api_key("sk-test-key");
        let path = EtcdKeyResolver::key_path("liter-llm/keys", &hash);
        assert!(path.starts_with("liter-llm/keys/"));
        assert_eq!(path.len(), "liter-llm/keys/".len() + 64);
    }
}
