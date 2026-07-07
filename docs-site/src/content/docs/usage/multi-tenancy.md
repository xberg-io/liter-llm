---
description: "Multi-tenant API key management and request isolation."
title: "Multi-Tenancy"
---

Liter-llm uses virtual keys to isolate requests by tenant. A `TenantId`
identifies the organization issuing a request; each tenant's credentials are
resolved to access rules (allowed models, budget) via a pluggable backend.

## Core concepts

### TenantId

An opaque tenant identifier (transparent `String` newtype):

```rust
pub struct TenantId(pub String);
```

Use any format you like: UUIDs, slugs, numeric IDs. The proxy treats it as
an opaque token.

### KeyResolver

The proxy resolves API keys to tenant identity and access rules via a
`KeyResolver`:

```rust
pub trait KeyResolver: Send + Sync + 'static {
    fn resolve(
        &self,
        api_key: String,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedKey, KeyResolverError>> + Send + 'static>>;
}
```

Built-in implementations: `InMemoryKeyResolver` and `EtcdKeyResolver`. You can
implement a custom resolver against any backend.

### ResolvedKey

The record returned by a resolver:

```rust
pub struct ResolvedKey {
    pub tenant_id: TenantId,              // Identifies the tenant
    pub allowed_models: Vec<String>,      // Empty = unrestricted
    pub monthly_budget: Option<Decimal>,  // Per-period limit
    pub currency: Option<String>,         // ISO-4217
    pub metadata: HashMap<String, String>, // Custom attributes
    pub active: bool,                     // Is the key active?
}
```

## Request flow

Every HTTP request through the proxy passes through these stages:

1. **Extract API key** from `Authorization: Bearer <key>`
2. **Resolve via KeyResolver** → `validate_api_key()` calls `KeyResolver::resolve(key)`
3. **Build KeyContext** → `KeyContext::from_resolved()` extracts tenant_id, model access, budget
4. **Attach to LlmRequest** → `dispatch()` applies `LlmRequest::with_tenant_id(tenant_id)` before Tower stack
5. **Tower layers consume tenant_id** → `BudgetLedger::Tenant` and `TenantScopedStrategy` read it for isolation and quota

The tenant identity is immutable for the lifetime of the request.

## Master key

A special key named `MASTER_TENANT_ID = "master"` used for admin operations or
development:

```rust
pub const MASTER_TENANT_ID: &str = "master";
```

The proxy typically allows the master key to access any model without quota
restrictions. Configure the master key in `ProxyConfig.master_key`.

## Budget and routing by tenant

### Tenant-scoped budget

`BudgetLedger::Tenant` tracks spend per-tenant. When a request arrives with a
resolved budget, the budget layer checks if the tenant has remaining allowance
before forwarding to the provider.

### Tenant-scoped routing

`TenantScopedStrategy` routes requests to different provider backends based on
tenant-level attributes stored in `ResolvedKey.metadata`. For example:

```json
{
  "tenant_id": "acme-corp",
  "allowed_models": ["openai/gpt-4o", "anthropic/claude-3-5-sonnet"],
  "monthly_budget": "5000",
  "currency": "USD",
  "metadata": {
    "preferred_region": "eu-west-1",
    "sso_provider": "okta"
  }
}
```

Tower layers that read `tenant_id` can use metadata to make routing decisions
without re-resolving credentials.

## Wiring into the proxy

See [Key Resolvers](/server/key-resolvers/) and [Embedding the Proxy](/server/embedding/) for configuration examples.

### Command-line

Start the standalone proxy with a master key:

```bash
export LITER_LLM_MASTER_KEY="sk-proxy-$(openssl rand -hex 16)"
liter-llm api --config ./liter-llm-proxy.toml --master-key "$LITER_LLM_MASTER_KEY"
```

### Programmatic (Rust)

Inject a custom resolver into an embedded proxy:

```rust
use std::sync::Arc;
use liter_llm_proxy::ProxyServer;
use liter_llm::tenant::EtcdKeyResolver;

let resolver = Arc::new(EtcdKeyResolver::connect(config).await?);

ProxyServer::new(config)
    .with_key_resolver(resolver)
    .serve_with_shutdown(None)
    .await?;
```

## Verification

Extract tenant_id from a request token by decoding the resolved key:

```rust
let resolved = resolver.resolve("sk-xyz...").await?;
println!("tenant: {}", resolved.tenant_id);
println!("models: {:?}", resolved.allowed_models);
println!("budget: {:?}", resolved.monthly_budget);
```

All downstream Tower layers receive `tenant_id` on the request context and can
log, meter, or route based on it.
