---
status: accepted
date: 2026-04-20
decision-makers: ["Na'aman Hirschfeld"]
---

# Separate native and WebAssembly HTTP features

## Context and Problem Statement

The core crate sends requests with reqwest on Tokio under the default `native-http` feature. Tokio's multi-thread
runtime and mio do not build for `wasm32`, so the WebAssembly binding built the core with `default-features = false`
and had no HTTP client. The WebAssembly package needed working requests (8c24a83ed, 2026-04-20); the architecture docs list
its targets as browsers, Cloudflare Workers, Deno and Bun (`docs-site/src/content/docs/concepts/architecture.md:142`).

## Considered Options

* A `wasm-http` feature that enables reqwest over the platform fetch API without Tokio, beside `native-http`
* The core built with no HTTP feature for WebAssembly, exposing types only (the state before 8c24a83ed)

## Decision Outcome

Chosen option: "A `wasm-http` feature that enables reqwest over the platform fetch API without Tokio, beside
`native-http`", because it enables reqwest "without pulling in tokio/mio, which are incompatible with wasm32
targets" (8c24a83ed) and reuses the same client code on both targets. Only the parts that need Tokio are swapped: `gloo-timers` replaces `tokio::time::sleep` in the retry loop, and the client traits and stream types drop
`Send` on `wasm32`.

HTTP modules and public HTTP types are gated on `any(feature = "native-http", feature = "wasm-http")`. The
WebAssembly binding enables `wasm-http` only, and features that need Tokio or native auth (`bedrock`, `vertex-auth`,
`azure-auth` and others) depend on `native-http`.

### Consequences

* Good, because the WebAssembly package can send real requests with the same provider and transform code as the
  native build.
* Bad, because every HTTP path carries a two-feature `cfg`, and code that needs `Send` or Tokio must be gated off on
  `wasm32`.

### Confirmation

* `crates/liter-llm/Cargo.toml:44` defines `native-http` with Tokio, and `crates/liter-llm/Cargo.toml:58` defines
  `wasm-http` with `gloo-timers` and no Tokio.
* `crates/liter-llm-wasm/Cargo.toml:45` builds the core with `default-features = false` and `wasm-http`.
* `.github/workflows/ci-wasm.yaml:85` and `.github/workflows/ci-wasm.yaml:89` run clippy and the tests for
  `wasm32`.
