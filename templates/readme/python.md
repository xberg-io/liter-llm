# liter-llm — Python

{% include 'partials/badges.html' %}
{% include 'partials/banner.html' %}
{% include 'partials/discord.html' %}

{{ description }}

## What This Package Provides

- **Python-native client** — async/await, streaming, tools, structured output, and typed request/response objects.
- **Provider/model routing** — call `provider/model` names without provider-specific client branches.
- **Production controls** — retries, fallback, rate limits, cache layers, budgets, health checks, OpenTelemetry spans, and redacted secrets.
- **Same Rust core as every binding** — behavior matches the Rust, Node.js, Go, Java, .NET, Ruby, PHP, Elixir, Swift, Dart, Zig, WASM, and C FFI packages.

## Installation

```bash
pip install liter-llm
```

## Quick Start

{% include 'partials/quick_start.md' %}

## Features

{% include 'partials/features.md' %}

{% include 'partials/proxy_server.md' %}

## Documentation

- **[Documentation](https://docs.liter-llm.kreuzberg.dev)** -- Full docs and API reference
- **[GitHub Repository](https://github.com/kreuzberg-dev/liter-llm)** -- Source, issues, and discussions
- **[Provider Registry](https://github.com/kreuzberg-dev/liter-llm/blob/main/schemas/providers.json)** -- 143 supported providers

## Part of Kreuzberg.dev

- [Kreuzberg](https://github.com/kreuzberg-dev/kreuzberg) — document intelligence: text, tables, metadata from 91+ formats with optional OCR.
- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/kreuzberg-dev/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

{{ license }} License - see [LICENSE](../../LICENSE) for details.
