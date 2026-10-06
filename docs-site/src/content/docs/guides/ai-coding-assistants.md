---
description: "Install the liter-llm plugin into Claude Code, Codex, Cursor, Gemini, Copilot, and other coding agents — as a plugin, an opencode package, or an MCP server."
title: "AI Coding Assistants"
---

Give your coding assistant a working knowledge of liter-llm. The plugin teaches your agent how to call any of the 174 providers, stream responses, use tools, and generate embeddings — so it writes correct liter-llm code the first time instead of guessing at the API.

## What this plugin does

The liter-llm plugin ships a set of agent skills, installed from this repo's self-hosted marketplace ([`xberg-io/liter-llm`](https://github.com/xberg-io/liter-llm)):

- **Chat and streaming** — build requests, handle streamed responses, and switch models across providers.
- **Tool calling** — define tools and handle the model's tool calls.
- **Embeddings** — generate and use embeddings for search and retrieval.
- **MCP server** — bundles the liter-llm MCP server and auto-registers it, so your agent can call it with no manual config.

Once installed, your assistant applies these skills automatically when you ask it to work with liter-llm.

## Installing

Expand the section for your coding agent below.

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

```text
gemini extensions install https://github.com/xberg-io/liter-llm
```

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
<summary><strong>opencode</strong></summary>

Add the package to `opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": ["@xberg-io/opencode-liter-llm"]
}
```

</details>

## MCP server

Every harness above installs the same bundled MCP server and registers it for you — there is nothing to configure by hand. The plugin points at `scripts/mcp-launch.sh`, which resolves the `liter-llm` binary at runtime (`npx @xberg-io/liter-llm-cli`, `uvx --from liter-llm-cli`, Homebrew, or a downloaded release).

To register the server directly in any other MCP client (without the plugin), add it to that client's MCP config:

```json
{
  "mcpServers": {
    "liter-llm": {
      "command": "npx",
      "args": ["-y", "@xberg-io/liter-llm-cli@latest", "mcp", "--transport", "stdio"]
    }
  }
}
```

`uvx --from liter-llm-cli liter-llm mcp --transport stdio` works as an alternative command.

Because liter-llm is a provider proxy, the stdio MCP transport requires an auth binding in your liter-llm config — set either `mcp.stdio_trust_local = true` (fully trusted local environments) or `mcp.stdio_key_id` to bind a specific virtual key. Without one, the server refuses to start. See the [MCP server guide](/server/mcp-server/) for running and configuring it directly.

## Hermes

For the Hermes runtime, install the plugin from PyPI:

```bash
pip install liter-llm-hermes-plugin
```

## Learn more

The plugin, its skills, and support for more agents are maintained in this repository ([`xberg-io/liter-llm`](https://github.com/xberg-io/liter-llm)).
