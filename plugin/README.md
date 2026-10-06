# liter-llm

Universal LLM API client for 174 providers — chat, streaming, tools, embeddings, search, OCR, plus an OpenAI-compatible proxy and an MCP server, in your agent.

<!-- ~keep TODO: add screenshot -->

## Install

### From the marketplace (recommended)

Pending review for official Claude marketplace.

Self-host:

```text
/plugin marketplace add xberg-io/liter-llm
/plugin install liter-llm@liter-llm
```

### Binary requirement

The bundled MCP launcher (`scripts/mcp-launch.sh`) resolves a `liter-llm` binary
automatically on first run: it reuses one already on `PATH`, then tries
`npx`/`uvx`, then Homebrew, then a checksum-verified prebuilt binary from the
tool's latest GitHub release. No manual install is required to use the MCP
server.

To install the CLI yourself (also gives you the proxy server):

```bash
brew install xberg-io/tap/liter-llm
# or from crates.io (compiles from source):
cargo install liter-llm-cli
# or run it without a persistent install (the CLI proxy package self-installs the binary):
npx @xberg-io/liter-llm-cli --help
uvx --from liter-llm-cli liter-llm --help
# or download a prebuilt binary from the latest GitHub release:
#   https://github.com/xberg-io/liter-llm/releases/latest
# or install the CLI from crates.io:
cargo install liter-llm-cli
```

Set provider API keys via environment variables (`OPENAI_API_KEY`,
`ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, …).

## Skills shipped

| Skill | Trigger |
|-------|---------|
| **liter-llm** | Universal LLM client surface — chat, streaming, tools, embeddings, image/audio, moderation, search, OCR, reranking — across 174 providers and 14 language bindings. Use when writing LLM code in any supported language or choosing between the SDK, proxy, and MCP server. |
| **calling-llms** | Use when sending chat completions and routing to a specific provider via the `provider/model` prefix. |
| **streaming-responses** | Use when streaming tokens incrementally over SSE or async iterators. |
| **tool-calling** | Use when defining functions/tools for the model to call, or requesting structured outputs. |
| **embeddings-and-search** | Use when generating embeddings, calling the web-search providers, or running OCR over documents. |
| **running-the-proxy** | Use when running the `liter-llm api` OpenAI-compatible gateway with virtual keys, rate limits, and budgets. |
| **using-the-mcp-server** | Use when calling LLM APIs through the 22 MCP tools, and to decide when MCP beats the CLI or SDK. |

## MCP server

The plugin auto-registers an MCP server named `liter-llm`, launched via
`scripts/mcp-launch.sh` (which execs `liter-llm mcp --transport stdio`). It
exposes 22 tools mirroring the proxy's REST endpoints:

- `chat`, `embed`, `generate_image`
- `speech`, `transcribe`, `moderate`
- `rerank`, `search`, `ocr`, `list_models`
- file ops (`create_file`, `list_files`, `retrieve_file`, `delete_file`, `file_content`)
- batch ops (`create_batch`, `list_batches`, `retrieve_batch`, `cancel_batch`)
- Responses API (`create_response`, `retrieve_response`, `cancel_response`)

### Launcher / auto-install

The launcher resolves a working `liter-llm` binary in this order, falling
through to the next on any failure (override with
`LITER_LLM_LAUNCHER=auto|npx|uvx|brew|download`):

1. Any working `liter-llm` already cached in the plugin's `bin/` or on `PATH`.
2. `npx @xberg-io/liter-llm-cli` — the published npm CLI proxy package self-installs/runs
   the binary (probed first; falls through if not published yet).
3. `uvx --from liter-llm-cli liter-llm` — the published PyPI CLI proxy package
   (probed first; falls through if not published yet).
4. `brew install xberg-io/tap/liter-llm` if Homebrew is available.
5. A checksum-verified prebuilt binary from the tool's *latest* GitHub release
   (`liter-llm-<latest-version>-<target>.tar.gz`, verified against
   `SHA256SUMS-<latest-version>.txt`). The tool version is resolved from the
   GitHub API — it is independent of the plugin's own version.

The `@xberg-io/*` / `liter-llm` binding packages (NAPI-RS / PyO3) are language
SDKs, not the CLI. All launcher diagnostics go to stderr; stdout is the MCP
protocol channel.

## Proxy / CLI

The `liter-llm` CLI ships an OpenAI-compatible proxy alongside the MCP server:

```bash
liter-llm api --config liter-llm-proxy.toml   # start the proxy (22 endpoints)
liter-llm mcp --transport stdio               # start the MCP server
```

Call the proxy like OpenAI:

```bash
curl http://localhost:4000/v1/chat/completions \
  -H "Authorization: Bearer sk-your-key" \
  -d '{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello"}]}'
```

## Configuration

The proxy auto-discovers `liter-llm-proxy.toml`; the SDK auto-discovers
`liter-llm.toml` from the current directory upward.

```toml
# liter-llm-proxy.toml
[server]
host = "0.0.0.0"
port = 4000

[general]
master_key = "${LITER_LLM_MASTER_KEY}"

[[models]]
name = "gpt-4o"
provider_model = "openai/gpt-4o"
api_key = "${OPENAI_API_KEY}"

[[keys]]
key = "sk-team-frontend"
models = ["gpt-4o"]
rpm = 60
budget_limit = 50.0
```

See `skills/liter-llm/SKILL.md` for the full configuration surface.

## Examples

Route a chat completion to a configured model name:

```text
liter-llm proxy → POST /v1/chat/completions  {"model": "gpt-4o", ...}
```

Through the MCP server, ask the agent to:

```text
Embed these five paragraphs with openai/text-embedding-3-small and rank them against my query.
```

## Versioning

The plugin version tracks the liter-llm core release (`Cargo.toml`). See [CHANGELOG.md](../CHANGELOG.md) for release notes.

## License

MIT. Skill content references the upstream [liter-llm](https://github.com/xberg-io/liter-llm) repository.

## See also

- **Marketplace**: this repo — `/plugin marketplace add xberg-io/liter-llm`
- **Upstream**: [xberg-io/liter-llm](https://github.com/xberg-io/liter-llm)
