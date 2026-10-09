//! Binding-friendly API surface for FFI/polyglot bindings.
//!
//! This module provides simplified constructors that avoid trait objects and
//! opaque types, making it straightforward for alef-generated bindings to
//! create a [`DefaultClient`] from plain scalar values.

use std::time::Duration;

#[cfg(all(feature = "native-http", feature = "tower"))]
use std::sync::Arc;

use crate::DefaultClient;
#[cfg(all(feature = "native-http", feature = "tower"))]
use crate::client::managed::ManagedClient;
use crate::client::{ClientConfigBuilder, FileConfig, config_file::FileProviderConfig};
use crate::error::{LiterLlmError, Result};
use crate::provider::custom::register_custom_provider;

/// Binding-friendly options for constructing a client without an unbounded response body.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ClientOptions {
    /// Provider API key; an empty value omits the authentication header.
    pub api_key: String,
    /// Optional provider base URL override.
    pub base_url: Option<String>,
    /// Optional request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Optional retry count for retryable provider failures.
    pub max_retries: Option<u32>,
    /// Optional maximum number of response bytes retained in memory.
    pub max_response_bytes: Option<u64>,
    /// Optional model or provider hint used for initial provider selection.
    pub model_hint: Option<String>,
}

/// Create a client from binding-friendly options.
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] for a zero or platform-overflowing
/// `max_response_bytes`, plus the errors documented by [`create_client`].
#[cfg(feature = "native-http")]
pub fn create_client_with_options(options: ClientOptions) -> Result<DefaultClient> {
    let mut builder = ClientConfigBuilder::new(options.api_key);
    if let Some(url) = options.base_url {
        builder = builder.base_url(url);
    }
    if let Some(secs) = options.timeout_secs {
        builder = builder.timeout(Duration::from_secs(secs));
    }
    if let Some(retries) = options.max_retries {
        builder = builder.max_retries(retries);
    }
    if let Some(limit) = options.max_response_bytes {
        let limit = usize::try_from(limit).map_err(|_| LiterLlmError::BadRequest {
            message: "max_response_bytes exceeds the platform limit".into(),
            status: 400,
        })?;
        builder = builder.max_response_bytes(limit)?;
    }
    DefaultClient::new(builder.build(), options.model_hint.as_deref())
}

/// Create a new LLM client with simple scalar configuration.
///
/// This is the primary binding entry-point. All parameters except `api_key`
/// are optional — omitting them uses the same defaults as
/// [`ClientConfigBuilder`].
///
/// # Errors
///
/// Returns [`LiterLlmError`] if the underlying HTTP client cannot be
/// constructed, or if the resolved provider configuration is invalid.
pub fn create_client(
    api_key: String,
    base_url: Option<String>,
    timeout_secs: Option<u64>,
    max_retries: Option<u32>,
    model_hint: Option<String>,
) -> Result<DefaultClient> {
    let mut builder = ClientConfigBuilder::new(api_key);

    if let Some(url) = base_url {
        builder = builder.base_url(url);
    }
    if let Some(secs) = timeout_secs {
        builder = builder.timeout(Duration::from_secs(secs));
    }
    if let Some(retries) = max_retries {
        builder = builder.max_retries(retries);
    }

    let config = builder.build();
    DefaultClient::new(config, model_hint.as_deref())
}

/// Create a new LLM client from a JSON string.
///
/// The JSON object accepts the same fields as `liter-llm.toml` (snake_case).
/// Middleware keys (`cache`, `budget`, `cooldown_secs`, `rate_limit`,
/// `in_flight_limit`, `health_check_secs`, `cost_tracking`, `tracing`) are
/// applied: the returned client routes the `LlmClient` methods (`chat`,
/// `chat_stream`, `embed`, `list_models`, `image_generate`, `speech`,
/// `transcribe`, `moderate`, `rerank`, `search`, `ocr`) through the managed
/// Tower stack. File, batch and response operations bypass it. Each `providers`
/// entry is registered process-wide via [`register_custom_provider`](crate::register_custom_provider).
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] if `json` is not valid JSON, contains
/// unknown fields, or sets middleware keys in a build without the `tower` and
/// `native-http` features. Provider registration failures are propagated.
pub fn create_client_from_json(json: &str) -> Result<DefaultClient> {
    let file_config: FileConfig = serde_json::from_str(json).map_err(|error| LiterLlmError::BadRequest {
        message: format!("invalid client config JSON: {error}"),
        status: 400,
    })?;

    #[cfg(not(all(feature = "native-http", feature = "tower")))]
    reject_unsupported_middleware(&file_config)?;

    let model_hint = file_config.model_hint.clone();
    let providers: Vec<_> = file_config
        .providers()
        .iter()
        .map(FileProviderConfig::to_custom_provider_config)
        .collect();
    let config = file_config.into_builder().build();
    let client = DefaultClient::new(config.clone(), model_hint.as_deref())?;

    for provider in providers {
        register_custom_provider(provider)?;
    }

    #[cfg(all(feature = "native-http", feature = "tower"))]
    {
        let managed = ManagedClient::from_client(client.clone(), &config)?;
        if managed.has_middleware() {
            return Ok(client.with_managed(Arc::new(managed)));
        }
    }

    Ok(client)
}

/// Fail loudly instead of silently dropping middleware keys this build cannot apply.
#[cfg(not(all(feature = "native-http", feature = "tower")))]
fn reject_unsupported_middleware(file_config: &FileConfig) -> Result<()> {
    let set: Vec<&str> = [
        ("cache", file_config.cache.is_some()),
        ("budget", file_config.budget.is_some()),
        ("cooldown_secs", file_config.cooldown_secs.is_some()),
        ("rate_limit", file_config.rate_limit.is_some()),
        ("in_flight_limit", file_config.in_flight_limit.is_some()),
        ("health_check_secs", file_config.health_check_secs.is_some()),
        ("cost_tracking", file_config.cost_tracking == Some(true)),
        ("tracing", file_config.tracing == Some(true)),
    ]
    .into_iter()
    .filter_map(|(key, present)| present.then_some(key))
    .collect();

    if set.is_empty() {
        return Ok(());
    }
    Err(LiterLlmError::BadRequest {
        message: format!(
            "client config keys [{}] require the `tower` and `native-http` features, which this build lacks",
            set.join(", ")
        ),
        status: 400,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_client_with_defaults_succeeds() {
        assert!(create_client("sk-test".to_owned(), None, None, None, None).is_ok());
    }

    #[test]
    fn create_client_with_all_options_succeeds() {
        assert!(
            create_client(
                "sk-test".to_owned(),
                Some("https://api.openai.com/v1".to_owned()),
                Some(30),
                Some(5),
                Some("openai/gpt-4o".to_owned()),
            )
            .is_ok()
        );
    }

    #[test]
    fn create_client_from_json_minimal_succeeds() {
        assert!(create_client_from_json(r#"{"api_key": "sk-test"}"#).is_ok());
    }

    #[test]
    fn create_client_from_json_accepts_response_body_limit() {
        assert!(create_client_from_json(r#"{"api_key":"sk-test","max_response_bytes":33554432}"#).is_ok());
    }

    #[test]
    fn create_client_from_json_rejects_zero_response_body_limit() {
        let error = create_client_from_json(r#"{"api_key":"sk-test","max_response_bytes":0}"#)
            .err()
            .expect("zero must be rejected");
        assert!(matches!(error, LiterLlmError::BadRequest { status: 400, .. }));
    }

    #[cfg(feature = "native-http")]
    #[test]
    fn create_client_with_options_accepts_response_body_limit() {
        let client = create_client_with_options(ClientOptions {
            api_key: "sk-test".into(),
            max_response_bytes: Some(32 * 1024 * 1024),
            ..ClientOptions::default()
        });
        assert!(client.is_ok());
    }

    #[cfg(not(all(feature = "native-http", feature = "tower")))]
    #[test]
    fn create_client_from_json_rejects_middleware_keys_without_tower() {
        let err = create_client_from_json(
            r#"{"api_key": "sk-test", "cache": {"max_entries": 8}, "budget": {"global_limit": 1.0}, "tracing": false}"#,
        )
        .err()
        .expect("middleware keys must not be silently dropped");
        match err {
            LiterLlmError::BadRequest { message, status } => {
                assert_eq!(status, 400);
                assert!(message.contains("cache, budget"), "message = {message}");
                assert!(
                    !message.contains("tracing"),
                    "tracing=false is not a middleware request"
                );
            }
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn create_client_from_json_invalid_json_returns_error() {
        let result = create_client_from_json("not json {{{");
        let err = result.err().expect("invalid JSON should return an error");
        assert!(matches!(err, LiterLlmError::BadRequest { .. }));
    }
}
