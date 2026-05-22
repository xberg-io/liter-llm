# {{ name | replace("#", "\\#") }}

{% include 'partials/badges.html' %}
{% include 'partials/banner.html' %}
{% include 'partials/discord.html' %}

{{ description }}

## What This Package Provides

- **One provider surface** — chat, streaming, embeddings, images, audio, search, OCR, tools, and structured output across the provider registry.
- **Provider/model routing** — call models with the `provider/model` convention and keep provider-specific request code out of application paths.
- **Production controls** — retries, fallback, rate limits, cache layers, budgets, health checks, OpenTelemetry spans, and redacted secrets.
- **Same core as every binding** — Rust, Python, Node.js, Go, Java, PHP, Ruby, .NET, Elixir, WASM, Kotlin Android, Swift, Dart, Zig, and C FFI use the same Rust implementation.
{% if language == "typescript" %}
- **Node-first TypeScript API** — NAPI-RS package with typed requests/responses and async iterables for streaming.
{% elif language == "python" %}
- **Python package** — native async/await, streaming, and typed request/response objects.
{% elif language == "rust" %}
- **Rust crate** — canonical async client with Tower middleware and Tokio integration.
{% elif language == "go" %}
- **Go module** — context-aware API over the shared native client.
{% elif language == "java" %}
- **Java package** — type-safe FFM binding for JVM services.
{% elif language == "php" %}
- **PHP package** — PHP 8.2+ API for unified provider calls.
{% elif language == "ruby" %}
- **Ruby package** — native extension with idiomatic Ruby request and response objects.
{% elif language == "csharp" %}
- **.NET package** — async/await API with nullable-aware generated types.
{% elif language == "elixir" %}
- **BEAM package** — Rustler NIF binding for OTP services.
{% elif language == "wasm" %}
- **WASM package** — browser and edge-compatible client for provider calls from WebAssembly runtimes.
{% elif language == "kotlin_android" %}
- **Android AAR** — coroutine-friendly package for Android clients.
{% elif language == "swift" %}
- **SwiftPM package** — Swift Concurrency API with Codable request/response types.
{% elif language == "dart" %}
- **Dart package** — Future/Stream API for Dart and Flutter clients.
{% elif language == "zig" %}
- **Zig package** — allocator-aware API with explicit error sets.
{% endif %}

## Installation

{% include 'partials/installation.md' %}

## Quick Start

{% include 'partials/quick_start.md' %}

{% if language == "typescript" %}
{% include 'partials/napi_implementation.md' %}

{% endif %}

## Features

{% include 'partials/features.md' %}

{% if features.provider_routing %}

## Provider Routing

Route to 143+ providers using the `provider/model` prefix convention:

```text
openai/gpt-4o
anthropic/claude-3-5-sonnet-20241022
groq/llama-3.1-70b-versatile
mistral/mistral-large-latest
```

See the [provider registry](https://github.com/kreuzberg-dev/liter-llm/blob/main/schemas/providers.json) for the full list.

{% endif %}

{% include 'partials/proxy_server.md' %}

## Documentation

- **[Documentation](https://docs.liter-llm.kreuzberg.dev)** -- Full docs and API reference
- **[GitHub Repository](https://github.com/kreuzberg-dev/liter-llm)** -- Source, issues, and discussions
- **[Provider Registry](https://github.com/kreuzberg-dev/liter-llm/blob/main/schemas/providers.json)** -- 143 supported providers

## Part of Kreuzberg.dev

- [Kreuzberg](https://github.com/kreuzberg-dev/kreuzberg) — document intelligence: text, tables, metadata from 90+ formats with optional OCR.
- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](https://github.com/kreuzberg-dev/liter-llm/blob/main/CONTRIBUTING.md) for guidelines.

Join our [Discord community](https://discord.gg/xt9WY3GnKR) for questions and discussion.

## License

MIT -- see [LICENSE](https://github.com/kreuzberg-dev/liter-llm/blob/main/LICENSE) for details.
