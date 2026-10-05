//! Canonical, binding-friendly LLM client configuration.
//!
//! [`LlmConfig`] is plain serde data (no secrets, no trait objects, no
//! `Duration`) so it can be constructed directly from JSON/TOML/YAML in any
//! language binding and converted into a [`super::ClientConfigBuilder`] via
//! [`LlmConfig::into_client_builder`].
//!
//! Unlike [`super::FileConfig`] (which is `Deserialize`-only and hidden from
//! bindings), every type in this module is `Serialize + Deserialize` and
//! generates binding surface in every target language.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Canonical configuration for an LLM client.
///
/// All fields except `model` are optional so that partially-specified
/// configs (e.g. from environment-driven defaults) round-trip cleanly.
/// Convert to a runtime client configuration via
/// [`LlmConfig::into_client_builder`].
///
/// `temperature` and `max_tokens` are request-time parameters rather than
/// client-level settings; they are carried on this struct for callers to
/// read when building individual requests, and are intentionally **not**
/// mapped by [`LlmConfig::into_client_builder`].
///
/// Implements `Debug` manually (see below) so `api_key` and header values are
/// redacted rather than printed in full.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct LlmConfig {
    /// Model identifier (e.g. `"gpt-4o"`, `"bedrock/anthropic.claude-3-sonnet-20240229-v1:0"`).
    pub model: String,
    /// API key for authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Override base URL. When set, all requests go here and provider
    /// auto-detection is skipped. A leading `provider/` segment that names a
    /// known provider (`openai/gpt-4o-mini`) is stripped from the model sent
    /// to the server; other prefixes such as `meta-llama/llama-3` are
    /// forwarded unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Request timeout, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    /// Maximum number of retries on 429 / 5xx responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    /// Sampling temperature for requests built from this config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Maximum number of tokens to generate for requests built from this config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Automatically load the API key from the provider's environment variable
    /// when no explicit key is provided (default: `true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_env: Option<bool>,
    /// Extra headers sent on every request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Custom provider configurations, in addition to the built-in providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<LlmProviderConfig>>,
    /// Response cache configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<LlmCacheConfig>,
    /// Budget enforcement configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<LlmBudgetConfig>,
    /// Per-model rate limiting configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<LlmRateLimitConfig>,
    /// Global per-client in-flight provider request limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_flight_limit: Option<LlmInFlightLimitConfig>,
    /// Enable per-request cost tracking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_tracking: Option<bool>,
    /// Enable OpenTelemetry-compatible tracing spans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracing: Option<bool>,
    /// Cooldown duration after transient errors, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_secs: Option<u64>,
    /// Background health check interval, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_check_secs: Option<u64>,
    /// AWS Bedrock configuration (region, credentials, cross-region routing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bedrock: Option<BedrockConfig>,
}

/// Response cache configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct LlmCacheConfig {
    /// Maximum number of cached entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_entries: Option<usize>,
    /// Cache entry time-to-live, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    /// Cache backend name (e.g. `"memory"`, or an `opendal` scheme).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    /// Backend-specific configuration key/value pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_config: Option<HashMap<String, String>>,
}

/// Budget enforcement configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct LlmBudgetConfig {
    /// Global spend limit in USD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_limit: Option<f64>,
    /// Per-model spend limits in USD, keyed by model name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_limits: Option<HashMap<String, f64>>,
    /// Enforcement mode: `"hard"` (reject over-budget requests) or `"soft"` (log only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enforcement: Option<String>,
}

/// Per-model rate limiting configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct LlmRateLimitConfig {
    /// Requests per minute limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rpm: Option<u32>,
    /// Tokens per minute limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tpm: Option<u64>,
    /// Rate limit window, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_seconds: Option<u64>,
}

/// Global per-client in-flight provider request limit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct LlmInFlightLimitConfig {
    /// Maximum simultaneously outstanding provider requests. `None` means unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_in_flight: Option<usize>,
}

/// A custom provider configuration entry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct LlmProviderConfig {
    /// Provider name, used to key model prefix matching.
    pub name: String,
    /// Base URL for the provider's OpenAI-compatible API.
    pub base_url: String,
    /// Header name used to carry the API key (defaults to `Authorization` when unset).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_header: Option<String>,
    /// Model name prefixes routed to this provider (e.g. `["my-provider/"]`).
    #[serde(default)]
    pub model_prefixes: Vec<String>,
}

/// AWS Bedrock configuration.
///
/// All fields are optional; anything left unset falls back to the standard
/// AWS environment variables (`AWS_DEFAULT_REGION` / `AWS_REGION`,
/// `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`,
/// `BEDROCK_CROSS_REGION`).
///
/// A Bedrock API key in `AWS_BEARER_TOKEN_BEDROCK` authenticates on its own and
/// needs none of the credential fields below. It is read per request rather than
/// from this struct; in a build that can sign (the `bedrock` feature) a credential
/// set here takes precedence over it.
///
/// Implements `Debug` manually (see below) so the AWS credential fields are
/// redacted rather than printed in full.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct BedrockConfig {
    /// AWS region (e.g. `"us-east-1"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Cross-region inference profile prefix (e.g. `"us"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_region_prefix: Option<String>,
    /// Explicit AWS access key ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    /// Explicit AWS secret access key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    /// Explicit AWS session token (temporary credentials).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
}

/// Manual `Debug` impl: `access_key_id`, `secret_access_key`, and
/// `session_token` are AWS credentials and must never appear in full in logs
/// or error messages produced via `{:?}`.
impl std::fmt::Debug for BedrockConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BedrockConfig")
            .field("region", &self.region)
            .field("cross_region_prefix", &self.cross_region_prefix)
            .field("access_key_id", &self.access_key_id.as_ref().map(|_| "[redacted]"))
            .field(
                "secret_access_key",
                &self.secret_access_key.as_ref().map(|_| "[redacted]"),
            )
            .field("session_token", &self.session_token.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

/// Manual `Debug` impl: `api_key` must never appear in full via `{:?}`.
/// Header values are also redacted, since custom auth headers (e.g. a
/// bearer token in a non-standard header) can carry secrets too; header
/// *names* are kept so the shape of the config is still inspectable.
impl std::fmt::Debug for LlmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redacted_headers: Option<Vec<(&str, &str)>> = self.headers.as_ref().map(|headers| {
            let mut kvs: Vec<(&str, &str)> = headers.keys().map(|k| (k.as_str(), "[redacted]")).collect();
            kvs.sort_unstable_by_key(|(k, _)| *k);
            kvs
        });

        f.debug_struct("LlmConfig")
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("base_url", &self.base_url)
            .field("timeout_secs", &self.timeout_secs)
            .field("max_retries", &self.max_retries)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("load_env", &self.load_env)
            .field("headers", &redacted_headers)
            .field("providers", &self.providers)
            .field("cache", &self.cache)
            .field("budget", &self.budget)
            .field("rate_limit", &self.rate_limit)
            .field("in_flight_limit", &self.in_flight_limit)
            .field("cost_tracking", &self.cost_tracking)
            .field("tracing", &self.tracing)
            .field("cooldown_secs", &self.cooldown_secs)
            .field("health_check_secs", &self.health_check_secs)
            .field("bedrock", &self.bedrock)
            .finish()
    }
}

impl LlmConfig {
    /// Convert into a [`super::ClientConfigBuilder`], applying every field
    /// that is set.
    ///
    /// `model`, `temperature`, and `max_tokens` are request-time parameters
    /// and are not mapped onto the builder; read them directly from this
    /// struct when constructing individual requests.
    #[tracing::instrument(level = "debug", skip(self), fields(model = %self.model))]
    pub fn into_client_builder(self) -> super::ClientConfigBuilder {
        let api_key = self.api_key.unwrap_or_default();
        let mut builder = super::ClientConfigBuilder::new(api_key);

        if let Some(enabled) = self.load_env {
            builder = builder.load_env(enabled);
        }
        if let Some(url) = self.base_url {
            builder = builder.base_url(url);
        }
        if let Some(t) = self.timeout_secs {
            builder = builder.timeout(Duration::from_secs(t));
        }
        if let Some(r) = self.max_retries {
            builder = builder.max_retries(r);
        }

        #[cfg(any(feature = "native-http", feature = "wasm-http"))]
        if let Some(headers) = self.headers {
            for (k, v) in headers {
                if reqwest::header::HeaderName::from_bytes(k.as_bytes()).is_ok()
                    && reqwest::header::HeaderValue::from_str(&v).is_ok()
                {
                    builder.config.extra_headers.push((k, v));
                }
            }
        }

        #[cfg(feature = "tower")]
        {
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

            if let Some(secs) = self.cooldown_secs {
                builder = builder.cooldown(Duration::from_secs(secs));
            }

            if let Some(rl) = self.rate_limit {
                use crate::tower::RateLimitConfig;
                builder = builder.rate_limit(RateLimitConfig {
                    rpm: rl.rpm,
                    tpm: rl.tpm,
                    window: Duration::from_secs(rl.window_seconds.unwrap_or(60)),
                });
            }

            if let Some(limit) = self.in_flight_limit {
                use crate::tower::InFlightLimitConfig;
                builder = builder.in_flight_limit(InFlightLimitConfig {
                    max_in_flight: limit.max_in_flight,
                });
            }

            if let Some(secs) = self.health_check_secs {
                builder = builder.health_check(Duration::from_secs(secs));
            }

            if let Some(ct) = self.cost_tracking {
                builder = builder.cost_tracking(ct);
            }

            if let Some(t) = self.tracing {
                builder = builder.tracing(t);
            }
        }

        if let Some(bedrock) = self.bedrock {
            if let Some(region) = bedrock.region {
                builder = builder.bedrock_region(region);
            }
            if let Some(prefix) = bedrock.cross_region_prefix {
                builder = builder.bedrock_cross_region_prefix(prefix);
            }
            if bedrock.access_key_id.is_some() || bedrock.secret_access_key.is_some() {
                builder = builder.bedrock_credentials(
                    bedrock.access_key_id.unwrap_or_default(),
                    bedrock.secret_access_key.unwrap_or_default(),
                    bedrock.session_token,
                );
            }
        }

        builder
    }

    /// Get the custom provider configurations from this config.
    pub fn providers(&self) -> &[LlmProviderConfig] {
        self.providers.as_deref().unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> LlmConfig {
        LlmConfig {
            model: "gpt-4o".to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn default_has_empty_model_and_no_optionals() {
        let config = LlmConfig::default();
        assert_eq!(config.model, "");
        assert!(config.api_key.is_none());
        assert!(config.base_url.is_none());
        assert!(config.bedrock.is_none());
        assert!(config.providers.is_none());
    }

    #[test]
    fn empty_optionals_are_omitted_from_json() {
        let config = minimal();
        let json = serde_json::to_string(&config).expect("serialize should succeed");
        assert_eq!(json, r#"{"model":"gpt-4o"}"#);
    }

    #[test]
    fn empty_optionals_are_omitted_from_toml() {
        let config = minimal();
        let toml = toml::to_string(&config).expect("serialize should succeed");
        assert_eq!(toml.trim(), r#"model = "gpt-4o""#);
    }

    #[test]
    fn json_round_trip_with_parity_fields() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_owned(), "value".to_owned());

        let config = LlmConfig {
            model: "gpt-4o".to_owned(),
            api_key: Some("sk-test".to_owned()),
            base_url: Some("https://api.example.com/v1".to_owned()),
            timeout_secs: Some(120),
            max_retries: Some(5),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            load_env: Some(false),
            headers: Some(headers),
            ..Default::default()
        };

        let json = serde_json::to_string(&config).expect("serialize should succeed");
        let round_tripped: LlmConfig = serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(round_tripped, config);
    }

    #[test]
    fn toml_round_trip_with_bedrock_and_custom_provider() {
        let toml = r#"
model = "bedrock/anthropic.claude-3-sonnet-20240229-v1:0"
api_key = "unused-for-sigv4"

[bedrock]
region = "eu-central-1"
cross_region_prefix = "eu"

[[providers]]
name = "my-provider"
base_url = "https://my-llm.example.com/v1"
model_prefixes = ["my-provider/"]
"#;
        let config: LlmConfig = toml::from_str(toml).expect("TOML should parse");
        assert_eq!(config.model, "bedrock/anthropic.claude-3-sonnet-20240229-v1:0");
        assert_eq!(
            config.bedrock.as_ref().and_then(|b| b.region.as_deref()),
            Some("eu-central-1")
        );
        assert_eq!(
            config.bedrock.as_ref().and_then(|b| b.cross_region_prefix.as_deref()),
            Some("eu")
        );
        assert_eq!(config.providers().len(), 1);
        assert_eq!(config.providers()[0].name, "my-provider");

        let serialized = toml::to_string(&config).expect("serialize should succeed");
        let round_tripped: LlmConfig = toml::from_str(&serialized).expect("re-parse should succeed");
        assert_eq!(round_tripped, config);
    }

    #[test]
    fn into_client_builder_maps_core_fields() {
        let config = LlmConfig {
            model: "gpt-4o".to_owned(),
            api_key: Some("sk-test".to_owned()),
            timeout_secs: Some(30),
            max_retries: Some(2),
            ..Default::default()
        };
        let client_config = config.into_client_builder().build();
        assert_eq!(client_config.timeout, Duration::from_secs(30));
        assert_eq!(client_config.max_retries, 2);
    }

    #[test]
    fn into_client_builder_maps_bedrock_region() {
        let config = LlmConfig {
            model: "bedrock/anthropic.claude-3-sonnet-20240229-v1:0".to_owned(),
            bedrock: Some(BedrockConfig {
                region: Some("eu-central-1".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let client_config = config.into_client_builder().build();
        assert_eq!(client_config.bedrock_region.as_deref(), Some("eu-central-1"));
    }

    #[test]
    fn into_client_builder_maps_bedrock_credentials() {
        let config = LlmConfig {
            model: "bedrock/anthropic.claude-3-sonnet-20240229-v1:0".to_owned(),
            bedrock: Some(BedrockConfig {
                access_key_id: Some("AKIAEXAMPLE".to_owned()),
                secret_access_key: Some("secret".to_owned()),
                session_token: Some("token".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let client_config = config.into_client_builder().build();
        assert_eq!(client_config.bedrock_access_key_id.as_deref(), Some("AKIAEXAMPLE"));
        assert_eq!(client_config.bedrock_secret_access_key.as_deref(), Some("secret"));
        assert_eq!(client_config.bedrock_session_token.as_deref(), Some("token"));
    }

    #[test]
    fn debug_format_redacts_llm_config_api_key_and_header_values() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_owned(), "Bearer super-secret-token".to_owned());

        let config = LlmConfig {
            model: "gpt-4o".to_owned(),
            api_key: Some("sk-live-do-not-leak-me".to_owned()),
            headers: Some(headers),
            ..Default::default()
        };

        let debug_output = format!("{config:?}");
        assert!(
            !debug_output.contains("sk-live-do-not-leak-me"),
            "api_key must not appear in Debug output: {debug_output}"
        );
        assert!(
            !debug_output.contains("super-secret-token"),
            "header value must not appear in Debug output: {debug_output}"
        );
        assert!(
            debug_output.contains("Authorization"),
            "header name should still be visible for inspectability: {debug_output}"
        );
        assert!(
            debug_output.contains("gpt-4o"),
            "non-secret fields should still be visible: {debug_output}"
        );
    }

    #[test]
    fn debug_format_redacts_bedrock_config_credentials() {
        let bedrock = BedrockConfig {
            region: Some("us-east-1".to_owned()),
            access_key_id: Some("AKIALEAKEDVALUE".to_owned()),
            secret_access_key: Some("do-not-leak-secret".to_owned()),
            session_token: Some("do-not-leak-token".to_owned()),
            ..Default::default()
        };

        let debug_output = format!("{bedrock:?}");
        assert!(
            !debug_output.contains("AKIALEAKEDVALUE"),
            "access_key_id must not appear in Debug output: {debug_output}"
        );
        assert!(
            !debug_output.contains("do-not-leak-secret"),
            "secret_access_key must not appear in Debug output: {debug_output}"
        );
        assert!(
            !debug_output.contains("do-not-leak-token"),
            "session_token must not appear in Debug output: {debug_output}"
        );
        assert!(
            debug_output.contains("us-east-1"),
            "non-secret fields should still be visible: {debug_output}"
        );
    }

    #[test]
    fn debug_format_redacts_bedrock_credentials_nested_under_llm_config() {
        let config = LlmConfig {
            model: "bedrock/anthropic.claude-3-sonnet-20240229-v1:0".to_owned(),
            bedrock: Some(BedrockConfig {
                secret_access_key: Some("nested-leak-check".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let debug_output = format!("{config:?}");
        assert!(
            !debug_output.contains("nested-leak-check"),
            "bedrock secret nested inside LlmConfig must not leak: {debug_output}"
        );
    }
}
