---
status: accepted
date: 2026-03-25
decision-makers: ["Na'aman Hirschfeld"]
---

# OpenAI wire format as the canonical type model

## Context and Problem Statement

liter-llm talks to many LLM providers through one client surface. Each provider has its own request and response
shapes, so the client needs one type model that every provider maps to and from, and that every language binding
exposes. The first commit (8ea17d284, 2026-03-25) added `schemas/api/` with "115 OpenAI types from official spec"
and core types for chat completions, embeddings and models, together with 142 providers.

## Considered Options

* The OpenAI REST API types as the canonical request and response model, with per-provider transforms

No alternatives were recorded.

## Decision Outcome

Chosen option: "The OpenAI REST API types as the canonical request and response model, with per-provider
transforms", because most providers already speak the OpenAI format (114 of the 174 registry entries are not
`complex_providers`), so their requests and replies pass through unchanged, and only providers with a native format
need code of their own.

The request and response types in `crates/liter-llm/src/types/` follow the OpenAI wire format, and
`schemas/api/` holds JSON Schema definitions extracted from the OpenAI OpenAPI specification. A provider with a
different native format overrides `Provider::transform_request` and `Provider::transform_response` to convert to and
from that format; the defaults pass the body through unchanged.

### Consequences

* Good, because an OpenAI-compatible provider needs only a registry entry, with no code
  ([ADR 0002](0002-provider-registry-embedded-from-providers-json.md)).
* Good, because the proxy can serve the same types as an OpenAI-compatible API
  ([ADR 0004](0004-proxy-server-and-mcp-tools-in-their-own-crate.md)).
* Bad, because a provider feature that the OpenAI format cannot express needs a transform or an extension of the
  canonical types.

### Confirmation

* `crates/liter-llm/src/provider/mod.rs:659` defines the default no-op `transform_request`, and
  `crates/liter-llm/src/provider/mod.rs:684` documents `transform_response` as normalizing a non-OpenAI reply into
  OpenAI-compatible JSON before deserialization.
* `crates/liter-llm/src/provider/anthropic.rs:198` and `crates/liter-llm/src/provider/anthropic.rs:211` override
  both for Anthropic's native format.
* `schemas/api/README.md:3` states that the schemas are extracted from the OpenAI OpenAPI specification.
* `crates/liter-llm-proxy/src/routes/mod.rs:120` serves the canonical types at `/v1/chat/completions`.
