---
type: Concept
title: Area Map
x-ai-rulez:
  kind: context
  id: area-map
  metadata:
    priority: high
---

# Area Map

liter-llm is a Rust universal LLM API client with polyglot bindings.

- `crates/liter-llm/` — core client and provider implementations
- `crates/liter-llm-proxy/` — OpenAI-compatible proxy server (`proxy` rule)
- `crates/liter-llm-catalog-gen/`, `schemas/` — model catalog generator and API schemas (`model-catalog` rule)
- `crates/liter-llm-cli/` — command-line binary
- `crates/liter-llm-{py,node,php,wasm,ffi,jni}/` — binding crates
- `packages/`, `e2e/`: standard alef layout (`binding-architecture` skill); `fixtures/` e2e inputs
- `test_apps/<lang>/` — suites for published packages
- `tests/` — proxy test fixtures
- `docs-site/`, `templates/` — docs site and templates
- `skills/`, `plugin/` — published skills, agent plugin
- `docker/`, `scripts/` — proxy image, CI and release scripts
