---
status: accepted
date: 2026-03-29
decision-makers: ["Na'aman Hirschfeld"]
---

# Proxy server and MCP tools in their own crate

## Context and Problem Statement

Besides the in-process library, liter-llm serves applications that cannot embed a binding, and agents that call
tools over MCP. Both need a long-running server with authentication, per-model middleware and an OpenAPI
description, which the core library should not carry. One commit (712678786, 2026-03-29) added
`liter-llm-proxy`, an axum proxy with REST endpoints, Tower middleware, virtual API keys, OpenDAL file storage, SSE
streaming, an OpenAPI 3.1 document and an rmcp MCP server, plus `liter-llm-cli` with `api` and `mcp` subcommands and
a Docker image.

## Considered Options

* A separate `liter-llm-proxy` library crate that holds both the OpenAI-compatible HTTP server and the MCP server,
  run by the `liter-llm-cli` binary

No alternatives were recorded.

## Decision Outcome

Chosen option: "A separate `liter-llm-proxy` library crate that holds both the OpenAI-compatible HTTP server and the
MCP server, run by the `liter-llm-cli` binary", because the core crate "never depends on any binding" and "all
knowledge flows in one direction: core outward to bindings, CLI, and proxy"
(`docs-site/src/content/docs/concepts/architecture.md:32`). The server dependencies, axum and rmcp, stay out of the
core crate and its bindings, and one binary and one container image serve both protocols.

The proxy builds one Tower stack per configured model at startup and holds them in a `ServicePool` indexed by model
name and alias ([ADR 0003](0003-tower-middleware-behind-the-tower-feature.md)). It depends on the core crate, never
the other way round. The CLI's `api` and `mcp` subcommands start the two servers, and the Docker image ships the CLI.

### Consequences

* Good, because the core crate and every language binding stay free of server dependencies.
* Good, because the HTTP API and the MCP tools share one configuration, one service pool and one binary.
* Bad, because the proxy is a second public surface whose OpenAPI document and MCP tools must follow every change to
  the core types.

### Confirmation

* `crates/liter-llm-proxy/Cargo.toml:69` depends on the core crate, and `crates/liter-llm/Cargo.toml` has no
  dependency on the proxy.
* `crates/liter-llm-proxy/src/service_pool.rs:57` defines `ServicePool`, and `crates/liter-llm-proxy/src/mcp/`
  holds the MCP server.
* `crates/liter-llm-cli/src/main.rs:61` and `crates/liter-llm-cli/src/main.rs:62` dispatch the `api` and `mcp`
  subcommands.
* `docker/Dockerfile:15` builds the `liter-llm-cli` package into the image.
