---
description: "OpenTelemetry tracing and cost tracking for liter-llm requests."
title: "Observability"
---

Liter-llm emits OpenTelemetry-compatible tracing spans for every LLM request via two Tower middleware layers: `TracingLayer` and `CostTrackingLayer`. Spans follow the [OpenTelemetry GenAI semantic conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/).

## Feature flags

| Flag      | Purpose                                                                                                                                    |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `tracing` | Enables `TracingLayer` and `CostTrackingLayer`. Required for any span emission.                                                            |
| `otel`    | Re-exports `tracing_opentelemetry` and `opentelemetry` crates so callers can wire a full OTEL pipeline without adding direct dependencies. |

Enable in `Cargo.toml`:

```toml
[dependencies]
liter-llm = { version = "...", features = ["tracing"] }
# Add "otel" to export spans to an OTEL collector:
liter-llm = { version = "...", features = ["tracing", "otel"] }
```

## Span attributes

Each request creates a `gen_ai` span. The following attributes are populated according to the GenAI semantic conventions:

| Attribute                        | Type   | When set                                                                                                                                                  |
| -------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `gen_ai.operation.name`          | string | Always. Values: `"chat"`, `"embeddings"`, `"list_models"`, `"image_generate"`, `"speech"`, `"transcribe"`, `"moderate"`, `"rerank"`, `"search"`, `"ocr"`. |
| `gen_ai.request.model`           | string | Always. Empty string for `list_models`.                                                                                                                   |
| `gen_ai.system`                  | string | Always. The provider prefix from the model name (e.g. `"openai"` for `"openai/gpt-4"`). Empty when no prefix is present.                                  |
| `gen_ai.response.id`             | string | Successful chat responses.                                                                                                                                |
| `gen_ai.response.model`          | string | Successful chat and embedding responses.                                                                                                                  |
| `gen_ai.response.finish_reasons` | string | Successful chat responses. Space-separated finish reason names (e.g. `"stop"` or `"length tool_calls"`).                                                  |
| `gen_ai.usage.input_tokens`      | int    | Successful chat and embedding responses when usage data is present.                                                                                       |
| `gen_ai.usage.output_tokens`     | int    | Successful chat responses when usage data is present.                                                                                                     |
| `gen_ai.usage.cost`              | float  | Set by `CostTrackingLayer` when the model appears in the pricing registry. Value is USD.                                                                  |
| `error.type`                     | string | On error. Set to the `LiterLlmError` variant name (e.g. `"RateLimited"`, `"Timeout"`).                                                                    |

## Enabling tracing

`TracingLayer` runs inside the Tower middleware stack. In Rust, build the stack via `ManagedClient` with a `ClientConfig` that turns tracing on; the proxy server enables tracing through its `[general] tracing = true` flag. The current language bindings do not expose Tower-stack toggles directly — to use tracing from a binding, run requests through the proxy.

```rust
use liter_llm::{ClientConfigBuilder, ManagedClient};

let config = ClientConfigBuilder::new(std::env::var("OPENAI_API_KEY")?)
    .tracing(true)
    .build();

let client = ManagedClient::new(config, None)?;
```

For the proxy, set the flag in `liter-llm-proxy.toml`:

```toml
[general]
tracing = true
```

## Exporting spans with OpenTelemetry (Rust)

The `otel` feature re-exports `tracing_opentelemetry` and `opentelemetry` at `liter_llm::tower::tracing::otel`. Wire a subscriber that sends spans to an OTEL collector:

```rust
use liter_llm::tower::tracing::otel::{
    tracing_opentelemetry::OpenTelemetryLayer,
    opentelemetry,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Build an OTLP exporter sending to localhost:4317.
let tracer = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint("http://localhost:4317"),
    )
    .install_batch(opentelemetry_sdk::runtime::Tokio)?;

// Attach the OTEL layer to the tracing subscriber.
tracing_subscriber::registry()
    .with(OpenTelemetryLayer::new(tracer))
    .with(tracing_subscriber::fmt::layer())
    .init();

// Now construct the client with tracing=true.
let config = ClientConfigBuilder::new("sk-...").tracing(true).build();
let client = DefaultClient::new(config, None)?;
```

Any OTEL-compatible backend accepts these spans: Jaeger, Tempo, Honeycomb, Datadog, etc.

## Cost tracking

`CostTrackingLayer` records estimated USD cost as `gen_ai.usage.cost` on the active tracing span after each successful response. It looks up pricing from the embedded pricing registry (`crates/liter-llm/schemas/catalog.json`). Models not in the registry produce no attribute.

Enable cost tracking independently of tracing by composing the layer manually in Rust, or by setting `cost_tracking = true` in the proxy `[general]` section:

```rust
use liter_llm::tower::{cost::CostTrackingLayer, service::LlmService};
use tower::ServiceBuilder;

let inner = LlmService::new(client);
let service = ServiceBuilder::new()
    .layer(CostTrackingLayer)
    .service(inner);
```

The cost value is also accessible directly on successful response objects via `estimated_cost()`:

```rust
let resp = client.chat(req).await?;
if let Some(cost_usd) = resp.estimated_cost() {
    println!("cost: ${:.6}", cost_usd);
}
```

The pricing registry lives at `crates/liter-llm/schemas/catalog.json`. Models not in the registry produce no `gen_ai.usage.cost` attribute.

## Proxy trace context forwarding

When running behind the proxy server, incoming `traceparent` and `tracestate` headers are forwarded to the upstream provider request. The proxy creates a child span for each routed request, which allows distributed traces to span the client, proxy, and provider in a single trace tree.

Enable tracing on the proxy by setting `tracing = true` in the `[general]` section of the proxy configuration file. See [Proxy Configuration](/server/proxy-configuration/) for the full field reference.

## Tower layer composition

`TracingLayer` and `CostTrackingLayer` are standard Tower layers and compose with any `Service<LlmRequest>`. The recommended order wraps `CostTrackingLayer` inside `TracingLayer` so the cost attribute is recorded on the same span:

```rust
use liter_llm::tower::{CostTrackingLayer, LlmService, TracingLayer};
use tower::ServiceBuilder;

let service = ServiceBuilder::new()
    .layer(TracingLayer)          // outer: opens the gen_ai span
    .layer(CostTrackingLayer)     // inner: records cost inside the open span
    .service(LlmService::new(client));
```

## Usage events

Every request (success or failure) emits a `UsageEvent` after completion. Use
`UsageSink` implementations to capture token counts, latency, cache state, and
cost for billing, dashboards, or analytics.

### UsageEvent fields

| Field | Type | When set | Notes |
|-------|------|----------|-------|
| `request_id` | string | Always | Idempotency key or monotonic counter; use for deduplication |
| `tenant_id` | Option<TenantId> | When tenant context attached | Populated by `LlmRequest::with_tenant_id` |
| `model` | string | Always | Model name as submitted in the request |
| `provider` | string | Always | Provider prefix (part before `/`); empty if no prefix |
| `prompt_tokens` | u64 | Always | Zero for request types that do not report token counts |
| `completion_tokens` | u64 | Always | Zero for non-streaming responses |
| `cached_tokens` | u64 | Always | Provider-reported cached prompt tokens; zero if not reported |
| `total_tokens` | u64 | Always | prompt + completion tokens |
| `cost_usd` | Decimal | Always | Estimated USD cost; zero if model has no pricing entry |
| `effective_model` | Option<String> | Sometimes | Provider-echoed model name from response when available |
| `finish_reason` | Option<String> | Sometimes | Finish reason string from chat response choice |
| `cache_state` | CacheState | Always | `Bypass`, `Miss`, `ExactHit`, `SemanticHit`, or `StaleHit` |
| `outcome` | UsageEventOutcome | Always | `Success`, `Error`, `Cancelled`, or `TimedOut` |
| `latency_ms` | u64 | Always | Request latency measured by the hooks layer, milliseconds |
| `metadata` | HashMap | Always | Free-form attributes; sinks can inspect without adding struct fields |
| `received_at` | SystemTime | Always | Wall-clock time event was created |

### Effective_model

Differs from `model` when routing or fallback rewrites the request. For example:

- Request asks for `"openai/gpt-4o"`, provider echoes `"openai/gpt-4o-2024-08-06"`
- Request asks for `"claude-3-5-sonnet"`, fallback chain retries via `"claude-3-5-opus"` and provider echoes the actual model

Set to `None` for response variants that do not carry a model field: streaming
responses, speech, image generation, transcription, rerank, list-models. Also
`None` on error paths where no response body is available.

### Cache state

`CacheState` values describe the cache layer outcome:

```rust
pub enum CacheState {
    Miss,        // No cache entry; provider was called
    ExactHit,    // Exact-match cache hit; provider not called
    SemanticHit, // Semantic-similarity hit; provider not called
    StaleHit,    // TTL expired but stale entry served
    Bypass,      // Cache lookup skipped (default; streaming, etc.)
}
```

Task-local cell `CACHE_STATE_CELL` is written by `CacheService` and
`cache_singleflight`. `HooksService` reads it after the inner service resolves
and populates the event.

## UsageSink trait

Implement `UsageSink` to consume events:

```rust
pub trait UsageSink: Send + Sync + 'static {
    fn emit(&self, event: UsageEvent)
        -> impl Future<Output = Result<(), UsageSinkError>> + Send;
}
```

Implementations should be cheap on the hot path — defer heavy I/O to a
background task or channel. Errors from `emit` are logged but do not propagate
to the LLM request caller.

### LoggingUsageSink

Built-in sink that emits each event as a structured `tracing` INFO event on
the `gen_ai.usage` target:

```rust
use liter_llm::observability::LoggingUsageSink;

let sink = LoggingUsageSink;
// Use sink...
```

Useful in development and as a smoke-test default. No I/O is performed; always
cheap.

### MultiUsageSink

Fan-out multiple sinks concurrently:

```rust
use liter_llm::observability::{LoggingUsageSink, MultiUsageSink};

let multi = MultiUsageSink::from_sinks(vec![
    Arc::new(LoggingUsageSink),
]);

// Or mix heterogeneous types via from_erased:
let multi = MultiUsageSink::from_erased(vec![
    Arc::new(LoggingUsageSink) as Arc<dyn UsageSinkErased>,
    Arc::new(MyCustomSink::new()) as Arc<dyn UsageSinkErased>,
]);
```

Individual sink errors are logged but do not cause `emit` to return `Err`.

### Custom sink example

```rust
use liter_llm::observability::{UsageEvent, UsageSink, UsageSinkError};

struct MetricsSink;

impl UsageSink for MetricsSink {
    async fn emit(&self, event: UsageEvent) -> Result<(), UsageSinkError> {
        // Record to your metrics backend
        println!(
            "request_id={} model={} cost=${:.6} latency_ms={}",
            event.request_id, event.model, event.cost_usd, event.latency_ms
        );
        Ok(())
    }
}
```

## Wiring sinks

### Tower stack (Rust)

Add `HooksLayer` to your tower stack with a sink:

```rust
use liter_llm::tower::hooks::HooksLayer;
use tower::ServiceBuilder;

let sink = Arc::new(LoggingUsageSink);
let service = ServiceBuilder::new()
    .layer(HooksLayer::new().with_usage_sink(sink))
    .service(inner);
```

`HooksLayer` wraps the inner service and emits `UsageEvent` after every
completed request.

### Proxy (embedded or standalone)

Inject a sink via `ProxyServer::with_usage_sink`:

```rust
use std::sync::Arc;
use liter_llm_proxy::ProxyServer;
use liter_llm::observability::LoggingUsageSink;

let sink = Arc::new(LoggingUsageSink);

ProxyServer::new(config)
    .with_usage_sink(sink)
    .serve_with_shutdown(None)
    .await?;
```

See [Embedding the Proxy](/server/embedding/) for a complete example.

## Observability integration

Export events to your observability backend by implementing `UsageSink`. For
example:

- Send to a message bus (Kafka, NATS, Pub/Sub) for downstream analytics
- Write to a time-series database for dashboarding
- Stream to a billing system for usage-based pricing
- Log to an audit trail for compliance

The `UsageEvent` is self-contained; no additional context is needed.
