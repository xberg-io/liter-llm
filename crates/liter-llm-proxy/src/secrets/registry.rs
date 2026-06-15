//! [`SecretManagerRegistry`] — typed router that selects a backend by URI scheme.
//!
//! # URI scheme routing
//!
//! | Prefix | Backend |
//! |--------|---------|
//! | `env://NAME` | [`EnvVarSecretManager`](super::EnvVarSecretManager) |
//! | `aws://PATH` | [`AwsSecretsManagerProvider`](super::AwsSecretsManagerProvider) (feature `secrets-aws`) |
//! | `vault://PATH` | [`HashiCorpVaultProvider`](super::HashiCorpVaultProvider) (feature `secrets-vault`) |
//!
//! When no matching scheme is found, the request is forwarded to the
//! configured default backend using the full name as-is.
//!
//! # Rotation warnings
//!
//! Every successful `get` call through the registry checks the returned
//! [`SecretMetadata::expires_at`].  A [`tracing::warn!`] is emitted and the
//! OTel gauge `gen_ai.secret.expires_in_seconds` is recorded when expiry is
//! within 24 h.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use secrecy::SecretString;

use super::{SecretError, SecretManager, SecretMetadata, SecretValue, check_rotation_warning};

/// A typed router that dispatches secret operations to the correct backend
/// based on the URI scheme prefix of the secret name.
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use liter_llm_proxy::secrets::{SecretManagerRegistry, EnvVarSecretManager};
///
/// let registry = SecretManagerRegistry::builder()
///     .register("env", Arc::new(EnvVarSecretManager::new()))
///     .default(Arc::new(EnvVarSecretManager::new()))
///     .build();
///
/// let (backend, path) = registry.resolve("env://MY_API_KEY");
/// assert_eq!(path, "MY_API_KEY");
/// ```
pub struct SecretManagerRegistry {
    backends: HashMap<String, Arc<dyn SecretManager>>,
    default: Arc<dyn SecretManager>,
}

impl SecretManagerRegistry {
    /// Return a builder for the registry.
    pub fn builder() -> SecretManagerRegistryBuilder {
        SecretManagerRegistryBuilder::default()
    }

    /// Resolve a secret name to the correct backend and stripped path.
    ///
    /// If `name` contains `"://"`, the part before `"://"` is used as the
    /// backend key and the part after is used as the path forwarded to the
    /// backend.  Otherwise the full `name` is forwarded to the default backend.
    ///
    /// Returns `(backend, path)`.
    pub fn resolve<'a>(&'a self, name: &'a str) -> (Arc<dyn SecretManager>, &'a str) {
        if let Some(idx) = name.find("://") {
            let scheme = &name[..idx];
            let path = &name[idx + 3..];
            if let Some(backend) = self.backends.get(scheme) {
                return (Arc::clone(backend), path);
            }
        }
        (Arc::clone(&self.default), name)
    }

    /// Fetch a secret through the registry.
    ///
    /// Routes by URI scheme, then emits a rotation warning if `expires_at` is
    /// within 24 h.
    pub fn get<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<SecretValue, SecretError>> + Send + 'a>> {
        Box::pin(async move {
            let (backend, path) = self.resolve(name);
            let result = backend.get(path).await?;
            check_rotation_warning(path, &result.metadata, backend.backend());
            Ok(result)
        })
    }

    /// Write a secret through the registry.
    pub fn set<'a>(
        &'a self,
        name: &'a str,
        value: SecretString,
        tags: HashMap<String, String>,
    ) -> Pin<Box<dyn Future<Output = Result<SecretMetadata, SecretError>> + Send + 'a>> {
        Box::pin(async move {
            let (backend, path) = self.resolve(name);
            backend.set(path, value, tags).await
        })
    }

    /// Delete a secret through the registry.
    pub fn delete<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<(), SecretError>> + Send + 'a>> {
        Box::pin(async move {
            let (backend, path) = self.resolve(name);
            backend.delete(path).await
        })
    }

    /// Resolve a potentially-prefixed value as an API key.
    ///
    /// - If `value` starts with a known scheme (`aws://`, `vault://`, `env://`),
    ///   fetch it from the registry and return the secret value string.
    /// - Otherwise treat `value` as a literal and return it as-is.
    ///
    /// This is the integration point for `ModelEntry::api_key` resolution.
    pub async fn resolve_api_key(&self, value: &str) -> Result<String, SecretError> {
        if has_secret_scheme(value) {
            use secrecy::ExposeSecret;
            let sv = self.get(value).await?;
            Ok(sv.value.expose_secret().to_owned())
        } else {
            Ok(value.to_owned())
        }
    }
}

/// Returns `true` if the string looks like a secret URI (`scheme://...`).
pub fn has_secret_scheme(value: &str) -> bool {
    matches!(
        value.split_once("://").map(|(s, _)| s),
        Some("aws") | Some("vault") | Some("env")
    )
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`SecretManagerRegistry`].
#[derive(Default)]
pub struct SecretManagerRegistryBuilder {
    backends: HashMap<String, Arc<dyn SecretManager>>,
    default: Option<Arc<dyn SecretManager>>,
}

impl SecretManagerRegistryBuilder {
    /// Register a backend under the given URI scheme (without `://`).
    pub fn register(mut self, scheme: impl Into<String>, backend: Arc<dyn SecretManager>) -> Self {
        self.backends.insert(scheme.into(), backend);
        self
    }

    /// Set the default backend (used when no scheme matches).
    pub fn default_backend(mut self, backend: Arc<dyn SecretManager>) -> Self {
        self.default = Some(backend);
        self
    }

    /// Build the registry.
    ///
    /// # Panics
    ///
    /// Panics if no default backend was configured.
    pub fn build(self) -> SecretManagerRegistry {
        SecretManagerRegistry {
            backends: self.backends,
            default: self.default.expect("SecretManagerRegistry requires a default backend"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};

    use tracing_test::traced_test;

    use super::*;
    use crate::secrets::{EnvVarSecretManager, SecretMetadata, check_rotation_warning};

    fn make_registry() -> SecretManagerRegistry {
        let env: Arc<dyn SecretManager> = Arc::new(EnvVarSecretManager::new());
        SecretManagerRegistry::builder()
            .register("env", Arc::clone(&env))
            .default_backend(Arc::clone(&env))
            .build()
    }

    // ── Routing ─────────────────────────────────────────────────────────────

    #[test]
    fn secret_manager_registry_routes_by_prefix() {
        let env: Arc<dyn SecretManager> = Arc::new(EnvVarSecretManager::new());
        let registry = SecretManagerRegistry::builder()
            .register("env", Arc::clone(&env))
            .default_backend(Arc::clone(&env))
            .build();

        let (backend, path) = registry.resolve("env://MY_API_KEY");
        assert_eq!(backend.backend(), "env");
        assert_eq!(path, "MY_API_KEY");
    }

    #[test]
    fn registry_unknown_scheme_falls_back_to_default() {
        let env: Arc<dyn SecretManager> = Arc::new(EnvVarSecretManager::new());
        let registry = SecretManagerRegistry::builder()
            .default_backend(Arc::clone(&env))
            .build();

        let (backend, path) = registry.resolve("literalkey");
        assert_eq!(backend.backend(), "env");
        assert_eq!(path, "literalkey");
    }

    #[test]
    fn registry_resolve_strips_scheme_from_path() {
        let env: Arc<dyn SecretManager> = Arc::new(EnvVarSecretManager::new());
        let registry = SecretManagerRegistry::builder()
            .register("aws", Arc::clone(&env))
            .default_backend(Arc::clone(&env))
            .build();

        let (backend, path) = registry.resolve("aws://prod/api-key");
        assert_eq!(backend.backend(), "env");
        assert_eq!(path, "prod/api-key");
    }

    // ── has_secret_scheme ───────────────────────────────────────────────────

    #[test]
    fn has_secret_scheme_detects_known_schemes() {
        assert!(has_secret_scheme("aws://prod/key"));
        assert!(has_secret_scheme("vault://secret/db"));
        assert!(has_secret_scheme("env://MY_VAR"));
    }

    #[test]
    fn has_secret_scheme_rejects_literal_values() {
        assert!(!has_secret_scheme("sk-openai-literal"));
        assert!(!has_secret_scheme(""));
        assert!(!has_secret_scheme("https://example.com"));
        assert!(!has_secret_scheme("gcp://not-supported-yet"));
    }

    // ── resolve_api_key ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn resolve_api_key_returns_literal_when_no_scheme() {
        let registry = make_registry();
        let result = registry.resolve_api_key("sk-literal-key").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sk-literal-key");
    }

    #[tokio::test]
    async fn resolve_api_key_fetches_env_var_when_scheme_present() {
        unsafe {
            std::env::set_var("LITER_REGISTRY_TEST_KEY", "fetched-value");
        }

        let registry = make_registry();
        let result = registry.resolve_api_key("env://LITER_REGISTRY_TEST_KEY").await;

        unsafe {
            std::env::remove_var("LITER_REGISTRY_TEST_KEY");
        }

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fetched-value");
    }

    // ── Rotation warning ────────────────────────────────────────────────────

    #[traced_test]
    #[test]
    fn secret_manager_rotation_warning_within_24h() {
        let meta = SecretMetadata {
            name: "expiring-key".to_owned(),
            version: "1".to_owned(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            // Expires in 23 hours — within the 24h threshold.
            expires_at: Some(SystemTime::now() + Duration::from_secs(23 * 60 * 60)),
            tags: HashMap::new(),
        };
        check_rotation_warning("expiring-key", &meta, "env");
        assert!(logs_contain("secret is expiring soon"));
    }

    #[traced_test]
    #[test]
    fn no_rotation_warning_when_not_expiring_soon() {
        let meta = SecretMetadata {
            name: "safe-key".to_owned(),
            version: "1".to_owned(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            // Expires in 48 hours — outside the 24h threshold.
            expires_at: Some(SystemTime::now() + Duration::from_secs(48 * 60 * 60)),
            tags: HashMap::new(),
        };
        check_rotation_warning("safe-key", &meta, "env");
        assert!(!logs_contain("secret is expiring soon"));
    }
}
