//! Integration tests for [`EtcdKeyResolver`].
//!
//! Tests that require a live etcd cluster are marked `#[ignore]` and must be
//! run explicitly with `--include-ignored` when etcd is available at
//! `localhost:2379` (the default from [`EtcdKeyResolverConfig::default`]).
//!
//! `testcontainers-modules` is not a workspace dependency, so the ignored
//! annotation remains in place.  Remove it if testcontainers support is added
//! to the workspace in the future.

#![cfg(feature = "etcd-key-resolver")]

use liter_llm::tenant::{EtcdKeyResolver, EtcdKeyResolverConfig, KeyResolver, KeyResolverError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write a JSON-serialised [`ResolvedKey`]-shaped document to etcd.
///
/// `ResolvedKey` does not derive `Serialize`; we build the JSON by hand so
/// tests can populate etcd without adding a `Serialize` bound to the
/// production type.
async fn put_resolved_key(api_key: &str, tenant_id: &str, active: bool) {
    let config = EtcdKeyResolverConfig::default();
    let mut client = etcd_client::Client::connect(config.endpoints, None)
        .await
        .expect("connect to etcd");
    let hash = EtcdKeyResolver::hash_api_key(api_key);
    let path = EtcdKeyResolver::key_path("liter-llm/keys", &hash);
    let json = serde_json::json!({
        "tenant_id": tenant_id,
        "allowed_models": ["gpt-4"],
        "monthly_budget": null,
        "currency": null,
        "metadata": {},
        "active": active,
    });
    client
        .put(path.as_bytes(), serde_json::to_vec(&json).expect("json"), None)
        .await
        .expect("put key");
}

/// Remove the etcd entry for `api_key` (best-effort cleanup).
async fn delete_resolved_key(api_key: &str) {
    let config = EtcdKeyResolverConfig::default();
    if let Ok(mut client) = etcd_client::Client::connect(config.endpoints, None).await {
        let hash = EtcdKeyResolver::hash_api_key(api_key);
        let path = EtcdKeyResolver::key_path("liter-llm/keys", &hash);
        let _ = client.delete(path.as_bytes(), None).await;
    }
}

// ---------------------------------------------------------------------------
// Integration tests (require live etcd at localhost:2379)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires etcd at localhost:2379; run with --include-ignored"]
async fn resolve_returns_stored_key() {
    let api_key = "sk-etcd-test-resolve-active";
    put_resolved_key(api_key, "acme", true).await;

    let resolver = EtcdKeyResolver::connect(EtcdKeyResolverConfig::default())
        .await
        .expect("connect");
    let resolved = resolver.resolve(api_key.to_owned()).await.expect("resolve");

    assert_eq!(resolved.tenant_id.as_ref(), "acme");
    assert_eq!(resolved.allowed_models, vec!["gpt-4"]);
    assert!(resolved.active);

    delete_resolved_key(api_key).await;
}

#[tokio::test]
#[ignore = "requires etcd at localhost:2379; run with --include-ignored"]
async fn inactive_key_returns_inactive_error() {
    let api_key = "sk-etcd-test-resolve-inactive";
    put_resolved_key(api_key, "acme", false).await;

    let resolver = EtcdKeyResolver::connect(EtcdKeyResolverConfig::default())
        .await
        .expect("connect");
    let result = resolver.resolve(api_key.to_owned()).await;

    assert!(
        matches!(result, Err(KeyResolverError::Inactive)),
        "expected Inactive, got {result:?}",
    );

    delete_resolved_key(api_key).await;
}

#[tokio::test]
#[ignore = "requires etcd at localhost:2379; run with --include-ignored"]
async fn not_found_returns_not_found_error() {
    let resolver = EtcdKeyResolver::connect(EtcdKeyResolverConfig::default())
        .await
        .expect("connect");
    let result = resolver.resolve("sk-etcd-nonexistent-key-xyz".to_owned()).await;

    assert!(
        matches!(result, Err(KeyResolverError::NotFound)),
        "expected NotFound, got {result:?}",
    );
}

#[tokio::test]
#[ignore = "requires etcd at localhost:2379; run with --include-ignored"]
async fn malformed_json_returns_backend_error() {
    let api_key = "sk-etcd-test-malformed-json";

    let config = EtcdKeyResolverConfig::default();
    let mut raw_client = etcd_client::Client::connect(config.endpoints.clone(), None)
        .await
        .expect("connect");
    let hash = EtcdKeyResolver::hash_api_key(api_key);
    let path = EtcdKeyResolver::key_path("liter-llm/keys", &hash);
    raw_client
        .put(path.as_bytes(), b"not valid json", None)
        .await
        .expect("put malformed");

    let resolver = EtcdKeyResolver::connect(config).await.expect("connect resolver");
    let result = resolver.resolve(api_key.to_owned()).await;

    assert!(
        matches!(result, Err(KeyResolverError::Backend(_))),
        "expected Backend error for malformed JSON, got {result:?}",
    );

    let _ = raw_client.delete(path.as_bytes(), None).await;
}

// ---------------------------------------------------------------------------
// Unit tests — pure logic, no network required
// ---------------------------------------------------------------------------

#[test]
fn hash_api_key_is_deterministic() {
    assert_eq!(
        EtcdKeyResolver::hash_api_key("sk-abc123"),
        EtcdKeyResolver::hash_api_key("sk-abc123"),
    );
}

#[test]
fn key_path_uses_prefix_and_hash() {
    let hash = EtcdKeyResolver::hash_api_key("sk-test");
    let path = EtcdKeyResolver::key_path("liter-llm/keys", &hash);
    assert!(path.starts_with("liter-llm/keys/"));
    assert_eq!(path.len(), "liter-llm/keys/".len() + 64);
}

#[test]
fn etcd_key_resolver_is_send_sync() {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<EtcdKeyResolver>();
}
