---
description: "Configure fallback and multi-deployment routing strategies for liter-llm."
title: "Fallback and Routing"
---

Liter-llm provides two composable Tower layers for multi-deployment scenarios: `FallbackLayer` for primary-plus-backup patterns, and `Router` for distributing load across a fleet of deployments with six selectable strategies.

## Fallback

`FallbackLayer` wraps a primary service with a single backup service. When the primary fails with a transient error, the same request is replayed on the fallback.

Non-transient errors (`Authentication`, `BadRequest`, `ContextWindowExceeded`, `ContentPolicy`, `NotFound`, `EndpointNotSupported`, `Serialization`, `InvalidHeader`, `HookRejected`) propagate immediately without consulting the fallback, because retrying them on a different endpoint produces the same result.

Transient errors that trigger the fallback: `RateLimited`, `ServiceUnavailable`, `Timeout`, `ServerError`, `Network`.

```rust
use liter_llm::tower::{FallbackLayer, LlmService};
use tower::{Layer, ServiceBuilder};

let primary = LlmService::new(openai_client);
let backup  = LlmService::new(anthropic_client);

let service = FallbackLayer::new(backup).layer(primary);
```

:::note[Streaming fallback]
`LlmService` forwards streaming responses (`ChatStream`) incrementally. Fallback applies only while the stream is
being opened, such as connection errors and error status codes. Every error yielded by the returned stream, including
one in its first item, is delivered to the consumer and does not trigger the backup service.
:::

## Router

`Router` distributes `LlmRequest`s across multiple service instances according to a `RoutingStrategy`. All deployments must be `Clone`.

```rust
use liter_llm::tower::{LlmService, Router, RoutingStrategy};

let deployments = vec![
    LlmService::new(client_us_east),
    LlmService::new(client_eu_west),
    LlmService::new(client_us_west),
];

let router = Router::new(deployments, RoutingStrategy::RoundRobin)?;
```

`Router::new` returns `LiterLlmError::BadRequest` if `deployments` is empty.

## Routing strategies

| Strategy         | Description                                                                                                                                                                                                                                  |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `RoundRobin`     | Cycles through deployments in order using an atomic counter.                                                                                                                                                                                 |
| `Fallback`       | Tries deployments in order. Advances to the next on a transient error. Returns the last error when all deployments fail.                                                                                                                     |
| `LatencyBased`   | Routes to the deployment with the lowest observed latency using an exponential moving average (alpha = 0.3). Deployments with no data default to EMA 0.0 (treated as fastest).                                                               |
| `CostBased`      | Tries deployments in order, logging estimated USD cost per response. In the current implementation, routes identically to `Fallback` while cost telemetry is recorded. Per-deployment provider metadata for true cost comparison is planned. |
| `WeightedRandom` | Selects a deployment by weighted random distribution. Weights need not sum to 1.0; they are used as relative proportions.                                                                                                                    |
| `Semantic`       | Routes by intent: calls a [`RouteClassifier`](#semantic) cascade with the prompt text and the deployments' real model IDs, then dispatches to whichever deployment serves the returned model ID. Falls back to round-robin when the classifier defers or names a model with no matching deployment. |

### RoundRobin

```rust
let router = Router::new(deployments, RoutingStrategy::RoundRobin)?;
```

Requests are distributed evenly in a cycle. All deployments receive traffic regardless of health.

### Fallback

```rust
let router = Router::new(deployments, RoutingStrategy::Fallback)?;
```

Tries each deployment in order. Only transient errors (same set as `FallbackLayer`) advance to the next. Non-transient errors short-circuit immediately.

### LatencyBased

```rust
let router = Router::new(deployments, RoutingStrategy::LatencyBased)?;
```

After each request completes, the observed wall-clock latency is folded into a per-deployment exponential moving average:

```text
EMA_new = 0.3 * sample + 0.7 * EMA_prev
```

The first sample for a deployment seeds the EMA directly. The deployment with the lowest EMA is selected on the next call. Deployments with no data have EMA 0.0 and are selected first (optimistic default).

### CostBased

```rust
let router = Router::new(deployments, RoutingStrategy::CostBased)?;
```

Attempts deployments in order. On a successful response, logs the estimated USD cost at `TRACE` level. Advances on transient errors. Pricing data comes from the embedded registry at `crates/liter-llm/schemas/catalog.json`.

### WeightedRandom

```rust
let router = Router::new(
    deployments,
    RoutingStrategy::WeightedRandom {
        weights: vec![3.0, 2.0, 1.0],
    },
)?;
```

The `weights` vec must have the same length as `deployments` and the total must be positive. Weight 3.0 means a deployment receives roughly three times as much traffic as one with weight 1.0.

`Router::new` returns `LiterLlmError::BadRequest` if the weights length does not match deployments or if all weights are zero.

### Semantic

```rust
use std::sync::Arc;

use liter_llm::tower::{
    KeywordClassifier, LlmService, RouteClassifier, Router, RoutingStrategy,
};
use regex::Regex;

let classifier = KeywordClassifier::new(vec![
    (Regex::new(r"(?i)sql|database")?, "gpt-4o".into()),
    (Regex::new(r"(?i)poem|haiku")?, "claude-3-5-sonnet".into()),
]);

let router = Router::new(deployments, RoutingStrategy::Semantic(Arc::new(classifier)))?
    // Deployments are addressed by real model ID, not positional index —
    // this call is required for `Semantic` to route anywhere at all.
    .with_deployment_models(vec!["gpt-4o".into(), "claude-3-5-sonnet".into()]);
```

`RoutingStrategy::Semantic` wraps a [`RouteClassifier`](/usage/observability/) cascade — built-ins are `KeywordClassifier` (regex rules), `EmbeddingSimilarityClassifier` (cosine similarity against precomputed intent-prototype embeddings), and `LlmClassifier` (delegates to a chat model); compose tiers with `CascadeClassifier`, and wrap any of them in `ClassifierVerdictCache` to memoise verdicts. See `crates/liter-llm/src/tower/route_classify.rs` for the full API.

`Router::with_deployment_models` is not optional for `Semantic`: without it, deployments are addressable only by their stringified positional index (`"0"`, `"1"`, ...), which no classifier built around real model names will ever return, so every verdict is discarded and the router silently falls back to round-robin.

## Composing with other layers

`Router` and `FallbackLayer` compose with the rest of the Tower stack. Budget, rate limit, cooldown, health, and tracing layers all wrap the same `Service<LlmRequest>` interface.

The proxy server wires the layers in this order (outermost to innermost): Tracing, Cost, Cache, Budget, RateLimit, Cooldown, Health, Fallback, Router.

```rust
use liter_llm::tower::{
    CostTrackingLayer, FallbackLayer, LlmService, Router, RoutingStrategy, TracingLayer,
};
use tower::ServiceBuilder;

let primary = Router::new(
    vec![LlmService::new(client_a), LlmService::new(client_b)],
    RoutingStrategy::LatencyBased,
)?;
let backup = LlmService::new(fallback_client);

let service = ServiceBuilder::new()
    .layer(TracingLayer)
    .layer(CostTrackingLayer)
    .layer(FallbackLayer::new(backup))
    .service(primary);
```

## Proxy configuration

In the proxy server, per-model fallbacks are declared in the `[[models]]` table:

```toml
[[models]]
name = "gpt-4o"
provider_model = "openai/gpt-4o"
fallbacks = ["gpt-4o-azure", "gpt-4-fallback"]

[[models]]
name = "gpt-4o-azure"
provider_model = "azure/gpt-4o"
api_key = "${AZURE_API_KEY}"

[[models]]
name = "gpt-4-fallback"
provider_model = "anthropic/claude-3-5-sonnet-20241022"
api_key = "${ANTHROPIC_API_KEY}"
```

The proxy applies the `Fallback` strategy for `fallbacks` lists. Separately, the `[routing]` table selects the `RoutingStrategy` applied to every multi-deployment `[[models]]` group (a name declared by more than one entry); it has no effect on single-deployment models or on `fallbacks` resolution. Omitting `[routing]` preserves the default: round-robin.

```toml
[routing]
strategy = "semantic"

[routing.classifier]
kind = "keyword"

[[routing.classifier.rules]]
pattern = "(?i)sql|database"
model = "gpt-4o"

[[routing.classifier.rules]]
pattern = "(?i)poem|haiku"
model = "claude-3-5-sonnet"
```

`strategy` is one of `round_robin`, `fallback`, `latency_based`, `cost_based`, `weighted_random`, or `semantic`. An unrecognised value is rejected at config-parse time with an actionable error rather than silently defaulting to round-robin. `weighted_random` additionally requires a `weights` array (one entry per deployment, in `[[models]]` declaration order — a mismatched length is rejected when the router is built). `semantic` requires a `[routing.classifier]` table:

- `kind = "keyword"` takes a `rules` array of `{ pattern, model }`, evaluated in order; the first regex match wins.
- `kind = "embedding"` takes `embedding_model`, optional `api_key` / `base_url` for the embedding call, a `threshold` in `[0, 1]`, and a `prototypes` array of `{ name, model, embedding }`. `embedding` is a **precomputed** vector — call the embedding provider once per example prompt yourself and paste the resulting numbers here; the proxy only embeds the live request prompt, not the prototypes.

An invalid `keyword` regex, or a classifier client that fails to construct, is rejected at proxy startup with a clear error. `[routing]` is global — it applies uniformly to every multi-deployment group, so a `weighted_random` weights array sized for one group's deployment count will fail to build for a group with a different count.

See [Proxy Configuration](/server/proxy-configuration/) for the full field reference.

See [Error Handling](/usage/error-handling/) for the full list of transient vs. non-transient error variants.
