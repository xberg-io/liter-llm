---
description: "Install the liter-llm plugin into Claude Code, Codex, Cursor, Gemini, Copilot, and other coding agents."
---

# AI Coding Assistants

Install the liter-llm plugin from the [`xberg-io/plugins`](https://github.com/xberg-io/plugins) marketplace. It ships the liter-llm agent skills — chat, streaming, tools, and embeddings across 143 providers — and works with every major coding agent.

## Installing

Expand the section for your harness below.

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
