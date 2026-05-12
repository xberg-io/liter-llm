//! Binding-friendly API surface for FFI/polyglot bindings.
//!
//! This module provides simplified constructors that avoid trait objects and
//! opaque types, making it straightforward for alef-generated bindings to
//! create a [`DefaultClient`] from plain scalar values.

use std::time::Duration;

use crate::DefaultClient;
use crate::client::{ClientConfigBuilder, FileConfig};
use crate::error::{LiterLlmError, Result};

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
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] if `json` is not valid JSON or
/// contains unknown fields.
pub fn create_client_from_json(json: &str) -> Result<DefaultClient> {
    let file_config: FileConfig = serde_json::from_str(json).map_err(|error| LiterLlmError::BadRequest {
        message: format!("invalid client config JSON: {error}"),
        status: 400,
    })?;

    let model_hint = file_config.model_hint.clone();
    let config = file_config.into_builder().build();
    DefaultClient::new(config, model_hint.as_deref())
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
    fn create_client_from_json_invalid_json_returns_error() {
        let result = create_client_from_json("not json {{{");
        let err = result.err().expect("invalid JSON should return an error");
        assert!(matches!(err, LiterLlmError::BadRequest { .. }));
    }
}
