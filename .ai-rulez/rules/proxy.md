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
- Unit and integration tests: `task proxy:test` (`cargo test -p liter-llm-proxy`); integration tests share `crates/liter-llm-proxy/tests/common/` (`mock_upstream`, `test_proxy`).
- Docker image tests: `task proxy:docker:test` builds `docker/Dockerfile` and runs `scripts/ci/docker/test_docker.py`.
- API contract tests: `task proxy:schemathesis` runs schemathesis against the image's `/openapi.json` (served from `crates/liter-llm-proxy/src/openapi.rs`).
