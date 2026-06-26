---
description: "Client configuration: sources, TOML schema, construction, options, cache, budget, rate limits, hooks, and custom providers."
---

# Configuration

!!! Tip "Snippets marked `compile-only`"
Snippets with the `<!-- snippet:compile-only -->` HTML comment are extracted from CI test fixtures, kept in sync with the API at every build, and may include test scaffolding (assertions, fixture setup). Strip the scaffolding when copying into your application.

Liter-llm reads configuration from three sources, applied in priority order (lower numbers win):

1. **Constructor arguments** — passed directly to the client factory (`create_client(...)` / `ClientConfigBuilder`).
2. **JSON string** — `create_client_from_json(json)` for bindings without a builder.
3. **TOML file** — `liter-llm.toml`, auto-discovered by walking up from the current working directory.

The Rust core exposes the same surface to every language binding via the C FFI. Methods like `chat`, `embed`, etc. are called on the _client instance_ returned by the factory; there is no top-level `LlmClient` class to subclass or pre-configure in the bindings.

## TOML file

Place a `liter-llm.toml` file in your project directory. `FileConfig::discover()` (Rust) and the proxy server walk up the directory tree looking for it.

```toml
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
model_hint = "openai"
timeout_secs = 120
max_retries = 5
cooldown_secs = 30
health_check_secs = 60
cost_tracking = true
tracing = true

[cache]
max_entries = 512
ttl_seconds = 600
backend = "memory"

[budget]
global_limit = 50.0
enforcement = "hard"

[budget.model_limits]
"openai/gpt-4o" = 25.0

[rate_limit]
rpm = 60
tpm = 100000
window_seconds = 60

[[providers]]
name = "my-provider"
base_url = "https://my-llm.example.com/v1"
auth_header = "Bearer"
model_prefixes = ["my-provider/"]
```

### Top-level fields

| Field               | Type   | Description                                                                      |
| ------------------- | ------ | -------------------------------------------------------------------------------- |
| `api_key`           | string | Provider API key. Wrapped in `SecretString` internally.                          |
| `base_url`          | string | Override the provider's base URL.                                                |
| `model_hint`        | string | Pre-resolve a provider (e.g. `"openai"`); skips prefix lookup.                   |
| `timeout_secs`      | int    | Per-request timeout in seconds (default: 60).                                    |
| `max_retries`       | int    | Retries on 429/5xx with exponential backoff (default: 3).                        |
| `cooldown_secs`     | int    | Circuit-breaker cooldown after transient errors.                                 |
| `health_check_secs` | int    | Interval for background health checks.                                           |
| `cost_tracking`     | bool   | Enable per-request cost calculation.                                             |
| `tracing`           | bool   | Emit `tracing` spans for each request (see [Observability](./observability.md)). |
| `extra_headers`     | map    | Additional headers attached to every outgoing request.                           |

### `[cache]`

| Field            | Type   | Description                                                                          |
| ---------------- | ------ | ------------------------------------------------------------------------------------ |
| `max_entries`    | int    | Maximum cached responses (default: 256).                                             |
| `ttl_seconds`    | int    | Time-to-live for each entry in seconds (default: 300).                               |
| `backend`        | string | `"memory"` (default) or an OpenDAL scheme (`"redis"`, `"s3"`, `"fs"`, `"gcs"`, ...). |
| `backend_config` | map    | OpenDAL backend-specific key-value config.                                           |

The `backend` and `backend_config` fields are only honored when liter-llm is compiled with the `opendal-cache` feature.

### `[budget]`

| Field          | Type   | Description                                            |
| -------------- | ------ | ------------------------------------------------------ |
| `global_limit` | float  | Maximum total spend in USD across all models.          |
| `model_limits` | map    | Per-model spend limits (model name → USD).             |
| `enforcement`  | string | `"hard"` (reject over-budget) or `"soft"` (warn only). |

### `[rate_limit]`

| Field            | Type | Description                               |
| ---------------- | ---- | ----------------------------------------- |
| `rpm`            | int  | Maximum requests per window.              |
| `tpm`            | int  | Maximum tokens per window.                |
| `window_seconds` | int  | Window duration in seconds (default: 60). |

### `[[providers]]`

Array of custom provider definitions. Each entry contains:

| Field            | Type     | Description                                      |
| ---------------- | -------- | ------------------------------------------------ |
| `name`           | string   | Unique provider name.                            |
| `base_url`       | string   | Provider's API base URL.                         |
| `auth_header`    | string   | Auth scheme (optional).                          |
| `model_prefixes` | string[] | Model name prefixes that route to this provider. |

## Construction

The constructors below match the actual binding surface. Other bindings (Go, Java, Kotlin Android, C#, Ruby, PHP, Elixir, Dart, Swift, Zig, WebAssembly) expose the same scalar arguments through their generated wrappers; JVM Kotlin applications use the Java binding from Kotlin. Use the matching API page under Reference for exact language signatures.

=== "Rust"

    ```rust
    use liter_llm::{ClientConfigBuilder, DefaultClient};
    use std::time::Duration;

    let config = ClientConfigBuilder::new("sk-...")
        .base_url("https://api.openai.com/v1")
        .timeout(Duration::from_secs(120))
        .max_retries(5)
        .build();

    let client = DefaultClient::new(config, Some("openai"))?;
    ```

    Load from a `liter-llm.toml`:

    ```rust
    use liter_llm::{FileConfig, DefaultClient};

    if let Some(file) = FileConfig::discover()? {
        let config = file.into_builder().build();
        let client = DefaultClient::new(config, None)?;
    }
    ```

    `ManagedClient::new(config, model_hint)` is available with the `tower` feature and wires the full middleware stack (cache, budget, rate limit, cooldown, health, hooks, tracing) from the same `ClientConfig`.

=== "Python"

    ```python
    from liter_llm import create_client

    client = create_client(
        api_key="sk-...",
        base_url="https://api.openai.com/v1",
        timeout_secs=120,
        max_retries=5,
        model_hint="openai",
    )

    response = await client.chat(...)
    ```

    For richer configuration (cache, budget, hooks), use `create_client_from_json` with a serialized `ClientConfig`:

    ```python
    from liter_llm import create_client_from_json
    import json

    client = create_client_from_json(json.dumps({
        "api_key": "sk-...",
        "timeout_secs": 120,
    }))
    ```

=== "TypeScript"

    ```typescript
    import { createClient } from "@xberg-io/liter-llm";

    const client = createClient(
      process.env.OPENAI_API_KEY!,
      "https://api.openai.com/v1",
      120,    // timeoutSecs
      5,      // maxRetries
      "openai", // modelHint
    );

    const response = await client.chat({ /* ... */ });
    ```

    For richer configuration, serialize a `ClientConfig` and pass it to `createClientFromJson`:

    ```typescript
    import { createClientFromJson } from "@xberg-io/liter-llm";

    const client = createClientFromJson(JSON.stringify({
      api_key: process.env.OPENAI_API_KEY,
      timeout_secs: 120,
    }));
    ```

### Per-language snippets

=== "Python"

    --8<-- "snippets/python/guides/configuration.md"

=== "TypeScript"

    --8<-- "snippets/typescript/guides/configuration.md"

=== "Rust"

    --8<-- "snippets/rust/usage/configuration.md"

=== "Go"

    --8<-- "snippets/go/guides/configuration.md"

=== "Java"

    --8<-- "snippets/java/usage/configuration.md"

=== "C#"

    --8<-- "snippets/csharp/usage/configuration.md"

=== "Ruby"

    --8<-- "snippets/ruby/usage/configuration.md"

=== "PHP"

    --8<-- "snippets/php/usage/configuration.md"

=== "Elixir"

    --8<-- "snippets/elixir/usage/configuration.md"

=== "WASM"

    --8<-- "snippets/wasm/usage/configuration.md"

## Scalar options

| Option         | Type   | Default       | Description                                               |
| -------------- | ------ | ------------- | --------------------------------------------------------- |
| `api_key`      | string | **required**  | Provider API key. Wrapped in `SecretString` internally.   |
| `base_url`     | string | from registry | Override the provider's base URL.                         |
| `model_hint`   | string | none          | Pre-resolve a provider at construction (e.g. `"openai"`). |
| `timeout_secs` | int    | 60            | Request timeout in seconds.                               |
| `max_retries`  | int    | 3             | Retries on 429/5xx responses with exponential backoff.    |

## API key environment variables

API keys passed to the constructor are wrapped in `secrecy::SecretString`. They are never logged, serialized, or included in error messages. If the constructor is given an empty `api_key`, the builder reads the standard environment variable for the active provider:

| Provider        | Environment variable                          |
| --------------- | --------------------------------------------- |
| OpenAI          | `OPENAI_API_KEY`                              |
| Anthropic       | `ANTHROPIC_API_KEY`                           |
| Google (Gemini) | `GEMINI_API_KEY`                              |
| Groq            | `GROQ_API_KEY`                                |
| Mistral         | `MISTRAL_API_KEY`                             |
| Cohere          | `CO_API_KEY`                                  |
| AWS Bedrock     | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |

## Custom base URLs

Override `base_url` to point at a local inference server or a corporate proxy:

```toml
# Ollama running locally
base_url = "http://localhost:11434/v1"
```

```rust
let config = ClientConfigBuilder::new("unused")
    .base_url("http://localhost:11434/v1")
    .build();
```

## Cache

The response cache is wired through the Tower middleware stack. In Rust, attach `CacheLayer` directly; in other languages, configure it via TOML (and use `ManagedClient` indirectly through the bindings' high-level clients).

=== "TOML"

    ```toml
    [cache]
    max_entries = 256
    ttl_seconds = 300
    backend = "memory"
    ```

=== "Rust"

    ```rust
    use std::time::Duration;
    use liter_llm::tower::cache::{CacheLayer, CacheConfig, CacheBackend};

    let config = CacheConfig {
        max_entries: 256,
        ttl: Duration::from_secs(300),
        backend: CacheBackend::Memory,
    };
    let layer = CacheLayer::new(config);
    ```

### OpenDAL-backed cache

With the `opendal-cache` feature, the cache backend can be any [OpenDAL service](https://opendal.apache.org/docs/category/services) — Redis, S3, GCS, Azure Blob, local filesystem, and more.

=== "TOML"

    ```toml
    [cache]
    ttl_seconds = 3600
    backend = "redis"

    [cache.backend_config]
    endpoint = "redis://localhost:6379"
    ```

=== "Rust"

    ```rust
    use std::collections::HashMap;
    use std::time::Duration;
    use liter_llm::tower::cache::{CacheConfig, CacheBackend};

    let mut backend_config = HashMap::new();
    backend_config.insert("endpoint".into(), "redis://localhost:6379".into());

    let config = CacheConfig {
        max_entries: 1024,
        ttl: Duration::from_secs(3600),
        backend: CacheBackend::OpenDal {
            scheme: "redis".into(),
            config: backend_config,
        },
    };
    ```

## Budget

Track and enforce spending limits per model and globally. Costs are recomputed after every successful response using `liter_llm::cost::completion_cost`.

=== "TOML"

    ```toml
    [budget]
    global_limit = 10.0
    enforcement = "hard"

    [budget.model_limits]
    "openai/gpt-4o" = 5.0
    ```

=== "Rust"

    ```rust
    use std::collections::HashMap;
    use std::sync::Arc;
    use liter_llm::tower::budget::{BudgetLayer, BudgetConfig, BudgetState, Enforcement};

    let mut model_limits = HashMap::new();
    model_limits.insert("openai/gpt-4o".into(), 5.0);

    let config = BudgetConfig {
        global_limit: Some(10.0),
        model_limits,
        enforcement: Enforcement::Hard,
    };
    let state = Arc::new(BudgetState::new());
    let layer = BudgetLayer::new(config, state);
    ```

`Enforcement::Hard` rejects requests with `LiterLlmError::BudgetExceeded` once the limit is reached. `Enforcement::Soft` emits a `tracing::warn!` but allows the request through.

## Rate limiting

Per-model RPM (requests per minute) and TPM (tokens per minute) limits using a fixed window.

=== "TOML"

    ```toml
    [rate_limit]
    rpm = 60
    tpm = 100000
    window_seconds = 60
    ```

=== "Rust"

    ```rust
    use std::time::Duration;
    use liter_llm::tower::rate_limit::{ModelRateLimitLayer, RateLimitConfig};

    let config = RateLimitConfig {
        rpm: Some(60),
        tpm: Some(100_000),
        window: Duration::from_secs(60),
    };
    let layer = ModelRateLimitLayer::new(config);
    ```

## Hooks

Hooks are implemented via the Rust `LlmHook` trait and are wired into the Tower stack through `HooksLayer`. They are not currently exposed through the language bindings.

```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use liter_llm::error::{LiterLlmError, Result};
use liter_llm::tower::hooks::{HooksLayer, LlmHook};
use liter_llm::tower::types::{LlmRequest, LlmResponse};

struct LoggingHook;

impl LlmHook for LoggingHook {
    fn on_request(&self, _req: &LlmRequest)
        -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>
    {
        Box::pin(async {
            tracing::info!("dispatching request");
            Ok(())
        })
    }

    fn on_response(&self, _req: &LlmRequest, _resp: &LlmResponse)
        -> Pin<Box<dyn Future<Output = ()> + Send + '_>>
    {
        Box::pin(async {
            tracing::info!("response received");
        })
    }

    fn on_error(&self, _req: &LlmRequest, err: &LiterLlmError)
        -> Pin<Box<dyn Future<Output = ()> + Send + '_>>
    {
        let message = err.to_string();
        Box::pin(async move {
            tracing::error!(error = %message, "request failed");
        })
    }
}

let hooks: Vec<Arc<dyn LlmHook>> = vec![Arc::new(LoggingHook)];
let layer = HooksLayer::new(hooks);
```

Returning `Err` from `on_request` short-circuits the service chain, so hooks can implement guardrails (content filtering, budget gating, etc.).

## Custom providers

Custom providers are registered in a process-wide global registry. Once registered, any request whose model name matches one of the provider's `model_prefixes` is routed there.

=== "Rust"

    ```rust
    use liter_llm::{register_custom_provider, CustomProviderConfig, AuthHeaderFormat};

    register_custom_provider(CustomProviderConfig {
        name: "my-provider".into(),
        base_url: "https://my-llm.example.com/v1".into(),
        auth_header: AuthHeaderFormat::Bearer,
        model_prefixes: vec!["my-provider/".into()],
    })?;
    ```

    Unregister with `unregister_custom_provider("my-provider")` (returns `bool`).

=== "Python"

    ```python
    from liter_llm import (
        register_custom_provider,
        unregister_custom_provider,
        CustomProviderConfig,
    )
    from liter_llm._internal_bindings import AuthHeaderFormat

    register_custom_provider(CustomProviderConfig(
        name="my-provider",
        base_url="https://my-llm.example.com/v1",
        auth_header=AuthHeaderFormat.Bearer,
        model_prefixes=["my-provider/"],
    ))

    unregister_custom_provider("my-provider")
    ```

=== "TOML"

    Custom providers declared in `liter-llm.toml` are registered automatically when a `ClientConfigBuilder` is constructed from the file:

    ```toml
    [[providers]]
    name = "my-provider"
    base_url = "https://my-llm.example.com/v1"
    auth_header = "Bearer"
    model_prefixes = ["my-provider/"]
    ```

`AuthHeaderFormat` has three variants: `Bearer` (sends `Authorization: Bearer <key>`), `ApiKey(header_name)` (sends a custom header), and `None` (no auth header).

## Tracing

The tracing reference has moved to [Observability](./observability.md). That page covers span attributes, OTEL exporter setup, cost tracking, and Tower layer composition.
