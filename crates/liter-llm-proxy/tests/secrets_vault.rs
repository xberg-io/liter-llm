//! Integration tests for [`liter_llm_proxy::secrets::HashCorpVaultProvider`].
//!
//! These tests are written against the public `HashCorpVaultProvider::get`
//! contract and target the KV-v2 `/v1/<mount>/data/<path>` response shape.
//!
//! # Mocking harness
//!
//! Uses [`wiremock`] (a dev-dependency) to stand up an HTTP mock that emulates
//! a Vault KV-v2 backend.  Note that wiremock binds to `127.0.0.1`, which the
//! outbound SSRF policy rejects under `DenyPrivate`.  These tests rely on the
//! library default policy of `OutboundPolicy::Off`.  The unit test
//! `vault_address_rejects_internal_endpoints` in `src/secrets/vault.rs` flips
//! the policy to `DenyPrivate` and resets to `Off` before returning, but it
//! lives in a different test binary, so test ordering is not a concern.

#![cfg(feature = "secrets-vault")]

use liter_llm_proxy::secrets::{HashCorpVaultProvider, SecretError, SecretManager};
use secrecy::ExposeSecret;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// `HashCorpVaultProvider::get` must extract the `.data.data.<field>` field
/// from a Vault KV-v2 read response.
#[tokio::test]
async fn fetch_kv_v2_path_extracts_data_data_field() {
    let server = MockServer::start().await;

    // KV-v2 read: GET /v1/secret/data/foo → { data: { data: { value: "abc" }, metadata: {...} } }
    Mock::given(method("GET"))
        .and(path("/v1/secret/data/foo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "request_id": "test-req-id",
            "lease_id": "",
            "renewable": false,
            "lease_duration": 0,
            "data": {
                "data": { "value": "abc" },
                "metadata": {
                    "created_time": "2024-01-01T00:00:00Z",
                    "custom_metadata": null,
                    "deletion_time": "",
                    "destroyed": false,
                    "version": 1
                }
            },
            "wrap_info": null,
            "warnings": null,
            "auth": null
        })))
        .mount(&server)
        .await;

    // Metadata read used opportunistically by fetch_from_vault.  We mount a
    // plausible response so the metadata branch succeeds; if it fails, the
    // provider falls back to default metadata which is still acceptable.
    Mock::given(method("GET"))
        .and(path("/v1/secret/metadata/foo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "request_id": "test-req-id",
            "lease_id": "",
            "renewable": false,
            "lease_duration": 0,
            "data": {
                "created_time": "2024-01-01T00:00:00Z",
                "current_version": 1,
                "max_versions": 0,
                "oldest_version": 0,
                "updated_time": "2024-01-01T00:00:00Z",
                "versions": {},
                "custom_metadata": null
            },
            "wrap_info": null,
            "warnings": null,
            "auth": null
        })))
        .mount(&server)
        .await;

    let provider = HashCorpVaultProvider::builder()
        .address(server.uri())
        .token("test-token")
        .build()
        .expect("builder must succeed when policy is Off");

    let secret = provider.get("foo").await.expect("must fetch secret");
    assert_eq!(
        secret.value.expose_secret(),
        "abc",
        "value field is extracted from data.data.value"
    );
    assert_eq!(secret.metadata.name, "foo");
}

/// A 403 response from Vault must be translated to [`SecretError::PermissionDenied`].
#[tokio::test]
async fn fetch_returns_error_on_403() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/secret/data/forbidden"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "errors": ["permission denied"]
        })))
        .mount(&server)
        .await;

    let provider = HashCorpVaultProvider::builder()
        .address(server.uri())
        .token("test-token")
        .build()
        .expect("builder must succeed when policy is Off");

    let result = provider.get("forbidden").await;
    match result {
        Ok(_) => panic!("403 response must surface as an error"),
        Err(SecretError::PermissionDenied(_)) => {}
        Err(other) => panic!("403 must map to PermissionDenied, got {other:?}"),
    }
}
