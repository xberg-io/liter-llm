//! [`EnvVarSecretManager`] — reads secrets from process environment variables.
//!
//! This backend is always available with no external dependencies.  It is the
//! recommended fallback for local development and CI environments.
//!
//! - `get` maps the `name` directly to `std::env::var(name)`.
//! - `set` and `delete` always return [`SecretError::PermissionDenied`].

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use secrecy::SecretString;

use super::{SecretError, SecretManager, SecretMetadata, SecretValue};

/// A read-only secret manager backed by process environment variables.
///
/// Secret names are looked up verbatim with [`std::env::var`].
///
/// # Thread safety
///
/// `EnvVarSecretManager` holds no mutable state; all operations are
/// `Send + Sync`.
#[derive(Debug, Clone, Default)]
pub struct EnvVarSecretManager;

impl EnvVarSecretManager {
    /// Create a new `EnvVarSecretManager`.
    pub fn new() -> Self {
        Self
    }
}

impl SecretManager for EnvVarSecretManager {
    fn backend(&self) -> &'static str {
        "env"
    }

    fn get<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<SecretValue, SecretError>> + Send + 'a>> {
        let name = name.to_owned();
        Box::pin(async move {
            match std::env::var(&name) {
                Ok(raw) => {
                    let now = SystemTime::now();
                    Ok(SecretValue {
                        value: SecretString::from(raw),
                        metadata: SecretMetadata {
                            name: name.clone(),
                            version: "current".to_owned(),
                            created_at: now,
                            updated_at: now,
                            expires_at: None,
                            tags: HashMap::new(),
                        },
                    })
                }
                Err(std::env::VarError::NotPresent) => Err(SecretError::NotFound(name)),
                Err(std::env::VarError::NotUnicode(_)) => Err(SecretError::backend_msg(format!(
                    "env var '{name}' contains invalid UTF-8"
                ))),
            }
        })
    }

    fn set<'a>(
        &'a self,
        name: &'a str,
        _value: SecretString,
        _tags: HashMap<String, String>,
    ) -> Pin<Box<dyn Future<Output = Result<SecretMetadata, SecretError>> + Send + 'a>> {
        let name = name.to_owned();
        Box::pin(async move { Err(SecretError::PermissionDenied(name)) })
    }

    fn delete<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = Result<(), SecretError>> + Send + 'a>> {
        let name = name.to_owned();
        Box::pin(async move { Err(SecretError::PermissionDenied(name)) })
    }
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;

    #[tokio::test]
    async fn secret_manager_env_returns_value_from_env_var() {
        // ~keep SAFETY: this test uses a unique env var, so parallel tests do not collide.
        unsafe {
            std::env::set_var("LITER_TEST_SECRET_12345", "supersecret");
        }

        let mgr = EnvVarSecretManager::new();
        let result = mgr.get("LITER_TEST_SECRET_12345").await;

        // ~keep SAFETY: clean up the test-only env var before any assertion can panic.
        unsafe {
            std::env::remove_var("LITER_TEST_SECRET_12345");
        }

        let val = result.expect("should return Ok");
        assert_eq!(val.value.expose_secret(), "supersecret");
        assert_eq!(val.metadata.name, "LITER_TEST_SECRET_12345");
        assert_eq!(val.metadata.version, "current");
        assert!(val.metadata.expires_at.is_none());
    }

    #[tokio::test]
    async fn secret_manager_env_not_found_returns_not_found_error() {
        let mgr = EnvVarSecretManager::new();
        let result = mgr.get("LITER_SURELY_NONEXISTENT_VAR_99999").await;
        assert!(matches!(result, Err(SecretError::NotFound(ref n)) if n == "LITER_SURELY_NONEXISTENT_VAR_99999"));
    }

    #[tokio::test]
    async fn secret_manager_env_set_returns_permission_denied() {
        let mgr = EnvVarSecretManager::new();
        let result = mgr.set("MY_SECRET", SecretString::from("value"), HashMap::new()).await;
        assert!(matches!(result, Err(SecretError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn secret_manager_env_delete_returns_permission_denied() {
        let mgr = EnvVarSecretManager::new();
        let result = mgr.delete("MY_SECRET").await;
        assert!(matches!(result, Err(SecretError::PermissionDenied(_))));
    }

    #[test]
    fn env_backend_identifier() {
        assert_eq!(EnvVarSecretManager::new().backend(), "env");
    }
}
