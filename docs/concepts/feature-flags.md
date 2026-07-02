---
description: "Cargo feature flags for liter-llm , covering what each flag enables and its binary-size impact."
---

# Feature Flags

Liter-llm uses Cargo feature flags to keep the default binary small and avoid pulling in dependencies that are unused in most deployments. The default build includes only the native HTTP stack.

## Reference

| Flag            | Default | Enables                                                                                                                                                                      | Pulls in                                                        |
| --------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `native-http`   | yes     | reqwest HTTP client, tokio runtime, SSE parser, base64 codec                                                                                                                 | `reqwest`, `tokio`, `memchr`, `base64`                          |
| `wasm-http`     | no      | Browser/Node fetch-API HTTP client for `wasm32` targets. Mutually exclusive with `native-http` — disable defaults when enabling this.                                        | `reqwest` (wasm), `memchr`, `base64`, `gloo-timers`             |
| `tracing`       | no      | `TracingLayer`, `#[instrument]` spans on HTTP and SSE functions                                                                                                              | `tracing`                                                       |
| `tower`         | no      | Full Tower middleware stack: `LlmService`, `TracingLayer`, `FallbackLayer`, `Router`, and all other layers                                                                   | `tower`, `tower-http`, `dashmap`, `futures-util`, and `tracing` |
| `otel`          | no      | Re-exports `tracing_opentelemetry` and `opentelemetry` at `liter_llm::tower::tracing::otel` so callers can build an OTEL pipeline without a direct dependency                | `tracing-opentelemetry`, `opentelemetry`                        |
| `http3`         | no      | Enables reqwest HTTP/3 negotiation for native clients. Requires `native-http`.                                                                                              | reqwest HTTP/3 stack                                            |
| `bedrock`       | no      | AWS SigV4 request signing in `BedrockProvider`. Without this flag, the provider still routes `bedrock/` model names but sends requests unsigned (suitable for mock servers). | `aws-sigv4`, `aws-credential-types`                             |
| `azure-auth`    | no      | `AzureAdCredentialProvider` (client-credentials OAuth2 flow for Azure OpenAI)                                                                                                | none beyond `native-http`                                       |
| `vertex-auth`   | no      | `VertexOAuthCredentialProvider` (service-account JWT flow for Vertex AI)                                                                                                     | `jsonwebtoken`                                                  |
| `bedrock-auth`  | no      | `WebIdentityCredentialProvider` (STS web identity / IRSA for Bedrock)                                                                                                        | none beyond `native-http`                                       |
| `copilot-auth`  | no      | GitHub Copilot OAuth Device Flow credential provider                                                                                                                         | `native-http`                                                   |
| `tokenizer`     | no      | `count_tokens()` and `count_request_tokens()` via HuggingFace tokenizers. Downloads tokenizer files from HuggingFace Hub on first use and caches them in-process.            | `tokenizers`                                                    |
| `opendal-cache` | no      | Distributed cache backend via OpenDAL (S3, GCS, Azure Blob, Redis, filesystem). Requires `tower`.                                                                            | `opendal`                                                       |
| `guardrail-cel` | no      | CEL policy DSL for `guardrail::cel::CelGuardrail`.                                                                                                                          | `cel-interpreter`                                               |
| `lite`          | no      | Alias for the native HTTP client without Tower, OpenDAL, or tokenizer dependencies.                                                                                          | `native-http`                                                   |
| `full`          | no      | All core features: `tower`, `tracing`, `otel`, `bedrock`, auth providers, `tokenizer`, and `opendal-cache`.                                                                  | All optional core dependencies                                  |

The proxy crate adds three server-side feature flags:

| Flag            | Default | Enables                                                                                         |
| --------------- | ------- | ----------------------------------------------------------------------------------------------- |
| `proxy`         | yes     | OpenAI-compatible proxy routes, MCP server, files, batches, and responses endpoints.             |
| `secrets-aws`   | no      | AWS Secrets Manager backend for `aws://` secret names.                                           |
| `secrets-vault` | no      | HashiCorp Vault KV-v2 backend for `vault://` secret names. The `env://` backend is always built. |

## Feature availability

| Feature | Status | Surface |
| ------- | ------ | ------- |
| Circuit breaker | <span class="version-badge">Available by v1.6</span> | `tower::circuit::{CircuitLayer, CircuitService}` |
| Hedging | <span class="version-badge">Available by v1.6</span> | `tower::hedge::{HedgeLayer, HedgeService}` |
| GenAI metrics | <span class="version-badge">Available by v1.6</span> | `tower::metrics` with `otel` |
| Dynamic router and discovery | <span class="version-badge">Available by v1.6</span> | `tower::router::{DynamicRouter, StaticDiscover}` |
| Transport config | <span class="version-badge">Available by v1.6</span> | `http::transport::TransportConfig`, re-exported at crate root |
| Graceful shutdown | <span class="version-badge">Available by v1.6</span> | `liter_llm_proxy::shutdown`, wired into `liter-llm api` |
| `/healthz` and `/readyz` | <span class="version-badge">Available by v1.6</span> | Proxy liveness and readiness endpoints |
| Stream/body bounds | <span class="version-badge">Available by v1.6</span> | `util::bounds` guard constants and `check_bound()` |
| Cache key strategies | <span class="version-badge">Available by v1.6</span> | `tower::cache_key::{ExactHashStrategy, SystemPromptAwareStrategy, TenantScopedStrategy}` |
| Semantic cache and vector stores | <span class="version-badge">Available by v1.6</span> | `tower::cache_policy`, `embedding`, and `vectorstore` modules |
| Singleflight cache coordination | <span class="version-badge">Available by v1.6</span> | `tower::cache_singleflight` |
| Negative cache | <span class="version-badge">Available by v1.6</span> | `tower::cache_negative` |
| Budget dimensions | <span class="version-badge">Available by v1.6</span> | `tower::budget::{BudgetDimension, BudgetLedger}` |
| Cost rate limits | <span class="version-badge">Available by v1.6</span> | `tower::rate_limit::CostRateLimitLayer` |
| Guardrails and CEL | <span class="version-badge">Available by v1.6</span> | `guardrail` module; CEL requires `guardrail-cel` |
| Semantic route classification | <span class="version-badge">Available by v1.6</span> | `tower::route_classify` and `RoutingStrategy::Semantic` |
| Provider capabilities | <span class="version-badge">Available by v1.6</span> | `ProviderCapabilities` and `capabilities(provider)` |
| Realtime proxying | <span class="version-badge">Available by v1.6</span> | `GET /v1/realtime` WebSocket proxy |
| Secret backends | <span class="version-badge">Available by v1.6</span> | `env://`, `aws://`, and `vault://` secret managers |
| Credential rotation warnings | <span class="version-badge">Available by v1.6</span> | `SecretMetadata::expires_at` warnings and OTel gauge |
| Config hot reload | <span class="version-badge">Available by v1.6</span> | `liter-llm api --watch` with file or etcd providers |

## Usage

Add only the flags your deployment needs:

```toml
# Minimal: just the HTTP client, no Tower, no tracing
liter-llm = "..."

# Production server: Tower stack + OTEL + Bedrock
liter-llm = { version = "...", features = ["tower", "otel", "bedrock", "bedrock-auth"] }

# Azure OpenAI + tracing only
liter-llm = { version = "...", features = ["tower", "tracing", "azure-auth"] }

# Everything
liter-llm = { version = "...", features = ["full"] }
```

## Flag dependencies

Some flags imply others:

```text
tower   → tracing
otel    → tracing
bedrock → native-http
http3   → native-http
azure-auth  → native-http
vertex-auth → native-http
bedrock-auth → native-http
copilot-auth → native-http
opendal-cache → tower
guardrail-cel → cel-interpreter
lite → native-http
full → native-http, tower, tracing, otel, bedrock, tokenizer, azure-auth, vertex-auth, vertex-adc, bedrock-auth, copilot-auth, opendal-cache
```

`native-http` is in `default`. Disable it with `default-features = false` when compiling for WebAssembly. The WASM binding uses the browser `fetch` API instead.

## Binary-size considerations

The `tokenizers` crate (pulled in by `tokenizer`) adds roughly 10-15 MB to a release binary due to the HuggingFace tokenizer runtime. Avoid this flag in size-constrained deployments. Token counts are only required when callers need to pre-flight prompt sizes before sending to a provider.

The `opendal` crate (pulled in by `opendal-cache`) varies in size depending on which storage backends are compiled in. Refer to the [OpenDAL documentation](https://opendal.apache.org) for feature-level granularity.

All other flags add less than 1 MB in a typical release build.
