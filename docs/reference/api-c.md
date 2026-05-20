---
title: "C API Reference"
---

## C API Reference <span class="version-badge">v1.4.0-rc.27</span>

### Functions

#### literllm_create_client()

Create a new LLM client with simple scalar configuration.

This is the primary binding entry-point. All parameters except `api_key`
are optional — omitting them uses the same defaults as
`ClientConfigBuilder`.

**Errors:**

Returns `LiterLlmError` if the underlying HTTP client cannot be
constructed, or if the resolved provider configuration is invalid.

**Signature:**

```c
LiterllmDefaultClient* literllm_create_client(const char* api_key, const char* base_url, uint64_t timeout_secs, uint32_t max_retries, const char* model_hint);
```

**Parameters:**

| Name           | Type           | Required | Description      |
| -------------- | -------------- | -------- | ---------------- |
| `api_key`      | `const char*`  | Yes      | The api key      |
| `base_url`     | `const char**` | No       | The base url     |
| `timeout_secs` | `uint64_t*`    | No       | The timeout secs |
| `max_retries`  | `uint32_t*`    | No       | The max retries  |
| `model_hint`   | `const char**` | No       | The model hint   |

**Returns:** `LiterllmDefaultClient`
**Errors:** Returns `NULL` on error.

---

#### literllm_create_client_from_json()

Create a new LLM client from a JSON string.

The JSON object accepts the same fields as `liter-llm.toml` (snake_case).

**Errors:**

Returns `LiterLlmError.BadRequest` if `json` is not valid JSON or
contains unknown fields.

**Signature:**

```c
LiterllmDefaultClient* literllm_create_client_from_json(const char* json);
```

**Parameters:**

| Name   | Type          | Required | Description |
| ------ | ------------- | -------- | ----------- |
| `json` | `const char*` | Yes      | The json    |

**Returns:** `LiterllmDefaultClient`
**Errors:** Returns `NULL` on error.

---

#### literllm_register_custom_provider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```c
void literllm_register_custom_provider(LiterllmCustomProviderConfig config);
```

**Parameters:**

| Name     | Type                           | Required | Description               |
| -------- | ------------------------------ | -------- | ------------------------- |
| `config` | `LiterllmCustomProviderConfig` | Yes      | The configuration options |

**Returns:** `void`
**Errors:** Returns `NULL` on error.

---

#### literllm_unregister_custom_provider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error only if the internal lock is poisoned.

**Signature:**

```c
bool literllm_unregister_custom_provider(const char* name);
```

**Parameters:**

| Name   | Type          | Required | Description |
| ------ | ------------- | -------- | ----------- |
| `name` | `const char*` | Yes      | The name    |

**Returns:** `bool`
**Errors:** Returns `NULL` on error.

---

#### literllm_all_providers()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.

**Signature:**

```c
LiterllmProviderConfig* literllm_all_providers();
```

**Returns:** `LiterllmProviderConfig*`
**Errors:** Returns `NULL` on error.

---

#### literllm_complex_provider_names()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```c
const char** literllm_complex_provider_names();
```

**Returns:** `const char**`
**Errors:** Returns `NULL` on error.

---

#### literllm_completion_cost()

Calculate the estimated cost of a completion given a model name and token
counts.

Returns `NULL` if the model is not present in the embedded pricing registry.
Returns `Some(cost_usd)` otherwise, where the value is in US dollars.

When an exact model name match is not found, progressively shorter prefixes
are tried by stripping from the last `-` or `.` separator. For example,
`gpt-4-0613` will match `gpt-4` if no `gpt-4-0613` entry exists.

**Signature:**

```c
double* literllm_completion_cost(const char* model, uint64_t prompt_tokens, uint64_t completion_tokens);
```

**Parameters:**

| Name                | Type          | Required | Description           |
| ------------------- | ------------- | -------- | --------------------- |
| `model`             | `const char*` | Yes      | The model             |
| `prompt_tokens`     | `uint64_t`    | Yes      | The prompt tokens     |
| `completion_tokens` | `uint64_t`    | Yes      | The completion tokens |

**Returns:** `double*`

---

#### literllm_completion_cost_with_cache()

Calculate the estimated cost of a completion, accounting for cached
(cache-hit) prompt tokens billed at the provider's discounted rate.

`cached_tokens` is the count of prompt tokens served from the provider's
prompt cache. It must be `<= prompt_tokens` (cached tokens are a subset of
the prompt). The non-cached portion is billed at `input_cost_per_token`
and the cached portion at `cache_read_input_token_cost` when the model
has cache pricing; otherwise the entire prompt is billed at the regular
input rate.

Returns `NULL` if the model is not present in the embedded pricing
registry, mirroring `completion_cost`.

**Signature:**

```c
double* literllm_completion_cost_with_cache(const char* model, uint64_t prompt_tokens, uint64_t cached_tokens, uint64_t completion_tokens);
```

**Parameters:**

| Name                | Type          | Required | Description           |
| ------------------- | ------------- | -------- | --------------------- |
| `model`             | `const char*` | Yes      | The model             |
| `prompt_tokens`     | `uint64_t`    | Yes      | The prompt tokens     |
| `cached_tokens`     | `uint64_t`    | Yes      | The cached tokens     |
| `completion_tokens` | `uint64_t`    | Yes      | The completion tokens |

**Returns:** `double*`

---

#### literllm_count_tokens()

Count tokens in a text string using the tokenizer for the given model.

The tokenizer is resolved from the model name prefix (e.g. `"gpt-4o"` maps
to the `Xenova/gpt-4o` HuggingFace tokenizer). Tokenizers are cached after
first load.

**Errors:**

Returns `LiterLlmError.BadRequest` if the tokenizer cannot be loaded
(e.g. network failure on first use) or if tokenization itself fails.

**Signature:**

```c
uintptr_t literllm_count_tokens(const char* model, const char* text);
```

**Parameters:**

| Name    | Type          | Required | Description |
| ------- | ------------- | -------- | ----------- |
| `model` | `const char*` | Yes      | The model   |
| `text`  | `const char*` | Yes      | The text    |

**Returns:** `uintptr_t`
**Errors:** Returns `NULL` on error.

---

#### literllm_count_request_tokens()

Count tokens for a full `ChatCompletionRequest`.

Sums tokens across all message text contents plus a per-message overhead
of ~4 tokens (for role, separators, and formatting metadata). Tool
definitions and multimodal content parts (images, audio, documents) are
not counted — only textual content contributes to the token total.

**Errors:**

Returns `LiterLlmError.BadRequest` if the tokenizer cannot be loaded or
if tokenization fails for any message.

**Signature:**

```c
uintptr_t literllm_count_request_tokens(const char* model, LiterllmChatCompletionRequest req);
```

**Parameters:**

| Name    | Type                            | Required | Description                 |
| ------- | ------------------------------- | -------- | --------------------------- |
| `model` | `const char*`                   | Yes      | The model                   |
| `req`   | `LiterllmChatCompletionRequest` | Yes      | The chat completion request |

**Returns:** `uintptr_t`
**Errors:** Returns `NULL` on error.

---

#### literllm_ensure_crypto_provider()

Install the `ring` crypto provider as the rustls process default, idempotently.

rustls 0.23+ removed the implicit default provider. This function installs
`ring` once per process. Subsequent calls are no-ops. Calling it from a
downstream Rust app that has already installed `aws-lc-rs` is safe — the
`Err` from `install_default()` is silently ignored.

Called automatically by every internal `reqwest.Client` constructor
(auth providers, default HTTP client). Bindings and downstream consumers
reach those constructors transitively, so no manual init is required.

WASM builds are exempt — the WASM target uses the browser/Node.js fetch
API instead of rustls, so no crypto provider is needed.

**Signature:**

```c
void literllm_ensure_crypto_provider();
```

**Returns:** `void`

---

### Types

#### LiterllmAssistantMessage

Assistant's response to a user message.

| Field           | Type                    | Default | Description                                                               |
| --------------- | ----------------------- | ------- | ------------------------------------------------------------------------- |
| `content`       | `const char**`          | `NULL`  | The assistant's text response. Absent if tool calls are returned instead. |
| `name`          | `const char**`          | `NULL`  | Optional name for the assistant.                                          |
| `tool_calls`    | `LiterllmToolCall**`    | `NULL`  | Tool calls the model wants to execute, if any.                            |
| `refusal`       | `const char**`          | `NULL`  | Refusal reason, if the model declined to respond per safety policies.     |
| `function_call` | `LiterllmFunctionCall*` | `NULL`  | Deprecated legacy function_call field; retained for API compatibility.    |

---

#### LiterllmAudioContent

Audio content part for speech-capable models.

| Field    | Type          | Default | Description                               |
| -------- | ------------- | ------- | ----------------------------------------- |
| `data`   | `const char*` | —       | Base64-encoded audio data.                |
| `format` | `const char*` | —       | Audio format (e.g., "wav", "mp3", "ogg"). |

---

#### LiterllmAuthConfig

Auth configuration block.

| Field       | Type               | Default | Description                                                                                                                         |
| ----------- | ------------------ | ------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `auth_type` | `LiterllmAuthType` | —       | Auth scheme classification.                                                                                                         |
| `env_var`   | `const char**`     | `NULL`  | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### LiterllmBatchListQuery

Query parameters for listing batches.

| Field   | Type           | Default | Description                                            |
| ------- | -------------- | ------- | ------------------------------------------------------ |
| `limit` | `uint32_t*`    | `NULL`  | Maximum number of results to return. Defaults to 20.   |
| `after` | `const char**` | `NULL`  | Pagination cursor: return results after this batch ID. |

---

#### LiterllmBatchListResponse

Response from listing batches.

| Field      | Type                   | Default | Description                                        |
| ---------- | ---------------------- | ------- | -------------------------------------------------- |
| `object`   | `const char*`          | —       | Object type (always `"list"`).                     |
| `data`     | `LiterllmBatchObject*` | `NULL`  | List of batch objects.                             |
| `has_more` | `bool*`                | `NULL`  | Whether more results are available.                |
| `first_id` | `const char**`         | `NULL`  | First batch ID in the result set (for pagination). |
| `last_id`  | `const char**`         | `NULL`  | Last batch ID in the result set (for pagination).  |

---

#### LiterllmBatchObject

A batch job object.

| Field               | Type                          | Default                        | Description                                             |
| ------------------- | ----------------------------- | ------------------------------ | ------------------------------------------------------- |
| `id`                | `const char*`                 | —                              | Unique batch ID.                                        |
| `object`            | `const char*`                 | —                              | Object type (always `"batch"`).                         |
| `endpoint`          | `const char*`                 | —                              | API endpoint (e.g., `"/v1/chat/completions"`).          |
| `input_file_id`     | `const char*`                 | —                              | ID of the input file.                                   |
| `completion_window` | `const char*`                 | —                              | Completion window (e.g., `"24h"`).                      |
| `status`            | `LiterllmBatchStatus`         | `LITERLLM_LITERLLM_VALIDATING` | Current job status.                                     |
| `output_file_id`    | `const char**`                | `NULL`                         | ID of the output file (present when completed).         |
| `error_file_id`     | `const char**`                | `NULL`                         | ID of the error file (present if some requests failed). |
| `created_at`        | `uint64_t`                    | —                              | Unix timestamp of batch creation.                       |
| `completed_at`      | `uint64_t*`                   | `NULL`                         | Unix timestamp of completion (if completed).            |
| `failed_at`         | `uint64_t*`                   | `NULL`                         | Unix timestamp of failure (if failed).                  |
| `expired_at`        | `uint64_t*`                   | `NULL`                         | Unix timestamp of expiration (if expired).              |
| `request_counts`    | `LiterllmBatchRequestCounts*` | `NULL`                         | Request processing counts.                              |
| `metadata`          | `void**`                      | `NULL`                         | Metadata attached to the batch.                         |

---

#### LiterllmBatchRequestCounts

Request processing counts for a batch.

| Field       | Type       | Default | Description                  |
| ----------- | ---------- | ------- | ---------------------------- |
| `total`     | `uint64_t` | —       | Total requests in the batch. |
| `completed` | `uint64_t` | —       | Completed requests.          |
| `failed`    | `uint64_t` | —       | Failed requests.             |

---

#### LiterllmBudgetConfig

Configuration for budget enforcement.

| Field          | Type                  | Default                  | Description                                                                                      |
| -------------- | --------------------- | ------------------------ | ------------------------------------------------------------------------------------------------ |
| `global_limit` | `double*`             | `NULL`                   | Maximum total spend across all models, in USD. `NULL` means unlimited.                           |
| `model_limits` | `void*`               | `NULL`                   | Per-model spending limits in USD. Models not listed here are only constrained by `global_limit`. |
| `enforcement`  | `LiterllmEnforcement` | `LITERLLM_LITERLLM_HARD` | Whether to reject requests or merely warn when a limit is exceeded.                              |

### Methods

#### literllm_default()

**Signature:**

```c
LiterllmBudgetConfig literllm_default();
```

---

#### LiterllmCacheConfig

Configuration for the response cache.

| Field         | Type                   | Default                    | Description                         |
| ------------- | ---------------------- | -------------------------- | ----------------------------------- |
| `max_entries` | `uintptr_t`            | `256`                      | Maximum number of cached entries.   |
| `ttl`         | `uint64_t`             | `300000ms`                 | Time-to-live for each cached entry. |
| `backend`     | `LiterllmCacheBackend` | `LITERLLM_LITERLLM_MEMORY` | Storage backend to use.             |

### Methods

#### literllm_default()

**Signature:**

```c
LiterllmCacheConfig literllm_default();
```

---

#### LiterllmChatCompletionChunk

A streamed chunk of a chat completion response.

| Field                | Type                    | Default | Description                                                                                                                                   |
| -------------------- | ----------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `id`                 | `const char*`           | —       | Unique identifier for this stream.                                                                                                            |
| `object`             | `const char*`           | —       | Always `"chat.completion.chunk"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created`            | `uint64_t`              | —       | Unix timestamp of chunk creation.                                                                                                             |
| `model`              | `const char*`           | —       | Model used to generate the chunk.                                                                                                             |
| `choices`            | `LiterllmStreamChoice*` | `NULL`  | Streaming choices (delta updates).                                                                                                            |
| `usage`              | `LiterllmUsage*`        | `NULL`  | Token usage (typically only in the final chunk).                                                                                              |
| `system_fingerprint` | `const char**`          | `NULL`  | Fingerprint of the system configuration (OpenAI-specific).                                                                                    |
| `service_tier`       | `const char**`          | `NULL`  | Service tier used (OpenAI-specific).                                                                                                          |

---

#### LiterllmChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field                 | Type                           | Default | Description                                                                                                                       |
| --------------------- | ------------------------------ | ------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `model`               | `const char*`                  | —       | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`).                                                                          |
| `messages`            | `LiterllmMessage*`             | `NULL`  | Conversation history from oldest to newest.                                                                                       |
| `temperature`         | `double*`                      | `NULL`  | Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0.                                               |
| `top_p`               | `double*`                      | `NULL`  | Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused.                                                                |
| `n`                   | `uint32_t*`                    | `NULL`  | Number of chat completions to generate. Defaults to 1.                                                                            |
| `stream`              | `bool*`                        | `NULL`  | Whether to stream the response. Managed by the client layer — do not set directly.                                                |
| `stop`                | `LiterllmStopSequence*`        | `NULL`  | Stop sequence(s) that halt token generation.                                                                                      |
| `max_tokens`          | `uint64_t*`                    | `NULL`  | Max output tokens. Different from max_completion_tokens in some providers.                                                        |
| `presence_penalty`    | `double*`                      | `NULL`  | Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics.                                                          |
| `frequency_penalty`   | `double*`                      | `NULL`  | Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens.                                                         |
| `logit_bias`          | `void**`                       | `NULL`  | Token bias map. Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user`                | `const char**`                 | `NULL`  | User identifier for request tracking and abuse detection.                                                                         |
| `tools`               | `LiterllmChatCompletionTool**` | `NULL`  | Tools the model can invoke.                                                                                                       |
| `tool_choice`         | `LiterllmToolChoice*`          | `NULL`  | Tool usage mode (auto, required, none, or specific tool).                                                                         |
| `parallel_tool_calls` | `bool*`                        | `NULL`  | Whether the model can call multiple tools in parallel. Defaults to true.                                                          |
| `response_format`     | `LiterllmResponseFormat*`      | `NULL`  | Output format constraint (text, JSON, JSON schema).                                                                               |
| `stream_options`      | `LiterllmStreamOptions*`       | `NULL`  | Streaming options (e.g., include_usage).                                                                                          |
| `seed`                | `int64_t*`                     | `NULL`  | Random seed for reproducible outputs. Provider support varies.                                                                    |
| `reasoning_effort`    | `LiterllmReasoningEffort*`     | `NULL`  | Reasoning effort level (low, medium, high) for extended-thinking models.                                                          |
| `extra_body`          | `void**`                       | `NULL`  | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc.      |

---

#### LiterllmChatCompletionResponse

Chat completion response from the API.

| Field                | Type              | Default | Description                                                                                                                                      |
| -------------------- | ----------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `id`                 | `const char*`     | —       | Unique identifier for this response.                                                                                                             |
| `object`             | `const char*`     | —       | Always `"chat.completion"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created`            | `uint64_t`        | —       | Unix timestamp of response creation.                                                                                                             |
| `model`              | `const char*`     | —       | Model used to generate the response.                                                                                                             |
| `choices`            | `LiterllmChoice*` | `NULL`  | List of completion choices.                                                                                                                      |
| `usage`              | `LiterllmUsage*`  | `NULL`  | Token usage statistics.                                                                                                                          |
| `system_fingerprint` | `const char**`    | `NULL`  | Fingerprint of the system configuration (OpenAI-specific).                                                                                       |
| `service_tier`       | `const char**`    | `NULL`  | Service tier used (OpenAI-specific).                                                                                                             |

---

#### LiterllmChatCompletionTool

A tool the model can invoke (currently, all tools are functions).

| Field       | Type                         | Default | Description                                                             |
| ----------- | ---------------------------- | ------- | ----------------------------------------------------------------------- |
| `tool_type` | `LiterllmToolType`           | —       | Tool type (always "function" in OpenAI spec).                           |
| `function`  | `LiterllmFunctionDefinition` | —       | Function definition with name, description, and JSON schema parameters. |

---

#### LiterllmChoice

A single completion choice.

| Field           | Type                       | Default | Description                                                                        |
| --------------- | -------------------------- | ------- | ---------------------------------------------------------------------------------- |
| `index`         | `uint32_t`                 | —       | Index of this choice in the choices array.                                         |
| `message`       | `LiterllmAssistantMessage` | —       | The assistant's message response.                                                  |
| `finish_reason` | `LiterllmFinishReason*`    | `NULL`  | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

---

#### LiterllmCreateBatchRequest

Request to create a batch job.

| Field               | Type          | Default | Description                                    |
| ------------------- | ------------- | ------- | ---------------------------------------------- |
| `input_file_id`     | `const char*` | —       | ID of the uploaded input file (JSONL format).  |
| `endpoint`          | `const char*` | —       | API endpoint (e.g., `"/v1/chat/completions"`). |
| `completion_window` | `const char*` | —       | Completion window (e.g., `"24h"`).             |
| `metadata`          | `void**`      | `NULL`  | Optional metadata to attach to the batch.      |

---

#### LiterllmCreateFileRequest

Request to upload a file.

| Field      | Type                  | Default                        | Description                                     |
| ---------- | --------------------- | ------------------------------ | ----------------------------------------------- |
| `file`     | `const char*`         | —                              | Base64-encoded file data.                       |
| `purpose`  | `LiterllmFilePurpose` | `LITERLLM_LITERLLM_ASSISTANTS` | Purpose for the file.                           |
| `filename` | `const char**`        | `NULL`                         | Optional filename to associate with the upload. |

---

#### LiterllmCreateImageRequest

Request to create images from a text prompt.

| Field             | Type           | Default | Description                                                            |
| ----------------- | -------------- | ------- | ---------------------------------------------------------------------- |
| `prompt`          | `const char*`  | —       | Text description of the image to generate.                             |
| `model`           | `const char**` | `NULL`  | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `n`               | `uint32_t*`    | `NULL`  | Number of images to generate. Defaults to 1.                           |
| `size`            | `const char**` | `NULL`  | Image size (e.g., `"1024x1024"`, `"1792x1024"`).                       |
| `quality`         | `const char**` | `NULL`  | Image quality: `"standard"` or `"hd"`.                                 |
| `style`           | `const char**` | `NULL`  | Style: `"natural"` or `"vivid"` (DALL-E 3 only).                       |
| `response_format` | `const char**` | `NULL`  | Response format: `"url"` or `"b64_json"`.                              |
| `user`            | `const char**` | `NULL`  | User identifier for request tracking.                                  |

---

#### LiterllmCreateResponseRequest

Request to create a structured response.

| Field               | Type                     | Default | Description                                               |
| ------------------- | ------------------------ | ------- | --------------------------------------------------------- |
| `model`             | `const char*`            | —       | Model ID.                                                 |
| `input`             | `void*`                  | —       | Input data to process (e.g., a document to extract from). |
| `instructions`      | `const char**`           | `NULL`  | Instructions for processing the input.                    |
| `tools`             | `LiterllmResponseTool**` | `NULL`  | Available tools the model can use.                        |
| `temperature`       | `double*`                | `NULL`  | Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0.    |
| `max_output_tokens` | `uint64_t*`              | `NULL`  | Maximum output tokens.                                    |
| `metadata`          | `void**`                 | `NULL`  | Optional metadata.                                        |

---

#### LiterllmCreateSpeechRequest

Request to generate speech audio from text.

| Field             | Type           | Default | Description                                                                         |
| ----------------- | -------------- | ------- | ----------------------------------------------------------------------------------- |
| `model`           | `const char*`  | —       | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`).                                           |
| `input`           | `const char*`  | —       | Text to synthesize into speech.                                                     |
| `voice`           | `const char*`  | —       | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `response_format` | `const char**` | `NULL`  | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`).        |
| `speed`           | `double*`      | `NULL`  | Playback speed in `[0.25, 4.0]`. Defaults to 1.0.                                   |

---

#### LiterllmCreateTranscriptionRequest

Request to transcribe audio into text.

| Field             | Type           | Default | Description                                                                           |
| ----------------- | -------------- | ------- | ------------------------------------------------------------------------------------- |
| `model`           | `const char*`  | —       | Model ID (e.g., `"whisper-1"`).                                                       |
| `file`            | `const char*`  | —       | Base64-encoded audio file data.                                                       |
| `language`        | `const char**` | `NULL`  | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `prompt`          | `const char**` | `NULL`  | Optional text to guide the model (improves accuracy for domain-specific terms).       |
| `response_format` | `const char**` | `NULL`  | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`).         |
| `temperature`     | `double*`      | `NULL`  | Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0.    |

---

#### LiterllmCustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field            | Type                       | Default | Description                                                                 |
| ---------------- | -------------------------- | ------- | --------------------------------------------------------------------------- |
| `name`           | `const char*`              | —       | Unique name for this provider (e.g., "my-provider").                        |
| `base_url`       | `const char*`              | —       | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `auth_header`    | `LiterllmAuthHeaderFormat` | —       | Authentication header format.                                               |
| `model_prefixes` | `const char**`             | —       | Model name prefixes that route to this provider (e.g., ["my-"]).            |

---

#### LiterllmDefaultClient

Default client implementation backed by `reqwest`.

Sends requests to 140+ LLM providers with automatic provider detection
and per-request routing. The provider is resolved at construction time
from `model_hint` (or defaults to OpenAI), but individual requests can
override the provider via model name prefix (e.g. `"anthropic/claude-3-5-sonnet"`
routes to Anthropic regardless of construction-time setting).

When the model prefix does not match any known provider, the construction-time
provider is used as the fallback. This enables seamless migration between
providers by changing only the model name.

The provider is stored behind an `Arc` so it can be shared cheaply into
async closures and streaming tasks. Pre-computed auth headers and extra
headers are cached at construction to avoid redundant encoding on every request.

### Methods

#### literllm_chat()

**Signature:**

```c
LiterllmChatCompletionResponse literllm_chat(LiterllmChatCompletionRequest req);
```

#### literllm_chat_stream()

**Signature:**

```c
const char* literllm_chat_stream(LiterllmChatCompletionRequest req);
```

#### literllm_embed()

**Signature:**

```c
LiterllmEmbeddingResponse literllm_embed(LiterllmEmbeddingRequest req);
```

#### literllm_list_models()

**Signature:**

```c
LiterllmModelsListResponse literllm_list_models();
```

#### literllm_image_generate()

**Signature:**

```c
LiterllmImagesResponse literllm_image_generate(LiterllmCreateImageRequest req);
```

#### literllm_speech()

**Signature:**

```c
const uint8_t* literllm_speech(LiterllmCreateSpeechRequest req);
```

#### literllm_transcribe()

**Signature:**

```c
LiterllmTranscriptionResponse literllm_transcribe(LiterllmCreateTranscriptionRequest req);
```

#### literllm_moderate()

**Signature:**

```c
LiterllmModerationResponse literllm_moderate(LiterllmModerationRequest req);
```

#### literllm_rerank()

**Signature:**

```c
LiterllmRerankResponse literllm_rerank(LiterllmRerankRequest req);
```

#### literllm_search()

**Signature:**

```c
LiterllmSearchResponse literllm_search(LiterllmSearchRequest req);
```

#### literllm_ocr()

**Signature:**

```c
LiterllmOcrResponse literllm_ocr(LiterllmOcrRequest req);
```

#### literllm_create_file()

**Signature:**

```c
LiterllmFileObject literllm_create_file(LiterllmCreateFileRequest req);
```

#### literllm_retrieve_file()

**Signature:**

```c
LiterllmFileObject literllm_retrieve_file(const char* file_id);
```

#### literllm_delete_file()

**Signature:**

```c
LiterllmDeleteResponse literllm_delete_file(const char* file_id);
```

#### literllm_list_files()

**Signature:**

```c
LiterllmFileListResponse literllm_list_files(LiterllmFileListQuery query);
```

#### literllm_file_content()

**Signature:**

```c
const uint8_t* literllm_file_content(const char* file_id);
```

#### literllm_create_batch()

**Signature:**

```c
LiterllmBatchObject literllm_create_batch(LiterllmCreateBatchRequest req);
```

#### literllm_retrieve_batch()

**Signature:**

```c
LiterllmBatchObject literllm_retrieve_batch(const char* batch_id);
```

#### literllm_list_batches()

**Signature:**

```c
LiterllmBatchListResponse literllm_list_batches(LiterllmBatchListQuery query);
```

#### literllm_cancel_batch()

**Signature:**

```c
LiterllmBatchObject literllm_cancel_batch(const char* batch_id);
```

#### literllm_create_response()

**Signature:**

```c
LiterllmResponseObject literllm_create_response(LiterllmCreateResponseRequest req);
```

#### literllm_retrieve_response()

**Signature:**

```c
LiterllmResponseObject literllm_retrieve_response(const char* response_id);
```

#### literllm_cancel_response()

**Signature:**

```c
LiterllmResponseObject literllm_cancel_response(const char* response_id);
```

---

#### LiterllmDeleteResponse

Response from a delete operation.

| Field     | Type          | Default | Description                                 |
| --------- | ------------- | ------- | ------------------------------------------- |
| `id`      | `const char*` | —       | ID of the deleted resource.                 |
| `object`  | `const char*` | —       | Object type.                                |
| `deleted` | `bool`        | —       | Confirmation that the resource was deleted. |

---

#### LiterllmDeveloperMessage

Developer message (system-like message for Claude models).

| Field     | Type           | Default | Description                                     |
| --------- | -------------- | ------- | ----------------------------------------------- |
| `content` | `const char*`  | —       | Developer-specific instructions or context.     |
| `name`    | `const char**` | `NULL`  | Optional name for the developer message source. |

---

#### LiterllmDocumentContent

PDF/document content part for vision-capable models.

| Field        | Type          | Default | Description                                      |
| ------------ | ------------- | ------- | ------------------------------------------------ |
| `data`       | `const char*` | —       | Base64-encoded document data or URL.             |
| `media_type` | `const char*` | —       | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### LiterllmEmbeddingObject

A single embedding vector.

| Field       | Type          | Default | Description                                                                                                                                |
| ----------- | ------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `object`    | `const char*` | —       | Always `"embedding"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `embedding` | `double*`     | —       | The embedding vector.                                                                                                                      |
| `index`     | `uint32_t`    | —       | Index in the batch (corresponds to input order).                                                                                           |

---

#### LiterllmEmbeddingRequest

Embedding request.

| Field             | Type                       | Default                    | Description                                                 |
| ----------------- | -------------------------- | -------------------------- | ----------------------------------------------------------- |
| `model`           | `const char*`              | —                          | Model ID (e.g., `"text-embedding-3-small"`).                |
| `input`           | `LiterllmEmbeddingInput`   | `LITERLLM_LITERLLM_SINGLE` | Text or texts to embed.                                     |
| `encoding_format` | `LiterllmEmbeddingFormat*` | `NULL`                     | Output format: float (native) or base64.                    |
| `dimensions`      | `uint32_t*`                | `NULL`                     | Requested embedding dimensions (if supported by the model). |
| `user`            | `const char**`             | `NULL`                     | User identifier for request tracking.                       |

---

#### LiterllmEmbeddingResponse

Embedding response.

| Field    | Type                       | Default | Description                                                                                                                           |
| -------- | -------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `object` | `const char*`              | —       | Always `"list"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data`   | `LiterllmEmbeddingObject*` | —       | List of embeddings.                                                                                                                   |
| `model`  | `const char*`              | —       | Model used to generate embeddings.                                                                                                    |
| `usage`  | `LiterllmUsage*`           | `NULL`  | Token usage (input tokens only; embeddings have zero output tokens).                                                                  |

---

#### LiterllmFileListQuery

Query parameters for listing files.

| Field     | Type           | Default | Description                                              |
| --------- | -------------- | ------- | -------------------------------------------------------- |
| `purpose` | `const char**` | `NULL`  | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `limit`   | `uint32_t*`    | `NULL`  | Maximum number of results to return. Defaults to 20.     |
| `after`   | `const char**` | `NULL`  | Pagination cursor: return results after this file ID.    |

---

#### LiterllmFileListResponse

Response from listing files.

| Field      | Type                  | Default | Description                         |
| ---------- | --------------------- | ------- | ----------------------------------- |
| `object`   | `const char*`         | —       | Object type (always `"list"`).      |
| `data`     | `LiterllmFileObject*` | `NULL`  | List of file objects.               |
| `has_more` | `bool*`               | `NULL`  | Whether more results are available. |

---

#### LiterllmFileObject

An uploaded file object.

| Field        | Type           | Default | Description                                            |
| ------------ | -------------- | ------- | ------------------------------------------------------ |
| `id`         | `const char*`  | —       | Unique file ID.                                        |
| `object`     | `const char*`  | —       | Object type (always `"file"`).                         |
| `bytes`      | `uint64_t`     | —       | File size in bytes.                                    |
| `created_at` | `uint64_t`     | —       | Unix timestamp of file creation.                       |
| `filename`   | `const char*`  | —       | Filename.                                              |
| `purpose`    | `const char*`  | —       | File purpose.                                          |
| `status`     | `const char**` | `NULL`  | Processing status (e.g., `"uploaded"`, `"processed"`). |

---

#### LiterllmFunctionCall

Function call details.

| Field       | Type          | Default | Description                                                  |
| ----------- | ------------- | ------- | ------------------------------------------------------------ |
| `name`      | `const char*` | —       | Function name.                                               |
| `arguments` | `const char*` | —       | Arguments as a JSON string (parse with serde_json.from_str). |

---

#### LiterllmFunctionDefinition

Function definition exposed to the model.

| Field         | Type           | Default | Description                                                            |
| ------------- | -------------- | ------- | ---------------------------------------------------------------------- |
| `name`        | `const char*`  | —       | Name of the function. Required and must be alphanumeric + underscores. |
| `description` | `const char**` | `NULL`  | Human-readable description explaining what the function does.          |
| `parameters`  | `void**`       | `NULL`  | JSON Schema defining the function's parameters.                        |
| `strict`      | `bool*`        | `NULL`  | If true, enforce strict JSON schema validation for arguments.          |

---

#### LiterllmFunctionMessage

Deprecated legacy function-role message body.

| Field     | Type          | Default | Description                |
| --------- | ------------- | ------- | -------------------------- |
| `content` | `const char*` | —       | The extracted text content |
| `name`    | `const char*` | —       | The name                   |

---

#### LiterllmImage

A single generated image, returned as either a URL or base64 data.

| Field            | Type           | Default | Description                                                    |
| ---------------- | -------------- | ------- | -------------------------------------------------------------- |
| `url`            | `const char**` | `NULL`  | Image URL (if response_format was "url").                      |
| `b64_json`       | `const char**` | `NULL`  | Base64-encoded image data (if response_format was "b64_json"). |
| `revised_prompt` | `const char**` | `NULL`  | The final prompt used to generate the image (DALL-E 3).        |

---

#### LiterllmImageUrl

An image URL reference with optional detail level for processing.

| Field    | Type                   | Default | Description                                                              |
| -------- | ---------------------- | ------- | ------------------------------------------------------------------------ |
| `url`    | `const char*`          | —       | URL of the image (data URI or HTTP/HTTPS URL).                           |
| `detail` | `LiterllmImageDetail*` | `NULL`  | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### LiterllmImagesResponse

Response containing generated images.

| Field     | Type             | Default | Description                       |
| --------- | ---------------- | ------- | --------------------------------- |
| `created` | `uint64_t`       | —       | Unix timestamp of image creation. |
| `data`    | `LiterllmImage*` | `NULL`  | List of generated images.         |

---

#### LiterllmJsonSchemaFormat

JSON Schema specification for constrained output.

| Field         | Type           | Default | Description                                         |
| ------------- | -------------- | ------- | --------------------------------------------------- |
| `name`        | `const char*`  | —       | Name of the schema (must be unique in the request). |
| `description` | `const char**` | `NULL`  | Description of what the schema represents.          |
| `schema`      | `void*`        | —       | JSON Schema object defining the output structure.   |
| `strict`      | `bool*`        | `NULL`  | If true, enforce strict schema validation.          |

---

#### LiterllmModelObject

A model available from the API.

| Field      | Type          | Default | Description                                                                                                                            |
| ---------- | ------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `id`       | `const char*` | —       | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`).                                                                                    |
| `object`   | `const char*` | —       | Always `"model"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created`  | `uint64_t`    | —       | Unix timestamp of model creation (or release date).                                                                                    |
| `owned_by` | `const char*` | —       | Organization or entity that owns the model.                                                                                            |

---

#### LiterllmModelsListResponse

Response listing available models.

| Field    | Type                   | Default | Description                                                                                                                           |
| -------- | ---------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `object` | `const char*`          | —       | Always `"list"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data`   | `LiterllmModelObject*` | `NULL`  | List of available models.                                                                                                             |

---

#### LiterllmModerationCategories

Boolean flags for each moderation category.

| Field                    | Type   | Default | Description                          |
| ------------------------ | ------ | ------- | ------------------------------------ |
| `sexual`                 | `bool` | —       | Sexual content.                      |
| `hate`                   | `bool` | —       | Hate speech.                         |
| `harassment`             | `bool` | —       | Harassment.                          |
| `self_harm`              | `bool` | —       | Self-harm content.                   |
| `sexual_minors`          | `bool` | —       | Sexual content involving minors.     |
| `hate_threatening`       | `bool` | —       | Hate speech that threatens violence. |
| `violence_graphic`       | `bool` | —       | Graphic violence.                    |
| `self_harm_intent`       | `bool` | —       | Intent to self-harm.                 |
| `self_harm_instructions` | `bool` | —       | Instructions for self-harm.          |
| `harassment_threatening` | `bool` | —       | Harassment that threatens violence.  |
| `violence`               | `bool` | —       | Non-graphic violence.                |

---

#### LiterllmModerationCategoryScores

Confidence scores for each moderation category.

| Field                    | Type     | Default | Description                                |
| ------------------------ | -------- | ------- | ------------------------------------------ |
| `sexual`                 | `double` | —       | Sexual content score.                      |
| `hate`                   | `double` | —       | Hate speech score.                         |
| `harassment`             | `double` | —       | Harassment score.                          |
| `self_harm`              | `double` | —       | Self-harm content score.                   |
| `sexual_minors`          | `double` | —       | Sexual content involving minors score.     |
| `hate_threatening`       | `double` | —       | Hate speech that threatens violence score. |
| `violence_graphic`       | `double` | —       | Graphic violence score.                    |
| `self_harm_intent`       | `double` | —       | Intent to self-harm score.                 |
| `self_harm_instructions` | `double` | —       | Instructions for self-harm score.          |
| `harassment_threatening` | `double` | —       | Harassment that threatens violence score.  |
| `violence`               | `double` | —       | Non-graphic violence score.                |

---

#### LiterllmModerationRequest

Request to classify content for policy violations.

| Field   | Type                      | Default                    | Description                                                                       |
| ------- | ------------------------- | -------------------------- | --------------------------------------------------------------------------------- |
| `input` | `LiterllmModerationInput` | `LITERLLM_LITERLLM_SINGLE` | Text or texts to check.                                                           |
| `model` | `const char**`            | `NULL`                     | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

---

#### LiterllmModerationResponse

Response from the moderation endpoint.

| Field     | Type                        | Default | Description                                    |
| --------- | --------------------------- | ------- | ---------------------------------------------- |
| `id`      | `const char*`               | —       | Unique identifier for this moderation request. |
| `model`   | `const char*`               | —       | Model used for classification.                 |
| `results` | `LiterllmModerationResult*` | —       | Results for each input string.                 |

---

#### LiterllmModerationResult

A single moderation classification result.

| Field             | Type                               | Default | Description                                 |
| ----------------- | ---------------------------------- | ------- | ------------------------------------------- |
| `flagged`         | `bool`                             | —       | True if any category was flagged.           |
| `categories`      | `LiterllmModerationCategories`     | —       | Boolean flags for each moderation category. |
| `category_scores` | `LiterllmModerationCategoryScores` | —       | Confidence scores for each category.        |

---

#### LiterllmOcrImage

An image extracted from an OCR page.

| Field          | Type           | Default | Description                                                     |
| -------------- | -------------- | ------- | --------------------------------------------------------------- |
| `id`           | `const char*`  | —       | Unique image identifier within the document.                    |
| `image_base64` | `const char**` | `NULL`  | Base64-encoded image data (if `include_image_base64` was true). |

---

#### LiterllmOcrPage

A single page of OCR output.

| Field        | Type                      | Default | Description                                                                   |
| ------------ | ------------------------- | ------- | ----------------------------------------------------------------------------- |
| `index`      | `uint32_t`                | —       | Page index (0-based).                                                         |
| `markdown`   | `const char*`             | —       | Extracted page content as Markdown.                                           |
| `images`     | `LiterllmOcrImage**`      | `NULL`  | Embedded images extracted from the page (if `include_image_base64` was true). |
| `dimensions` | `LiterllmPageDimensions*` | `NULL`  | Page dimensions in pixels, if available.                                      |

---

#### LiterllmOcrRequest

An OCR request.

| Field                  | Type                  | Default                 | Description                                                      |
| ---------------------- | --------------------- | ----------------------- | ---------------------------------------------------------------- |
| `model`                | `const char*`         | —                       | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document`             | `LiterllmOcrDocument` | `LITERLLM_LITERLLM_URL` | The document to process (URL or base64).                         |
| `pages`                | `uint32_t**`          | `NULL`                  | Specific pages to process (1-indexed). `NULL` means all pages.   |
| `include_image_base64` | `bool*`               | `NULL`                  | Whether to include base64-encoded images of each processed page. |

---

#### LiterllmOcrResponse

An OCR response.

| Field   | Type               | Default | Description                               |
| ------- | ------------------ | ------- | ----------------------------------------- |
| `pages` | `LiterllmOcrPage*` | —       | Extracted pages in order.                 |
| `model` | `const char*`      | —       | Model/provider used for OCR.              |
| `usage` | `LiterllmUsage*`   | `NULL`  | Token usage, if reported by the provider. |

---

#### LiterllmPageDimensions

Page dimensions in pixels.

| Field    | Type       | Default | Description       |
| -------- | ---------- | ------- | ----------------- |
| `width`  | `uint32_t` | —       | Width in pixels.  |
| `height` | `uint32_t` | —       | Height in pixels. |

---

#### LiterllmPromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage.prompt_tokens` — it is _not_ an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field           | Type       | Default | Description                                                          |
| --------------- | ---------- | ------- | -------------------------------------------------------------------- |
| `cached_tokens` | `uint64_t` | —       | Cached tokens present in the prompt. Defaults to 0 when absent.      |
| `audio_tokens`  | `uint64_t` | —       | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### LiterllmProviderConfig

Static configuration for a single provider entry in providers.json.

| Field            | Type                  | Default | Description                                                                                                                                                                                                                                      |
| ---------------- | --------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `name`           | `const char*`         | —       | Provider identifier (matches the entry key in providers.json).                                                                                                                                                                                   |
| `display_name`   | `const char**`        | `NULL`  | Human-readable provider name shown in UIs.                                                                                                                                                                                                       |
| `base_url`       | `const char**`        | `NULL`  | Base URL used as the default for this provider's HTTP client.                                                                                                                                                                                    |
| `auth`           | `LiterllmAuthConfig*` | `NULL`  | Authentication scheme metadata (auth type + env var holding the key).                                                                                                                                                                            |
| `endpoints`      | `const char***`       | `NULL`  | Supported endpoint kinds (e.g. `chat`, `embeddings`).                                                                                                                                                                                            |
| `model_prefixes` | `const char***`       | `NULL`  | Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`).                                                                                                                                                                           |
| `param_mappings` | `void**`              | `NULL`  | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`). Applied automatically by `ConfigDrivenProvider.transform_request`. |

---

#### LiterllmRateLimitConfig

Configuration for per-model rate limits.

| Field    | Type        | Default   | Description                                          |
| -------- | ----------- | --------- | ---------------------------------------------------- |
| `rpm`    | `uint32_t*` | `NULL`    | Maximum requests per window. `NULL` means unlimited. |
| `tpm`    | `uint64_t*` | `NULL`    | Maximum tokens per window. `NULL` means unlimited.   |
| `window` | `uint64_t`  | `60000ms` | Fixed window duration (defaults to 60 s).            |

### Methods

#### literllm_default()

**Signature:**

```c
LiterllmRateLimitConfig literllm_default();
```

---

#### LiterllmRerankRequest

Request to rerank documents by relevance to a query.

| Field              | Type                      | Default | Description                                                 |
| ------------------ | ------------------------- | ------- | ----------------------------------------------------------- |
| `model`            | `const char*`             | —       | Model ID (e.g., `"cohere/rerank-english-v3.0"`).            |
| `query`            | `const char*`             | —       | The search query.                                           |
| `documents`        | `LiterllmRerankDocument*` | `NULL`  | Documents to rerank.                                        |
| `top_n`            | `uint32_t*`               | `NULL`  | Return only the top N results. Optional.                    |
| `return_documents` | `bool*`                   | `NULL`  | Include the document content in results. Defaults to false. |

---

#### LiterllmRerankResponse

Response from the rerank endpoint.

| Field     | Type                    | Default | Description                                      |
| --------- | ----------------------- | ------- | ------------------------------------------------ |
| `id`      | `const char**`          | `NULL`  | Unique identifier for this rerank request.       |
| `results` | `LiterllmRerankResult*` | —       | Reranked documents in order of relevance.        |
| `meta`    | `void**`                | `NULL`  | Optional metadata about the reranking operation. |

---

#### LiterllmRerankResult

A single reranked document with its relevance score.

| Field             | Type                            | Default | Description                                                  |
| ----------------- | ------------------------------- | ------- | ------------------------------------------------------------ |
| `index`           | `uint32_t`                      | —       | Original document index in the input list.                   |
| `relevance_score` | `double`                        | —       | Relevance score in `[0, 1]`. Higher indicates more relevant. |
| `document`        | `LiterllmRerankResultDocument*` | `NULL`  | Original document content (if `return_documents` was true).  |

---

#### LiterllmRerankResultDocument

The text content of a reranked document, returned when `return_documents` is true.

| Field  | Type          | Default | Description    |
| ------ | ------------- | ------- | -------------- |
| `text` | `const char*` | —       | Document text. |

---

#### LiterllmResponseObject

Response from a structured response request.

| Field        | Type                          | Default | Description                               |
| ------------ | ----------------------------- | ------- | ----------------------------------------- |
| `id`         | `const char*`                 | —       | Unique response ID.                       |
| `object`     | `const char*`                 | —       | Object type (e.g., `"response"`).         |
| `created_at` | `uint64_t`                    | —       | Unix timestamp of response creation.      |
| `model`      | `const char*`                 | —       | Model used to generate the response.      |
| `status`     | `const char*`                 | —       | Status (e.g., `"succeeded"`, `"failed"`). |
| `output`     | `LiterllmResponseOutputItem*` | `NULL`  | Output items from the response.           |
| `usage`      | `LiterllmResponseUsage*`      | `NULL`  | Token usage.                              |
| `error`      | `void**`                      | `NULL`  | Error details (if status is "failed").    |

---

#### LiterllmResponseOutputItem

A single output item from the response.

| Field       | Type          | Default | Description                                          |
| ----------- | ------------- | ------- | ---------------------------------------------------- |
| `item_type` | `const char*` | —       | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `content`   | `void*`       | —       | Output content (flattened into the object).          |

---

#### LiterllmResponseTool

A tool available for the response request.

| Field       | Type          | Default | Description                                     |
| ----------- | ------------- | ------- | ----------------------------------------------- |
| `tool_type` | `const char*` | —       | Tool type (e.g., "extractor", "search").        |
| `config`    | `void*`       | —       | Tool configuration (flattened into the object). |

---

#### LiterllmResponseUsage

Token usage for a response.

| Field           | Type       | Default | Description         |
| --------------- | ---------- | ------- | ------------------- |
| `input_tokens`  | `uint64_t` | —       | Input tokens used.  |
| `output_tokens` | `uint64_t` | —       | Output tokens used. |
| `total_tokens`  | `uint64_t` | —       | Total tokens used.  |

---

#### LiterllmSearchRequest

A search request.

| Field                  | Type            | Default | Description                                                                    |
| ---------------------- | --------------- | ------- | ------------------------------------------------------------------------------ |
| `model`                | `const char*`   | —       | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`).      |
| `query`                | `const char*`   | —       | The search query string.                                                       |
| `max_results`          | `uint32_t*`     | `NULL`  | Maximum number of results to return.                                           |
| `search_domain_filter` | `const char***` | `NULL`  | Domain filter — restrict results to specific domains.                          |
| `country`              | `const char**`  | `NULL`  | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

---

#### LiterllmSearchResponse

A search response.

| Field     | Type                    | Default | Description                               |
| --------- | ----------------------- | ------- | ----------------------------------------- |
| `results` | `LiterllmSearchResult*` | —       | List of search results.                   |
| `model`   | `const char*`           | —       | Model/provider that performed the search. |

---

#### LiterllmSearchResult

An individual search result.

| Field     | Type           | Default | Description                                     |
| --------- | -------------- | ------- | ----------------------------------------------- |
| `title`   | `const char*`  | —       | Result title.                                   |
| `url`     | `const char*`  | —       | Result URL.                                     |
| `snippet` | `const char*`  | —       | Text snippet or excerpt from the page.          |
| `date`    | `const char**` | `NULL`  | Publication or last-updated date, if available. |

---

#### LiterllmSpecificFunction

Name of the specific function to invoke.

| Field  | Type          | Default | Description    |
| ------ | ------------- | ------- | -------------- |
| `name` | `const char*` | —       | Function name. |

---

#### LiterllmSpecificToolChoice

Directive to call a specific tool.

| Field         | Type                       | Default                      | Description                      |
| ------------- | -------------------------- | ---------------------------- | -------------------------------- |
| `choice_type` | `LiterllmToolType`         | `LITERLLM_LITERLLM_FUNCTION` | Tool type (always "function").   |
| `function`    | `LiterllmSpecificFunction` | —                            | The specific function to invoke. |

---

#### LiterllmStreamChoice

A streaming choice with incremental delta.

| Field           | Type                    | Default | Description                                                    |
| --------------- | ----------------------- | ------- | -------------------------------------------------------------- |
| `index`         | `uint32_t`              | —       | Index of this choice in the choices array.                     |
| `delta`         | `LiterllmStreamDelta`   | —       | Incremental update to the message (content, tool calls, etc.). |
| `finish_reason` | `LiterllmFinishReason*` | `NULL`  | Why the stream ended (present only in final chunk).            |

---

#### LiterllmStreamDelta

Incremental delta in a stream chunk.

| Field           | Type                          | Default | Description                                                            |
| --------------- | ----------------------------- | ------- | ---------------------------------------------------------------------- |
| `role`          | `const char**`                | `NULL`  | Role (typically present only in the first chunk).                      |
| `content`       | `const char**`                | `NULL`  | Partial content chunk (e.g., a few words of the response).             |
| `tool_calls`    | `LiterllmStreamToolCall**`    | `NULL`  | Partial tool calls being streamed.                                     |
| `function_call` | `LiterllmStreamFunctionCall*` | `NULL`  | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal`       | `const char**`                | `NULL`  | Partial refusal message.                                               |

---

#### LiterllmStreamFunctionCall

Partial function call details in a stream.

| Field       | Type           | Default | Description                                   |
| ----------- | -------------- | ------- | --------------------------------------------- |
| `name`      | `const char**` | `NULL`  | Function name (typically in the first chunk). |
| `arguments` | `const char**` | `NULL`  | Partial JSON arguments chunk.                 |

---

#### LiterllmStreamOptions

Options for streaming responses.

| Field           | Type    | Default | Description                                             |
| --------------- | ------- | ------- | ------------------------------------------------------- |
| `include_usage` | `bool*` | `NULL`  | If true, include token usage in the final stream chunk. |

---

#### LiterllmStreamToolCall

A streaming tool call being built incrementally.

| Field       | Type                          | Default | Description                                                |
| ----------- | ----------------------------- | ------- | ---------------------------------------------------------- |
| `index`     | `uint32_t`                    | —       | Index of this tool call in the tool_calls array.           |
| `id`        | `const char**`                | `NULL`  | Tool call ID (typically in the first chunk for this call). |
| `call_type` | `LiterllmToolType*`           | `NULL`  | Tool type (typically "function").                          |
| `function`  | `LiterllmStreamFunctionCall*` | `NULL`  | Partial function name and arguments.                       |

---

#### LiterllmSystemMessage

System message guiding model behavior for the entire conversation.

| Field     | Type           | Default | Description                                                     |
| --------- | -------------- | ------- | --------------------------------------------------------------- |
| `content` | `const char*`  | —       | Instructions or context that apply throughout the conversation. |
| `name`    | `const char**` | `NULL`  | Optional name for the system message source.                    |

---

#### LiterllmToolCall

A tool call the model wants to execute.

| Field       | Type                   | Default | Description                                                         |
| ----------- | ---------------------- | ------- | ------------------------------------------------------------------- |
| `id`        | `const char*`          | —       | Unique ID for this call, used to reference in tool result messages. |
| `call_type` | `LiterllmToolType`     | —       | Tool type (always "function").                                      |
| `function`  | `LiterllmFunctionCall` | —       | Function name and arguments.                                        |

---

#### LiterllmToolMessage

Tool execution result returned to the model.

| Field          | Type           | Default | Description                                  |
| -------------- | -------------- | ------- | -------------------------------------------- |
| `content`      | `const char*`  | —       | Result of the tool execution.                |
| `tool_call_id` | `const char*`  | —       | ID of the tool call this result responds to. |
| `name`         | `const char**` | `NULL`  | Optional tool/function name.                 |

---

#### LiterllmTranscriptionResponse

Response from a transcription request.

| Field      | Type                             | Default | Description                                                                  |
| ---------- | -------------------------------- | ------- | ---------------------------------------------------------------------------- |
| `text`     | `const char*`                    | —       | The transcribed text.                                                        |
| `language` | `const char**`                   | `NULL`  | Detected language (ISO-639-1 code).                                          |
| `duration` | `double*`                        | `NULL`  | Total audio duration in seconds.                                             |
| `segments` | `LiterllmTranscriptionSegment**` | `NULL`  | Detailed segment-level transcription (if response_format is "verbose_json"). |

---

#### LiterllmTranscriptionSegment

A segment of transcribed audio with timing information.

| Field   | Type          | Default | Description                        |
| ------- | ------------- | ------- | ---------------------------------- |
| `id`    | `uint32_t`    | —       | Segment index (0-based).           |
| `start` | `double`      | —       | Start time in seconds.             |
| `end`   | `double`      | —       | End time in seconds.               |
| `text`  | `const char*` | —       | Transcribed text for this segment. |

---

#### LiterllmUsage

Token-usage accounting returned by the provider on each completion / embedding call.

| Field                   | Type                           | Default | Description                                                                                                                                                                         |
| ----------------------- | ------------------------------ | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `prompt_tokens`         | `uint64_t`                     | —       | Prompt tokens used. Defaults to 0 when absent (some providers omit this).                                                                                                           |
| `completion_tokens`     | `uint64_t`                     | —       | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses).                                                                                                       |
| `total_tokens`          | `uint64_t`                     | —       | Total tokens used. Defaults to 0 when absent (some providers omit this).                                                                                                            |
| `prompt_tokens_details` | `LiterllmPromptTokensDetails*` | `NULL`  | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### LiterllmUserMessage

User message in the conversation.

| Field     | Type                  | Default                  | Description                                                                               |
| --------- | --------------------- | ------------------------ | ----------------------------------------------------------------------------------------- |
| `content` | `LiterllmUserContent` | `LITERLLM_LITERLLM_TEXT` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `name`    | `const char**`        | `NULL`                   | Optional name for the user.                                                               |

---

### Enums

#### LiterllmMessage

A chat message in a conversation.

| Value                | Description                                                                                                       |
| -------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `LITERLLM_SYSTEM`    | System — Fields: `0`: `LiterllmSystemMessage`                                                                     |
| `LITERLLM_USER`      | User — Fields: `0`: `LiterllmUserMessage`                                                                         |
| `LITERLLM_ASSISTANT` | Assistant — Fields: `0`: `LiterllmAssistantMessage`                                                               |
| `LITERLLM_TOOL`      | Tool — Fields: `0`: `LiterllmToolMessage`                                                                         |
| `LITERLLM_DEVELOPER` | Developer — Fields: `0`: `LiterllmDeveloperMessage`                                                               |
| `LITERLLM_FUNCTION`  | Deprecated legacy function-role message; retained for API compatibility. — Fields: `0`: `LiterllmFunctionMessage` |

---

#### LiterllmUserContent

User message content as either plain text or a list of multimodal parts.

| Value            | Description                                                                                    |
| ---------------- | ---------------------------------------------------------------------------------------------- |
| `LITERLLM_TEXT`  | Plain text content. — Fields: `0`: `const char*`                                               |
| `LITERLLM_PARTS` | Array of content parts (text, images, documents, audio). — Fields: `0`: `LiterllmContentPart*` |

---

#### LiterllmContentPart

A single content part in a user message — text, image, document, or audio.

| Value                  | Description                                                                                      |
| ---------------------- | ------------------------------------------------------------------------------------------------ |
| `LITERLLM_TEXT`        | Plain text. — Fields: `text`: `const char*`                                                      |
| `LITERLLM_IMAGE_URL`   | Image identified by URL (with optional detail level). — Fields: `image_url`: `LiterllmImageUrl`  |
| `LITERLLM_DOCUMENT`    | Document file (PDF, CSV, etc.) as base64 or URL. — Fields: `document`: `LiterllmDocumentContent` |
| `LITERLLM_INPUT_AUDIO` | Audio input as base64. — Fields: `input_audio`: `LiterllmAudioContent`                           |

---

#### LiterllmImageDetail

Image detail level controlling token cost and processing.

| Value           | Description                                                        |
| --------------- | ------------------------------------------------------------------ |
| `LITERLLM_LOW`  | Low detail: scales image to 512x512, uses fewer tokens.            |
| `LITERLLM_HIGH` | High detail: processes up to 2x2 grid of tiles, higher token cost. |
| `LITERLLM_AUTO` | Auto: model chooses low or high based on image dimensions.         |

---

#### LiterllmToolType

The type discriminator for tool/tool-call objects.

Per the OpenAI spec this is always `"function"`. Using an enum enforces
that constraint at the type level and rejects any other value on
deserialization.

| Value               | Description |
| ------------------- | ----------- |
| `LITERLLM_FUNCTION` | Function    |

---

#### LiterllmToolChoice

Tool usage mode or a specific tool to call.

| Value               | Description                                                                       |
| ------------------- | --------------------------------------------------------------------------------- |
| `LITERLLM_MODE`     | Predefined mode: auto, required, or none. — Fields: `0`: `LiterllmToolChoiceMode` |
| `LITERLLM_SPECIFIC` | Force a specific tool to be called. — Fields: `0`: `LiterllmSpecificToolChoice`   |

---

#### LiterllmToolChoiceMode

Tool choice mode.

| Value               | Description                                        |
| ------------------- | -------------------------------------------------- |
| `LITERLLM_AUTO`     | Model may or may not call tools; default behavior. |
| `LITERLLM_REQUIRED` | Model must call at least one tool.                 |
| `LITERLLM_NONE`     | Model must not call any tools.                     |

---

#### LiterllmResponseFormat

Response format constraint.

| Value                  | Description                                                                                           |
| ---------------------- | ----------------------------------------------------------------------------------------------------- |
| `LITERLLM_TEXT`        | Plain text output (default).                                                                          |
| `LITERLLM_JSON_OBJECT` | Output must be valid JSON object (no schema validation).                                              |
| `LITERLLM_JSON_SCHEMA` | Output must conform to the specified JSON schema. — Fields: `json_schema`: `LiterllmJsonSchemaFormat` |

---

#### LiterllmStopSequence

Stop sequence(s) that cause the model to stop generating.

| Value               | Description                                            |
| ------------------- | ------------------------------------------------------ |
| `LITERLLM_SINGLE`   | Single stop sequence. — Fields: `0`: `const char*`     |
| `LITERLLM_MULTIPLE` | Multiple stop sequences. — Fields: `0`: `const char**` |

---

#### LiterllmFinishReason

Why a choice stopped generating tokens.

| Value                     | Description                                                                                                                                                                                                                                                                                                                                                                              |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LITERLLM_STOP`           | Stop                                                                                                                                                                                                                                                                                                                                                                                     |
| `LITERLLM_LENGTH`         | Length                                                                                                                                                                                                                                                                                                                                                                                   |
| `LITERLLM_TOOL_CALLS`     | Tool calls                                                                                                                                                                                                                                                                                                                                                                               |
| `LITERLLM_CONTENT_FILTER` | Content filter                                                                                                                                                                                                                                                                                                                                                                           |
| `LITERLLM_FUNCTION_CALL`  | Deprecated legacy finish reason; retained for API compatibility.                                                                                                                                                                                                                                                                                                                         |
| `LITERLLM_OTHER`          | Catch-all for unknown finish reasons returned by non-OpenAI providers. Note: this intentionally does **not** carry the original string (e.g. `Other(String)`). Using `#[serde(other)]` requires a unit variant, and switching to `#[serde(untagged)]` would change deserialization semantics for all variants. The original value can be recovered by inspecting the raw JSON if needed. |

---

#### LiterllmReasoningEffort

Controls how much reasoning effort the model should use.

| Value             | Description |
| ----------------- | ----------- |
| `LITERLLM_LOW`    | Low         |
| `LITERLLM_MEDIUM` | Medium      |
| `LITERLLM_HIGH`   | High        |

---

#### LiterllmEmbeddingFormat

The format in which the embedding vectors are returned.

| Value             | Description                                         |
| ----------------- | --------------------------------------------------- |
| `LITERLLM_FLOAT`  | 32-bit floating-point numbers (default).            |
| `LITERLLM_BASE64` | Base64-encoded string representation of the floats. |

---

#### LiterllmEmbeddingInput

Text or texts to embed.

| Value               | Description                                                            |
| ------------------- | ---------------------------------------------------------------------- |
| `LITERLLM_SINGLE`   | Single text string. — Fields: `0`: `const char*`                       |
| `LITERLLM_MULTIPLE` | Multiple text strings (batch embedding). — Fields: `0`: `const char**` |

---

#### LiterllmModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value               | Description                                                             |
| ------------------- | ----------------------------------------------------------------------- |
| `LITERLLM_SINGLE`   | Single text string. — Fields: `0`: `const char*`                        |
| `LITERLLM_MULTIPLE` | Multiple text strings (batch moderation). — Fields: `0`: `const char**` |

---

#### LiterllmRerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value             | Description                                                                               |
| ----------------- | ----------------------------------------------------------------------------------------- |
| `LITERLLM_TEXT`   | Plain text document content. — Fields: `0`: `const char*`                                 |
| `LITERLLM_OBJECT` | Document with explicit text field (may include metadata). — Fields: `text`: `const char*` |

---

#### LiterllmOcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value             | Description                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------- |
| `LITERLLM_URL`    | A publicly accessible document URL. — Fields: `url`: `const char*`                                |
| `LITERLLM_BASE64` | Inline base64-encoded document data. — Fields: `data`: `const char*`, `media_type`: `const char*` |

---

#### LiterllmFilePurpose

Purpose of an uploaded file.

| Value                 | Description                       |
| --------------------- | --------------------------------- |
| `LITERLLM_ASSISTANTS` | File for use with Assistants API. |
| `LITERLLM_BATCH`      | File for batch processing.        |
| `LITERLLM_FINE_TUNE`  | File for fine-tuning.             |
| `LITERLLM_VISION`     | File for vision/image tasks.      |

---

#### LiterllmBatchStatus

Status of a batch job.

| Value                  | Description                    |
| ---------------------- | ------------------------------ |
| `LITERLLM_VALIDATING`  | Validating the input file.     |
| `LITERLLM_FAILED`      | Job failed.                    |
| `LITERLLM_IN_PROGRESS` | Job is running.                |
| `LITERLLM_FINALIZING`  | Finalizing results.            |
| `LITERLLM_COMPLETED`   | Job completed successfully.    |
| `LITERLLM_EXPIRED`     | Job expired before completion. |
| `LITERLLM_CANCELLING`  | Job is being cancelled.        |
| `LITERLLM_CANCELLED`   | Job has been cancelled.        |

---

#### LiterllmAuthHeaderFormat

How the API key is sent in the HTTP request.

| Value              | Description                                                          |
| ------------------ | -------------------------------------------------------------------- |
| `LITERLLM_BEARER`  | Bearer token: `Authorization: Bearer <key>`                          |
| `LITERLLM_API_KEY` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `const char*` |
| `LITERLLM_NONE`    | No authentication required.                                          |

---

#### LiterllmAuthType

Auth scheme used by a provider.

| Value              | Description                                                                    |
| ------------------ | ------------------------------------------------------------------------------ |
| `LITERLLM_BEARER`  | Standard `Authorization: Bearer <key>` header.                                 |
| `LITERLLM_API_KEY` | `x-api-key: <key>` header (also handles `"header"` and `"x-api-key"` aliases). |
| `LITERLLM_NONE`    | No authentication header required.                                             |
| `LITERLLM_UNKNOWN` | Unrecognised auth scheme — falls back to bearer.                               |

---

#### LiterllmEnforcement

How budget limits are enforced.

| Value           | Description                                                                       |
| --------------- | --------------------------------------------------------------------------------- |
| `LITERLLM_HARD` | Reject requests that would exceed the budget with `LiterLlmError.BudgetExceeded`. |
| `LITERLLM_SOFT` | Allow requests through but emit a `tracing.warn!` when the budget is exceeded.    |

---

#### LiterllmCacheBackend

Storage backend for the response cache.

| Value               | Description                                                                                                                          |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `LITERLLM_MEMORY`   | In-memory LRU cache (default). No external dependencies.                                                                             |
| `LITERLLM_OPEN_DAL` | OpenDAL-backed storage. Supports 40+ backends (S3, Redis, GCS, local FS, etc.). — Fields: `scheme`: `const char*`, `config`: `void*` |

---

### Errors

#### LiterllmLiterLlmError

All errors that can occur when using `liter-llm`.

| Variant                            | Description                                                                                                                                                                                                                                                                                                                                                      |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LITERLLM_AUTHENTICATION`          | `status` preserves the exact HTTP status code received (401 or 403).                                                                                                                                                                                                                                                                                             |
| `LITERLLM_RATE_LIMITED`            | rate limited: {message}                                                                                                                                                                                                                                                                                                                                          |
| `LITERLLM_BAD_REQUEST`             | `status` preserves the exact HTTP status code received (400, 405, 413, 422, …).                                                                                                                                                                                                                                                                                  |
| `LITERLLM_CONTEXT_WINDOW_EXCEEDED` | context window exceeded: {message}                                                                                                                                                                                                                                                                                                                               |
| `LITERLLM_CONTENT_POLICY`          | content policy violation: {message}                                                                                                                                                                                                                                                                                                                              |
| `LITERLLM_NOT_FOUND`               | not found: {message}                                                                                                                                                                                                                                                                                                                                             |
| `LITERLLM_SERVER_ERROR`            | `status` preserves the exact HTTP status code received (500, or other 5xx not covered by `ServiceUnavailable`).                                                                                                                                                                                                                                                  |
| `LITERLLM_SERVICE_UNAVAILABLE`     | `status` preserves the exact HTTP status code received (502, 503, or 504).                                                                                                                                                                                                                                                                                       |
| `LITERLLM_TIMEOUT`                 | request timeout                                                                                                                                                                                                                                                                                                                                                  |
| `LITERLLM_STREAMING`               | A catch-all for errors that occur during streaming response processing. This variant covers multiple sub-conditions including UTF-8 decoding failures, CRC/checksum mismatches (AWS EventStream), JSON parse errors in individual SSE chunks, and buffer overflow conditions. The `message` field contains a human-readable description of the specific failure. |
| `LITERLLM_ENDPOINT_NOT_SUPPORTED`  | provider {provider} does not support {endpoint}                                                                                                                                                                                                                                                                                                                  |
| `LITERLLM_INVALID_HEADER`          | invalid header {name:?}: {reason}                                                                                                                                                                                                                                                                                                                                |
| `LITERLLM_SERIALIZATION`           | serialization error: {0}                                                                                                                                                                                                                                                                                                                                         |
| `LITERLLM_BUDGET_EXCEEDED`         | budget exceeded: {message}                                                                                                                                                                                                                                                                                                                                       |
| `LITERLLM_HOOK_REJECTED`           | hook rejected: {message}                                                                                                                                                                                                                                                                                                                                         |
| `LITERLLM_INTERNAL_ERROR`          | An internal logic error (e.g. unexpected Tower response variant). This should never surface in normal operation — if it does, it indicates a bug in the library.                                                                                                                                                                                                 |

---
