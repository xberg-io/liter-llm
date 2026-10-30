---
status: accepted
date: 2026-03-26
decision-makers: ["Na'aman Hirschfeld"]
---

# Tower middleware behind the tower feature

## Context and Problem Statement

Callers of an LLM client need cross-cutting behavior around each request: tracing, fallback to a backup provider,
caching, rate limits, budgets and routing across deployments. A library user who only wants to send requests should
not pay for these dependencies. The first middleware landed as Tower layers in an optional `tower` feature
(fc5811041, 2026-03-26: `LlmService`, `TracingLayer`, `FallbackLayer`), and the same day added rate limiting,
caching, cooldown, health checks and routing strategies as further layers (32f1f2f9e).

## Considered Options

* Tower `Layer` and `Service` middleware in an optional `tower` feature of the core crate

No alternatives were recorded.

## Decision Outcome

Chosen option: "Tower `Layer` and `Service` middleware in an optional `tower` feature of the core crate", because, as
the architecture page puts it (`docs-site/src/content/docs/concepts/architecture.md:73`), "the proxy builds the full stack; library users assemble any subset they need", and
the default build "avoids heavy deps" (`crates/liter-llm/Cargo.toml:110`).

`LlmService` bridges `LlmClient` into the Tower `Service` trait, and each concern is its own layer under
`crates/liter-llm/src/tower/`. The `tower` module is compiled only with the feature, and the default feature set is
`native-http` alone. The `full` feature and the proxy enable it.

### Consequences

* Good, because each layer composes with any other Tower middleware and can be tested in isolation.
* Good, because a default build carries no middleware dependencies.
* Bad, because every type and binding that touches middleware is gated on the feature, which adds `cfg` branches.

### Confirmation

* `crates/liter-llm/Cargo.toml:62` defines the optional `tower` feature and `crates/liter-llm/Cargo.toml:111` keeps
  `default = ["native-http"]`.
* `crates/liter-llm/src/lib.rs:63` gates `pub mod tower` on the feature.
* `crates/liter-llm-proxy/Cargo.toml:69` enables `tower` for the proxy, which builds one stack per model
  (`crates/liter-llm-proxy/src/service_pool.rs:52`).
