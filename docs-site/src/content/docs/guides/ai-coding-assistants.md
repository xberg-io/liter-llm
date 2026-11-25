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
- **MCP server** — registers the liter-llm MCP server with your client. Use the compatible CLI described below.

Once installed, your assistant applies these skills automatically when you ask it to work with liter-llm.

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

From a checkout of this repository:

```sh
gemini extensions install ./plugin --skip-settings
```

The manifest is inside `plugin/`. See the [Gemini extension reference](https://geminicli.com/docs/extensions/reference/)
for local installation and updates.

Configure the provider you use, for example `gemini extensions config liter-llm OPENAI_API_KEY`.
Gemini filters sensitive shell variables; the extension declares these settings for secure
credential injection. Skipping settings during installation avoids prompts for unused providers.

</details>

<details>
<summary><strong>Kimi installation and compatibility</strong></summary>

From a checkout of this repository, install with the Python Kimi CLI version used for native
plugin validation:

```sh
uvx --from kimi-cli==1.51.0 kimi plugin install ./plugin
```

Native plugin installation was validated with Python Kimi CLI 1.51.0. The replacement
`@moonshot-ai/kimi-code` 2.1.1 does not provide a `plugin` CLI subcommand; its interactive
plugin loading has not been verified. These results do not establish compatibility with
newer Kimi releases.

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

## MCP server

The MCP-capable integrations above install the same bundled MCP server and register it for you — there is nothing to configure by hand. The plugin points at `scripts/mcp-launch.sh`, which resolves the `liter-llm` binary at runtime (a cached or `PATH` binary at the pinned version, then `npx @xberg-io/liter-llm-cli@<version>`, then a checksum-verified downloaded release; Homebrew only with `LITER_LLM_LAUNCHER=brew`).

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

For the released 2.2.3 CLI, the stdio transport requires the auth binding described above. The source build trusts local stdio by default. See the [MCP server guide](/server/mcp-server/) for transport and provider configuration.

## Hermes

For the Hermes runtime, install the plugin from PyPI:

```bash
pip install liter-llm-hermes-plugin
```

## Learn more

The plugin, its skills, and support for more agents are maintained in this repository ([`xberg-io/liter-llm`](https://github.com/xberg-io/liter-llm)).

## Runtime resources and prerequisites

The generated bundle targets Claude Code, Codex, Cursor, Gemini CLI, Kimi, OpenCode,
Factory Droid, Hermes, Pi, Agent Plugins and GitHub Copilot CLI. Use
[the plugin installation guide](https://github.com/xberg-io/liter-llm/blob/main/plugin/README.md)
for the runtime's installation method and configuration.

| Runtime | Integration |
| --- | --- |
| Claude, Codex, Cursor, Factory | Native plugin manifests, skills and MCP configuration |
| Gemini CLI | Extension manifest, command templates and MCP configuration |
| Kimi | Plugin manifest and session instructions |
| OpenCode | npm adapter, instructions and MCP configuration |
| Hermes | Python plugin, explicitly selected skills and a local stdio MCP bridge |
| Pi | Skills and prompt templates; execute operations through the CLI |
| Agent Plugins, Copilot | Agent Plugins 1.0 manifest, skills and MCP configuration |

Install the product CLI before using Hermes or Pi. Hermes starts the installed CLI directly;
its wheel does not include the auto-installing launcher used by other MCP clients.
Select `list_tools` through the product's Hermes bridge to inspect the supported tools and
request schemas. Select skills explicitly when needed; they are not appended automatically
to every conversation.

From a checkout, install the generated Hermes package into Hermes' Python environment:

```sh
pip install ./plugin/.hermes/package
```

Enable the plugin in Hermes configuration. Loading project plugins also requires
`HERMES_ENABLE_PROJECT_PLUGINS=true`. After a release publishes the updated package, the
registry install is `pip install liter-llm-hermes-plugin`.

Pi resources can be installed from the checkout with `pi install ./plugin`.
After a plugin release includes them, install `npm:@xberg-io/opencode-liter-llm` through Pi.
Pi does not register an MCP server, hooks or executable extensions from this package.

## Standalone public skills

The [public skills directory](https://github.com/xberg-io/liter-llm/tree/main/skills) contains
instructions you can install independently of the runtime plugin. Use the
[skills CLI](https://github.com/vercel-labs/skills) to list and select them:

```sh
npx skills add xberg-io/liter-llm --list
npx skills add xberg-io/liter-llm
```

Choose the target agents interactively. For a local checkout, use `npx skills add ./skills`.
This installs instructions; it does not install the product CLI, enable MCP, download models
or set provider credentials.

The local stdio MCP server trusts its harness by default and starts without provider keys.
Configure provider keys before making provider-backed requests; a missing key produces a
request error. HTTP MCP clients must still supply a master or virtual key. Use
`liter-llm api` for the proxy and `liter-llm mcp` for the tool server. Chat and streaming are
SDK, proxy or MCP operations, rather than additional CLI subcommands.
