//! TOML-based configuration file loading.
//!
//! Load client configuration from `liter-llm.toml` files with auto-discovery
//! (searches current directory and parents).

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use crate::error::{LiterLlmError, Result};

/// TOML file representation of client configuration.
///
/// All fields are optional — missing fields use defaults from [`ClientConfigBuilder`].
/// Convert to a builder via [`FileConfig::into_builder`].
///
/// # Example `liter-llm.toml`
///
/// ```toml
/// api_key = "sk-..."
/// base_url = "https://api.openai.com/v1"
/// timeout_secs = 120
/// max_retries = 5
///
/// [cache]
/// max_entries = 512
/// ttl_seconds = 600
/// backend = "memory"
///
/// [budget]
/// global_limit = 50.0
/// enforcement = "hard"
///
/// [[providers]]
/// name = "my-provider"
/// base_url = "https://my-llm.example.com/v1"
/// model_prefixes = ["my-provider/"]
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct FileConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model_hint: Option<String>,
    pub timeout_secs: Option<u64>,
    pub max_retries: Option<u32>,
    pub extra_headers: Option<HashMap<String, String>>,
    pub cache: Option<FileCacheConfig>,
    pub budget: Option<FileBudgetConfig>,
    pub cooldown_secs: Option<u64>,
    pub rate_limit: Option<FileRateLimitConfig>,
    pub health_check_secs: Option<u64>,
    pub cost_tracking: Option<bool>,
    pub tracing: Option<bool>,
    pub providers: Option<Vec<FileProviderConfig>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct FileCacheConfig {
    pub max_entries: Option<usize>,
    pub ttl_seconds: Option<u64>,
    pub backend: Option<String>,
    pub backend_config: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct FileBudgetConfig {
    pub global_limit: Option<f64>,
    pub model_limits: Option<HashMap<String, f64>>,
    pub enforcement: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct FileRateLimitConfig {
    pub rpm: Option<u32>,
    pub tpm: Option<u64>,
    pub window_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(alef, alef(skip))]
pub struct FileProviderConfig {
    pub name: String,
    pub base_url: String,
    pub auth_header: Option<String>,
    pub model_prefixes: Vec<String>,
}

#[cfg_attr(alef, alef(skip))]
impl FileConfig {
    /// Load from a TOML file path.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| LiterLlmError::InternalError {
            message: format!("failed to read config file {}: {e}", path.display()),
        })?;
        Self::from_toml_str(&content)
    }

    /// Parse from a TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| LiterLlmError::InternalError {
            message: format!("invalid TOML config: {e}"),
        })
    }

    /// Discover `liter-llm.toml` by walking from current directory to filesystem root.
    ///
    /// Returns `Ok(None)` if no config file is found.
    pub fn discover() -> Result<Option<Self>> {
        let mut current = std::env::current_dir().map_err(|e| LiterLlmError::InternalError {
            message: format!("failed to get current directory: {e}"),
        })?;
        loop {
            let config_path = current.join("liter-llm.toml");
            if config_path.exists() {
                return Ok(Some(Self::from_toml_file(config_path)?));
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }
        Ok(None)
    }

    /// Convert into a [`ClientConfigBuilder`](super::ClientConfigBuilder),
    /// applying all fields that are set.
    ///
    /// Fields not present in the TOML file use the builder's defaults.
    pub fn into_builder(self) -> super::ClientConfigBuilder {
        let api_key = self.api_key.unwrap_or_default();
        let mut builder = super::ClientConfigBuilder::new(api_key);

        if let Some(url) = self.base_url {
            builder = builder.base_url(url);
        }
        if let Some(t) = self.timeout_secs {
            builder = builder.timeout(Duration::from_secs(t));
        }
        if let Some(r) = self.max_retries {
            builder = builder.max_retries(r);
        }

        // Extra headers: push validated headers directly to the builder's
        // internal config.  We cannot use `builder.header()` in a loop because
        // it consumes `self` and on `Err` the builder is lost.  Since we are
        // in the same crate, we can access `pub(crate)` fields.
        #[cfg(any(feature = "native-http", feature = "wasm-http"))]
        if let Some(headers) = self.extra_headers {
            for (k, v) in headers {
                // Validate header name and value before pushing.
                if reqwest::header::HeaderName::from_bytes(k.as_bytes()).is_ok()
                    && reqwest::header::HeaderValue::from_str(&v).is_ok()
                {
                    builder.config.extra_headers.push((k, v));
                }
            }
        }

        // Tower middleware configs
        #[cfg(feature = "tower")]
        {
            // Cache
            if let Some(cache) = self.cache {
                use crate::tower::{CacheBackend, CacheConfig};
                let backend = match cache.backend.as_deref() {
                    Some("memory") | None => CacheBackend::Memory,
                    #[cfg(feature = "opendal-cache")]
                    Some(scheme) => CacheBackend::OpenDal {
                        scheme: scheme.to_string(),
                        config: cache.backend_config.unwrap_or_default(),
                    },
                    #[cfg(not(feature = "opendal-cache"))]
                    Some(_) => CacheBackend::Memory,
                };
                builder = builder.cache(CacheConfig {
                    max_entries: cache.max_entries.unwrap_or(256),
                    ttl: Duration::from_secs(cache.ttl_seconds.unwrap_or(300)),
                    backend,
                });
            }

            // Budget
            if let Some(budget) = self.budget {
                use crate::tower::{BudgetConfig, Enforcement};
                builder = builder.budget(BudgetConfig {
                    global_limit: budget.global_limit,
                    model_limits: budget.model_limits.unwrap_or_default(),
                    enforcement: match budget.enforcement.as_deref() {
                        Some("soft") => Enforcement::Soft,
                        _ => Enforcement::Hard,
                    },
                });
            }

            // Cooldown
            if let Some(secs) = self.cooldown_secs {
                builder = builder.cooldown(Duration::from_secs(secs));
            }

            // Rate limit
            if let Some(rl) = self.rate_limit {
                use crate::tower::RateLimitConfig;
                builder = builder.rate_limit(RateLimitConfig {
                    rpm: rl.rpm,
                    tpm: rl.tpm,
                    window: Duration::from_secs(rl.window_seconds.unwrap_or(60)),
                });
            }

            // Health check
            if let Some(secs) = self.health_check_secs {
                builder = builder.health_check(Duration::from_secs(secs));
            }

            // Cost tracking
            if let Some(ct) = self.cost_tracking {
                builder = builder.cost_tracking(ct);
            }

            // Tracing
            if let Some(t) = self.tracing {
                builder = builder.tracing(t);
            }
        }

        builder
    }

    /// Get the custom provider configurations from this file config.
    pub fn providers(&self) -> &[FileProviderConfig] {
        self.providers.as_deref().unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml = r#"api_key = "sk-test""#;
        let config = FileConfig::from_toml_str(toml).expect("TOML should parse");
        assert_eq!(config.api_key.as_deref(), Some("sk-test"));
        assert!(config.base_url.is_none());
        assert!(config.cache.is_none());
    }

    #[test]
    fn parse_full_config() {
        let toml = r#"
api_key = "sk-test"
base_url = "https://api.example.com/v1"
model_hint = "openai"
timeout_secs = 120
max_retries = 5
cooldown_secs = 30
health_check_secs = 60
cost_tracking = true
tracing = true

[cache]
max_entries = 512
ttl_seconds = 600
backend = "memory"

[budget]
global_limit = 50.0
enforcement = "hard"

[budget.model_limits]
"openai/gpt-4o" = 25.0

[rate_limit]
rpm = 60
tpm = 100000

[extra_headers]
"X-Custom" = "value"

[[providers]]
name = "my-provider"
base_url = "https://my-llm.example.com/v1"
auth_header = "Authorization"
model_prefixes = ["my-provider/"]
"#;
        let config = FileConfig::from_toml_str(toml).expect("TOML should parse");
        assert_eq!(config.timeout_secs, Some(120));
        assert_eq!(config.max_retries, Some(5));
        assert!(config.cache.is_some());
        assert!(config.budget.is_some());
        assert_eq!(config.providers().len(), 1);
        assert_eq!(config.providers()[0].name, "my-provider");
    }

    #[test]
    fn rejects_unknown_fields() {
        let toml = r#"
api_key = "sk-test"
unknown_field = true
"#;
        assert!(FileConfig::from_toml_str(toml).is_err());
    }

    #[test]
    fn into_builder_produces_valid_config() {
        let toml = r#"
api_key = "sk-test"
timeout_secs = 30
max_retries = 2
"#;
        let file_config = FileConfig::from_toml_str(toml).expect("TOML should parse");
        let config = file_config.into_builder().build();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 2);
    }

    #[test]
    fn empty_config_is_valid() {
        let config = FileConfig::from_toml_str("").expect("TOML should parse");
        assert!(config.api_key.is_none());
    }
}
