---
status: accepted
date: 2026-03-25
decision-makers: ["Na'aman Hirschfeld"]
---

# Provider registry embedded from providers.json

## Context and Problem Statement

liter-llm supports well over a hundred providers (142 in the first commit, 174 today). Most of them differ only in
base URL, auth header, environment variable and model prefixes, so writing a Rust type per provider would repeat the
same code. The first commit (8ea17d284, 2026-03-25) took "142 providers from litellm data, embedded at compile time
via include_str" with "schema-driven provider config". Two days later the embedded file had to move: the
`include_str!` path pointed at the repository root, outside the crate, so the published crates.io package did not
contain it (b7bd3adb9, 2026-03-27).

## Considered Options

* A crate-local `crates/liter-llm/schemas/providers.json`, embedded with `include_str!`, with a root copy kept
  byte-equal
* The repository-root `schemas/providers.json`, embedded with `include_str!` by a relative path outside the crate

## Decision Outcome

Chosen option: "A crate-local `crates/liter-llm/schemas/providers.json`, embedded with `include_str!`, with a root
copy kept byte-equal", because a provider that only needs a base URL and a bearer token becomes a data entry with no
code, and only a file inside the crate ships in the crates.io package (b7bd3adb9).

The registry is parsed once, lazily, and a parse failure surfaces as an error at call time instead of a panic.
Providers that need their own auth or request code (Bedrock SigV4, Vertex OAuth2, Azure and others) are listed under
`complex_providers`, are matched by prefix before the registry lookup, and are never routed as config-driven
providers. The root copy at `schemas/providers.json` feeds tooling such as the generated providers page.

### Consequences

* Good, because adding an OpenAI-compatible provider is a JSON change, with no new Rust type.
* Good, because the registry is compiled into the binary and needs no network or file access at runtime.
* Bad, because the two copies of the file can drift; a CI test exists only for that reason.
* Bad, because the provider count is pinned by a test that every registry change must update.

### Confirmation

* `crates/liter-llm/src/provider/mod.rs:151` embeds `../../schemas/providers.json` with `include_str!`.
* `crates/liter-llm/src/provider/mod.rs:1254` limits a config-driven route to entries with a `base_url` that are not
  complex providers, and `crates/liter-llm/src/provider/mod.rs:1495` skips complex providers in the prefix walk.
* `crates/liter-llm/src/tests.rs:2140` (`schema_provider_count_is_174`) checks the embedded registry.
* `scripts/ci/test_schema_copies.py:21` fails when the root and crate copies differ; `mise.toml:81` adds it to
  `test:release-regressions`, which `.github/workflows/ci-lint.yaml:83` runs.
