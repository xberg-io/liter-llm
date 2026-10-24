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

### Kimi installation and compatibility

From a checkout of this repository, install with the Python Kimi CLI version used for native
plugin validation:

```sh
uvx --from kimi-cli==1.51.0 kimi plugin install ./plugin
```

Native plugin installation was validated with Python Kimi CLI 1.51.0. The replacement
`@moonshot-ai/kimi-code` 2.1.1 does not provide a `plugin` CLI subcommand; its interactive
plugin loading has not been verified. These results do not establish compatibility with
newer Kimi releases.

### Pi

From a checkout of this repository, run:

```bash
pi install ./plugin
```

Once a plugin release includes Pi support, use the shared OpenCode/Pi npm package:

```bash
pi install npm:@xberg-io/opencode-liter-llm
```

Pi discovers skills from `.pi/skills` and prompt templates from `.pi/prompts`.
Use `/liter-llm-chat` and `/liter-llm-stream` to start a task.
Install the CLI separately using the instructions below. The Pi package does not
register MCP servers, hooks, or executable extensions. The MCP integration
sections below apply to clients with MCP support.

Git installs use the repository root's `package.json` resource paths. GitHub
hosts the checkout resources; the existing npm plugin release publishes the same
Pi content alongside the OpenCode adapter. No separate Pi release job is needed.

### Binary requirement

The bundled MCP launcher (`scripts/mcp-launch.sh`) resolves a `liter-llm` binary
automatically on first run: it reuses one already on `PATH`, then tries
`npx`/`uvx`, then Homebrew, then a prebuilt binary from the
tool's latest GitHub release. If every method fails, install a CLI with MCP support using the commands below.

To install the CLI yourself (also gives you the proxy server):

```bash
brew install xberg-io/tap/liter-llm
# or from crates.io (compiles from source):
cargo install liter-llm-cli
# or run it without a persistent install (the CLI proxy package self-installs the binary):
pnpm dlx @xberg-io/liter-llm-cli --help
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
| **liter-llm-calling-llms** | Use when sending chat completions and routing to a specific provider via the `provider/model` prefix. |
| **liter-llm-streaming-responses** | Use when streaming tokens incrementally over SSE or async iterators. |
| **liter-llm-tool-calling** | Use when defining functions/tools for the model to call, or requesting structured outputs. |
| **liter-llm-embeddings-and-search** | Use when generating embeddings, calling the web-search providers, or running OCR over documents. |
| **liter-llm-running-the-proxy** | Use when running the `liter-llm api` OpenAI-compatible gateway with virtual keys, rate limits, and budgets. |
| **liter-llm-using-the-mcp-server** | Use when calling LLM APIs through the 22 MCP tools, and to decide when MCP beats the CLI or SDK. |

## CLI release compatibility

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
2. `pnpm dlx @xberg-io/liter-llm-cli` — the published npm CLI proxy package self-installs/runs
   the binary (probed first; falls through if not published yet).
3. `uvx --from liter-llm-cli liter-llm` — the published PyPI CLI proxy package
   (probed first; falls through if not published yet).
4. `brew install xberg-io/tap/liter-llm` if Homebrew is available.
5. A prebuilt binary from the tool's *latest* GitHub release
   (`liter-llm-<latest-version>-<target>.tar.gz`). Its hash is checked against
   `SHA256SUMS-<latest-version>.txt` when the manifest and a hashing tool are available.
   Missing checksums cause an explicit warning and HTTPS-only download fallback;
   a hash mismatch fails. The tool version is resolved from the
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

## Maintaining the plugin

Edit `plugin/.ai-rulez` sources, then run these commands from the repository root
with a Pi-capable ai-rulez v5 binary installed:

```bash
task plugin:generate
task plugin:verify
task plugin:test
```

CI builds the generator from a pinned source revision and verifies the generated
bundles, OpenCode tools, and actual npm archive before publication. The shared
npm package publishes both OpenCode and Pi resources; Hermes uses its existing
PyPI release job. GitHub hosts the remaining plugin bundles and marketplace indexes.

## License

MIT. Skill content references the upstream [liter-llm](https://github.com/xberg-io/liter-llm) repository.

## See also

- **Marketplace**: this repo — `/plugin marketplace add xberg-io/liter-llm`
- **Upstream**: [xberg-io/liter-llm](https://github.com/xberg-io/liter-llm)

## Generated runtime bundles

`plugin/.ai-rulez/config.toml` loads `config.plugins.toml` through the `plugins` layer;
skills, commands and runtime adapters are authored alongside it. AI-Rulez 5.6.0 generates
all eleven supported runtimes: Claude, Cursor, Codex, Gemini, Kimi, OpenCode, Factory,
Hermes, Pi, Agent Plugins and Copilot. Codex ships both manifest layouts; repository-root
Claude and Codex marketplace indexes point to the `plugin` bundle. Cursor also has a marketplace index.

Pi packages skills and prompts without MCP integration. Copilot packages skills and MCP configuration, but does not package slash commands or hooks.
Hermes includes a Python package under `.hermes/package`; OpenCode and Pi share the npm package.
The Agent Plugins manifest uses the stable 1.0 specification. Runtime capabilities differ;
install the CLI before using skills in a runtime without MCP registration.

Run `ai-rulez generate --plugin` from the repository root to regenerate and
`ai-rulez verify --plugin` to compare current source output and provenance.
The release workflow packages every runtime into a deterministic archive, checksums,
and marketplace metadata; npm and PyPI retain their existing publishing jobs.

## Hermes runtime setup

Install the product CLI with MCP support and put it on Hermes' PATH before using the plugin.
The Hermes adapter registers every bundled skill under the product namespace and provides
`liter_llm_call`, which calls the selected native MCP tool with `tool_arguments` through a local
stdio session. Select `tool: "list_tools"` first to inspect the server's exact request schemas. Hermes skills are selected explicitly; they are not automatically appended to the
system prompt. Enable the plugin in Hermes configuration; project plugins also require
`HERMES_ENABLE_PROJECT_PLUGINS=true`.

The bridge uses the installed CLI directly. The auto-installing launchers used by other MCP clients
are not included in the Hermes Python wheel. Provider credentials are inherited from the environment,
and are needed for provider-backed requests. Missing CLIs, unavailable tools and model preparation
fail with an actionable request error. CLI result data remains untrusted document content.

## Public Agent Skills

The repository-root `skills/` directory contains the same generated skills and supporting
resources as `plugin/skills/`. Install them independently of a plugin runtime:

```sh
npx skills add xberg-io/liter-llm --skill liter-llm
```

Edit `plugin/.ai-rulez/skills`, then run `task plugin:generate` to regenerate runtime
bundles and synchronize the public export. `task plugin:verify` checks both projections;
`task plugin:skills:test` verifies resource freshness, ownership and symlink protection.
The public ownership manifest records SHA-256 hashes of every exported file.

## Gemini provider credentials

Gemini filters sensitive shell environment variables. This extension declares provider keys
through native sensitive settings; values are entered in Gemini and stored in its keychain.
To install without entering credentials for every provider:

```sh
gemini extensions install ./plugin --skip-settings
gemini extensions config liter-llm "OpenAI key"
```

Configure only the provider used by your `liter-llm-proxy.toml` model routing. The local stdio
server can start without provider credentials; provider-backed requests require a configured
model and its credentials. For a provider with another variable name, add an appropriate
`[[plugin.gemini.settings]]` entry in the source and regenerate, or configure a Gemini MCP
server override. Never put key values in source or a generated manifest.

Launcher and visual assets are authored in `plugin/.ai-rulez/scripts/` and
`plugin/.ai-rulez/assets/`; `task plugin:generate` copies them into the runtime bundle.
