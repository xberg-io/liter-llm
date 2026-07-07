---
description: "Install the liter-llm plugin into Claude Code, Codex, Cursor, Gemini, Copilot, and other coding agents."
title: "AI Coding Assistants"
---

Give your coding assistant a working knowledge of liter-llm. The plugin teaches your agent how to call any of the 143 providers, stream responses, use tools, and generate embeddings — so it writes correct liter-llm code the first time instead of guessing at the API.

## What this plugin does

The liter-llm plugin ships a set of agent skills from the [`xberg-io/plugins`](https://github.com/xberg-io/plugins) marketplace:

- **Chat and streaming** — build requests, handle streamed responses, and switch models across providers.
- **Tool calling** — define tools and handle the model's tool calls.
- **Embeddings** — generate and use embeddings for search and retrieval.
- **MCP server** — auto-registers the liter-llm MCP server, so your agent can call it with no manual config.

Once installed, your assistant applies these skills automatically when you ask it to work with liter-llm.

## Installing

Expand the section for your coding agent below.

<details open>
<summary><strong>Claude Code</strong></summary>

```text
/plugin marketplace add xberg-io/plugins
/plugin install liter-llm@xberg
```

</details>

<details>
<summary><strong>Codex CLI</strong></summary>

```text
/plugins add https://github.com/xberg-io/plugins
```

Then search for `liter-llm` and select **Install Plugin**.
</details>

<details>
<summary><strong>Cursor</strong></summary>

Settings → Plugins → Add from URL → `https://github.com/xberg-io/plugins`, then select **liter-llm**.
</details>

<details>
<summary><strong>Gemini CLI</strong></summary>

```text
gemini extensions install https://github.com/xberg-io/plugins
```

</details>

<details>
<summary><strong>Factory Droid</strong></summary>

```text
droid plugin marketplace add https://github.com/xberg-io/plugins
droid plugin install liter-llm@xberg
```

</details>

<details>
<summary><strong>GitHub Copilot CLI</strong></summary>

```text
copilot plugin marketplace add https://github.com/xberg-io/plugins
copilot plugin install liter-llm@xberg
```

</details>

<details>
<summary><strong>opencode</strong></summary>

Not yet published as an opencode package. Install via any harness above (self-hosted marketplace); opencode support is tracked in [`xberg-io/plugins`](https://github.com/xberg-io/plugins).
</details>

## Learn more

The plugin, its skills, and support for more agents are maintained in the [`xberg-io/plugins`](https://github.com/xberg-io/plugins) repository.
