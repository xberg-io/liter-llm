<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-dark.svg">
    <img alt="Xberg" width="420" src="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-light.svg">
  </picture>
</p>

# liter-llm

{% include 'partials/badges.html' %}

{% include 'partials/discord.html' %}

**One API for every LLM — in your language, without the rewrites.**

## What and Why?

liter-llm gives you a single, consistent way to call any large language model. Reach 174 providers — OpenAI, Anthropic, Google, Bedrock, and more — through one client, and switch models by changing a name instead of your code. The same client ships to 14 languages, all built on one Rust core, so you get identical behavior everywhere. Need a gateway? Point your existing OpenAI SDK at the built-in, drop-in proxy — no rewrites, no separate service to run.

- **Work with any provider without rewrites** — 174 providers behind one API; change the model name to switch, your code stays the same.
- **Use it from your language** — Python, TypeScript, Go, Java, Kotlin, C#, Ruby, PHP, Elixir, Dart, Swift, Zig, and WebAssembly, all built on the same Rust core, plus a C/FFI surface for everything else.
- **Secure by default** — API keys are wrapped, redacted from logs, and never serialized; cloud auth for Azure, AWS, and Vertex refreshes and rotates for you.
- **One small binary** — the client, a drop-in OpenAI-compatible proxy, and an MCP server for AI agents ship in a single 35 MB binary with no interpreter and no dependency tree to compromise.
- **Stay up when a provider fails** — route by cost, latency, or weight and fall back automatically, with built-in rate limiting, caching, and cost tracking.

Deeper details — observability, middleware, and the full endpoint list — live in the [documentation](https://docs.liter-llm.xberg.io).

We credit [litellm](https://github.com/BerriAI/litellm) for proving the category; our provider registry was bootstrapped from theirs. See [ATTRIBUTIONS.md](ATTRIBUTIONS.md).

### Features

| Feature | Description |
| ------- | ----------- |
| **174 providers** | OpenAI, Anthropic, Google, AWS Bedrock, Groq, Mistral, Together, Fireworks, DeepSeek, Cohere, and 130+ more — compiled at build time |
| **14 native bindings** | Rust, Python, Node.js, Go, Java, Kotlin, C#, Ruby, PHP, Elixir, Dart, Swift, Zig, WebAssembly — plus a shared C/FFI surface |
| **First-class streaming** | SSE and AWS EventStream binary protocol with zero-copy buffers |
| **Proxy & MCP server** | Drop-in OpenAI-compatible proxy (22 endpoints) and MCP tool server in a 35 MB Docker image |
| **Tower middleware** | Rate limiting, caching (40+ OpenDAL backends), cost tracking, budget enforcement, health checks, and fallback — all composable |
| **Observability** | OpenTelemetry with GenAI semantic conventions, cost-tracking spans, and HTTP-level tracing |
| **Tool calling** | Parallel tools, structured outputs, and JSON-schema validation |
| **Search & OCR** | Web search across 12 providers, document OCR across 4 |
| **TOML configuration** | `liter-llm.toml` auto-discovery, custom providers, cache backends, and middleware config |
| **Local LLM support** | Ollama through its native API; LM Studio, vLLM, llama.cpp, LocalAI, and llamafile through OpenAI-compatible APIs |

<p align="center"><strong>⭐ <a href="https://github.com/xberg-io/liter-llm/stargazers">Star this repo</a> to show your support — it helps others discover liter-llm.</strong></p>

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
pnpm add @xberg-io/liter-llm
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
go get github.com/xberg-io/liter-llm/packages/go/v2
```

See [Go README](packages/go/README.md) for full documentation.

</details>

<details>
<summary><strong>Java</strong></summary>

Available on Maven Central as `io.xberg.literllm:liter-llm`. See [Java README](packages/java/README.md) for the dependency snippet and current version.

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
composer require xberg-io/liter-llm
```

See [PHP README](crates/liter-llm-php/README.md) for full documentation.

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

Available on Maven Central as `io.xberg.literllm:liter-llm-android`. See [Kotlin README](packages/kotlin-android/README.md) for the dependency snippet and current version.

</details>

<details>
<summary><strong>Zig</strong></summary>

See [Zig README](packages/zig/README.md) for installation and usage.

</details>

<details>
<summary><strong>WebAssembly</strong></summary>

```sh
pnpm add @xberg-io/liter-llm-wasm
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
brew install xberg-io/tap/liter-llm
cargo install liter-llm-cli                 # from crates.io
cargo binstall liter-llm-cli               # prebuilt binary from GitHub Releases
pnpm dlx @xberg-io/liter-llm-cli --help         # npm (self-installs the binary)
docker run -p 4000:4000 -e LITER_LLM_MASTER_KEY=sk-your-key ghcr.io/xberg-io/liter-llm
```

Windows users can install the same binary through [Scoop](https://scoop.sh):

```powershell
scoop bucket add xberg https://github.com/xberg-io/scoop-bucket
scoop install liter-llm
```

Then run the proxy or the MCP server:

```sh
liter-llm api --config liter-llm-proxy.toml   # OpenAI-compatible proxy (22 endpoints)
liter-llm mcp --transport stdio               # MCP tool server (stdio)
liter-llm mcp --transport http --port 3001    # MCP tool server (Streamable HTTP)
```

See the [MCP server guide](https://docs.liter-llm.xberg.io/server/mcp-server/) and the [proxy guide](https://docs.liter-llm.xberg.io/server/proxy-server/) for transports, routing, virtual keys, and budgets. To use the MCP server inside a coding agent, install the **liter-llm plugin** (below) — it registers the server for your MCP client. The CLI release compatibility requirements below also apply to auto-installed binaries.

</details>

### AI Coding Assistants

Install the liter-llm plugin from [`xberg-io/liter-llm`](https://github.com/xberg-io/liter-llm). It ships the liter-llm agent skills (chat, streaming, tools, embeddings across 174 providers) — select your runtime below.

The bundle targets Claude Code, Codex, Cursor, Gemini CLI, Kimi, OpenCode, Factory Droid,
Hermes, Pi, Agent Plugins and GitHub Copilot CLI. Read [the plugin guide](https://github.com/xberg-io/liter-llm/blob/main/plugin/README.md)
for runtime resources, CLI prerequisites and configuration. MCP-capable adapters expose the
product's tools; Hermes uses a local stdio bridge and explicitly selected skills. Pi loads
skills and prompt templates and requires the product CLI for executable operations.

Install only the public [Agent Skills](https://github.com/xberg-io/liter-llm/tree/main/skills),
without a plugin or MCP registration, using the [skills CLI](https://github.com/vercel-labs/skills):

```sh
npx skills add xberg-io/liter-llm --list
npx skills add xberg-io/liter-llm
```

Choose the skills and target agents interactively. Installing instructions does not install the
product CLI or configure provider credentials.

Published CLI 2.2.3 predates the local-stdio trust and deferred-provider-credential fixes.
Until a CLI release includes them, use a source build for startup without an extra auth config:

```sh
cargo install --git https://github.com/xberg-io/liter-llm \
  --rev f14fb8862f9cc73e2c4ab38a2876efe77fab8bf4 --locked liter-llm-cli
```

Put the installed binary on the assistant's PATH. If a plugin has already cached an older binary,
remove that plugin's `bin/liter-llm` (or `bin/liter-llm.exe`) before retrying so it selects the source build.
The released 2.2.3 binary requires `mcp.stdio_trust_local = true` or a configured `mcp.stdio_key_id`;
configured providers may also require credentials at startup. The source build defers missing provider
credentials until a request uses that provider. HTTP authentication remains configured separately.

The source-built stdio MCP server trusts its local host by default and starts without a provider key.
Set provider credentials such as `OPENAI_API_KEY` or `ANTHROPIC_API_KEY` before sending
requests to that provider; missing credentials fail the request. HTTP MCP access retains
its configured authentication. Use `liter-llm api` for the proxy and `liter-llm mcp` for MCP;
chat and streaming are SDK, proxy or MCP operations rather than CLI subcommands.

<details open>
<summary><strong>Claude Code</strong></summary>

```text
/plugin marketplace add xberg-io/liter-llm
/plugin install liter-llm@liter-llm
```

</details>

<details>
<summary><strong>Codex CLI</strong></summary>

```text
/plugins add https://github.com/xberg-io/liter-llm
```

Then search for `liter-llm` and select **Install Plugin**.

</details>

<details>
<summary><strong>Cursor</strong></summary>

Settings → Plugins → Add from URL → `https://github.com/xberg-io/liter-llm`, then select **liter-llm**.

</details>

<details>
<summary><strong>Gemini CLI</strong></summary>

From a checkout of this repository:

```sh
gemini extensions install ./plugin
```

The manifest is inside `plugin/`. See the [Gemini extension reference](https://geminicli.com/docs/extensions/reference/)
for local installation and updates.

</details>

<details>
<summary><strong>Factory Droid</strong></summary>

```text
droid plugin marketplace add https://github.com/xberg-io/liter-llm
droid plugin install liter-llm@liter-llm
```

</details>

<details>
<summary><strong>GitHub Copilot CLI</strong></summary>

```text
copilot plugin marketplace add https://github.com/xberg-io/liter-llm
copilot plugin install liter-llm@liter-llm
```

</details>

<details>
<summary><strong>Pi</strong></summary>

Install from a local checkout now:

```bash
pi install ./plugin
```

After the next plugin release, install the same package used by OpenCode:

```bash
pi install npm:@xberg-io/opencode-liter-llm
```

Pi loads the bundled skills and prompt templates. Use `/liter-llm-chat` and `/liter-llm-stream`
to start a task. Install the CLI separately as
shown in [the plugin README](https://github.com/xberg-io/liter-llm/blob/main/plugin/README.md).
This package does not register MCP servers, hooks, or executable extensions in Pi.

</details>

<details>
<summary><strong>opencode</strong></summary>

Add the package to `opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": ["@xberg-io/opencode-liter-llm"]
}
```

</details>

## Documentation

Full guides, the unified `chat()` API for every binding, multimodal I/O, the proxy/gateway, and the complete provider list live at **[docs.liter-llm.xberg.io](https://docs.liter-llm.xberg.io)**.

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Join our [Discord community](https://discord.gg/xt9WY3GnKR) for questions and discussion.

## Part of Xberg.io

- [Xberg](https://github.com/xberg-io/xberg) — document intelligence: text, tables, metadata from 101 formats with optional OCR.
- [Xberg Enterprise](https://github.com/xberg-io/xberg-enterprise) — managed extraction API with SDKs, dashboards, and observability.
- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 174 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.

## License

MIT — see [LICENSE](LICENSE) for details.
