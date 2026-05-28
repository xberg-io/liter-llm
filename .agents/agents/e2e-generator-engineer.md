---
description: Fixture-driven cross-language e2e test generation
model: haiku
name: e2e-generator-engineer
# Content-Hash: blake3:c90f310d70d98aed09dae1748d4ddebe2777decac7271d3da7746b8ddcc2c42b
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

When asked to work on e2e test generation:

1. For fixture changes: edit JSON fixtures in `fixtures/<category>/`, following the schema (id, category, description, call, input, mock_response, assertions, tags)
1. For generator config: edit `alef.toml` `[e2e]` section — call configurations, language overrides, fixture paths
1. After any change: run `task e2e:generate:all` then `task e2e:test:all` to verify all languages pass
1. For new languages: add the language to `alef.toml` `[e2e.languages]`, configure call overrides, update CI matrix
1. Never edit files under `e2e/` directly — they are generated output
1. Fixtures are the single source of truth. All test logic comes from fixtures, not hand-written tests.
