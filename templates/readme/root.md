# liter-llm

{% include 'partials/badges.html' %}

{% include 'partials/discord.html' %}

**A lighter, faster, safer universal LLM API client** — one Rust core, 14 native language bindings, 143 providers.

## What and Why?

liter-llm is a universal LLM API client compiled from the ground up in Rust: one core, 14 native language bindings, and 143 providers. No interpreter, no transitive dependency tree, no supply-chain surface area — and a drop-in OpenAI-compatible proxy plus an MCP server in a single 35 MB binary.

- **Compiled Rust core** — no `pip install` supply chain, no `.pth` auto-execution hooks, no runtime dependency tree to compromise.
- **Secrets stay secret** — API keys are wrapped in [`secrecy::SecretString`](https://docs.rs/secrecy/): zeroed on drop, redacted in logs, never serialized.
- **Polyglot from day one** — Python, TypeScript, Go, Java, Kotlin, C#, Ruby, PHP, Elixir, Dart, Swift, Zig, and WebAssembly, all thin wrappers over the same Rust core, plus a C/FFI surface for everything else.
- **Observability built in** — production-grade [OpenTelemetry](https://opentelemetry.io/) with GenAI semantic conventions, not an afterthought callback system.
- **Composable middleware** — rate limiting, caching, cost tracking, health checks, and fallback as [Tower](https://docs.rs/tower/) layers you stack like building blocks.

We credit [litellm](https://github.com/BerriAI/litellm) for proving the category; our provider registry was bootstrapped from theirs. See [ATTRIBUTIONS.md](ATTRIBUTIONS.md).

### Features

| Feature | Description |
| ------- | ----------- |
| **143 providers** | OpenAI, Anthropic, Google, AWS Bedrock, Groq, Mistral, Together, Fireworks, DeepSeek, Cohere, and 130+ more — compiled at build time |
| **14 native bindings** | Rust, Python, Node.js, Go, Java, Kotlin, C#, Ruby, PHP, Elixir, Dart, Swift, Zig, WebAssembly — plus a shared C/FFI surface |
| **First-class streaming** | SSE and AWS EventStream binary protocol with zero-copy buffers |
| **Proxy & MCP server** | Drop-in OpenAI-compatible proxy (22 endpoints) and MCP tool server in a 35 MB Docker image |
| **Tower middleware** | Rate limiting, caching (40+ OpenDAL backends), cost tracking, budget enforcement, health checks, and fallback — all composable |
| **Observability** | OpenTelemetry with GenAI semantic conventions, cost-tracking spans, and HTTP-level tracing |
| **Tool calling** | Parallel tools, structured outputs, and JSON-schema validation |
| **Search & OCR** | Web search across 12 providers, document OCR across 4 |
| **TOML configuration** | `liter-llm.toml` auto-discovery, custom providers, cache backends, and middleware config |
| **Local LLM support** | Ollama, LM Studio, vLLM, llama.cpp, LocalAI, and llamafile via OpenAI-compatible APIs |

<div align="center">
  <a href="https://github.com/kreuzberg-dev/liter-llm/stargazers">
    <img src="docs/assets/star.gif" alt="Star liter-llm on GitHub" width="640">
  </a>
</div>

<p align="center"><strong>⭐ Star this repo to show your support — it helps others discover liter-llm.</strong></p>

## Quick Start

### Language Packages

<details open>
<summary><strong>Python</strong></summary>

```sh
pip install liter-llm
```

See [Python README](packages/python/README.md) for full documentation.

</details>

<details>
<summary><strong>Node.js</strong></summary>

```sh
pnpm add @kreuzberg/liter-llm
```

See [Node.js README](crates/liter-llm-node/README.md) for full documentation.

</details>

<details>
<summary><strong>Rust</strong></summary>

```sh
cargo add liter-llm
```

See [Rust crate](crates/liter-llm) for full documentation.

</details>

<details>
<summary><strong>Go</strong></summary>

```sh
go get github.com/kreuzberg-dev/liter-llm/packages/go
```

See [Go README](packages/go/README.md) for full documentation.

</details>

<details>
<summary><strong>Java</strong></summary>

Available on Maven Central as `dev.kreuzberg.literllm:liter-llm`. See [Java README](packages/java/README.md) for the dependency snippet and current version.

</details>

<details>
<summary><strong>Ruby</strong></summary>

```sh
gem install liter_llm
```

See [Ruby README](packages/ruby/README.md) for full documentation.

</details>

<details>
<summary><strong>PHP</strong></summary>

```sh
composer require kreuzberg-dev/liter-llm
```

See [PHP README](packages/php/README.md) for full documentation.

</details>

<details>
<summary><strong>C#</strong></summary>

```sh
dotnet add package LiterLlm
```

See [.NET README](packages/csharp/README.md) for full documentation.

</details>

<details>
<summary><strong>Elixir</strong></summary>

Add `{:liter_llm, "~> 1.6"}` to your `mix.exs` dependencies. See [Elixir README](packages/elixir/README.md) for full documentation.

</details>

<details>
<summary><strong>Dart / Flutter</strong></summary>

```sh
dart pub add liter_llm
```

See [Dart README](packages/dart/README.md) for full documentation.

</details>

<details>
<summary><strong>Swift</strong></summary>

Add via Swift Package Manager. See [Swift README](packages/swift/README.md) for full documentation.

</details>

<details>
<summary><strong>Kotlin (Android)</strong></summary>

Available on Maven Central as `dev.kreuzberg:liter-llm-android`. See [Kotlin README](packages/kotlin-android/README.md) for the dependency snippet and current version.

</details>

<details>
<summary><strong>Zig</strong></summary>

See [Zig README](packages/zig/README.md) for installation and usage.

</details>

<details>
<summary><strong>WebAssembly</strong></summary>

```sh
pnpm add @kreuzberg/liter-llm-wasm
```

See [WebAssembly README](crates/liter-llm-wasm/README.md) for full documentation.

</details>

<details>
<summary><strong>C/C++ (FFI)</strong></summary>

Build from source as part of this workspace. See [FFI crate](crates/liter-llm-ffi) for full documentation.

</details>

<details>
<summary><strong>CLI & Proxy Server</strong></summary>

The `liter-llm` CLI ships both the OpenAI-compatible proxy and the MCP tool server.

```sh
brew install kreuzberg-dev/tap/liter-llm
```

```sh
docker run -p 4000:4000 -e LITER_LLM_MASTER_KEY=sk-your-key ghcr.io/kreuzberg-dev/liter-llm
```

```sh
liter-llm api --config liter-llm.toml   # OpenAI-compatible proxy (22 endpoints)
liter-llm mcp --transport stdio         # MCP tool server
```

See the [proxy guide](https://docs.liter-llm.kreuzberg.dev/) for routing, virtual keys, and budgets.

</details>

### AI Coding Assistants

Install the liter-llm plugin from the [`kreuzberg-dev/plugins`](https://github.com/kreuzberg-dev/plugins) marketplace. It ships the liter-llm agent skills (chat, streaming, tools, embeddings across 143 providers) and works with every major coding agent — expand your harness below.

<details open>
<summary><strong>Claude Code</strong></summary>

```text
/plugin marketplace add kreuzberg-dev/plugins
/plugin install liter-llm@kreuzberg
```

</details>

<details>
<summary><strong>Codex CLI</strong></summary>

```text
/plugins add https://github.com/kreuzberg-dev/plugins
```

Then search for `liter-llm` and select **Install Plugin**.

</details>

<details>
<summary><strong>Cursor</strong></summary>

Settings → Plugins → Add from URL → `https://github.com/kreuzberg-dev/plugins`, then select **liter-llm**.

</details>

<details>
<summary><strong>Gemini CLI</strong></summary>

```text
gemini extensions install https://github.com/kreuzberg-dev/plugins
```

</details>

<details>
<summary><strong>Factory Droid</strong></summary>

```text
droid plugin marketplace add https://github.com/kreuzberg-dev/plugins
droid plugin install liter-llm@kreuzberg
```

</details>

<details>
<summary><strong>GitHub Copilot CLI</strong></summary>

```text
copilot plugin marketplace add https://github.com/kreuzberg-dev/plugins
copilot plugin install liter-llm@kreuzberg
```

</details>

<details>
<summary><strong>opencode</strong></summary>

Not yet published as an opencode package. Install via any harness above (self-hosted marketplace); opencode support is tracked in [`kreuzberg-dev/plugins`](https://github.com/kreuzberg-dev/plugins).

</details>

## Documentation

Full guides, the unified `chat()` API for every binding, multimodal I/O, the proxy/gateway, and the complete provider list live at **[docs.liter-llm.kreuzberg.dev](https://docs.liter-llm.kreuzberg.dev)**.

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Join our [Discord community](https://discord.gg/xt9WY3GnKR) for questions and discussion.

## Part of Kreuzberg.dev

- [Kreuzberg](https://github.com/kreuzberg-dev/kreuzberg) — document intelligence: text, tables, metadata from 91+ formats with optional OCR.
- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/kreuzberg-dev/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.

## License

MIT — see [LICENSE](LICENSE) for details.
