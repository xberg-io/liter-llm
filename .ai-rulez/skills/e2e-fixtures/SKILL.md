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
- Categories match the directory (`smoke`, `chat`, `streaming`, `error-handling`, `proxy`, `files`, `batches`, `rerank`, `embed`, ...); `providers/` holds fixtures whose `category` names the API they exercise.
- Fields: `docs {topic, stem, title}`, `tags`, `call`, `input`, `mock_response {status, body}`, `assertions`, and optionally `env`, `skip {languages, reason}`, `requirements`.
- `call` names an API: `chat` is the default (`[crates.e2e.call]` in `alef.toml`); `chat_stream`, `embed`, `rerank`, `image_generate`, `list_models`, `moderate`, `search`, `create_response`, `speech`, `transcribe`, `ocr`, `create_batch` and the rest are `[crates.e2e.calls.<name>]` entries. `input` is the request body.
- Assertion types in use: `equals`, `not_error`, `error`, `count_equals`, `count_min`, `not_empty`, `is_true`, `greater_than`, `contains`. Each assertion is `{type, field, value}` (`values` for `contains_all`); `field` is a dotted path into the call result. `error` takes the error kind as `value` (for example `Authentication`); assert the HTTP status with `field: error.status_code`.
