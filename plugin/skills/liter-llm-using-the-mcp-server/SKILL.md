---
description: Use when calling LLM APIs through the liter-llm MCP server's 22 tools, and to decide when MCP beats the CLI or SDK. Covers the tool surface, the auto-installing launcher, and authentication.
name: liter-llm-using-the-mcp-server
---

<!--
AI-RULEZ :: GENERATED FILE — DO NOT EDIT
Content-Hash: blake3:6925dd58f9790a1cc57f093167cf3b430886393ff3792d28003f15269b5546a2
Source-Hash: blake3:d3660ff50caaec152c20dce3dd60288dc9d9290a8e656c7f6d8bb74c7727852d
Schema-Version: v1
-->

# Using the MCP Server

The `liter-llm` MCP server exposes 22 tools that mirror the proxy's REST
endpoints, so an MCP-compatible client (Claude Code, Claude Desktop) can call
174 LLM providers as tools with no glue code.

## Standalone skill installation

Installing this skill alone does not register an MCP server or include the plugin launcher.
Install the CLI (for example, `brew install xberg-io/tap/liter-llm`), then configure your MCP client with
`liter-llm mcp --transport stdio` as its stdio command. The full product plugin supplies registration where
the runtime supports MCP.

## How it runs in this plugin

The plugin auto-registers the server. Its command is the bundled launcher
`scripts/mcp-launch.sh`, which execs `liter-llm mcp --transport stdio`. On first
run the launcher resolves a `liter-llm` binary: it reuses one on `PATH`, then
tries `npx`/`uvx`, then Homebrew, then downloads a prebuilt
from the tool's latest GitHub release. Override with
`LITER_LLM_LAUNCHER=auto|npx|uvx|brew|download`.

To run it manually:

```bash
liter-llm mcp --transport stdio          # for Claude Code / Claude Desktop
liter-llm mcp --transport http --port 3001
```

## Configure model routing

Provider-backed tools use the proxy model registry. Create `liter-llm-proxy.toml` in the
working directory, or pass `--config /absolute/path/liter-llm-proxy.toml` when launching:

```toml
[[models]]
name = "gpt-4o"
provider_model = "openai/gpt-4o"
```

Supply `OPENAI_API_KEY` in the MCP process environment, then call `chat` using the
configured name `gpt-4o`. `list_models` shows configured names; an unknown model is a
request error. Missing credentials are also reported on provider-backed requests.
Gemini filters host credentials: install with `--skip-settings` and configure the selected
provider through `gemini extensions config liter-llm "OpenAI key"`. Local stdio does not
need a separate proxy virtual key.

## The 22 tools

Modality tools: `chat`, `embed`, `generate_image`, `speech`, `transcribe`,
`moderate`, `rerank`, `search`, `ocr`.

Model listing: `list_models`.

File operations: `create_file`, `list_files`, `retrieve_file`, `delete_file`,
`file_content`.

Batch operations: `create_batch`, `list_batches`, `retrieve_batch`,
`cancel_batch`.

Responses API: `create_response`, `retrieve_response`, `cancel_response`.

## Authentication

Local stdio trusts the invoking local client by default, so the registered command starts
without extra proxy authentication configuration. Provider API keys still come from environment
variables and are needed for provider-backed requests.

To apply a restricted virtual key, configure `mcp.stdio_key_id`; an unknown key fails startup.
Set `mcp.stdio_trust_local = false` to require an explicit virtual key. HTTP transport retains
its authentication policy; the local stdio default does not disable HTTP authorization.

## When to prefer MCP over the CLI or SDK

- **Prefer MCP** inside an agent session: the agent invokes tools directly, with
  no code to write or process to manage.
- **Prefer the SDK** when building application software in a specific language.
- **Prefer the proxy** when many apps or teams need a shared OpenAI-compatible
  endpoint with keys, budgets, and rate limits.
