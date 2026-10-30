---
title: "Decision records"
description: "Architecture decisions recorded as MADR records under adrs/ in the liter-llm repository."
---

Lasting decisions about liter-llm are recorded as [MADR](https://adr.github.io/madr/) records under
[`adrs/`](https://github.com/xberg-io/liter-llm/tree/main/adrs), checked by
[adrlint](https://github.com/Goldziher/adrlint). An accepted record is never edited: a change is a new record that
supersedes or amends it. Organization-wide decisions live in the
[agent-conventions log](https://github.com/xberg-io/agent-conventions/tree/main/adrs). Add a row here in the same
change as a new record.

| ADR | Title | Status | Date |
| --- | ----- | ------ | ---- |
| [0000](https://github.com/xberg-io/liter-llm/blob/main/adrs/0000-record-architecture-decisions.md) | Record architecture decisions | accepted | 2026-10-30 |
| [0001](https://github.com/xberg-io/liter-llm/blob/main/adrs/0001-openai-wire-format-as-the-canonical-type-model.md) | OpenAI wire format as the canonical type model | accepted | 2026-03-25 |
| [0002](https://github.com/xberg-io/liter-llm/blob/main/adrs/0002-provider-registry-embedded-from-providers-json.md) | Provider registry embedded from providers.json | accepted | 2026-03-27 |
| [0003](https://github.com/xberg-io/liter-llm/blob/main/adrs/0003-tower-middleware-behind-the-tower-feature.md) | Tower middleware behind the tower feature | accepted | 2026-03-26 |
| [0004](https://github.com/xberg-io/liter-llm/blob/main/adrs/0004-proxy-server-and-mcp-tools-in-their-own-crate.md) | Proxy server and MCP tools in their own crate | accepted | 2026-03-29 |
| [0005](https://github.com/xberg-io/liter-llm/blob/main/adrs/0005-separate-native-and-webassembly-http-features.md) | Separate native and WebAssembly HTTP features | accepted | 2026-04-20 |
| [0006](https://github.com/xberg-io/liter-llm/blob/main/adrs/0006-model-catalog-generated-from-models-dev.md) | Model catalog generated from models.dev | accepted | 2026-07-19 |
