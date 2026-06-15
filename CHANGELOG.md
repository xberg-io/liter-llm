# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `liter_llm::tower::FallbackChainLayer` — walk an ordered `Vec<S>` of services, advancing on transient errors via pluggable `RetryPolicy`. `DefaultRetryPolicy` treats 5xx/timeouts/429 as transient; auth and validation errors as terminal. Exports `RetryClass`, `RetryPolicy`, `DefaultRetryPolicy`, `FallbackChainLayer`, and `FallbackChainService`.
- `liter_llm::tenant::{TenantId, TenantContext, KeyResolver, ResolvedKey, KeyResolverError, InMemoryKeyResolver}` — generic multi-tenant primitives.
- `LlmRequest::with_tenant_id` / `LlmRequest::tenant_id` for tenant propagation through the Tower stack.
- `LlmRequestKind` — the discriminant enum extracted from `LlmRequest` to carry the variant payload; re-exported from `liter_llm::tower`.

### Changed

- `LlmRequest` is now a struct (`kind: LlmRequestKind`, `tenant_id: Option<TenantId>`) rather than a plain enum. All existing constructor call sites (`LlmRequest::Chat(r)`, `LlmRequest::Embed(r)`, etc.) continue to compile unchanged via `#[allow(non_snake_case)]` associated functions. Pattern-match sites that directly match on `LlmRequest::Variant` must be updated to match on `req.kind`.
- `liter-llm-proxy` `AppState` gains `key_resolver: Arc<dyn KeyResolver>` alongside the existing `key_store: Arc<KeyStore>`. `KeyStore` implements `KeyResolver`; behaviour is unchanged.
- Cache (`CacheService`, `SingleflightService`) reads `tenant_id` from the request via `LlmRequest::tenant_id()` so cached responses are scoped to the correct tenant automatically.

## [1.6.0-rc.0] - 2026-06-15

### Added

- **`tower::circuit` module** — `CircuitPolicy` trait with `ExponentialBackoffCircuit` default impl, `CircuitState` enum (Closed→Open→HalfOpen), `CircuitLayer` and `CircuitService` for fault isolation. State transitions on configurable consecutive-failure threshold; half-open probes reset after configurable interval. (`crates/liter-llm/src/tower/circuit.rs`)
- **`tower::hedge` module** — `HedgePolicy` trait with `FixedDelayHedge` default impl, `HedgeLayer` and `HedgeService` for concurrent retry with jitter. Races `max_attempts` copies staggered by fixed delay; cancels losers via `tokio::task::JoinSet::abort_all()`. Fast path when `max_attempts == 1` skips `JoinSet` entirely. (`crates/liter-llm/src/tower/hedge.rs`)
- **`tower::metrics` module** — `MetricsLayer` and `MetricsService` with OTel-native GenAI semantic-convention meters (gated behind `otel` feature with no-op fallback when disabled). Emits: `gen_ai.client.operation.duration` histogram (request latency, success/failure/circuit-open labels), `gen_ai.cache.{hit,miss,stale}` counters, `gen_ai.circuit.trip` counter, `gen_ai.retry.attempt` counter, plus `gen_ai.client.token.usage` histogram and `gen_ai.request.cost_usd` histogram. Instruments cached in `OnceLock<Arc<Instruments>>` to eliminate per-request meter lookups. (`crates/liter-llm/src/tower/metrics.rs`)
- **`tower::router` module** — `Weight(u32)` saturating wrapper with NaN/Inf-safe `from_f64`, `UpstreamDiscover` trait alias, `StaticDiscover` stream-based discovery, `DynamicRouter<D>` wrapping tower's `Discover` trait with per-upstream `ConcurrencyLimit` (default 256). `HealthCheckConfig` struct with interval/timeout/unhealthy_threshold/healthy_threshold, `HealthChecker` trait, `HttpProbeHealthChecker` default impl, and `PerProviderHealthCheck` service for per-provider health status tracking. (`crates/liter-llm/src/tower/health.rs`, `crates/liter-llm/src/tower/router.rs`)
- **`http::transport::TransportConfig`** — exposed public module with configurable knobs: `pool_max_idle_per_host` (default 32), `pool_idle_timeout` (default 90 s), `tcp_keepalive` (default 60 s), `http2_prior_knowledge` (default false), `dns_cache_ttl` (default 30 s, best-effort — reqwest 0.13 lacks DNS TTL setter), `enable_http3` (default false, gated behind `http3` feature flag). Builder pattern with sensible defaults. Wired into `ClientConfig` via new `transport` field; `DefaultClient::new` applies all settings to reqwest `ClientBuilder` except dns_cache_ttl. (`crates/liter-llm/src/http/transport.rs`)
- **`client::ClientConfig::transport`** field of type `TransportConfig` with default impl for backward compatibility.
- **`liter-llm-cli` runtime flags** — `--tokio-worker-threads N` and `--tokio-max-blocking-threads N` for runtime tuning, applied to both `api` and `mcp` subcommands via explicit `tokio::runtime::Builder::new_multi_thread()` (replaces `#[tokio::main]` macro). Defaults: physical CPU count (workers), 512 (blocking threads).
- **`liter-llm-proxy::shutdown` module** — `ShutdownCoordinator`, `Drainable` trait, `ShutdownPhase` enum (Idle→Draining→Drained/Aborted), `DrainResult` enum, `ShutdownHandle` for signal handling and graceful shutdown. Signal pre-registration eliminates the miss window between first and second SIGTERM/SIGINT handlers. `spawn_signal_handler` orchestrates two-signal escalation (first → Draining, second within 5 s or 30 s hard deadline → Aborted); concurrent drain via `FuturesUnordered` so slow `Drainable`s don't block faster ones. (`crates/liter-llm-proxy/src/shutdown.rs`)
- **`liter-llm-proxy::routes::health` module enhancements** — `/healthz` (liveness: 200 always, never blocks) and `/readyz` (readiness: 200 if all probes pass, 503 otherwise). `ReadinessProbe` trait for composable health checks; built-in probes: `ServicePoolProbe` (at least one upstream configured), `TokioQueueDepthProbe` (injection queue depth < 1000). Probes run sequentially; pub `run_probes()` allows custom implementations. (`crates/liter-llm-proxy/src/routes/health.rs`)
- **`util::bounds` module** — memory-budget guard constants (`SSE_BUFFER_MAX_BYTES = 1 MiB`, `EVENT_STREAM_BUFFER_MAX_BYTES = 16 MiB`, `RESPONSE_BODY_MAX_BYTES = 32 MiB`) and `check_bound()` helper for stream overflow detection (returns `Err(LiterLlmError::Streaming)` with `tracing::warn!` if exceeded). (`crates/liter-llm/src/util/bounds.rs`)
- **Workspace `[lints.clippy]` policy** — deny `correctness/suspicious/perf`; warn `style`. Document allow overrides: `unused-unit` (generated FFI), `needless-pass-by-value` (FFI ABI), `module-name-repetitions` (library ergonomics), `missing-errors-doc`, `missing-panics-doc`. (`Cargo.toml`)
- **Feature partition** — `liter-llm`: new `lite` (native-http only, no tower/opendal/tokenizer), `http3`, `otel`, and per-auth-method gates with explicit defaults and doc comments. `liter-llm-proxy`: `otel`, `opendal-cache`, `proxy` (named surface); default = `proxy`. `liter-llm-cli`: `mimalloc`, `jemalloc` (mutually exclusive allocator selection; defaults to system allocator). Workspace dependency pinning added for allocators. (`Cargo.toml`, `crates/*/Cargo.toml`)
- **Global allocator selection** — `crates/liter-llm-cli/src/allocator.rs` gates `#[global_allocator]` behind `mimalloc`/`jemalloc` features; `compile_error` if both enabled simultaneously.
- **`tower::cache_key` module** — `CacheKeyStrategy` trait with three impls: `ExactHashStrategy` (SHA256 hash of full request), `SystemPromptAwareStrategy` (omits system-prompt field from hash), `TenantScopedStrategy` (includes tenant ID). All three hash deterministically via `serde_json` to stable JSON. (`crates/liter-llm/src/tower/cache_key.rs`)
- **`crates/liter-llm/src/{embedding,vectorstore}` modules** — `EmbeddingProvider` trait with two impls: `SelfHostedEmbeddingProvider` (calls local LLM endpoint for embeddings via `embed()` method), `NoOpEmbeddingProvider` (returns zero vectors for unit tests). `VectorStore` trait with `InMemoryVectorStore` (DashMap-backed with brute-force cosine similarity), `OpenDalVectorStore` (persists embeddings to OpenDAL backends, gated behind `opendal-cache` feature). Both impls support `store(key, embedding)`, `retrieve_similar(query, threshold, top_k)`, `delete(key)`. (`crates/liter-llm/src/tower/vectorstore/{mod,memory,opendal}.rs`, `crates/liter-llm/src/tower/embedding.rs`)
- **`tower::cache.rs` trait extensions** — `CacheStore` gains `set_ttl(key, ttl)`, `iter_keys()`, `metadata(key) -> CacheMetadata` (expiry, creation_time, hit_count). Default no-op bodies preserve backward compat for existing impls. `CachedResponse` new variant: `Error { error: Arc<LiterLlmError>, expires_at: Instant }` for transient-error caching with custom `Serialize` impl that rejects persistence to external backends (in-memory only). (`crates/liter-llm/src/tower/cache.rs`)
- **`tower::cache_policy` module** — `CachePolicy` trait with `StandardCachePolicy` impl. Controls: `bypass_cache()` (per-request bypass), `ttl()` (seconds), `semantic_similarity_threshold` (0.85), `stale_while_revalidate` (5 minutes). `CacheService::call()` implements three-tier lookup: exact-hash match → semantic similarity via `EmbeddingProvider` → streaming-replay from stored chunks. `warm(requests)` async warming hook for batch pre-population. (`crates/liter-llm/src/tower/cache_policy.rs`)
- **`tower::cache_singleflight` module** — `SingleflightCoordinator` trait with `InMemorySingleflight` impl backed by DashMap. Coordinates concurrent identical requests: first caller blocks all followers; response broadcast via `tokio::sync::broadcast`. Eliminates thundering-herd when cache miss aligns with identical in-flight requests. (`crates/liter-llm/src/tower/cache_singleflight.rs`)
- **`tower::cache_negative` module** — `NegativeCachePolicy` trait with `FixedWindowNegativeCache` impl (caches transient errors only: retryable status 429/5xx, defaults to 60-second window). `CachedResponse::Error` variant with custom Serialize that prevents persistence to non-memory backends. (`crates/liter-llm/src/tower/cache_negative.rs`)
- **`tower::budget` module** — `BudgetLedger` trait with `CostRecordContext`, `CostCheckContext`, `BudgetVerdict`, `BudgetDimension` enum (Global/Model/Tenant/User/ApiKey), `BudgetSnapshot` struct. `InMemoryBudgetLedger` impl backed by DashMap per dimension; `export_csv()` for chargeback/reconciliation. Every `record_cost(context, usd_amount)` call checks all applicable dimensions atomically. (`crates/liter-llm/src/tower/budget.rs`)
- **`tower::rate_limit` module** — `CostRateLimitConfig { max_usd_per_minute, max_usd_per_hour, max_usd_per_day }` and `CostRateLimitLayer`/`CostRateLimitService` for hard spend ceilings. Integrates with `BudgetLedger` dimension checks; returns `Err(LiterLlmError::BudgetExceeded)` when cost would exceed any ceiling. `should_hedge()` helper returns true/false based on cost and latency signals for intelligent hedging. (`crates/liter-llm/src/tower/rate_limit.rs`)
- **`tower::metrics` OTel additions** — new meters: `gen_ai.budget.spend_usd` (histogram, labeled by dimension), `gen_ai.budget.rejection` (counter, labeled by dimension + reason). Emitted by `BudgetLedger` impls and `CostRateLimitService`. (`crates/liter-llm/src/tower/metrics.rs`)
- **`guardrail` module** — `Guardrail` trait (`name`, `supported_stages`, `check(context) -> GuardrailDecision`). `GuardrailStage` enum (Input/Output/OutputChunk). `GuardrailDecision` enum (Allow/Block/Mutate). `GuardrailContext` struct (request, response, reason). Built-in guardrails: `RegexGuardrail`, `AllowListGuardrail`, `DenyListGuardrail`, `LengthCapGuardrail`, `PromptInjectionHeuristic` (10-pattern keyword check, documented as heuristic not classifier). `GuardrailRegistry` global via `OnceLock<RwLock<…>>` matching `provider::custom` pattern. `GuardrailLayer`/`GuardrailService` Tower wrapper — runs Input on request, Output on full response, OutputChunk per streaming chunk, short-circuits on Block. CEL policy DSL gated behind `guardrail-cel` feature via `cel-interpreter` crate; eval errors fail-open with `tracing::warn!`. (`crates/liter-llm/src/guardrail/{mod,builtin,registry,cel,tests}.rs`, `crates/liter-llm/src/tower/guardrail.rs`)
- **`tower::route_classify` module** — `RouteClassifier` trait (`classify(context) -> ClassifyResult`, `confidence_threshold`). Built-in classifiers: `KeywordClassifier` (regex-pair → model), `EmbeddingSimilarityClassifier` (reuses `EmbeddingProvider` from 2.A), `LlmClassifier` (delegates to an LLM), `CascadeClassifier` (priority-ordered composition). `ClassifierVerdictCache` caches verdicts via `CacheStore`. `RoutingStrategy::Semantic(Arc<dyn RouteClassifier>)` variant in `tower/router.rs`; falls back to round-robin when classifier defers. OTel meters: `gen_ai.route.classify.duration` histogram + `gen_ai.route.classify.tier{keyword,embedding,llm}.hit` counters. (`crates/liter-llm/src/tower/route_classify.rs`)
- **Type-state builder pattern** — `ClientBuilder<HasApiKey, HasProvider>` with marker types `NoApiKey`/`WithApiKey`/`NoProvider`/`WithProvider`. `build()` only callable on `ClientBuilder<WithApiKey, WithProvider>` (compile-time error otherwise). Enforces API key and provider selection before use. (`crates/liter-llm/src/client/builder.rs`)
- **`ProviderCapabilities` struct** — `vision`, `reasoning`, `structured_output`, `function_calling`, `audio_in`, `audio_out`, `video_in` bools. Exposed via `pub fn capabilities(provider_name: &str) -> &'static ProviderCapabilities`. (`crates/liter-llm/src/provider/mod.rs`)
- **142 provider schema entries updated** — `crates/liter-llm/schemas/providers.json` now carries explicit `capabilities` object and `streaming_format` field ("sse" everywhere except Bedrock = "aws_event_stream") for every provider. Enables capability-aware client construction and streaming-format detection. (`crates/liter-llm/schemas/providers.json`)

### Phase 3 — Realtime streaming, secret backends, credential rotation, and config hot-reload

- **`streaming` module** — unified ingress/egress streaming with three composable layers: `IngressStream<S, P>` (typed SSE decoder), `StreamPipeline<S>` (ordered per-chunk middleware via `ChunkMiddleware` trait), `EgressStream<S>` (typed OpenAI SSE encoder). When ingress format == egress format and no middleware is registered, `EgressStream` enters passthrough mode for zero-copy forwarding without deserialise/re-serialise cycle. `StreamFormat` (SSE vs. AWS EventStream) promoted to `pub` for explicit wire-format selection. Per-thread `BytesMut` pool in `EGRESS_BYTES_POOL` threadlocal reuses frame buffers under load. `CancellationToken` threaded through every layer; each `poll_next` checks it first for clean abort on client disconnect. (`crates/liter-llm/src/streaming.rs`)

- **`liter-llm-proxy::secrets` module** — `SecretManager` trait (object-safe via `Pin<Box<dyn Future>>`) with `get(name) -> SecretValue` (field: zeroed `SecretString` + `SecretMetadata`), `set(name, value, tags)`, `delete(name)`. URI-scheme routing: `env://NAME` (always available), `aws://PATH` (requires `secrets-aws` feature), `vault://PATH` (requires `secrets-vault` feature). Built-in impls: `EnvVarSecretManager` (environment variables), `AwsSecretsManagerProvider` (AWS Secrets Manager with key rotation warnings), `HashCorpVaultProvider` (Vault KV-v2 with expiry tracking). `SecretManagerRegistry` routes by scheme and holds one singleton per backend. OTel gauge `gen_ai.secret.expires_in_seconds` (gated behind `otel` feature) emitted when secret expires within 24 h. (`crates/liter-llm-proxy/src/secrets/{mod,env,aws,vault}.rs`, `crates/liter-llm-proxy/src/secrets/registry.rs`)

- **`liter-llm-proxy::config::ConfigProvider` trait** — `load() -> ProxyConfig` (single snapshot) and `watch() -> mpsc::Receiver<ConfigEvent>` (live updates). Impls: `StaticFileConfigProvider` (TOML file, no hot-reload), `FileWatchConfigProvider` (OS file watch via `notify` crate), `EtcdConfigProvider` (distributed etcd key prefix watch with `Put`/`Delete`/`Resync` semantics). `ProxyConfig` interpolation now supports `${SECRET_URI}` syntax so `base_url = "${env://ANTHROPIC_BASE_URL}"` fetches at startup; secret rotation does not auto-reload URLs. (`crates/liter-llm-proxy/src/config/{provider,watcher}.rs`)

- **`liter-llm-proxy::provider::CredentialPool` trait** — rotates per-provider API keys on 429/5xx rate-limit signals. Methods: `current(provider) -> CredentialHandle` (round-robin active credential), `mark_exhausted(provider, handle, cooldown)` (park for cool-down, advance to next), `snapshot(provider) -> PoolSnapshot` (observability: total/active/exhausted counts + next recovery time). `InMemoryCredentialPool` impl backed by `DashMap` with per-credential cooldown state. `ProviderCredential` struct (model `ProviderCredential` in `VirtualKeyConfig` with `id`, `api_key: String`, `model_allowlist`) seeds pool entries from TOML. Decouples proxy credential cycling from `SecretManager` — supports static inline keys and external secret backends interchangeably. (`crates/liter-llm-proxy/src/provider/{credential_pool,credential_pool_memory}.rs`)

- **`liter-llm::realtime` module** — unified envelope + event types for vendor-neutral realtime streaming. `RealtimeEvent` enum (24 variants: SessionCreated, ConversationItemCreated, ResponseCreated, ResponseTextDelta, ResponseAudioDelta, ResponseFunctionCallArgumentsDelta, InputAudioBufferAppend, RateLimitsUpdated, Error, Raw, …). `ContentPart` enum (Text, Audio, ImageRef) used in conversation items. `ResponseStatus` enum (Completed, Cancelled, Failed, Incomplete). `RealtimeEnvelope` wraps event + optional `event_id`. `RealtimeTranslator` trait for pluggable per-provider translation (maps wire format ↔ unified schema, object-safe, thread-safe). Built-in impl: `openai::OpenAiRealtimeTranslator` (1-to-1 mapping because OpenAI's schema is already the reference shape). `crates/liter-llm/src/realtime/{mod,openai}.rs`)

- **`AppState` refactor** — `config` field changed from `Arc<ProxyConfig>` to `Arc<ArcSwap<ProxyConfig>>` for atomic hot-reload without blocking in-flight requests. New `secret_registry: Arc<SecretManagerRegistry>` field for resolving secret URIs in model configs. Callers must call `state.config.load()` to obtain a consistent snapshot per request. (`crates/liter-llm-proxy/src/state.rs`)

### Migration notes

- `AppState` now requires `secret_registry: Arc<SecretManagerRegistry>` and `config: Arc<ArcSwap<ProxyConfig>>` fields. Applications using `ProxyServer::builder` are unaffected; manual state construction must update both fields.
- New optional feature flags: `secrets-aws`, `secrets-vault`, `secrets-env` (env backend always enabled, others optional). `mimalloc`, `jemalloc` for allocator selection. `http3` for HTTP/3 support. `tokenizer` for `count_tokens` availability.
- `VirtualKeyConfig` gains new `provider_credentials: Vec<ProviderCredential>` field (defaults to empty). Inline credentials in TOML via repeated `[[keys.provider_credentials]]` blocks; proxy auto-rotates among them on 429/5xx.
- Workspace clippy is now `-D warnings`; downstream consumers compiling with strict lints should review suppressions — the main crate is now warnings-clean.

### Changed

- **Bindings regenerated against alef v0.25.9**; refreshes all 16 language surfaces, e2e suites, and README templates. New `[crates.e2e.fields_c_types]` entry `chat_completion_response.usage = "Usage"` and per-call C# e2e override `class = "LiterLlmConverter"` to satisfy alef v0.25.9's stricter intermediate-accessor checks.
- **`tower/router.rs`**: `WeightedRandom` now uses the new `Weight(u32)` saturating type (handles f64 NaN/Inf cleanly). `DynamicRouter` replaces ad-hoc hardcoded routing with tower::discover integration.
- **`tower/health.rs`**: health-check configuration is now per-provider (`HealthCheckConfig { interval, timeout, unhealthy_threshold, healthy_threshold }`) instead of a single global setting.
- **`http/streaming.rs`**: SSE pipeline now propagates a `tokio_util::sync::CancellationToken` end-to-end via `post_stream_with_cancel()` so client disconnect aborts the upstream stream cleanly. Threadlocal `BytesMut` pool wired in for SSE frame buffers (currently used by tests; production callers will be added in Phase 2).
- **`cli/main.rs`**: explicit `tokio::runtime::Builder::new_multi_thread()` replaces `#[tokio::main]`. Worker/blocking-thread counts now configurable.
- **Clippy policy enforcement** — workspace-wide `cargo clippy --workspace -- -D warnings` is now clean without per-crate suppressions. Narrower allow lists (correctness pass, style warnings only) reduce oversight surface.
- **`tower/cache.rs` trait extensions** — `CacheStore` method signatures extended with `set_ttl(key, ttl)`, `iter_keys()`, `metadata(key)` with default no-op bodies; backward compatible with existing impls. `CachedResponse` struct gained `Error { error: Arc<LiterLlmError>, expires_at: Instant }` variant with custom serialization.
- **`tower/router.rs` `RoutingStrategy` enum** — gained `Semantic(Arc<dyn RouteClassifier>)` variant for classifier-driven routing. Removed `#[derive(Debug)]` and now has manual `Debug` impl (dyn Trait is not Debug). Round-robin fallback when classifier defers.

### Fixed

- **Test diagnostic clarity** — all 417 test `.unwrap()` calls replaced with `.expect("descriptive message")` naming the asserted invariant, improving failure diagnostics when assertions fire. Production code paths remain unwrap-clean. (`crates/liter-llm-*/src/**`)
- **`tower/circuit.rs`**: `record_failure()` no longer spawns a tokio task to flip state (uses synchronous CAS loop) — eliminates duplicate-spawn race under burst failure and removes the runtime dependency that made `record_failure` panic outside async contexts.
- **`tower/hedge.rs`**: `HedgeService::call` now honours the Tower `ServiceExt::ready()` readiness contract via `std::mem::replace`, so wrapping a `ConcurrencyLimit`-protected upstream no longer silently bypasses the semaphore. Hedge fast-path (max_attempts == 1) skips `JoinSet` entirely.
- **`tower/metrics.rs`**: instrument lookups cached in `OnceLock<Arc<Instruments>>` instead of constructed per-request. Removes ~8k redundant meter lookups/sec at 1k req/s production load.
- **`http/streaming.rs`**: dead `BytesMut` scratch field removed from `SseParser` — was acquired from threadlocal pool but never read/written, pinning ~4 MiB across 1k concurrent streams. Pool helpers gated under `#[cfg(test)]` since production has no remaining callers.
- **`liter-llm-proxy/shutdown.rs`**: pre-registered SIGTERM/SIGINT handles eliminate the miss window between first signal returning and second-signal listener registering. Concurrent drain via `FuturesUnordered` ensures slow `Drainable`s don't block faster ones before 30 s hard deadline.
- **`liter-llm-proxy/routes/health.rs`**: `/readyz` now uses stable `tokio::runtime::RuntimeMetrics::num_alive_tasks()` (original `injection_queue_depth` only exists behind `tokio_unstable` cfg).
- **1081 alef-generated file conflict markers** — `git stash`-introduced merge conflict markers (<<<<<<, ======, >>>>>>) systematically scrubbed from bindings, e2e suites, test_apps, and generated docs. The C# `LiterLlmConverter.cs` FFI null-check pattern required manual resolution; workspace builds clean. (commit `892500ec6`)
- **Cache singleflight flake elimination** — test race where the leader completed before followers attached to the broadcast channel eliminated via atomic `Arc<Broadcast>` initialization before channel send. Fast mock services under parallel load now stable. (commit `4e3a3e51e`)

### Tooling

- **Workspace clippy lint policy enforcement** via `[workspace.lints.clippy]` blocks; per-crate suppressions consolidated at source.
- **Feature flag audit** — split composite features (e.g., `native-http` still depends on `http2`, now gated on both); avoid silent breakage from feature interaction.
- **Allocator build variants** — `BUILD_PROFILE=release task build` with `--features jemalloc` for performance-sensitive deployments; system allocator is default for lighter containers.
- **New optional dep `cel-interpreter`** (~110 KB compressed) behind `guardrail-cel` feature flag for CEL policy DSL evaluation in guardrails module.
- **`regex` workspace dependency exposed** — already present in transitive tree; now explicit for `guardrail::builtin::RegexGuardrail` and `KeywordClassifier`.

## [1.5.1] - 2026-06-13

### Changed

- **publish workflow**: migrate every push, release-asset upload, and homebrew-tap commit to the `kreuzberg-dev-publisher[bot]` GitHub App via `actions/create-github-app-token@v2`, replacing `secrets.GITHUB_TOKEN` and `secrets.HOMEBREW_TOKEN` with scoped app installation tokens.
- **Bindings regenerated against the latest alef**, refreshing all 16 language surfaces and e2e suites.

### Fixed

- **Dart binding**: named parameters and null-safety annotations, plus per-language README sync and updated method/type counts (#133).
- **PyO3 0.29 method rename**: `pyo3::Bound::downcast_into` callsites in `crates/liter-llm-py/src/lib.rs` migrated to the new `cast_into` name so the Python binding builds against pyo3 0.29.
- **PMD ruleset**: exclude `UnnecessaryWarningSuppression` from `category/java/bestpractices.xml`. Alef emits a blanket `@SuppressWarnings("PMD")` on every generated DTO record; PMD flags some as unnecessary depending on which rules fire on the surrounding record, breaking the Java hook on every regeneration.

## [1.5.0] - 2026-06-07

### Security

External security audit identified six exploitable gaps in the v1.4.1 codebase. All six are fixed here with regression tests; releasing as a minor version because three of them change defaults.

- **(F1, CRITICAL) Master-key constant-time comparison** — `KeyStore::is_master_key` previously compared the bearer token to the configured master key via `==`, exposing a per-request timing sidechannel. Now stores the master key in `secrecy::SecretString` and compares via `subtle::ConstantTimeEq::ct_eq` on the raw bytes. (`crates/liter-llm-proxy/src/auth/key_store.rs`, new `subtle = "2.6"` dep in `crates/liter-llm-proxy/Cargo.toml`.)
- **(F2, HIGH, BREAKING) SSRF guard on outbound provider URLs** — `CustomProviderConfig::base_url` accepted arbitrary URLs and the `reqwest::Client` had no DNS-resolution policy, so a malicious custom-provider registration could point at `127.0.0.1` / `169.254.169.254` / RFC1918 networks. New `liter_llm::provider::OutboundPolicy { Off, DenyPrivate, Allowlist(_) }` chokepoint validates URLs at registration time and a `GuardedResolver` re-applies the policy per-request via `reqwest`'s `dns_resolver` hook, including redirect-hop validation. Library default is `Off` (back-compat preserves embedded/FFI behaviour); proxy default is `DenyPrivate`. New `LiterLlmError::OutboundForbidden` variant maps to HTTP 502. New TOML key `[security] outbound_policy = "deny_private" | "off" | { allowlist = ["…"] }`. (`crates/liter-llm/src/provider/outbound_policy.rs`, `crates/liter-llm/src/provider/custom.rs`, `crates/liter-llm/src/client/mod.rs`, `crates/liter-llm-proxy/src/config/server.rs`, `crates/liter-llm-cli/src/commands/serve.rs`.)
- **(F3, HIGH, BREAKING) MCP per-tool model-access gate + HTTP transport auth** — every `#[tool]` handler in `crates/liter-llm-proxy/src/mcp/mod.rs` (chat, embed, list_models, generate_image, speech, transcribe, moderate, rerank, search, ocr, create_response, plus all file and batch management tools) now resolves a `KeyContext` from the rmcp `RequestContext.extensions` and pre-flight-checks `can_access_model(&params.model)` or `is_master` before routing through `ServicePool` / `FileStore`. The HTTP/SSE MCP transport mounted in `crates/liter-llm-cli/src/commands/mcp.rs` is wrapped with the same `validate_api_key` middleware as the OpenAI endpoint, so virtual-key restrictions apply uniformly. Stdio transport requires an explicit `mcp.stdio_key_id` / `mcp.stdio_trust_local = true` opt-in or refuses to start.
- **(F4, MED-HIGH) Error message sanitization** — SSE error events and `ProxyError::from(LiterLlmError)` previously embedded raw provider error strings via `Display` with no truncation or control-character handling. New `crates/liter-llm-proxy/src/error.rs::sanitize_message` (UTF-8-safe 200-char truncation, control-character strip except `\t`/`\n`) is applied at the single `From<LiterLlmError>` chokepoint; SSE payloads now build via `serde_json` rather than string interpolation, and `ProxyError::to_sse_payload` is the canonical serializer.
- **(F5, MED-HIGH) Mutex poisoning recovery** — `SyncService::clone_service` (`crates/liter-llm/src/client/managed.rs`) previously panicked when the inner `std::sync::Mutex` was poisoned. The lock guard only protects the clone step over a `BoxCloneService`, which is `Clone` and stateless across the lock, so recovery is safe: poisoned guards are now reclaimed via `PoisonError::into_inner` and the next request proceeds normally.
- **(F7, MED-HIGH, BREAKING) CORS default is empty + wildcard origin loses Authorization header** — the proxy's `default_cors()` is now `vec![]` instead of `vec!["*"]`; with no `cors_origins`, the router skips the `CorsLayer` entirely. When `cors_origins` is set to `"*"`, the wildcard branch restricts `allow_headers` to a fixed list (`CONTENT_TYPE`, `ACCEPT`, …) and explicitly does **not** include `Authorization` — wildcard origins must not see credentialed headers per CORS-fetch spec. `liter-llm-cli serve` also logs a `tracing::warn!` when `cors_origins.contains("*") && host == "0.0.0.0"`.

### Changed

- **Bindings regenerated against alef v0.23.28** (was v0.23.16). All 16 language surfaces — Python, Node, Ruby, PHP, Go, Java, Kotlin Android, C#, Elixir, WASM, C/FFI, Zig, Dart, Swift, R, Homebrew — and the e2e suites refresh end-to-end. The new alef ships my upstream java/magnus/go template patches (PMD braces, jinja whitespace, `MethodHandle.invoke` `throws Throwable` wrap, `data_enum` close-brace, magnus top-level module doc) plus the parallel agent's brew/zig/php/dart/snippets/kotlin/swift fixes.
- **Tighter Rust clippy allow surface** in the core and proxy crates: removed three unused `#[allow]` annotations, the unused `get_json` helper in `crates/liter-llm/src/http/request.rs`, and a now-dead `serde::de::DeserializeOwned` import. `cargo clippy --workspace … -- -D warnings` is clean without the deleted suppressions.

### Tooling

- **`kreuzberg-dev/pre-commit-hooks` bumped to v2.1.10** — picks up the consumer-side `alef-sync-versions --no-regen` fix (full regen no longer fires on every commit), the palantir-java-format multi-platform sha256 manifest acceptance, the ktfmt checksum entry, and the `godoc-lint` / `golangci-lint` go.work-aware module discovery (no longer scans stale `test_apps/swift_e2e/.build/checkouts/.../e2e/go/`).
- **Project-local PMD ruleset** at `packages/java/pmd-ruleset.xml` wired into the `pmd` hook to suppress alef-generated FFI patterns that PMD's quickstart ruleset misflags (`AvoidCatchingGenericException`, `PreserveStackTrace`, `CloseResource`, `UnusedLocalVariable`, `UnnecessaryFullyQualifiedName`, `VariableCanBeInlined`, `ReturnEmptyCollectionRatherThanNull`).
- **`deny.toml`** ignores `RUSTSEC-2023-0071` (Marvin Attack timing sidechannel in `rsa@0.9.x`, transitive via `opendal -> reqsign-core`). No safe upstream version yet; the underlying RSA private-key signing path is not exercised on our network-observable code paths.
- **`alef-docs-fresh` hook and the CI `Verify alef-generated code is up-to-date` step soft-disabled** pending an alef v0.23.28 `inputs-hash` regression fix — `alef verify` currently flags files as stale immediately after a fresh `alef all` run (the hash recomputed during verify disagrees with the hash written at emit time).
- **`markdownlint-rumdl-strict`** exclude expanded to cover the root `README.md` (alef-generated badge row uses inline HTML), `CONTRIBUTING.md`, `templates/readme/`, and `.github/PULL_REQUEST_TEMPLATE.md`.

### Migration notes

The three behaviour-changing defaults above (`cors_origins = []`, `outbound_policy = "deny_private"`, MCP per-tool model gate) are all reversible via explicit config. Operators who relied on the old defaults should add to their proxy config:

```toml
cors_origins = ["*"]                # opt back into the v1.4.x wildcard CORS default
[security]
outbound_policy = "off"             # opt back into the v1.4.x unguarded outbound HTTP
```

Virtual-key holders who previously hit MCP tools without a model-access policy need their `[[virtual_keys]]` entries updated to include the model names they expect to call — or be granted `is_master = true`.

## [1.4.1] - 2026-06-05

### Fixed

- **Docker build**: removed stale `COPY tools/ tools/` from `docker/Dockerfile` — the `tools/` directory was deleted in v1.3.0 and the unfixed copy was failing every Docker image build since.
- **`publish-crates` job timeout**: bumped `.github/workflows/publish.yaml` `publish-crates` `timeout-minutes` from 30 to 60. The 30-minute ceiling was cancelling mid-publish on busy `crates.io` index-propagation days, which (combined with the Python stdout buffering issue below) made cancelled runs look like silent failures with no per-crate log output.
- **Upstream `kreuzberg-dev/actions` to v1.8.29**: `publish-crates/scripts/publish.py` now line-buffers stdout/stderr (`sys.stdout.reconfigure(line_buffering=True)`), so per-crate "Publishing X (n/total)..." progress survives job cancellation. Before this fix, GitHub Actions' block-buffered Python stdout swallowed all in-flight progress when the job hit `timeout-minutes`, hiding which crate was actually mid-publish.

### Notes

- v1.4.0 was a no-op release because `task version:bump` was not run before tagging — the tree still carried `1.4.0-rc.61` in `Cargo.toml`, so every publish job either re-shipped `rc.61` artifacts (already on the registry) or failed verification looking for `1.4.0`. v1.4.1 is the first real `1.4.x` release.
- alef pin advanced to `0.23.16` (was `0.23.12`) — no functional codegen changes vs. `0.23.12`; bump tracks the latest released `0.23.x`.

## [1.4.0] - 2026-06-05

### Added

- `feat(provider/vertex): auto-install VertexAdcCredentialProvider in DefaultClient::new` — when the resolved provider is `vertex_ai` and the caller supplied neither an explicit `api_key` nor a `credential_provider`, the client now auto-constructs `VertexAdcCredentialProvider::new()` and installs it on the config. This is the canonical auth path for GKE Workload Identity / Cloud Run / Compute Engine deployments — short-lived OAuth2 tokens are fetched from the metadata server (with a `gcp_auth` ADC discovery fallback for local development) and cached with a 5-minute pre-expiry refresh buffer. Pre-obtained tokens supplied via `api_key` and explicit `credential_provider`s continue to take precedence. The ADC module is now reachable through the `native-http` feature (gated behind `native-http` instead of `vertex-adc`, with `vertex-adc` retained as a back-compat alias).
- `feat(provider/azure): per-model `base_url` overrides for Azure deployments` — `[[models]]` entries that pin a `base_url` for an `azure/...` `provider_model` now route through `AzureProvider::with_base_url(...)`, producing the required `{base_url}/openai/deployments/{model}{path}?api-version=…` shape instead of the generic OpenAI-compatible URL. Unblocks multi-resource Azure setups (different deployments per region/subscription). Closes #83.
- `feat(wasm-backend): emit chat_stream returning JS async iterator` — the WASM binding now exposes `WasmDefaultClient.chat_stream(req)` alongside the existing `chat`, `embed`, etc. The streaming adapter buffers the underlying `BoxStream<ChatCompletionChunk>` into an array and returns it as a `JsValue`, mirroring the NAPI binding's streaming semantics.
- CLI binary tarballs (Linux x86_64/aarch64, macOS aarch64, Windows x86_64) attached to GitHub Releases for direct download — closes #64.
- `schemas/pricing.json` regenerated from [models.dev](https://models.dev) and now covers 4,219 models (up from 35); `scripts/generate_pricing.py` wired into `task generate:pricing`, `task update`, and `task upgrade`. Closes #48.
- `Usage::prompt_tokens_details` (`{ cached_tokens, audio_tokens }`) deserialised from the OpenAI-compatible response body, plus `cost::completion_cost_with_cache` and matching `cache_read_input_token_cost` / `cache_creation_input_token_cost` fields on `ModelPricing`. `ChatCompletionResponse::estimated_cost` and the `CostTrackingLayer` now bill cached prompt tokens at the provider's discounted cache-read rate. `schemas/pricing.json` carries cache-read/cache-creation costs for the 1,500+ models on models.dev that publish them. Closes #65.
- `ci-mobile`: new `.github/workflows/ci-mobile.yaml` running `android-check` (ubuntu, `arm64-v8a` + `x86_64` via `cargo ndk`), `ios-check` (macos, `aarch64-apple-ios` + `aarch64-apple-ios-sim`), and `xcframework-build` (macos, SPM-ready XCFramework + SHA256 checksum). Uses shared composite actions from `kreuzberg-dev/actions@v1`.
- **Alef migration to v0.23.11**: the entire polyglot surface (16 language bindings — Python, Node, Ruby, PHP, Go, Java, C#, Kotlin Android, Elixir, WASM, C/FFI, Zig, Dart, Swift, Homebrew + Rust core) is regenerated end-to-end via [alef](https://github.com/kreuzberg-dev/alef). Streaming (`chat_stream`) is available across every applicable language, including Go (cgo channel bridge), Dart (FRB v2 `StreamSink<T>`), and WASM. Skipped-assertion total across e2e suites: 354 → 0.

### Changed

- **API rename**: `ResponseClient::retrieve_response` / `cancel_response` now take a parameter named `response_id` (was `id`). Positional callers are unaffected; named-arg callers must update. Consistent with `file_id` / `batch_id` on the file and batch clients, and unblocks the alef-generated Python binding from shadowing the `id` builtin.
- **GitHub Release CLI assets** ship a single sorted `SHA256SUMS-<version>.txt` instead of one `.sha256` per archive — closes #67.
- **WebAssembly build verified `mio`-free.** `liter-llm` exposes two mutually exclusive HTTP-stack features — `native-http` (reqwest + tokio + memchr + base64) and `wasm-http` (reqwest + memchr + base64 + gloo-timers, _no_ tokio). `liter-llm-wasm` enables only `wasm-http`; reqwest is pinned with `default-features = false, features = ["json", "stream", "rustls", "multipart", "form"]`. `cargo build --target wasm32-unknown-unknown -p liter-llm-wasm` pulls neither `mio` nor `tokio` — reqwest auto-routes to the browser/Node `fetch` API on `wasm32` targets.
- **Ruby publish** vendors core crates exclusively via the shared `kreuzberg-dev/actions/rewrite-native-deps@v1` action (alef `publish prepare`, `vendor_mode = "core-only"`). The bespoke `scripts/ci/ruby/vendor-liter-llm-core.py`, the local `ruby:vendor` Task, and the `ruby:build` dependency on it are removed.
- **Repo hygiene**: `.gitattributes` marks all alef-generated output directories (`packages/**`, `crates/*-{py,php,ffi,node,wasm}/**`, `e2e/**`) as `linguist-generated=true` so generated files collapse in GitHub PR diffs.

### Fixed

- **TLS ABI floor**: reqwest crypto provider switched from `aws-lc-rs` to `ring` (`rustls-no-provider` feature + explicit `rustls` dep with `ring` backend). Eliminates `__isoc23_strtol` and related glibc 2.38+ symbols emitted by `aws-lc-sys` 0.40.0, restoring the GLIBC_2.28 ABI floor required by downstream users (e.g. Node.js aarch64 bindings).
- **HTTP retry jitter on `wasm32-unknown-unknown`**: the jitter calculation called `std::time::SystemTime::now()` which panics with `RuntimeError: unreachable` on bare wasm32 (std time is not implemented). On `wasm32` the jitter step is skipped; native targets keep the existing `[0.5x, 1.0x]` jitter. Unblocks WASM e2e tests that exercise 429/5xx retry paths.
- **WASM and JNI bindings** no longer fail to compile against the `tokenizer`-gated `count_tokens` / `count_request_tokens` functions. Both now `exclude_functions` in `alef.toml`; apps that need token counting on those targets should call a server-side endpoint.
- **C/FFI header** emits the opaque `typedef struct LITERLLMLiterLlmError LITERLLMLiterLlmError;` referenced by the `literllm_liter_llm_error_{status_code,is_transient,error_type}` accessors.
- **Java** `ResponseObject` / `ResponseTool` DTOs round-trip the full OpenAI Responses payload. `ResponseOutputItem.content` is a `List<…>` (was a misaligned `LinkedHashMap`); `ResponseTool` accepts `description` via the `@JsonAnyGetter` / `@JsonAnySetter` flatten path. Fixes `MismatchedInputException` and `UnrecognizedPropertyException` thrown by `createResponse` / `retrieveResponse` / `cancelResponse`.
- **Node (NAPI) streaming** HTTP-init errors (400 content-policy, 401 unauthorized on `chatStream`) now reject through the iterator. Binding remains lazy (parity with Python's `async for _ in stream: pass`).
- **Python `api.py` wrapper** emits the correct shape for non-streaming methods (22 `DefaultClient` ops). Previously every method was wrapped as a streaming `AsyncIterator`; only `chat_stream` is genuinely streaming now. Also fixes `String` → `str` and `bytes::Bytes` → `bytes` mappings.

## [1.3.0] - 2026-04-23

### Changed

- **Alef migration**: All language bindings are now auto-generated by [alef](https://github.com/kreuzberg-dev/alef) instead of hand-written
- `BoxFuture`/`BoxStream` type aliases no longer wrap `Result<T>` — all method signatures now explicitly return `Result<T>`
- `provider` module is now public (was `pub(crate)`)
- `ChatCompletionRequest.stream` field is now public (was `pub(crate)`)
- Switched spell checker from codespell to [typos](https://github.com/crate-ci/typos)
- CI no longer runs code generation — only `alef verify --exit-code` for freshness checks
- Updated alef to v0.5.9

### Added

- `alef.toml` configuration for 10 language targets, 23 API method call configs, mock server support
- `bindings.rs` adapter module with `create_client` and `create_client_from_json` binding-friendly constructors
- `Default` derives on all public types for binding compatibility
- `Clone` derive on `DefaultClient`
- E2E test fixtures converted to alef format (167+ fixtures across 23 categories)
- E2E tests regenerated for 13 languages with mock HTTP server support
- Test apps generated with `alef e2e generate --registry`
- API reference documentation auto-generated with `alef docs` for all 10 languages
- Package READMEs generated with `alef readme` using restored Jinja templates
- `alef-verify` and `alef-sync-versions` pre-commit hooks
- `alef verify --exit-code` step in CI validation workflow
- `.lychee.toml` link checker configuration
- `_typos.toml` spell checker configuration
- Auto-load API keys from environment variables
- FFI callback streaming support
- `chat_stream` method across all bindings

### Removed

- `liter-llm-bindings-core` crate — replaced by alef codegen
- `tools/e2e-generator` crate — replaced by `alef e2e generate`
- `scripts/sync_versions.py` — replaced by `alef sync-versions`
- `scripts/generate_readme.py` — replaced by `alef readme`
- `scripts/readme_config.yaml` and `scripts/readme_templates/` — replaced by `templates/readme/`
- `tests/test_apps/` — replaced by `test_apps/` (alef registry mode)
- Hand-written binding source in `crates/liter-llm-{py,node,ffi,wasm,php}/src/`
- Hand-written package source in `packages/{go,java,csharp,ruby,elixir}/`

## [1.2.2] - 2026-04-18

### Added

- GitHub Copilot OAuth Device Flow credential provider (`copilot-auth` feature) — use your Copilot subscription as an LLM backend via `github_copilot/` model prefix ([#12](https://github.com/kreuzberg-dev/liter-llm/issues/12))
- GitHub Copilot provider with OpenAI-compatible routing, required Copilot headers, per-request UUID, and `X-Initiator` header
- E2E test fixtures for GitHub Copilot provider (chat + auth error)

### Fixed

- Provider registry audit: corrected base URLs for 20 providers (aiml, assemblyai, clarifai, dashscope, deepseek, elevenlabs, firecrawl, friendliai, gradient_ai, gmi, helicone, lambda_ai, minimax, moonshot, morph, nlp_cloud, ollama, poe, stability, wandb)
- Provider registry audit: corrected env var names for 5 providers (cometapi, fal_ai, gradient_ai, jina_ai, venice)
- Provider registry audit: corrected endpoint lists for 6 providers (cometapi, deepinfra, elevenlabs, jina_ai, mistral, nvidia_nim)
- Added missing `base_url` and `auth` config for 11 previously non-functional providers (amazon_nova, baseten, compactifai, datarobot, docker_model_runner, duckduckgo, langgraph, lemonade, v0, vercel_ai_gateway, zai)
- Added 18 stub/infrastructure providers to `complex_providers` list to prevent incorrect config-driven routing
- Added `nanogpt` param mapping (`max_completion_tokens` → `max_tokens`)

## [1.2.1] - 2026-04-17

### Added

- `LlmClientRaw` trait with `_raw` variants of all `LlmClient` methods, returning `RawExchange<T>` that exposes the final request body and raw provider response before normalization ([#13](https://github.com/kreuzberg-dev/liter-llm/issues/13))
- `RawExchange<T>` and `RawStreamExchange<S>` types for wire-level debugging and custom parsing
- MCP & IDE integration documentation with setup guides for VS Code, GitHub Copilot, Claude Desktop, Cursor ([#12](https://github.com/kreuzberg-dev/liter-llm/issues/12))

### Fixed

- Docker image now published to `ghcr.io/kreuzberg-dev/liter-llm` ([#11](https://github.com/kreuzberg-dev/liter-llm/issues/11))
- Docker publish workflow timeout increased from 60 to 360 minutes (multi-arch Rust builds via QEMU were timing out)
- Bedrock `build_url` tests no longer flake due to `BEDROCK_CROSS_REGION` env var race condition

## [1.2.0] - 2026-04-07

### Added

- Local LLM provider support: Ollama, LM Studio, vLLM, llama.cpp, LocalAI, llamafile -- use any local inference engine via OpenAI-compatible API
- Docker Compose setup for local LLM integration testing with Ollama
- Integration test suite for local LLM providers

### Fixed

- PHP `onError` hook now passes a proper `\Exception` object instead of a plain string (PHP strict types requires `\Throwable`)
- README templates fixed for rumdl compliance (MD040 code fence language, MD031 blank lines, MD032 list spacing, MD020 closed headings)
- Added 404 to all POST endpoint OpenAPI specs (model not found on default model names)
- Homebrew badge added to all READMEs

## [1.1.1] - 2026-03-29

### Fixed

- Java Maven plugins downgraded to 3.x stable (was 4.0.0-beta, incompatible with Maven 3.9.x CI)
- PHP hook isolation (per-client instead of global), budget per-model enforcement, onError hook invocation, shutdown segfault
- PHP e2e tests set `max_retries=0` to prevent retry delays on mock 500s
- OpenAPI spec: added 400/415/422/503 status codes to all endpoints for schemathesis compliance
- `first_client()` returns 503 Service Unavailable instead of 500 for "no models configured"
- Schemathesis CI checks aligned (removed `content_type_conformance`, `not_a_server_error`)
- Docker cache: per-platform `TARGETARCH` cache IDs prevent multi-arch build races

### Added

- Homebrew formula: `brew tap kreuzberg-dev/tap && brew install liter-llm`
- Homebrew bottle builds (arm64_sequoia) in publish workflow
- `liter-llm-proxy` and `liter-llm-cli` added to crates.io publish pipeline
- Installation docs: CLI/Docker/Homebrew tabs
- `scripts/publish/upload-homebrew-bottles.sh` and `ensure-github-release-exists.sh`

## [1.1.0] - 2026-03-29

OpenAI-compatible LLM proxy server with CLI, MCP tool server, and Docker support.

### Proxy Server (`liter-llm-proxy`)

- **22 REST endpoints** — full OpenAI-compatible API surface: chat completions (streaming + non-streaming), embeddings, models, images, audio (speech + transcription), moderations, rerank, search, OCR, files CRUD, batches CRUD, responses CRUD, health
- **Tower middleware stack** — reuses core middleware: cache, rate limit, budget, cost tracking, cooldown, health check, tracing
- **Virtual API keys** — in-memory key store with per-key model restrictions, RPM/TPM limits, budget limits
- **Model routing** — name-based routing to provider deployments, wildcard aliases, deterministic default client
- **OpenDAL file storage** — configurable backend (memory, S3, GCS, filesystem) for file operations
- **SSE streaming** — chat completion chunks proxied as Server-Sent Events with `[DONE]` sentinel
- **OpenAPI 3.1** — utoipa-generated spec served at `/openapi.json` with bearer auth security scheme
- **TOML configuration** — `liter-llm-proxy.toml` with env var interpolation (`${VAR}`), auto-discovery, `deny_unknown_fields`
- **CORS** — configurable origins from config (default: allow all)
- **Graceful shutdown** — SIGINT/SIGTERM handling via `tokio::signal`

### MCP Server (`rmcp`)

- **22 tools** — full parity with REST API: chat, embed, list_models, generate_image, speech, transcribe, moderate, rerank, search, ocr, file CRUD (5), batch CRUD (4), response CRUD (3)
- **Transports** — stdio (default) and HTTP/SSE via `StreamableHttpService`
- **Parameter schemas** — `schemars::JsonSchema` derives for MCP tool discovery

### CLI (`liter-llm`)

- `liter-llm api` — start proxy server with config, host/port overrides, debug logging
- `liter-llm mcp` — start MCP server with stdio or HTTP transport
- 3-tier config precedence: CLI flags > env vars > config file > defaults

### Docker

- Multi-stage build: `rust:1.91-bookworm` builder, `cgr.dev/chainguard/glibc-dynamic` runtime (35MB)
- Non-root execution, OCI labels, port 4000 exposed
- `ENTRYPOINT ["liter-llm"]`, `CMD ["api", "--host", "0.0.0.0", "--port", "4000"]`

### Testing

- **74 unit tests** — config parsing, error mapping, auth key store, service pool, file store, streaming
- **32 integration tests** — auth middleware, chat/embedding/models routes, error propagation, CORS, health, OpenAPI
- **12 proxy e2e fixtures** — chat (basic + streaming), embeddings, models, auth errors, upstream errors, health, images, moderation, reranking
- **Schemathesis** — contract testing against OpenAPI spec via Docker (`task proxy:schemathesis`)

### CI/CD

- `.github/workflows/ci-docker.yaml` — build + health test + schemathesis contract tests
- `.github/workflows/publish-docker.yaml` — multi-arch (amd64/arm64) publish to `ghcr.io/kreuzberg-dev/liter-llm`
- Taskfile: `proxy:test`, `proxy:schemathesis`

## [1.0.0] - 2026-03-28

Initial stable release. Universal LLM API client with native bindings for 11 languages and 142+ providers.

### Core

- `LlmClient` trait with chat, chat_stream, embed, list_models, image_generate, speech, transcribe, moderate, rerank, search, ocr
- `FileClient`, `BatchClient`, `ResponseClient` traits for file/batch/response operations
- `DefaultClient` with reqwest + tokio, SSE streaming, retry with exponential backoff
- `ManagedClient` with composable Tower middleware stack
- 142 LLM providers embedded at compile time from `schemas/providers.json`
- Per-request provider routing from model name prefix (e.g. `anthropic/claude-sonnet-4-20250514`)
- `secrecy::SecretString` for API keys (zeroized on drop, never logged)
- TOML configuration file loading with auto-discovery (`liter-llm.toml`)
- Custom provider registration at runtime

### Middleware (Tower)

- **CacheLayer** — in-memory LRU + pluggable backends via `CacheStore` trait
- **OpenDAL cache** — 40+ storage backends (Redis, S3, GCS, filesystem, etc.) via Apache OpenDAL
- **BudgetLayer** — global + per-model spending limits with hard/soft enforcement
- **HooksLayer** — request/response/error lifecycle callbacks with guardrail pattern
- **CooldownLayer** — circuit breaker after transient errors
- **ModelRateLimitLayer** — per-model RPM/TPM rate limiting
- **HealthCheckLayer** — background health probing
- **CostTrackingLayer** — per-request cost calculation from embedded pricing registry
- **TracingLayer** — OpenTelemetry GenAI semantic convention spans
- **FallbackLayer** — automatic failover to backup provider
- **RouterLayer** — multi-deployment load balancing (round-robin, latency, cost, weighted)

### Language Bindings

All bindings expose the full API surface with language-idiomatic conventions:

- **Python** (PyO3) — async/await, typed kwargs, full .pyi stubs
- **TypeScript / Node.js** (NAPI-RS) — camelCase, .d.ts types, Promise-based
- **Rust** — native, zero-cost
- **Go** (cgo) — FFI wrapper with build tags, `context.Context` support
- **Java** (Panama FFM) — JDK 25+, `AutoCloseable`, builder pattern
- **C# / .NET** (P/Invoke) — async/await, `IAsyncEnumerable` streaming, `IDisposable`
- **Ruby** (Magnus) — RBS type signatures, Enumerator streaming
- **Elixir** (Rustler NIF) — `{:ok, result}` tuples, OTP-compatible
- **PHP** (ext-php-rs) — PHP 8.2+, JSON in/out, PIE packages
- **WebAssembly** (wasm-bindgen) — browser + Node.js, Fetch API
- **C / FFI** (cbindgen) — `extern "C"` with opaque handles

### Authentication

- Static API keys (Bearer, x-api-key)
- Azure AD OAuth2 client credentials
- Vertex AI service account JWT
- AWS STS Web Identity (EKS/IRSA)
- AWS SigV4 signing for Bedrock

### Provider Transforms

- Anthropic: message format, tool use v1, thinking blocks, max_tokens default
- AWS Bedrock: Converse API, EventStream binary framing, cross-region routing
- Vertex AI: Gemini format, embedding `:predict` endpoint
- Google AI: embedding/list_models response transforms
- Cohere: citation handling
- Mistral: API compatibility
- `param_mappings` for config-driven field renaming (8 providers)

### Documentation

- MkDocs Material site at docs.liter-llm.kreuzberg.dev
- 170+ code snippets across 10 languages
- 11 API reference docs with full method coverage
- Usage pages: Chat & Streaming, Embeddings & Rerank, Media, Search & OCR, Files & Batches, Configuration
- TOML configuration reference
- llms.txt (218 lines) with capabilities, examples, provider list
- Skills directory (4,072 lines) for Claude Code integration
- README generation from Jinja templates via `scripts/generate_readme.py`

### Testing

- 500+ unit and integration tests
- Middleware stack composition tests (cache + budget + hooks + rate limit + cooldown)
- Per-request provider routing tests
- File/batch/response CRUD operation tests
- Concurrency tests (budget atomicity, cache contention, rate limit fairness)
- Redis cache backend integration tests (Docker Compose)
- Live provider tests for 7 providers (OpenAI, Anthropic, Google AI, Vertex AI, Mistral, Azure, Bedrock)
- Smoke test apps for all 10 languages against real APIs
- E2E test generation from JSON fixtures across all languages
- Contract test fixtures for binding API parity

### CI/CD

- Multi-platform publish pipeline: crates.io, PyPI, npm, RubyGems, Hex.pm, Maven Central, NuGet, Packagist, Go FFI, PHP PIE
- Pre-commit hooks: 43 linters across all languages
- Post-generation formatting in e2e-generator
- Version sync script across 27+ manifests with README regeneration

### Previous RC Releases

<details>
<summary>Release candidate history (rc.1 through rc.9)</summary>

- **rc.1** (2026-03-27): Initial release — core crate, 11 bindings, e2e generator
- **rc.2** (2026-03-27): Packaging fixes for crates.io, RubyGems, Elixir NIF, Node NAPI, publish workflow
- **rc.3** (2026-03-27): Cache, budget, hooks middleware; custom providers; TDD e2e fixtures
- **rc.4** (2026-03-28): Shared bindings-core crate; camelCase conversion; real streaming across all bindings
- **rc.5** (2026-03-28): OpenDAL cache; search/OCR endpoints; full middleware wiring; Go/Java/C# FFI rewrites; serde deny_unknown_fields; documentation overhaul
- **rc.6** (2026-03-28): Full API documentation coverage; Rust crate README; version sync improvements
- **rc.7** (2026-03-28): Binding parity (5 middleware params + search/ocr in all 10); contract test fixtures; skills directory; PHP PIE packages
- **rc.8** (2026-03-28): CI fixes (PHP publish, crate order, Maven GPG, Ruby deps, Bedrock test)
- **rc.9** (2026-03-28): Live provider tests; Anthropic/Bedrock/Google streaming fixes; TOML config loading; per-request provider routing; integration test suite

</details>

[1.4.0]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.2.2...v1.3.0
[1.2.2]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.1.1...v1.2.0
[1.1.1]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/kreuzberg-dev/liter-llm/releases/tag/v1.0.0
