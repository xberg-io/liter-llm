# liter-llm

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
	<a href="https://central.sonatype.com/artifact/dev.kreuzberg.literllm/liter-llm">
		<img src="https://img.shields.io/maven-central/v/dev.kreuzberg.literllm/liter-llm?label=Java&color=007ec6" alt="Java" />
	</a>
	<a href="https://github.com/kreuzberg-dev/liter-llm/tree/main/packages/go">
		<img src="https://img.shields.io/github/v/tag/kreuzberg-dev/liter-llm?label=Go&color=007ec6" alt="Go" />
	</a>
	<a href="https://www.nuget.org/packages/LiterLlm">
		<img src="https://img.shields.io/nuget/v/LiterLlm?label=C%23&color=007ec6" alt="C#" />
	</a>
	<a href="https://packagist.org/packages/kreuzberg-dev/liter-llm">
		<img src="https://img.shields.io/packagist/v/kreuzberg-dev/liter-llm?label=PHP&color=007ec6" alt="PHP" />
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

<div align="center" style="display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; margin: 28px 0 24px">
	<a href="https://discord.gg/xt9WY3GnKR">
		<img
			height="22"
			src="https://img.shields.io/badge/Discord-Chat-007ec6?logo=discord&logoColor=white"
			alt="Join Discord"
		/>
	</a>
</div>

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
<summary><strong>CLI, Proxy & MCP Server</strong></summary>

The `liter-llm` CLI ships both the OpenAI-compatible proxy and the MCP tool server. Install it any of these ways:

```sh
brew install kreuzberg-dev/tap/liter-llm
cargo install liter-llm-cli                 # from crates.io
npx @kreuzberg/liter-llm-cli --help         # npm (self-installs the binary)
docker run -p 4000:4000 -e LITER_LLM_MASTER_KEY=sk-your-key ghcr.io/kreuzberg-dev/liter-llm
```

Then run the proxy or the MCP server:

```sh
liter-llm api --config liter-llm-proxy.toml   # OpenAI-compatible proxy (22 endpoints)
liter-llm mcp --transport stdio               # MCP tool server (stdio)
liter-llm mcp --transport http --port 3001    # MCP tool server (Streamable HTTP)
```

See the [MCP server guide](https://docs.liter-llm.kreuzberg.dev/server/mcp-server/) and the [proxy guide](https://docs.liter-llm.kreuzberg.dev/server/proxy-server/) for transports, routing, virtual keys, and budgets. To use the MCP server inside a coding agent, install the **liter-llm plugin** (below) — it auto-registers the server, no manual config required.

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
