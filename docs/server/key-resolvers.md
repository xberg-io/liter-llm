---
description: "Virtual key resolver backends for multi-tenant API key management."
---

# Key Resolvers

The proxy resolves API keys to tenant identity and access rules via pluggable
resolver backends. A `KeyResolver` translates a raw API token string into a
`ResolvedKey` record containing tenant ID, allowed models, budget, and metadata.

## Trait overview

Implement `KeyResolver` to plug in any backend:

```rust
pub trait KeyResolver: Send + Sync + 'static {
    fn resolve(
        &self,
        api_key: String,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'static>>;
}
```

The returned future is `'static` so you can spawn it onto a Tokio executor or
store it without borrowing the resolver.

## ResolvedKey structure

Every resolved key carries:

```rust
pub struct ResolvedKey {
    pub tenant_id: TenantId,              // Tenant this key belongs to
    pub allowed_models: Vec<String>,      // Empty = unrestricted
    pub monthly_budget: Option<Decimal>,  // Per-period spending cap
    pub currency: Option<String>,         // ISO-4217 (e.g., "EUR")
    pub metadata: HashMap<String, String>, // Custom attributes
    pub active: bool,                     // Is the key active?
}
```

## InMemoryKeyResolver

The default resolver for tests and small deployments:

```rust
use std::collections::HashMap;
use liter_llm::tenant::{InMemoryKeyResolver, ResolvedKey};

let resolver = InMemoryKeyResolver::with_entries(vec![
    (
        "sk-my-key".to_string(),
        ResolvedKey {
            tenant_id: "my-tenant".into(),
            allowed_models: vec!["openai/gpt-4o".to_string()],
            monthly_budget: None,
            currency: None,
            metadata: HashMap::new(),
            active: true,
        },
    ),
]);
```

Lookups are O(1) HashMap gets; no I/O is performed. Use for local dev, testing,
and low-volume embedded deployments.

## EtcdKeyResolver

Distributed resolver backed by an etcd cluster (requires feature
`etcd-key-resolver`):

Enable in `Cargo.toml`:

```toml
[dependencies]
liter-llm = { version = "...", features = ["etcd-key-resolver"] }
```

### Configuration

```rust
use std::time::Duration;
use liter_llm::tenant::{EtcdKeyResolver, EtcdKeyResolverConfig};

let config = EtcdKeyResolverConfig {
    endpoints: vec!["http://127.0.0.1:2379".into()],
    prefix: "liter-llm/keys".into(),
    connect_timeout: Duration::from_secs(5),
    request_timeout: Duration::from_secs(2),
    username: None,
    password: None,
};

let resolver = EtcdKeyResolver::connect(config).await?;
```

### Key storage

Keys are stored as JSON-serialized `ResolvedKey` records at
`{prefix}/{sha256_hex(api_key)}` in etcd:

```json
{
  "tenant_id": "my-tenant",
  "allowed_models": ["openai/gpt-4o"],
  "monthly_budget": "1000.00",
  "currency": "USD",
  "metadata": { "tier": "pro" },
  "active": true
}
```

The API key is hashed with SHA-256 before lookup so raw key material never
appears in the etcd key space — only the hex digest becomes a path component.

### HA deployment

For redundancy, point `EtcdKeyResolverConfig.endpoints` at multiple etcd nodes.
The underlying `etcd_client::Client` handles failover and connection pooling:

```rust
let config = EtcdKeyResolverConfig {
    endpoints: vec![
        "http://etcd-1:2379".into(),
        "http://etcd-2:2379".into(),
        "http://etcd-3:2379".into(),
    ],
    ..Default::default()
};
```

## Custom backends

Implement `KeyResolver` to plug in any storage backend (relational DB, cache,
Vault, etc.):

```rust
use std::pin::Pin;
use liter_llm::tenant::{KeyResolver, ResolvedKey, KeyResolverError};

struct CustomKeyResolver {
    backend_client: MyBackendClient,
}

impl KeyResolver for CustomKeyResolver {
    fn resolve(
        &self,
        api_key: String,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'static>> {
        let client = self.backend_client.clone();
        Box::pin(async move {
            let key = client
                .lookup(&api_key)
                .await
                .map_err(|e| KeyResolverError::Backend(format!("lookup failed: {}", e)))?
                .ok_or(KeyResolverError::NotFound)?;

            if !key.active {
                return Err(KeyResolverError::Inactive);
            }

            Ok(key)
        })
    }
}
```

Return `KeyResolverError::NotFound` when no record matches, `Inactive` when
the record exists but `active == false`, or `Backend(msg)` for infrastructure
errors. The error is converted to a 401/403 HTTP response by the auth layer.

## Wiring into the proxy

Pass your resolver to `ProxyServer::with_key_resolver`:

```rust
use std::sync::Arc;
use liter_llm_proxy::ProxyServer;

let resolver = Arc::new(CustomKeyResolver { backend_client });

ProxyServer::new(config)
    .with_key_resolver(resolver)
    .serve_with_shutdown(None)
    .await?;
```

See [Embedding the Proxy](embedding.md) for a complete example.
