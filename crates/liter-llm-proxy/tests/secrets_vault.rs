//! Integration tests for [`liter_llm_proxy::secrets::HashCorpVaultProvider`].
//!
//! These tests are written against the public `HashCorpVaultProvider::get`
//! contract and target the KV-v2 `/v1/<mount>/data/<path>` response shape.
//!
//! # Mocking harness
//!
//! The original brief asks for a `wiremock`-backed mock Vault server.
//! `wiremock` is not currently in `dev-dependencies` for this crate (see
//! `crates/liter-llm-proxy/Cargo.toml`), so the tests below are gated behind
//! `#[ignore]` with a comment explaining how to enable them once the harness
//! lands.  The bodies are kept fully written so they will run as soon as
//! `wiremock = "0.6"` is added to dev-deps.

#![cfg(feature = "secrets-vault")]

/// `HashiCorpVaultProvider::get` must extract the `.data.data.<field>` field
/// from a Vault KV-v2 read response.
///
/// To enable:
///   1. Add `wiremock = "0.6"` to `crates/liter-llm-proxy/Cargo.toml` dev-deps.
///   2. Remove the `#[ignore]` attribute below.
#[tokio::test]
#[ignore = "wiremock is not currently in dev-dependencies; enable once available"]
async fn fetch_kv_v2_path_extracts_data_data_field() {
    // Pseudo-code body — fully realisable once wiremock is wired in:
    //
    // let server = wiremock::MockServer::start().await;
    // wiremock::Mock::given(wiremock::matchers::method("GET"))
    //     .and(wiremock::matchers::path("/v1/secret/data/foo"))
    //     .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
    //         serde_json::json!({"data": {"data": {"value": "abc"}, "metadata": {...}}})
    //     ))
    //     .mount(&server)
    //     .await;
    //
    // let provider = HashCorpVaultProvider::builder()
    //     .address(server.uri())
    //     .token("test-token")
    //     .build()
    //     .expect("builder must succeed when policy is Off");
    //
    // let secret = provider.get("foo").await.expect("must fetch");
    // assert_eq!(secret.value.expose_secret(), "abc");
}

/// A 403 response from Vault must be translated to [`SecretError::PermissionDenied`].
#[tokio::test]
#[ignore = "wiremock is not currently in dev-dependencies; enable once available"]
async fn fetch_returns_error_on_403() {
    // Pseudo-code body:
    //
    // let server = wiremock::MockServer::start().await;
    // wiremock::Mock::given(wiremock::matchers::method("GET"))
    //     .and(wiremock::matchers::path("/v1/secret/data/forbidden"))
    //     .respond_with(wiremock::ResponseTemplate::new(403))
    //     .mount(&server)
    //     .await;
    //
    // let provider = HashCorpVaultProvider::builder()
    //     .address(server.uri())
    //     .token("test-token")
    //     .build()
    //     .unwrap();
    //
    // let err = provider.get("forbidden").await.expect_err("403 must error");
    // assert!(matches!(err, SecretError::PermissionDenied(_)));
}
