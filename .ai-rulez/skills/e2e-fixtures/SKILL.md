---
type: Reference
title: E2E Fixtures
description: "liter-llm e2e fixture schema: per-category fixture files, call names, mock provider responses, env and skip, and assertion types. Use when adding or editing a liter-llm e2e fixture."
paths:
  - "fixtures/**"
  - "e2e/**"
x-ai-rulez:
  kind: skill
  id: e2e-fixtures
  metadata:
    name: e2e-fixtures
---

# liter-llm E2E Fixtures

- One fixture object per file, `fixtures/<category>/<id>.json`. `fixtures/schema.json` still requires an `api` field that no fixture carries; the generator reads `call`, so follow existing fixtures, not the schema.
- Categories match the directory (`smoke`, `chat`, `streaming`, `error-handling`, `proxy`, `files`, `batches`, `rerank`, `embed`, ...); the exception is `providers/`, whose fixtures carry the test kind as `category` (`smoke`, `error-handling`, `streaming`, `tool-calling`), not the directory.
- Fields: `id`, `description`, `category`, `docs {topic, stem, title}`, `tags`, `call`, `input`, `mock_response {status, body}`, `assertions`, and optionally `env`, `skip {languages, reason}`, `requirements`.
- `call` names an API: `chat` is the default; named calls include `chat_stream`, `embed`, `rerank`, `image_generate`, `list_models`, `moderate`, `search`, `create_response`, `speech`, `transcribe`, `ocr`, `create_batch` (full set in `alef.toml`). `input` is the request body.
- Assertion types in use: `equals`, `not_error`, `error`, `count_equals`, `count_min`, `not_empty`, `is_true`, `greater_than`, `contains`. Each assertion is `{type, field, value}`; `field` is a dotted path into the call result. `error` takes the error kind as `value` (for example `Authentication`); assert the HTTP status with `field: error.status_code`.
- Loop: `task e2e:generate` then `task e2e:test` (aliases of the `:all` tasks); full pipeline `task e2e:verify:all` (alias `task test:e2e`).
