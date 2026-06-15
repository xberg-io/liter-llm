# Kotlin (Android)

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0">
	<!-- Built with -->
	<a href="https://github.com/kreuzberg-dev/alef">
		<img src="https://img.shields.io/badge/Bindings-alef%20%D7%90-007ec6" alt="Bindings" />
	</a>
	<!-- Language Bindings -->
	<a href="https://crates.io/crates/liter-llm">
		<img src="https://img.shields.io/crates/v/liter-llm?label=Rust&color=007ec6" alt="Rust" />
	</a>
	<a href="https://pypi.org/project/liter-llm/">
		<img src="https://img.shields.io/pypi/v/liter-llm?label=Python&color=007ec6" alt="Python" />
	</a>
	<a href="https://www.npmjs.com/package/@kreuzberg/liter-llm">
		<img src="https://img.shields.io/npm/v/@kreuzberg/liter-llm?label=Node.js&color=007ec6" alt="Node.js" />
	</a>
	<a href="https://www.npmjs.com/package/@kreuzberg/liter-llm-wasm">
		<img src="https://img.shields.io/npm/v/@kreuzberg/liter-llm-wasm?label=WASM&color=007ec6" alt="WASM" />
	</a>
	<a href="https://central.sonatype.com/artifact/dev.kreuzberg/liter-llm">
		<img src="https://img.shields.io/maven-central/v/dev.kreuzberg/liter-llm?label=Java&color=007ec6" alt="Java" />
	</a>
	<a href="https://github.com/kreuzberg-dev/liter-llm/tree/main/packages/go">
		<img src="https://img.shields.io/github/v/tag/kreuzberg-dev/liter-llm?label=Go&color=007ec6" alt="Go" />
	</a>
	<a href="https://www.nuget.org/packages/LiterLlm">
		<img src="https://img.shields.io/nuget/v/LiterLlm?label=C%23&color=007ec6" alt="C#" />
	</a>
	<a href="https://packagist.org/packages/kreuzberg/liter-llm">
		<img src="https://img.shields.io/packagist/v/kreuzberg/liter-llm?label=PHP&color=007ec6" alt="PHP" />
	</a>
	<a href="https://rubygems.org/gems/liter_llm">
		<img src="https://img.shields.io/gem/v/liter_llm?label=Ruby&color=007ec6" alt="Ruby" />
	</a>
	<a href="https://hex.pm/packages/liter_llm">
		<img src="https://img.shields.io/hexpm/v/liter_llm?label=Elixir&color=007ec6" alt="Elixir" />
	</a>
	<a href="https://github.com/kreuzberg-dev/liter-llm/pkgs/container/liter-llm">
		<img src="https://img.shields.io/badge/Docker-007ec6?logo=docker&logoColor=white" alt="Docker" />
	</a>
	<a href="https://github.com/kreuzberg-dev/homebrew-tap/blob/main/Formula/liter-llm.rb">
		<img src="https://img.shields.io/badge/Homebrew-007ec6?logo=homebrew&logoColor=white" alt="Homebrew" />
	</a>
	<a href="https://github.com/kreuzberg-dev/liter-llm/tree/main/crates/liter-llm-ffi">
		<img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C FFI" />
	</a>

	<!-- Project Info -->
	<a href="https://github.com/kreuzberg-dev/liter-llm/blob/main/LICENSE">
		<img src="https://img.shields.io/badge/License-MIT-007ec6" alt="License" />
	</a>
	<a href="https://docs.liter-llm.kreuzberg.dev">
		<img src="https://img.shields.io/badge/Docs-liter--llm-007ec6" alt="Docs" />
	</a>
</div>
<div align="center" style="margin: 24px 0 0">
	<a href="https://kreuzberg.dev">
		<img
			alt="kreuzberg.dev"
			src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8"
		/>
	</a>
</div>
<div align="center" style="display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; margin: 28px 0 24px">
	<a href="https://discord.gg/xt9WY3GnKR">
		<img
			height="22"
			src="https://img.shields.io/badge/Discord-Chat-007ec6?logo=discord&logoColor=white"
			alt="Join Discord"
		/>
	</a>
</div>

Universal LLM API client for Android, packaged as an AAR with bundled JNI libraries for `arm64-v8a` and `x86_64`.
Server-side Kotlin/JVM consumers should use the `dev.kreuzberg.literllm:liter-llm` Java package directly — Panama FFM is unavailable on Android, which is why Android ships as its own artifact.

## What This Package Provides

- **One provider surface** — chat, streaming, embeddings, images, audio, search, OCR, tools, and structured output across the provider registry.
- **Provider/model routing** — call models with the `provider/model` convention and keep provider-specific request code out of application paths.
- **Production controls** — retries, fallback, rate limits, cache layers, budgets, health checks, OpenTelemetry spans, and redacted secrets.
- **Same core as every binding** — Rust, Python, Node.js, Go, Java, PHP, Ruby, .NET, Elixir, WASM, Kotlin Android, Swift, Dart, Zig, and C FFI use the same Rust implementation.
- **Android AAR** — coroutine-friendly package for Android clients.

## Installation

### Package Installation

Install via one of the supported package managers:

**Gradle:**

```gradle
implementation 'dev.kreuzberg:liter-llm:1.6.0-rc.0'
```

**Maven:**

```xml
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>liter-llm</artifactId>
    <version>1.6.0-rc.0</version>
</dependency>
```

### System Requirements

- See [Installation Guide](https://github.com/kreuzberg-dev/liter-llm#installation) for requirements

## Quick Start

### Basic Chat

Send a message to any provider using the `provider/model` prefix:

<!-- snippet not found: getting-started/basic_chat.md -->

### Common Use Cases

### Next Steps

- **[Provider Registry](https://github.com/kreuzberg-dev/liter-llm/blob/main/schemas/providers.json)** - Full list of supported providers
- **[GitHub Repository](https://github.com/kreuzberg-dev/liter-llm)** - Source, issues, and discussions

## Features

### Supported Providers (143+)

Route to any provider using the `provider/model` prefix convention:

| Provider           | Example Model                                                 |
| ------------------ | ------------------------------------------------------------- |
| **OpenAI**         | `openai/gpt-4o`, `openai/gpt-4o-mini`                         |
| **Anthropic**      | `anthropic/claude-3-5-sonnet-20241022`                        |
| **Groq**           | `groq/llama-3.1-70b-versatile`                                |
| **Mistral**        | `mistral/mistral-large-latest`                                |
| **Cohere**         | `cohere/command-r-plus`                                       |
| **Together AI**    | `together/meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo`       |
| **Fireworks**      | `fireworks/accounts/fireworks/models/llama-v3p1-70b-instruct` |
| **Google Vertex**  | `vertexai/gemini-1.5-pro`                                     |
| **Amazon Bedrock** | `bedrock/anthropic.claude-3-5-sonnet-20241022-v2:0`           |

**[Complete Provider List](https://github.com/kreuzberg-dev/liter-llm/blob/main/schemas/providers.json)**

### Key Capabilities

- **Provider Routing** -- Single client for 143+ LLM providers via `provider/model` prefix
- **Local LLMs** — Connect to locally-hosted models via Ollama, LM Studio, vLLM, llama.cpp, and other local inference servers
- **Unified API** -- Consistent `chat`, `chat_stream`, `embeddings`, `list_models` interface
- **Tool Calling** -- Function calling and tool use across all supporting providers
- **Type Safe** -- Schema-driven types compiled from JSON schemas
- **Secure** -- API keys never logged or serialized, managed via environment variables
- **Observability** -- Built-in [OpenTelemetry](https://opentelemetry.io/docs/specs/semconv/gen-ai/) with GenAI semantic conventions
- **Error Handling** -- Structured errors with provider context and retry hints

### Performance

Built on a compiled Rust core for speed and safety:

- **Provider resolution** at client construction -- zero per-request overhead
- **Configurable timeouts** and connection pooling
- **Zero-copy streaming** with SSE and AWS EventStream support
- **API keys** wrapped in secure memory, zeroed on drop

## Provider Routing

Route to 143+ providers using the `provider/model` prefix convention:

```text
openai/gpt-4o
anthropic/claude-3-5-sonnet-20241022
groq/llama-3.1-70b-versatile
mistral/mistral-large-latest
```

See the [provider registry](https://github.com/kreuzberg-dev/liter-llm/blob/main/schemas/providers.json) for the full list.

## Proxy Server

liter-llm also ships as an OpenAI-compatible proxy server with Docker support:

```bash
docker run -p 4000:4000 -e LITER_LLM_MASTER_KEY=sk-your-key ghcr.io/kreuzberg-dev/liter-llm
```

See the [proxy server documentation](https://docs.liter-llm.kreuzberg.dev/server/proxy-server/) for configuration, CLI usage, and MCP integration.

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
