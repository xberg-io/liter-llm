use serde_json::json;

use serial_test::serial;

use super::*;
use crate::provider::Provider;

pub(super) fn provider() -> BedrockProvider {
    // ~keep SAFETY: env vars are process-global; `#[serial]` on callers prevents races.
    unsafe { std::env::remove_var("BEDROCK_BASE_URL") };
    // ~keep Explicit dummy credentials so `transform_request`'s per-request
    // `validate()` check (see #42) doesn't fail non-signing-focused tests
    // regardless of the ambient AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY env state.
    // Without the feature, `validate()` rejects explicit credentials it cannot sign,
    // and passes a provider with none.
    let p = BedrockProvider::new("us-east-1");
    if cfg!(feature = "bedrock") {
        p.with_credentials(
            Some("AKIATESTDUMMY".to_owned()),
            Some("test-dummy-secret".to_owned()),
            None,
        )
    } else {
        p
    }
}

#[test]
#[serial]
fn from_config_prefers_explicit_region_over_env() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::set_var("AWS_DEFAULT_REGION", "us-west-2") };
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let p = BedrockProvider::from_config(Some("eu-central-1".to_owned()), None, None, None, None);
    assert_eq!(p.region(), "eu-central-1");
    unsafe { std::env::remove_var("AWS_DEFAULT_REGION") };
}

#[test]
#[serial]
fn from_config_falls_back_to_env_region_when_unset() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("AWS_DEFAULT_REGION") };
    unsafe { std::env::set_var("AWS_REGION", "ap-southeast-1") };
    let p = BedrockProvider::from_config(None, None, None, None, None);
    assert_eq!(p.region(), "ap-southeast-1");
    unsafe { std::env::remove_var("AWS_REGION") };
}

#[test]
#[serial]
fn from_config_falls_back_to_default_region() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("AWS_DEFAULT_REGION") };
    unsafe { std::env::remove_var("AWS_REGION") };
    let p = BedrockProvider::from_config(None, None, None, None, None);
    assert_eq!(p.region(), DEFAULT_REGION);
}

#[test]
#[serial]
#[cfg(feature = "bedrock")]
fn with_credentials_overrides_env_for_signing() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("AWS_ACCESS_KEY_ID") };
    unsafe { std::env::remove_var("AWS_SECRET_ACCESS_KEY") };
    unsafe { std::env::remove_var("AWS_SESSION_TOKEN") };
    let headers = sigv4_sign(
        "POST",
        "https://bedrock-runtime.us-east-1.amazonaws.com/model/foo/converse",
        b"{}",
        "us-east-1",
        SigV4Credentials {
            access_key_id: Some("AKIAEXPLICIT"),
            secret_access_key: Some("explicit-secret"),
            session_token: Some("explicit-token"),
        },
    )
    .expect("signing should succeed with explicit credentials");
    assert!(
        headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("authorization"))
    );
}

#[test]
#[serial]
#[cfg(feature = "bedrock")]
fn sigv4_sign_fails_without_any_credentials() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("AWS_ACCESS_KEY_ID") };
    unsafe { std::env::remove_var("AWS_SECRET_ACCESS_KEY") };
    let result = sigv4_sign(
        "POST",
        "https://bedrock-runtime.us-east-1.amazonaws.com/model/foo/converse",
        b"{}",
        "us-east-1",
        SigV4Credentials {
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
        },
    );
    assert!(result.is_err(), "signing without credentials should fail");
}

#[test]
#[serial]
#[cfg(feature = "bedrock")]
fn signing_headers_propagates_signing_failure_instead_of_returning_empty() {
    // ~keep Regression test for #42. `sigv4_sign_fails_without_any_credentials` proves the
    // signer errors; this proves `signing_headers` PROPAGATES that error rather than
    // swallowing it into an empty header vec, which is what sent unsigned requests. The
    // sibling test covering the empty-vec return is compiled out under this feature, so
    // without this test the with-feature path has no coverage at all.
    unsafe { std::env::remove_var("AWS_ACCESS_KEY_ID") };
    unsafe { std::env::remove_var("AWS_SECRET_ACCESS_KEY") };
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
    let provider = BedrockProvider::new("us-east-1");
    let result = provider.signing_headers(
        "POST",
        "https://bedrock-runtime.us-east-1.amazonaws.com/model/foo/converse",
        b"{}",
    );
    assert!(
        result.is_err(),
        "signing_headers must surface the signing failure, not return empty headers"
    );
}

#[test]
#[serial]
#[cfg(feature = "bedrock")]
fn transform_request_fails_hard_without_credentials_rather_than_sending_unsigned() {
    // ~keep Regression test for #42: before the fix, `signing_headers` swallowed a
    // signing failure via `.unwrap_or_default()` and the request went out with no
    // Authorization header. `transform_request` must now hard-error before any
    // network I/O so an unsigned Bedrock request can never be sent.
    unsafe { std::env::remove_var("AWS_ACCESS_KEY_ID") };
    unsafe { std::env::remove_var("AWS_SECRET_ACCESS_KEY") };
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
    let p = BedrockProvider::new("us-east-1");
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}]
    });
    let result = p.transform_request(&mut body);
    assert!(
        result.is_err(),
        "transform_request must hard-error when Bedrock has no credentials, not silently \
         succeed and let signing_headers send an unsigned request"
    );
}

#[test]
#[serial]
fn bearer_token_reads_env_when_no_explicit_credentials() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::set_var("AWS_BEARER_TOKEN_BEDROCK", "ABSKtest") };
    let p = BedrockProvider::new("us-east-1");
    assert_eq!(p.bearer_token().as_deref(), Some("ABSKtest"));
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
}

#[test]
#[serial]
fn bearer_token_ignores_empty_env_value() {
    // ~keep An exported-but-blank variable is how a shell says "unset". Treating it as
    // a credential would send `Authorization: Bearer ` and mask real SigV4 credentials.
    unsafe { std::env::set_var("AWS_BEARER_TOKEN_BEDROCK", "") };
    let p = BedrockProvider::new("us-east-1");
    assert!(p.bearer_token().is_none());
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
}

#[test]
#[serial]
#[cfg(feature = "bedrock")]
fn bearer_token_yields_to_any_explicit_credential_field() {
    // ~keep Each field falls back to the environment on its own, so one explicit field
    // is a SigV4 configuration an ambient token must not redirect to another principal.
    unsafe { std::env::set_var("AWS_BEARER_TOKEN_BEDROCK", "ABSKtest") };
    let explicit = || Some("explicit".to_owned());
    for (access_key_id, secret_access_key, session_token) in [
        (explicit(), explicit(), None),
        (explicit(), None, None),
        (None, explicit(), None),
        (None, None, explicit()),
    ] {
        let p = BedrockProvider::new("us-east-1").with_credentials(
            access_key_id.clone(),
            secret_access_key.clone(),
            session_token.clone(),
        );
        assert!(
            p.bearer_token().is_none(),
            "an ambient token must not override {access_key_id:?}/{secret_access_key:?}/{session_token:?}"
        );
    }
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
}

#[test]
#[serial]
#[cfg(not(feature = "bedrock"))]
fn bearer_token_wins_over_explicit_credentials_that_cannot_sign() {
    // ~keep Without the feature an explicit pair is ignored by `signing_headers`, so
    // letting it suppress the token would send the request unauthenticated.
    unsafe { std::env::set_var("AWS_BEARER_TOKEN_BEDROCK", "ABSKtest") };
    let p = BedrockProvider::new("us-east-1").with_credentials(
        Some("AKIAEXPLICIT".to_owned()),
        Some("explicit-secret".to_owned()),
        None,
    );
    assert_eq!(p.bearer_token().as_deref(), Some("ABSKtest"));
    let headers = p
        .signing_headers(
            "POST",
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/foo/converse",
            b"{}",
        )
        .expect("a bearer token needs no signing and cannot fail");
    assert_eq!(
        headers,
        vec![("authorization".to_owned(), "Bearer ABSKtest".to_owned())]
    );
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
}

#[test]
#[serial]
#[cfg(not(feature = "bedrock"))]
fn validate_rejects_explicit_credentials_it_cannot_sign() {
    // ~keep Without this, the request goes out with no Authorization header and the
    // caller sees an opaque 403 from AWS.
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
    let p = BedrockProvider::new("us-east-1").with_credentials(
        Some("AKIAEXPLICIT".to_owned()),
        Some("explicit-secret".to_owned()),
        None,
    );
    let err = p
        .validate()
        .expect_err("explicit credentials cannot sign without the feature");
    assert!(
        matches!(err, LiterLlmError::Authentication { .. }),
        "expected an authentication error, got: {err:?}"
    );
}

#[test]
#[serial]
fn bearer_token_survives_blank_credential_fields() {
    // ~keep A blank value is no credential anyone configured, so it must not suppress
    // the token.
    unsafe { std::env::set_var("AWS_BEARER_TOKEN_BEDROCK", "ABSKtest") };
    let p = BedrockProvider::new("us-east-1").with_credentials(
        Some(String::new()),
        Some(String::new()),
        Some(String::new()),
    );
    assert_eq!(p.bearer_token().as_deref(), Some("ABSKtest"));
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
}

#[test]
#[serial]
fn signing_headers_uses_bearer_token_without_signing() {
    // ~keep Ungated on purpose: the bearer path must hold in both builds.
    unsafe { std::env::remove_var("AWS_ACCESS_KEY_ID") };
    unsafe { std::env::remove_var("AWS_SECRET_ACCESS_KEY") };
    unsafe { std::env::set_var("AWS_BEARER_TOKEN_BEDROCK", "ABSKtest") };
    let p = BedrockProvider::new("us-east-1");
    let headers = p
        .signing_headers(
            "POST",
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/foo/converse",
            b"{}",
        )
        .expect("a bearer token needs no signing and cannot fail");
    assert_eq!(
        headers.len(),
        1,
        "bearer auth sends one header, not a signed set: {headers:?}"
    );
    let (name, value) = &headers[0];
    assert!(name.eq_ignore_ascii_case("authorization"));
    assert_eq!(value, "Bearer ABSKtest");
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
}

#[test]
#[serial]
fn validate_accepts_bearer_token_without_access_keys() {
    // ~keep `validate()` re-runs from `transform_request` per request, so an
    // access-key-only check rejects a bearer request before any I/O.
    unsafe { std::env::remove_var("AWS_ACCESS_KEY_ID") };
    unsafe { std::env::remove_var("AWS_SECRET_ACCESS_KEY") };
    unsafe { std::env::set_var("AWS_BEARER_TOKEN_BEDROCK", "ABSKtest") };
    let p = BedrockProvider::new("us-east-1");
    assert!(p.validate().is_ok());
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
}

#[test]
#[serial]
#[cfg(feature = "bedrock")]
fn signing_headers_prefers_explicit_credentials_over_bearer_token() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("AWS_ACCESS_KEY_ID") };
    unsafe { std::env::remove_var("AWS_SECRET_ACCESS_KEY") };
    unsafe { std::env::set_var("AWS_BEARER_TOKEN_BEDROCK", "ABSKtest") };
    let p = provider();
    let headers = p
        .signing_headers(
            "POST",
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/foo/converse",
            b"{}",
        )
        .expect("signing should succeed with the explicit dummy credentials");
    let authorization = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.clone())
        .expect("an Authorization header should be present");
    assert!(
        authorization.starts_with("AWS4-HMAC-SHA256"),
        "explicit credentials must still sign with SigV4, got: {authorization}"
    );
    unsafe { std::env::remove_var("AWS_BEARER_TOKEN_BEDROCK") };
}

#[test]
#[serial]
#[cfg(feature = "bedrock")]
fn validate_accepts_explicit_credentials_without_env() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("AWS_ACCESS_KEY_ID") };
    let p = BedrockProvider::from_config(
        Some("us-east-1".to_owned()),
        None,
        Some("AKIAEXPLICIT".to_owned()),
        Some("explicit-secret".to_owned()),
        None,
    );
    assert!(p.validate().is_ok());
}

#[test]
#[serial]
fn with_cross_region_prefix_normalizes_trailing_dot() {
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let p = BedrockProvider::new("us-east-1").with_cross_region_prefix(Some("us".to_owned()));
    let url = p.build_url("/chat/completions", "anthropic.claude-3-sonnet-20240229-v1:0");
    assert!(
        url.contains("us.anthropic.claude-3-sonnet"),
        "cross-region prefix should be applied: {url}"
    );
}

#[test]
#[serial]
fn build_url_chat_completions() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let p = provider();
    let url = p.build_url("/chat/completions", "anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(
        url,
        "https://bedrock-runtime.us-east-1.amazonaws.com/model/anthropic.claude-3-sonnet-20240229-v1%3A0/converse"
    );
}

#[test]
#[serial]
fn build_url_embeddings() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let p = provider();
    let url = p.build_url("/embeddings", "amazon.titan-embed-text-v1");
    assert_eq!(
        url,
        "https://bedrock-runtime.us-east-1.amazonaws.com/model/amazon.titan-embed-text-v1/invoke"
    );
}

#[test]
#[serial]
fn build_url_other_path() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let p = provider();
    let url = p.build_url("/models", "any-model");
    assert_eq!(url, "https://bedrock-runtime.us-east-1.amazonaws.com/models");
}

#[test]
#[serial]
fn build_url_eusc_region() {
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    unsafe { std::env::remove_var("BEDROCK_BASE_URL") };
    let p = BedrockProvider::new("eusc-de-east-1");
    let url = p.build_url("/chat/completions", "anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(
        url,
        "https://bedrock-runtime.eusc-de-east-1.amazonaws.eu/model/anthropic.claude-3-sonnet-20240229-v1%3A0/converse"
    );
}

#[test]
#[serial]
fn build_url_china_region() {
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    unsafe { std::env::remove_var("BEDROCK_BASE_URL") };
    let p = BedrockProvider::new("cn-north-1");
    let url = p.build_url("/chat/completions", "anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(
        url,
        "https://bedrock-runtime.cn-north-1.amazonaws.com.cn/model/anthropic.claude-3-sonnet-20240229-v1%3A0/converse"
    );
}

#[test]
#[serial]
fn build_url_base_url_override() {
    unsafe { std::env::set_var("BEDROCK_BASE_URL", "https://custom.endpoint.example.com") };
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let p = BedrockProvider::new("us-east-1");
    let url = p.build_url("/chat/completions", "anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(
        url,
        "https://custom.endpoint.example.com/model/anthropic.claude-3-sonnet-20240229-v1%3A0/converse"
    );
    unsafe { std::env::remove_var("BEDROCK_BASE_URL") };
}

#[test]
#[serial]
fn build_url_base_url_trailing_slash_trimmed() {
    unsafe { std::env::set_var("BEDROCK_BASE_URL", "https://custom.endpoint.example.com/") };
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let p = BedrockProvider::new("us-east-1");
    let url = p.build_url("/chat/completions", "anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(
        url,
        "https://custom.endpoint.example.com/model/anthropic.claude-3-sonnet-20240229-v1%3A0/converse"
    );
    unsafe { std::env::remove_var("BEDROCK_BASE_URL") };
}

#[test]
#[serial]
fn build_url_base_url_override_ignores_cross_region() {
    unsafe { std::env::set_var("BEDROCK_BASE_URL", "https://custom.endpoint.example.com") };
    unsafe { std::env::set_var("BEDROCK_CROSS_REGION", "eu") };
    let p = BedrockProvider::new("us-east-1");
    let url = p.build_url("/chat/completions", "anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(
        url,
        "https://custom.endpoint.example.com/model/anthropic.claude-3-sonnet-20240229-v1%3A0/converse"
    );
    unsafe { std::env::remove_var("BEDROCK_BASE_URL") };
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
}

#[test]
fn dns_suffix_standard_regions() {
    assert_eq!(dns_suffix_for_region("us-east-1"), "amazonaws.com");
    assert_eq!(dns_suffix_for_region("eu-west-1"), "amazonaws.com");
    assert_eq!(dns_suffix_for_region("us-gov-west-1"), "amazonaws.com");
}

#[test]
fn dns_suffix_eusc_regions() {
    assert_eq!(dns_suffix_for_region("eusc-de-east-1"), "amazonaws.eu");
}

#[test]
fn dns_suffix_china_regions() {
    assert_eq!(dns_suffix_for_region("cn-north-1"), "amazonaws.com.cn");
    assert_eq!(dns_suffix_for_region("cn-northwest-1"), "amazonaws.com.cn");
}

#[test]
fn percent_encode_model_colon() {
    let encoded = percent_encode_model("anthropic.claude-3-sonnet-20240229-v1:0");
    assert!(
        encoded.contains("%3A"),
        "colon should be percent-encoded with uppercase hex: {encoded}"
    );
    assert!(!encoded.contains("%3a"), "lowercase hex must not appear: {encoded}");
    assert!(!encoded.contains(':'), "raw colon should not remain: {encoded}");
}

#[test]
fn percent_encode_model_safe_chars() {
    let encoded = percent_encode_model("amazon.titan-embed-text-v1");
    assert_eq!(encoded, "amazon.titan-embed-text-v1");
}

#[test]
#[serial]
fn transform_request_basic_chat() {
    let p = provider();
    let mut body = json!({
        "model": "anthropic.claude-3-sonnet",
        "messages": [
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "Hello!"}
        ],
        "max_tokens": 100,
        "temperature": 0.7
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["system"][0]["text"], "You are helpful.");

    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"][0]["text"], "Hello!");

    assert_eq!(body["inferenceConfig"]["maxTokens"], 100);
    assert_eq!(body["inferenceConfig"]["temperature"], 0.7);
}

/// Revert line: delete
/// `crate::provider::validate_sampling_param_range(body, "temperature", "Bedrock", 0.0, 1.0)?;`
/// in `bedrock::request::transform_converse_request` to make this test fail.
#[test]
#[serial]
fn transform_request_rejects_temperature_above_bedrock_maximum() {
    let p = provider();
    let mut body = json!({
        "model": "anthropic.claude-3-sonnet",
        "messages": [{"role": "user", "content": "hi"}],
        "temperature": 1.5
    });

    let err = p
        .transform_request(&mut body)
        .expect_err("temperature above Bedrock's 1.0 maximum should be rejected");

    assert_eq!(err.status_code(), 400);
    let message = err.to_string();
    assert!(
        message.contains("temperature=1.5"),
        "error message should name the offending value: {message}"
    );
    assert!(
        message.contains("Bedrock"),
        "error message should name the provider: {message}"
    );
}

/// Revert line: delete
/// `crate::provider::validate_sampling_param_range(body, "top_p", "Bedrock", 0.0, 1.0)?;`
/// in `bedrock::request::transform_converse_request` to make this test fail.
#[test]
#[serial]
fn transform_request_rejects_top_p_above_bedrock_maximum() {
    let p = provider();
    let mut body = json!({
        "model": "anthropic.claude-3-sonnet",
        "messages": [{"role": "user", "content": "hi"}],
        "top_p": 1.2
    });

    let err = p
        .transform_request(&mut body)
        .expect_err("top_p above Bedrock's 1.0 maximum should be rejected");

    assert_eq!(err.status_code(), 400);
    assert!(
        err.to_string().contains("top_p=1.2"),
        "error message should name the offending value: {err}"
    );
}

/// `max_completion_tokens` maps to `inferenceConfig.maxTokens` when `max_tokens` is absent.
/// Previously untested even though the mapping itself predates this fix.
#[test]
#[serial]
fn transform_request_max_completion_tokens_maps_to_max_tokens() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "max_completion_tokens": 512
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["inferenceConfig"]["maxTokens"], 512);
}

/// `service_tier` maps to Bedrock Converse's `serviceTier: {type}` for every value except
/// `"auto"`, which has no Bedrock counterpart and is left unset (provider default).
#[test]
#[serial]
fn transform_request_service_tier_maps_to_bedrock_service_tier() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "service_tier": "flex"
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["serviceTier"]["type"], "flex");
}

#[test]
#[serial]
fn transform_request_service_tier_auto_is_omitted() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "service_tier": "auto"
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert!(body.get("serviceTier").is_none());
}

/// `metadata` maps to Bedrock Converse's `requestMetadata`, the same string-to-string
/// tag-map shape used for CloudTrail/CloudWatch filtering.
#[test]
#[serial]
fn transform_request_metadata_maps_to_request_metadata() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "metadata": {"run": "nightly", "team": "platform"}
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["requestMetadata"]["run"], "nightly");
    assert_eq!(body["requestMetadata"]["team"], "platform");
}

/// `logprobs`/`top_logprobs`/`audio`/`web_search_options` have no Bedrock equivalent; the
/// wholesale body rebuild already dropped them, this pins that they never leak onto the wire.
#[test]
#[serial]
fn transform_request_unmappable_openai_only_fields_dropped() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "logprobs": true,
        "top_logprobs": 5,
        "store": true,
        "prediction": {"type": "content", "content": "draft"},
        "audio": {"voice": "alloy", "format": "wav"},
        "web_search_options": {"search_context_size": "medium"}
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    for key in &[
        "logprobs",
        "top_logprobs",
        "store",
        "prediction",
        "audio",
        "web_search_options",
    ] {
        assert!(body.get(key).is_none(), "`{key}` must not be forwarded to Bedrock");
    }
}

#[test]
#[serial]
fn transform_request_with_tool_calls() {
    let p = provider();
    let mut body = json!({
        "messages": [
            {"role": "user", "content": "What is the weather?"},
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_abc",
                    "type": "function",
                    "function": {"name": "get_weather", "arguments": "{\"city\":\"Berlin\"}"}
                }]
            },
            {
                "role": "tool",
                "tool_call_id": "call_abc",
                "content": "Sunny, 22°C"
            }
        ]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let messages = body["messages"].as_array().expect("messages should be an array");
    assert_eq!(messages.len(), 3);

    let assistant = &messages[1];
    assert_eq!(assistant["role"], "assistant");
    let tool_use = &assistant["content"][0]["toolUse"];
    assert_eq!(tool_use["toolUseId"], "call_abc");
    assert_eq!(tool_use["name"], "get_weather");
    assert_eq!(tool_use["input"]["city"], "Berlin");

    let tool_result_msg = &messages[2];
    assert_eq!(tool_result_msg["role"], "user");
    let tool_result = &tool_result_msg["content"][0]["toolResult"];
    assert_eq!(tool_result["toolUseId"], "call_abc");
    assert_eq!(tool_result["status"], "success");
}

/// Regression test: before the shared `convert_content_to_bedrock_blocks` helper,
/// a `Parts` tool result silently dropped to an empty string because the "tool"
/// arm only read `content.as_str()`. This asserts the image part now reaches
/// Bedrock as a native `image` content block instead of being lost.
#[test]
#[serial]
fn transform_request_tool_result_image_part_maps_to_bedrock_image_block() {
    let p = provider();
    let mut body = json!({
        "messages": [
            {"role": "user", "content": "Take a screenshot"},
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_shot",
                    "type": "function",
                    "function": {"name": "take_screenshot", "arguments": "{}"}
                }]
            },
            {
                "role": "tool",
                "tool_call_id": "call_shot",
                "content": [
                    {"type": "text", "text": "Here is the screenshot"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc123"}}
                ]
            }
        ]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let messages = body["messages"].as_array().expect("messages should be an array");
    let tool_result_msg = &messages[2];
    let result_content = tool_result_msg["content"][0]["toolResult"]["content"]
        .as_array()
        .expect("toolResult content should be an array");
    assert_eq!(result_content.len(), 2);
    assert_eq!(result_content[0], json!({"text": "Here is the screenshot"}));
    assert_eq!(
        result_content[1],
        json!({"image": {"format": "png", "source": {"bytes": "abc123"}}})
    );
}

#[test]
#[serial]
fn transform_request_tools_schema() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "tools": [{
            "type": "function",
            "function": {
                "name": "search",
                "description": "Search the web",
                "parameters": {"type": "object", "properties": {"query": {"type": "string"}}}
            }
        }]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let tools = body["toolConfig"]["tools"]
        .as_array()
        .expect("tools should be an array");
    assert_eq!(tools.len(), 1);
    let spec = &tools[0]["toolSpec"];
    assert_eq!(spec["name"], "search");
    assert_eq!(spec["description"], "Search the web");
    assert_eq!(spec["inputSchema"]["json"]["type"], "object");
}

#[test]
#[serial]
fn transform_response_basic() {
    let p = provider();
    let mut body = json!({
        "requestId": "req-123",
        "stopReason": "end_turn",
        "output": {
            "message": {
                "role": "assistant",
                "content": [{"text": "Hello, world!"}]
            }
        },
        "usage": {
            "inputTokens": 10,
            "outputTokens": 5
        }
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["id"], "req-123");
    assert_eq!(body["choices"][0]["message"]["content"], "Hello, world!");
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert_eq!(body["usage"]["prompt_tokens"], 10);
    assert_eq!(body["usage"]["completion_tokens"], 5);
    assert_eq!(body["usage"]["total_tokens"], 15);
}

#[test]
#[serial]
fn transform_response_tool_calls() {
    let p = provider();
    let mut body = json!({
        "stopReason": "tool_use",
        "output": {
            "message": {
                "role": "assistant",
                "content": [
                    {"toolUse": {
                        "toolUseId": "call_xyz",
                        "name": "get_weather",
                        "input": {"city": "Berlin"}
                    }}
                ]
            }
        },
        "usage": {"inputTokens": 20, "outputTokens": 10}
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["choices"][0]["finish_reason"], "tool_calls");
    let tool_calls = body["choices"][0]["message"]["tool_calls"]
        .as_array()
        .expect("tool_calls should be an array");
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["id"], "call_xyz");
    assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
    let args: serde_json::Value = serde_json::from_str(
        tool_calls[0]["function"]["arguments"]
            .as_str()
            .expect("arguments should be a string"),
    )
    .expect("arguments should be valid JSON");
    assert_eq!(args["city"], "Berlin");
}

#[test]
#[serial]
fn transform_response_finish_reason_mapping() {
    let p = provider();

    for (bedrock_reason, expected_oai_reason) in [
        ("end_turn", "stop"),
        ("tool_use", "tool_calls"),
        ("max_tokens", "length"),
        ("stop_sequence", "stop"),
        ("content_filtered", "content_filter"),
        ("guardrail_intervened", "content_filter"),
        ("unknown_future_reason", "stop"),
    ] {
        let mut body = json!({
            "stopReason": bedrock_reason,
            "output": {"message": {"role": "assistant", "content": [{"text": ""}]}},
            "usage": {"inputTokens": 0, "outputTokens": 0}
        });
        p.transform_response(&mut body)
            .expect("transform_response should not fail");
        assert_eq!(
            body["choices"][0]["finish_reason"], expected_oai_reason,
            "bedrock stopReason '{bedrock_reason}' should map to '{expected_oai_reason}'"
        );
    }
}
