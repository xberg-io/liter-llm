# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.0-rc.13] - 2026-04-29

### Fixed

- Switch reqwest TLS crypto provider from `aws-lc-rs` to `ring` by using `rustls-no-provider` feature and adding an explicit `rustls` dependency with `ring` backend. This eliminates `__isoc23_strtol` and related glibc 2.38+ symbols emitted by `aws-lc-sys` 0.40.0, restoring the GLIBC_2.28 ABI floor required by downstream users (e.g. Node.js aarch64 bindings).

## [Unreleased]

### Changed

- `iter26/wrap`: skip `edge_file_large_upload` for wasm — reqwest's `wasm32` multipart backend builds the body via `web_sys::FormData` + Blob and the resulting POST is rejected at fetch time (`error sending request`) in the vitest/Node environment before reaching the mock server. The 13 other backends use native reqwest multipart and pass the same fixture. Bumped alef pin to `0.15.57` and ran `prek autoupdate`. Added pre-commit exclusions to keep host-language formatters (ruff, php-cs-fixer, rubocop, oxfmt) from rewriting alef-managed binding files, which was producing a verify/format ping-pong; excluded the alef-emitted swift workflow from actionlint and the alef-generated swift test sources from typos.
- `iter26`: bump alef pin to `0.15.57`, regenerate all 14 language bindings + e2e suites. Picks up: kotlin scaffold `srcDir(".")` so the alef-emitted facade ends up in the jar (fixes `Unresolved reference 'LiterLlm'` in e2e), kotlin streaming codegen sticky-nullability through chained method calls and `chunks.last().usage()` rewrite for streaming-virtual fields, go transcribe value-typed element nil-check, swift `not_empty`/`is_empty` over `RustVec<T>` via `.len()` instead of `.toString().isEmpty`, kotlin mock-URL system-property fallback for the in-process MockServerListener, and Java FFM `Arena.ofShared()` so kotlin coroutines (and any thread-hopping caller) don't trip `WrongThreadException`. Fixture skips for `local_chat_ollama`, `local_list_models_ollama`, and `error_file_bad_purpose` extended to include kotlin to match python's platform-divergence rationale.

- `iter22` (complete, local alef): wired `StreamingFieldResolver` into kotlin/python/elixir/zig/swift codegen — every streaming-virtual assertion (`chunks`, `chunks.length`, `stream_content`, `stream_complete`, `no_chunks_after_done`, `tool_calls`, `finish_reason`) and deep-nested paths (`tool_calls[0].function.name`) now resolve against the per-language collected list. Broadened `is_streaming` detection in python/elixir/kotlin to also fire when the fixture asserts on a streaming-virtual field even if `mock_response.stream_chunks` is empty (fixes `empty_stream`). Regenerated all 15 language bindings + e2e suites. **Skipped-assertion total: 354 → 0 across every language.**
- `iter22` (in flight, local alef): swift `{f}` literal placeholder fixed in alef-e2e/codegen/swift.rs (same class of double-brace escape bug iter21 Phase C fixed in kotlin). Generated swift streaming tests now show real field names instead of `'{f}'`. Skipped-assertion total drops 228 → 126.
- `iter22/fixtures`: remove `equals` assertion with `value: null` from `fixtures/chat/finish_reason_content_filter.json` — the codegen can't model "expected null" cleanly and the remaining assertions (no error, 1 choice, finish_reason == 'content_filter') already cover the content-filter semantics.
- `iter21`: bump alef to `0.15.47`, regenerate all 15 language bindings + e2e suites. Skipped-assertion comments across e2e suites drop from 354 → 228 (~36%). Headline wins: swift `XCTSkipIf("swift: json_object …")` stubs 140 → 0 (alef-backend-swift FromJson generalization), dart `// skipped: unknown assertion type` 132 → 0 (dart renderer parity with rust/kotlin/go — all 24 fixture assertion types), kotlin `{f}` literal placeholder fixed, user-typed array indices `choices[0].message.content` now parse correctly in fixture field paths.

### Removed

- `iter21/gleam`: drop the gleam binding (packages/gleam/, e2e/gleam/, test_apps/gleam/, .github/workflows/gleam.yml, docs/reference/api-gleam.md) and all gleam alef.toml config blocks (22 entries: `languages` list, `[crates.gleam]`, workspace.sync entry, 20 per-call override blocks). Reasoning: same BEAM VM as elixir → zero runtime/perf differentiation, ~15k devs vs elixir's ~200k, codegen DSL gaps in alef-e2e/gleam would require ongoing per-language work to close.

### Added

- `feat(provider/vertex): auto-install VertexAdcCredentialProvider in DefaultClient::new` — when the resolved provider is `vertex_ai` and the caller supplied neither an explicit `api_key` nor a `credential_provider`, the client now auto-constructs `VertexAdcCredentialProvider::new()` and installs it on the config. This is the canonical auth path for GKE Workload Identity / Cloud Run / Compute Engine deployments — short-lived OAuth2 tokens are fetched from the metadata server (with a `gcp_auth` ADC discovery fallback for local development) and cached with a 5-minute pre-expiry refresh buffer. Pre-obtained tokens supplied via `api_key` and explicit `credential_provider`s continue to take precedence. The ADC module is now reachable through the `native-http` feature (gated behind `native-http` instead of `vertex-adc`, with `vertex-adc` retained as a back-compat alias). Closes the kreuzberg-cloud Vertex-AI 401 path where `kreuzberg::LlmConfig` leaves `api_key = None` and the bare `Authorization: Bearer` header was being sent. Adds three regression tests in `client::build_provider_tests`: auto-install fires for vertex with no creds, auto-install skipped when `api_key` is set, auto-install does not overwrite an explicit `credential_provider`. Pre-existing `auth::vertex_adc::tests` that mutate `VERTEX_AI_SCOPE` now use `#[serial_test::serial(vertex_adc_env)]` so they remain stable now that the module compiles on every `native-http` build.
- `iter20`: bump alef to `0.15.45`, regenerate 16 language bindings + e2e suites. Removes `chat_stream` skip for go (cgo channel bridge) and dart (FRB v2 `StreamSink<T>`). Adds `options_via = "from_json"` per-call overrides for kotlin, gleam, and swift across 13 json-object calls (`chat`, `chat_stream`, `embed`, `image_generate`, `speech`, `transcribe`, `moderate`, `rerank`, `search`, `ocr`, `create_file`, `create_batch`, `responses`).
- `ci-mobile`: new `.github/workflows/ci-mobile.yaml` running `android-check` (ubuntu, `arm64-v8a` + `x86_64` via `cargo ndk`), `ios-check` (macos, `aarch64-apple-ios` + `aarch64-apple-ios-sim`), and `xcframework-build` (macos, SPM-ready XCFramework + SHA256 checksum). Uses shared composite actions from `kreuzberg-dev/actions@v1`.

### Fixed

- `chore(readme/go): fix broken jinja substitution on the GoDoc link` — the markdown autolink wrapper `<...>` around the URL was confusing the template parser into seeing `{{>` as a tag opener and aborting `task alef:all`.

- `fix(provider/azure): honour per-model base_url overrides for Azure deployments` — `[[models]]` entries that pinned a `base_url` for an `azure/...` `provider_model` were silently routed through the generic OpenAI-compatible URL builder (`{base_url}{path}`), bypassing Azure's required `{base_url}/openai/deployments/{model}{path}?api-version=…` shape and producing 404s. `build_provider` now detects azure-prefixed model hints with a `base_url` override and constructs an `AzureProvider` with the per-model URL via the new `AzureProvider::with_base_url(...)` constructor. Unblocks multi-resource Azure setups (e.g. different deployments in different regions/subscriptions per model). Closes #83.
- `fix(http/retry): avoid `SystemTime::now()`jitter on`wasm32-unknown-unknown``— the retry-jitter calculation called`std::time::SystemTime::now()`which panics with`RuntimeError: unreachable`on the bare`wasm32-unknown-unknown`target (std time is not implemented). On`wasm32`the jitter step is skipped and the capped exponential delay is used as-is; native targets keep the existing`[0.5x, 1.0x]` jitter. This unblocks WASM e2e tests that exercise 429/5xx retry paths (`list_models_error_500`, `error_image_rate_limit`).

### Added

- `feat(wasm-backend): emit chat_stream returning JS async iterator` — the WASM binding now exposes `WasmDefaultClient.chat_stream(req)` (alongside the existing `chat`, `embed`, etc.). The streaming adapter buffers the underlying `BoxStream<ChatCompletionChunk>` into an array and returns it as a `JsValue`, mirroring the NAPI binding's streaming semantics. Required regenerating `crates/liter-llm-wasm/src/lib.rs` once the backend's cfg-gate evaluator was fixed to honour `any(feature = "native-http", feature = "wasm-http")` (previously the binding silently dropped `DefaultClient` and every method on it).
- `fix(e2e/wasm): use --target nodejs build artifact for e2e import path` — added `task wasm:build:nodejs` (alias for `wasm:build:node`) and `task wasm:e2e:setup` which builds the local `pkg/` with `wasm-pack build --target nodejs` and runs `pnpm install` in `e2e/wasm/`. Test files now import the package by name (resolved via `pkg/package.json` `main`); the historical alef rewrite to a non-existent `<pkg>/dist-node` subpath is gone.
- CLI binary tarballs (Linux x86_64/aarch64, macOS aarch64, Windows x86_64) attached to GitHub Releases for direct download — closes #64
- `scripts/generate_pricing.py` regenerates `schemas/pricing.json` from [models.dev](https://models.dev), wired into `task generate:pricing`, `task update`, and `task upgrade`
- `Usage::prompt_tokens_details` (`{ cached_tokens, audio_tokens }`) deserialised from the OpenAI-compatible response body, plus `cost::completion_cost_with_cache` and matching `cache_read_input_token_cost` / `cache_creation_input_token_cost` fields on `ModelPricing`. `ChatCompletionResponse::estimated_cost` and the `CostTrackingLayer` now bill cached prompt tokens at the provider's discounted cache-read rate when the model has cache pricing in `schemas/pricing.json` — closes #65
- `schemas/pricing.json` carries `cache_read_input_token_cost` / `cache_creation_input_token_cost` for the 1,500+ models on models.dev that publish cache pricing

### Changed

- `schemas/pricing.json` now covers 4,219 models (up from 35) sourced from models.dev — closes #48
- GitHub Release CLI assets ship a single sorted `SHA256SUMS-<version>.txt` (sha256sum-verifiable) instead of one `.sha256` per archive — closes #67
- **WebAssembly build verified `mio`-free.** The `liter-llm` crate exposes two mutually exclusive HTTP-stack features — `native-http` (reqwest + tokio + memchr + base64) and `wasm-http` (reqwest + memchr + base64 + gloo-timers, _no_ tokio dependency). The `liter-llm-wasm` crate enables only `wasm-http`; the workspace's `reqwest` is pinned with `default-features = false, features = ["json", "stream", "rustls", "multipart", "form"]`. As a result, `cargo build --target wasm32-unknown-unknown -p liter-llm-wasm` pulls neither `mio` nor `tokio` into the dependency tree — reqwest auto-routes to the browser/Node `fetch` API on `wasm32` targets.

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

[Unreleased]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.2.2...HEAD
[1.2.2]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.1.1...v1.2.0
[1.1.1]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/kreuzberg-dev/liter-llm/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/kreuzberg-dev/liter-llm/releases/tag/v1.0.0
