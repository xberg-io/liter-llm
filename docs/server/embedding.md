---
description: "Embed the liter-llm proxy as a library in your Rust application."
---

# Embedding the Proxy

The `liter-llm-proxy` crate exports `ProxyServer` as a library, so you can mount
the proxy inside a larger Rust binary and control authentication and usage
tracking directly.

## Quick start

Build and serve the proxy with custom authentication:

```rust
use std::sync::Arc;
use liter_llm_proxy::{ProxyServer, config::ProxyConfig};
use liter_llm::tenant::InMemoryKeyResolver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ProxyConfig::from_file("liter-llm-proxy.toml")?;

    // Create a custom key resolver (in-memory for this example)
    let resolver = Arc::new(InMemoryKeyResolver::with_entries(vec![
        (
            "sk-my-api-key".to_string(),
            liter_llm::tenant::ResolvedKey {
                tenant_id: "tenant-123".into(),
                allowed_models: vec!["openai/gpt-4o".to_string()],
                monthly_budget: None,
                currency: None,
                metadata: std::collections::HashMap::new(),
                active: true,
            },
        ),
    ]));

    // Build and serve with the custom resolver
    ProxyServer::new(config)
        .with_key_resolver(resolver)
        .serve_with_shutdown(None)
        .await?;

    Ok(())
}
```

## Builder methods

### `with_key_resolver`

Override the default `KeyStore`-backed key resolver:

```rust
pub fn with_key_resolver(
    mut self,
    resolver: Arc<dyn KeyResolver>
) -> Self
```

When set, the supplied resolver is used instead of the built-in `KeyStore`
constructed from `ProxyConfig.keys`. Use this to plug in database-backed or
remote resolvers without forking `AppState`.

### `with_usage_sink`

Attach a custom usage sink:

```rust
pub fn with_usage_sink<S: UsageSink>(
    mut self,
    sink: Arc<S>
) -> Self
```

When set, `HooksLayer` is pushed outermost in the Tower middleware stack so
every completed request (success or error) emits a `UsageEvent` to the sink.
The sink receives request metadata, token counts, latency, and cache state.
Default behaviour (no sink) is unchanged when unset.

## Wiring both builders

Combine key resolver and usage sink overrides:

```rust
use std::sync::Arc;
use liter_llm_proxy::ProxyServer;
use liter_llm::observability::LoggingUsageSink;
use liter_llm::tenant::InMemoryKeyResolver;

let config = ProxyConfig::from_file("liter-llm-proxy.toml")?;

let resolver = Arc::new(InMemoryKeyResolver::new(my_keys));
let sink = Arc::new(LoggingUsageSink);

ProxyServer::new(config)
    .with_key_resolver(resolver)
    .with_usage_sink(sink)
    .serve_with_shutdown(None)
    .await?;
```

## When to use embedding

Embed the proxy when you:

- Control a larger Rust microservice and want the proxy to run in-process
- Need fine-grained control over auth without implementing a separate HTTP service
- Want to emit usage events to a custom backend or observability system
- Already have credential and storage infrastructure to wire in

For standalone deployments, run `liter-llm api` as a separate binary.

## Default behavior

When neither builder is called, the proxy uses:

- Built-in `KeyStore` loaded from `ProxyConfig.keys` (static or etcd-synced)
- No usage sink (requests complete without emission)

Calling the builders is always optional.
