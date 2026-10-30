---
type: Decision
title: Proxy Server
paths:
  - "crates/liter-llm-proxy/**"
  - "docker/Dockerfile"
x-ai-rulez:
  kind: rule
  id: proxy
---

- `crates/liter-llm-proxy` is the OpenAI-compatible proxy server; user docs live in `docs-site/src/content/docs/server/`.
- Unit and integration tests: `cargo test -p liter-llm-proxy` (CI runs them in `mise run rust:test`); integration tests share `crates/liter-llm-proxy/tests/common/` (`mock_upstream`, `test_proxy`).
- The OpenAPI document the proxy serves at `/openapi.json` comes from `crates/liter-llm-proxy/src/openapi.rs`.
