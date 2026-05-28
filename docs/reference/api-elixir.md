---
title: "Elixir API Reference"
---

## Elixir API Reference <span class="version-badge">v1.4.0-rc.42</span>

### Functions

#### create_client()

Create a new LLM client with simple scalar configuration.

This is the primary binding entry-point. All parameters except `api_key`
are optional — omitting them uses the same defaults as
`ClientConfigBuilder`.

**Errors:**

Returns `LiterLlmError` if the underlying HTTP client cannot be
constructed, or if the resolved provider configuration is invalid.

**Signature:**

```elixir
@spec create_client(api_key, base_url, timeout_secs, max_retries, model_hint) :: {:ok, term()} | {:error, term()}
def create_client(api_key, base_url, timeout_secs, max_retries, model_hint)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `api_key` | `String.t()` | Yes | The api key |
| `base_url` | `String.t() \| nil` | No | The base url |
| `timeout_secs` | `integer() \| nil` | No | The timeout secs |
| `max_retries` | `integer() \| nil` | No | The max retries |
| `model_hint` | `String.t() \| nil` | No | The model hint |

**Returns:** `DefaultClient`
**Errors:** Returns `{:error, reason}`

---

#### create_client_from_json()

Create a new LLM client from a JSON string.

The JSON object accepts the same fields as `liter-llm.toml` (snake_case).

**Errors:**

Returns `LiterLlmError.BadRequest` if `json` is not valid JSON or
contains unknown fields.

**Signature:**

```elixir
@spec create_client_from_json(json) :: {:ok, term()} | {:error, term()}
def create_client_from_json(json)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `json` | `String.t()` | Yes | The json |

**Returns:** `DefaultClient`
**Errors:** Returns `{:error, reason}`

---

#### register_custom_provider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```elixir
@spec register_custom_provider(config) :: {:ok, term()} | {:error, term()}
def register_custom_provider(config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `CustomProviderConfig` | Yes | The configuration options |

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### unregister_custom_provider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error only if the internal lock is poisoned.

**Signature:**

```elixir
@spec unregister_custom_provider(name) :: {:ok, term()} | {:error, term()}
def unregister_custom_provider(name)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String.t()` | Yes | The name |

**Returns:** `boolean()`
**Errors:** Returns `{:error, reason}`

---

#### all_providers()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.

**Signature:**

```elixir
@spec all_providers() :: {:ok, term()} | {:error, term()}
def all_providers()
```

**Returns:** `list(ProviderConfig)`
**Errors:** Returns `{:error, reason}`

---

#### complex_provider_names()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```elixir
@spec complex_provider_names() :: {:ok, term()} | {:error, term()}
def complex_provider_names()
```

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### completion_cost()

Calculate the estimated cost of a completion given a model name and token
counts.

Returns `nil` if the model is not present in the embedded pricing registry.
Returns `Some(cost_usd)` otherwise, where the value is in US dollars.

When an exact model name match is not found, progressively shorter prefixes
are tried by stripping from the last `-` or `.` separator. For example,
`gpt-4-0613` will match `gpt-4` if no `gpt-4-0613` entry exists.

**Signature:**

```elixir
@spec completion_cost(model, prompt_tokens, completion_tokens) :: {:ok, term()} | {:error, term()}
def completion_cost(model, prompt_tokens, completion_tokens)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String.t()` | Yes | The model |
| `prompt_tokens` | `integer()` | Yes | The prompt tokens |
| `completion_tokens` | `integer()` | Yes | The completion tokens |

**Returns:** `float() | nil`

---

#### completion_cost_with_cache()

Calculate the estimated cost of a completion, accounting for cached
(cache-hit) prompt tokens billed at the provider's discounted rate.

`cached_tokens` is the count of prompt tokens served from the provider's
prompt cache. It must be `<= prompt_tokens` (cached tokens are a subset of
the prompt). The non-cached portion is billed at `input_cost_per_token`
and the cached portion at `cache_read_input_token_cost` when the model
has cache pricing; otherwise the entire prompt is billed at the regular
input rate.

Returns `nil` if the model is not present in the embedded pricing
registry, mirroring `completion_cost`.

**Signature:**

```elixir
@spec completion_cost_with_cache(model, prompt_tokens, cached_tokens, completion_tokens) :: {:ok, term()} | {:error, term()}
def completion_cost_with_cache(model, prompt_tokens, cached_tokens, completion_tokens)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String.t()` | Yes | The model |
| `prompt_tokens` | `integer()` | Yes | The prompt tokens |
| `cached_tokens` | `integer()` | Yes | The cached tokens |
| `completion_tokens` | `integer()` | Yes | The completion tokens |

**Returns:** `float() | nil`

---

#### count_tokens()

Count tokens in a text string using the tokenizer for the given model.

The tokenizer is resolved from the model name prefix (e.g. `"gpt-4o"` maps
to the `Xenova/gpt-4o` HuggingFace tokenizer). Tokenizers are cached after
first load.

**Errors:**

Returns `LiterLlmError.BadRequest` if the tokenizer cannot be loaded
(e.g. network failure on first use) or if tokenization itself fails.

**Signature:**

```elixir
@spec count_tokens(model, text) :: {:ok, term()} | {:error, term()}
def count_tokens(model, text)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String.t()` | Yes | The model |
| `text` | `String.t()` | Yes | The text |

**Returns:** `integer()`
**Errors:** Returns `{:error, reason}`

---

#### count_request_tokens()

Count tokens for a full `ChatCompletionRequest`.

Sums tokens across all message text contents plus a per-message overhead
of ~4 tokens (for role, separators, and formatting metadata). Tool
definitions and multimodal content parts (images, audio, documents) are
not counted — only textual content contributes to the token total.

**Errors:**

Returns `LiterLlmError.BadRequest` if the tokenizer cannot be loaded or
if tokenization fails for any message.

**Signature:**

```elixir
@spec count_request_tokens(model, req) :: {:ok, term()} | {:error, term()}
def count_request_tokens(model, req)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String.t()` | Yes | The model |
| `req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `integer()`
**Errors:** Returns `{:error, reason}`

---

#### ensure_crypto_provider()

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

```elixir
@spec ensure_crypto_provider() :: {:ok, term()} | {:error, term()}
def ensure_crypto_provider()
```

**Returns:** `:ok`

---

### Types

#### AssistantMessage

Assistant's response to a user message.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t() \| nil` | `nil` | The assistant's text response. Absent if tool calls are returned instead. |
| `name` | `String.t() \| nil` | `nil` | Optional name for the assistant. |
| `tool_calls` | `list(ToolCall) \| nil` | `[]` | Tool calls the model wants to execute, if any. |
| `refusal` | `String.t() \| nil` | `nil` | Refusal reason, if the model declined to respond per safety policies. |
| `function_call` | `FunctionCall \| nil` | `nil` | Deprecated legacy function_call field; retained for API compatibility. |

---

#### AudioContent

Audio content part for speech-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `String.t()` | — | Base64-encoded audio data. |
| `format` | `String.t()` | — | Audio format (e.g., "wav", "mp3", "ogg"). |

---

#### AuthConfig

Auth configuration block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auth_type` | `AuthType` | — | Auth scheme classification. |
| `env_var` | `String.t() \| nil` | `nil` | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### BatchListQuery

Query parameters for listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `integer() \| nil` | `nil` | Maximum number of results to return. Defaults to 20. |
| `after` | `String.t() \| nil` | `nil` | Pagination cursor: return results after this batch ID. |

---

#### BatchListResponse

Response from listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String.t()` | — | Object type (always `"list"`). |
| `data` | `list(BatchObject)` | `[]` | List of batch objects. |
| `has_more` | `boolean() \| nil` | `nil` | Whether more results are available. |
| `first_id` | `String.t() \| nil` | `nil` | First batch ID in the result set (for pagination). |
| `last_id` | `String.t() \| nil` | `nil` | Last batch ID in the result set (for pagination). |

---

#### BatchObject

A batch job object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Unique batch ID. |
| `object` | `String.t()` | — | Object type (always `"batch"`). |
| `endpoint` | `String.t()` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `input_file_id` | `String.t()` | — | ID of the input file. |
| `completion_window` | `String.t()` | — | Completion window (e.g., `"24h"`). |
| `status` | `BatchStatus` | `:validating` | Current job status. |
| `output_file_id` | `String.t() \| nil` | `nil` | ID of the output file (present when completed). |
| `error_file_id` | `String.t() \| nil` | `nil` | ID of the error file (present if some requests failed). |
| `created_at` | `integer()` | — | Unix timestamp of batch creation. |
| `completed_at` | `integer() \| nil` | `nil` | Unix timestamp of completion (if completed). |
| `failed_at` | `integer() \| nil` | `nil` | Unix timestamp of failure (if failed). |
| `expired_at` | `integer() \| nil` | `nil` | Unix timestamp of expiration (if expired). |
| `request_counts` | `BatchRequestCounts \| nil` | `nil` | Request processing counts. |
| `metadata` | `term() \| nil` | `nil` | Metadata attached to the batch. |

---

#### BatchRequestCounts

Request processing counts for a batch.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total` | `integer()` | — | Total requests in the batch. |
| `completed` | `integer()` | — | Completed requests. |
| `failed` | `integer()` | — | Failed requests. |

---

#### BudgetConfig

Configuration for budget enforcement.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `global_limit` | `float() \| nil` | `nil` | Maximum total spend across all models, in USD.  `nil` means unlimited. |
| `model_limits` | `map()` | `%{}` | Per-model spending limits in USD.  Models not listed here are only constrained by `global_limit`. |
| `enforcement` | `Enforcement` | `:hard` | Whether to reject requests or merely warn when a limit is exceeded. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### CacheConfig

Configuration for the response cache.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_entries` | `integer()` | `256` | Maximum number of cached entries. |
| `ttl` | `integer()` | `300000ms` | Time-to-live for each cached entry. |
| `backend` | `CacheBackend` | `:memory` | Storage backend to use. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### ChatCompletionChunk

A streamed chunk of a chat completion response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Unique identifier for this stream. |
| `object` | `String.t()` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created` | `integer()` | — | Unix timestamp of chunk creation. |
| `model` | `String.t()` | — | Model used to generate the chunk. |
| `choices` | `list(StreamChoice)` | `[]` | Streaming choices (delta updates). |
| `usage` | `Usage \| nil` | `nil` | Token usage (typically only in the final chunk). |
| `system_fingerprint` | `String.t() \| nil` | `nil` | Fingerprint of the system configuration (OpenAI-specific). |
| `service_tier` | `String.t() \| nil` | `nil` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`). |
| `messages` | `list(Message)` | `[]` | Conversation history from oldest to newest. |
| `temperature` | `float() \| nil` | `nil` | Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0. |
| `top_p` | `float() \| nil` | `nil` | Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused. |
| `n` | `integer() \| nil` | `nil` | Number of chat completions to generate. Defaults to 1. |
| `stream` | `boolean() \| nil` | `nil` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `stop` | `StopSequence \| nil` | `nil` | Stop sequence(s) that halt token generation. |
| `max_tokens` | `integer() \| nil` | `nil` | Max output tokens. Different from max_completion_tokens in some providers. |
| `presence_penalty` | `float() \| nil` | `nil` | Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics. |
| `frequency_penalty` | `float() \| nil` | `nil` | Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens. |
| `logit_bias` | `map() \| nil` | `%{}` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user` | `String.t() \| nil` | `nil` | User identifier for request tracking and abuse detection. |
| `tools` | `list(ChatCompletionTool) \| nil` | `[]` | Tools the model can invoke. |
| `tool_choice` | `ToolChoice \| nil` | `nil` | Tool usage mode (auto, required, none, or specific tool). |
| `parallel_tool_calls` | `boolean() \| nil` | `nil` | Whether the model can call multiple tools in parallel. Defaults to true. |
| `response_format` | `ResponseFormat \| nil` | `nil` | Output format constraint (text, JSON, JSON schema). |
| `stream_options` | `StreamOptions \| nil` | `nil` | Streaming options (e.g., include_usage). |
| `seed` | `integer() \| nil` | `nil` | Random seed for reproducible outputs. Provider support varies. |
| `reasoning_effort` | `ReasoningEffort \| nil` | `nil` | Reasoning effort level (low, medium, high) for extended-thinking models. |
| `extra_body` | `term() \| nil` | `nil` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

#### ChatCompletionResponse

Chat completion response from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Unique identifier for this response. |
| `object` | `String.t()` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `integer()` | — | Unix timestamp of response creation. |
| `model` | `String.t()` | — | Model used to generate the response. |
| `choices` | `list(Choice)` | `[]` | List of completion choices. |
| `usage` | `Usage \| nil` | `nil` | Token usage statistics. |
| `system_fingerprint` | `String.t() \| nil` | `nil` | Fingerprint of the system configuration (OpenAI-specific). |
| `service_tier` | `String.t() \| nil` | `nil` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionTool

A tool the model can invoke (currently, all tools are functions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tool_type` | `ToolType` | — | Tool type (always "function" in OpenAI spec). |
| `function` | `FunctionDefinition` | — | Function definition with name, description, and JSON schema parameters. |

---

#### Choice

A single completion choice.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `integer()` | — | Index of this choice in the choices array. |
| `message` | `AssistantMessage` | — | The assistant's message response. |
| `finish_reason` | `FinishReason \| nil` | `nil` | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

---

#### CreateBatchRequest

Request to create a batch job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_file_id` | `String.t()` | — | ID of the uploaded input file (JSONL format). |
| `endpoint` | `String.t()` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `completion_window` | `String.t()` | — | Completion window (e.g., `"24h"`). |
| `metadata` | `term() \| nil` | `nil` | Optional metadata to attach to the batch. |

---

#### CreateFileRequest

Request to upload a file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | `String.t()` | — | Base64-encoded file data. |
| `purpose` | `FilePurpose` | `:assistants` | Purpose for the file. |
| `filename` | `String.t() \| nil` | `nil` | Optional filename to associate with the upload. |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt` | `String.t()` | — | Text description of the image to generate. |
| `model` | `String.t() \| nil` | `nil` | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `n` | `integer() \| nil` | `nil` | Number of images to generate. Defaults to 1. |
| `size` | `String.t() \| nil` | `nil` | Image size (e.g., `"1024x1024"`, `"1792x1024"`). |
| `quality` | `String.t() \| nil` | `nil` | Image quality: `"standard"` or `"hd"`. |
| `style` | `String.t() \| nil` | `nil` | Style: `"natural"` or `"vivid"` (DALL-E 3 only). |
| `response_format` | `String.t() \| nil` | `nil` | Response format: `"url"` or `"b64_json"`. |
| `user` | `String.t() \| nil` | `nil` | User identifier for request tracking. |

---

#### CreateResponseRequest

Request to create a structured response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | Model ID. |
| `input` | `term()` | — | Input data to process (e.g., a document to extract from). |
| `instructions` | `String.t() \| nil` | `nil` | Instructions for processing the input. |
| `tools` | `list(ResponseTool) \| nil` | `[]` | Available tools the model can use. |
| `temperature` | `float() \| nil` | `nil` | Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0. |
| `max_output_tokens` | `integer() \| nil` | `nil` | Maximum output tokens. |
| `metadata` | `term() \| nil` | `nil` | Optional metadata. |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`). |
| `input` | `String.t()` | — | Text to synthesize into speech. |
| `voice` | `String.t()` | — | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `response_format` | `String.t() \| nil` | `nil` | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`). |
| `speed` | `float() \| nil` | `nil` | Playback speed in `[0.25, 4.0]`. Defaults to 1.0. |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | Model ID (e.g., `"whisper-1"`). |
| `file` | `String.t()` | — | Base64-encoded audio file data. |
| `language` | `String.t() \| nil` | `nil` | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `prompt` | `String.t() \| nil` | `nil` | Optional text to guide the model (improves accuracy for domain-specific terms). |
| `response_format` | `String.t() \| nil` | `nil` | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`). |
| `temperature` | `float() \| nil` | `nil` | Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0. |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Unique name for this provider (e.g., "my-provider"). |
| `base_url` | `String.t()` | — | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `auth_header` | `AuthHeaderFormat` | — | Authentication header format. |
| `model_prefixes` | `list(String.t())` | — | Model name prefixes that route to this provider (e.g., `["my-"]`). |

---

#### DefaultClient

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

### Functions

#### chat()

**Signature:**

```elixir
def chat(req)
```

#### chat_stream()

**Signature:**

```elixir
def chat_stream(req)
```

#### embed()

**Signature:**

```elixir
def embed(req)
```

#### list_models()

**Signature:**

```elixir
def list_models()
```

#### image_generate()

**Signature:**

```elixir
def image_generate(req)
```

#### speech()

**Signature:**

```elixir
def speech(req)
```

#### transcribe()

**Signature:**

```elixir
def transcribe(req)
```

#### moderate()

**Signature:**

```elixir
def moderate(req)
```

#### rerank()

**Signature:**

```elixir
def rerank(req)
```

#### search()

**Signature:**

```elixir
def search(req)
```

#### ocr()

**Signature:**

```elixir
def ocr(req)
```

#### create_file()

**Signature:**

```elixir
def create_file(req)
```

#### retrieve_file()

**Signature:**

```elixir
def retrieve_file(file_id)
```

#### delete_file()

**Signature:**

```elixir
def delete_file(file_id)
```

#### list_files()

**Signature:**

```elixir
def list_files(query)
```

#### file_content()

**Signature:**

```elixir
def file_content(file_id)
```

#### create_batch()

**Signature:**

```elixir
def create_batch(req)
```

#### retrieve_batch()

**Signature:**

```elixir
def retrieve_batch(batch_id)
```

#### list_batches()

**Signature:**

```elixir
def list_batches(query)
```

#### cancel_batch()

**Signature:**

```elixir
def cancel_batch(batch_id)
```

#### create_response()

**Signature:**

```elixir
def create_response(req)
```

#### retrieve_response()

**Signature:**

```elixir
def retrieve_response(response_id)
```

#### cancel_response()

**Signature:**

```elixir
def cancel_response(response_id)
```

---

#### DeleteResponse

Response from a delete operation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | ID of the deleted resource. |
| `object` | `String.t()` | — | Object type. |
| `deleted` | `boolean()` | — | Confirmation that the resource was deleted. |

---

#### DeveloperMessage

Developer message (system-like message for Claude models).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Developer-specific instructions or context. |
| `name` | `String.t() \| nil` | `nil` | Optional name for the developer message source. |

---

#### DocumentContent

PDF/document content part for vision-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `String.t()` | — | Base64-encoded document data or URL. |
| `media_type` | `String.t()` | — | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### EmbeddingObject

A single embedding vector.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String.t()` | — | Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `embedding` | `list(float())` | — | The embedding vector. |
| `index` | `integer()` | — | Index in the batch (corresponds to input order). |

---

#### EmbeddingRequest

Embedding request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | Model ID (e.g., `"text-embedding-3-small"`). |
| `input` | `EmbeddingInput` | `:single` | Text or texts to embed. |
| `encoding_format` | `EmbeddingFormat \| nil` | `nil` | Output format: float (native) or base64. |
| `dimensions` | `integer() \| nil` | `nil` | Requested embedding dimensions (if supported by the model). |
| `user` | `String.t() \| nil` | `nil` | User identifier for request tracking. |

---

#### EmbeddingResponse

Embedding response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String.t()` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `list(EmbeddingObject)` | — | List of embeddings. |
| `model` | `String.t()` | — | Model used to generate embeddings. |
| `usage` | `Usage \| nil` | `/* serde(default) */` | Token usage (input tokens only; embeddings have zero output tokens). |

---

#### FileListQuery

Query parameters for listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `purpose` | `String.t() \| nil` | `nil` | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `limit` | `integer() \| nil` | `nil` | Maximum number of results to return. Defaults to 20. |
| `after` | `String.t() \| nil` | `nil` | Pagination cursor: return results after this file ID. |

---

#### FileListResponse

Response from listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String.t()` | — | Object type (always `"list"`). |
| `data` | `list(FileObject)` | `[]` | List of file objects. |
| `has_more` | `boolean() \| nil` | `nil` | Whether more results are available. |

---

#### FileObject

An uploaded file object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Unique file ID. |
| `object` | `String.t()` | — | Object type (always `"file"`). |
| `bytes` | `integer()` | — | File size in bytes. |
| `created_at` | `integer()` | — | Unix timestamp of file creation. |
| `filename` | `String.t()` | — | Filename. |
| `purpose` | `String.t()` | — | File purpose. |
| `status` | `String.t() \| nil` | `nil` | Processing status (e.g., `"uploaded"`, `"processed"`). |

---

#### FunctionCall

Function call details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Function name. |
| `arguments` | `String.t()` | — | Arguments as a JSON string (parse with serde_json.from_str). |

---

#### FunctionDefinition

Function definition exposed to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Name of the function. Required and must be alphanumeric + underscores. |
| `description` | `String.t() \| nil` | `/* serde(default) */` | Human-readable description explaining what the function does. |
| `parameters` | `term() \| nil` | `/* serde(default) */` | JSON Schema defining the function's parameters. |
| `strict` | `boolean() \| nil` | `/* serde(default) */` | If true, enforce strict JSON schema validation for arguments. |

---

#### FunctionMessage

Deprecated legacy function-role message body.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | The extracted text content |
| `name` | `String.t()` | — | The name |

---

#### Image

A single generated image, returned as either a URL or base64 data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String.t() \| nil` | `nil` | Image URL (if response_format was "url"). |
| `b64_json` | `String.t() \| nil` | `nil` | Base64-encoded image data (if response_format was "b64_json"). |
| `revised_prompt` | `String.t() \| nil` | `nil` | The final prompt used to generate the image (DALL-E 3). |

---

#### ImageUrl

An image URL reference with optional detail level for processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String.t()` | — | URL of the image (data URI or HTTP/HTTPS URL). |
| `detail` | `ImageDetail \| nil` | `nil` | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### ImagesResponse

Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `created` | `integer()` | — | Unix timestamp of image creation. |
| `data` | `list(Image)` | `[]` | List of generated images. |

---

#### JsonSchemaFormat

JSON Schema specification for constrained output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Name of the schema (must be unique in the request). |
| `description` | `String.t() \| nil` | `nil` | Description of what the schema represents. |
| `schema` | `term()` | — | JSON Schema object defining the output structure. |
| `strict` | `boolean() \| nil` | `nil` | If true, enforce strict schema validation. |

---

#### ModelObject

A model available from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`). |
| `object` | `String.t()` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `integer()` | — | Unix timestamp of model creation (or release date). |
| `owned_by` | `String.t()` | — | Organization or entity that owns the model. |

---

#### ModelsListResponse

Response listing available models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String.t()` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `list(ModelObject)` | `[]` | List of available models. |

---

#### ModerationCategories

Boolean flags for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `boolean()` | — | Sexual content. |
| `hate` | `boolean()` | — | Hate speech. |
| `harassment` | `boolean()` | — | Harassment. |
| `self_harm` | `boolean()` | — | Self-harm content. |
| `sexual_minors` | `boolean()` | — | Sexual content involving minors. |
| `hate_threatening` | `boolean()` | — | Hate speech that threatens violence. |
| `violence_graphic` | `boolean()` | — | Graphic violence. |
| `self_harm_intent` | `boolean()` | — | Intent to self-harm. |
| `self_harm_instructions` | `boolean()` | — | Instructions for self-harm. |
| `harassment_threatening` | `boolean()` | — | Harassment that threatens violence. |
| `violence` | `boolean()` | — | Non-graphic violence. |

---

#### ModerationCategoryScores

Confidence scores for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `float()` | — | Sexual content score. |
| `hate` | `float()` | — | Hate speech score. |
| `harassment` | `float()` | — | Harassment score. |
| `self_harm` | `float()` | — | Self-harm content score. |
| `sexual_minors` | `float()` | — | Sexual content involving minors score. |
| `hate_threatening` | `float()` | — | Hate speech that threatens violence score. |
| `violence_graphic` | `float()` | — | Graphic violence score. |
| `self_harm_intent` | `float()` | — | Intent to self-harm score. |
| `self_harm_instructions` | `float()` | — | Instructions for self-harm score. |
| `harassment_threatening` | `float()` | — | Harassment that threatens violence score. |
| `violence` | `float()` | — | Non-graphic violence score. |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input` | `ModerationInput` | `:single` | Text or texts to check. |
| `model` | `String.t() \| nil` | `nil` | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Unique identifier for this moderation request. |
| `model` | `String.t()` | — | Model used for classification. |
| `results` | `list(ModerationResult)` | — | Results for each input string. |

---

#### ModerationResult

A single moderation classification result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `flagged` | `boolean()` | — | True if any category was flagged. |
| `categories` | `ModerationCategories` | — | Boolean flags for each moderation category. |
| `category_scores` | `ModerationCategoryScores` | — | Confidence scores for each category. |

---

#### OcrImage

An image extracted from an OCR page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Unique image identifier within the document. |
| `image_base64` | `String.t() \| nil` | `/* serde(default) */` | Base64-encoded image data (if `include_image_base64` was true). |

---

#### OcrPage

A single page of OCR output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `integer()` | — | Page index (0-based). |
| `markdown` | `String.t()` | — | Extracted page content as Markdown. |
| `images` | `list(OcrImage) \| nil` | `/* serde(default) */` | Embedded images extracted from the page (if `include_image_base64` was true). |
| `dimensions` | `PageDimensions \| nil` | `/* serde(default) */` | Page dimensions in pixels, if available. |

---

#### OcrRequest

An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document` | `OcrDocument` | `:url` | The document to process (URL or base64). |
| `pages` | `list(integer()) \| nil` | `[]` | Specific pages to process (1-indexed). `nil` means all pages. |
| `include_image_base64` | `boolean() \| nil` | `nil` | Whether to include base64-encoded images of each processed page. |

---

#### OcrResponse

An OCR response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pages` | `list(OcrPage)` | — | Extracted pages in order. |
| `model` | `String.t()` | — | Model/provider used for OCR. |
| `usage` | `Usage \| nil` | `/* serde(default) */` | Token usage, if reported by the provider. |

---

#### PageDimensions

Page dimensions in pixels.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `integer()` | — | Width in pixels. |
| `height` | `integer()` | — | Height in pixels. |

---

#### PromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage.prompt_tokens` — it is *not* an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cached_tokens` | `integer()` | — | Cached tokens present in the prompt. Defaults to 0 when absent. |
| `audio_tokens` | `integer()` | — | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### ProviderConfig

Static configuration for a single provider entry in providers.json.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Provider identifier (matches the entry key in providers.json). |
| `display_name` | `String.t() \| nil` | `nil` | Human-readable provider name shown in UIs. |
| `base_url` | `String.t() \| nil` | `nil` | Base URL used as the default for this provider's HTTP client. |
| `auth` | `AuthConfig \| nil` | `nil` | Authentication scheme metadata (auth type + env var holding the key). |
| `endpoints` | `list(String.t()) \| nil` | `nil` | Supported endpoint kinds (e.g. `chat`, `embeddings`). |
| `model_prefixes` | `list(String.t()) \| nil` | `nil` | Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`). |
| `param_mappings` | `map() \| nil` | `nil` | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`).  Applied automatically by `ConfigDrivenProvider.transform_request`. |

---

#### RateLimitConfig

Configuration for per-model rate limits.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rpm` | `integer() \| nil` | `nil` | Maximum requests per window.  `nil` means unlimited. |
| `tpm` | `integer() \| nil` | `nil` | Maximum tokens per window.  `nil` means unlimited. |
| `window` | `integer()` | `60000ms` | Fixed window duration (defaults to 60 s). |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | Model ID (e.g., `"cohere/rerank-english-v3.0"`). |
| `query` | `String.t()` | — | The search query. |
| `documents` | `list(RerankDocument)` | `[]` | Documents to rerank. |
| `top_n` | `integer() \| nil` | `nil` | Return only the top N results. Optional. |
| `return_documents` | `boolean() \| nil` | `nil` | Include the document content in results. Defaults to false. |

---

#### RerankResponse

Response from the rerank endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t() \| nil` | `nil` | Unique identifier for this rerank request. |
| `results` | `list(RerankResult)` | — | Reranked documents in order of relevance. |
| `meta` | `term() \| nil` | `/* serde(default) */` | Optional metadata about the reranking operation. |

---

#### RerankResult

A single reranked document with its relevance score.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `integer()` | — | Original document index in the input list. |
| `relevance_score` | `float()` | — | Relevance score in `[0, 1]`. Higher indicates more relevant. |
| `document` | `RerankResultDocument \| nil` | `/* serde(default) */` | Original document content (if `return_documents` was true). |

---

#### RerankResultDocument

The text content of a reranked document, returned when `return_documents` is true.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String.t()` | — | Document text. |

---

#### ResponseObject

Response from a structured response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Unique response ID. |
| `object` | `String.t()` | — | Object type (e.g., `"response"`). |
| `created_at` | `integer()` | — | Unix timestamp of response creation. |
| `model` | `String.t()` | — | Model used to generate the response. |
| `status` | `String.t()` | — | Status (e.g., `"succeeded"`, `"failed"`). |
| `output` | `list(ResponseOutputItem)` | `[]` | Output items from the response. |
| `usage` | `ResponseUsage \| nil` | `nil` | Token usage. |
| `error` | `term() \| nil` | `nil` | Error details (if status is "failed"). |

---

#### ResponseOutputItem

A single output item from the response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `item_type` | `String.t()` | — | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `content` | `term()` | — | Output content (flattened into the object). |

---

#### ResponseTool

A tool available for the response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tool_type` | `String.t()` | — | Tool type (e.g., "extractor", "search"). |
| `config` | `term()` | — | Tool configuration (flattened into the object). |

---

#### ResponseUsage

Token usage for a response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_tokens` | `integer()` | — | Input tokens used. |
| `output_tokens` | `integer()` | — | Output tokens used. |
| `total_tokens` | `integer()` | — | Total tokens used. |

---

#### SearchRequest

A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query` | `String.t()` | — | The search query string. |
| `max_results` | `integer() \| nil` | `nil` | Maximum number of results to return. |
| `search_domain_filter` | `list(String.t()) \| nil` | `[]` | Domain filter — restrict results to specific domains. |
| `country` | `String.t() \| nil` | `nil` | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

---

#### SearchResponse

A search response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `results` | `list(SearchResult)` | — | List of search results. |
| `model` | `String.t()` | — | Model/provider that performed the search. |

---

#### SearchResult

An individual search result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String.t()` | — | Result title. |
| `url` | `String.t()` | — | Result URL. |
| `snippet` | `String.t()` | — | Text snippet or excerpt from the page. |
| `date` | `String.t() \| nil` | `/* serde(default) */` | Publication or last-updated date, if available. |

---

#### SpecificFunction

Name of the specific function to invoke.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Function name. |

---

#### SpecificToolChoice

Directive to call a specific tool.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `choice_type` | `ToolType` | `:function` | Tool type (always "function"). |
| `function` | `SpecificFunction` | — | The specific function to invoke. |

---

#### StreamChoice

A streaming choice with incremental delta.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `integer()` | — | Index of this choice in the choices array. |
| `delta` | `StreamDelta` | — | Incremental update to the message (content, tool calls, etc.). |
| `finish_reason` | `FinishReason \| nil` | `nil` | Why the stream ended (present only in final chunk). |

---

#### StreamDelta

Incremental delta in a stream chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `role` | `String.t() \| nil` | `nil` | Role (typically present only in the first chunk). |
| `content` | `String.t() \| nil` | `nil` | Partial content chunk (e.g., a few words of the response). |
| `tool_calls` | `list(StreamToolCall) \| nil` | `[]` | Partial tool calls being streamed. |
| `function_call` | `StreamFunctionCall \| nil` | `nil` | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal` | `String.t() \| nil` | `nil` | Partial refusal message. |

---

#### StreamFunctionCall

Partial function call details in a stream.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t() \| nil` | `nil` | Function name (typically in the first chunk). |
| `arguments` | `String.t() \| nil` | `nil` | Partial JSON arguments chunk. |

---

#### StreamOptions

Options for streaming responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_usage` | `boolean() \| nil` | `nil` | If true, include token usage in the final stream chunk. |

---

#### StreamToolCall

A streaming tool call being built incrementally.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `integer()` | — | Index of this tool call in the tool_calls array. |
| `id` | `String.t() \| nil` | `nil` | Tool call ID (typically in the first chunk for this call). |
| `call_type` | `ToolType \| nil` | `nil` | Tool type (typically "function"). |
| `function` | `StreamFunctionCall \| nil` | `nil` | Partial function name and arguments. |

---

#### SystemMessage

System message guiding model behavior for the entire conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Instructions or context that apply throughout the conversation. |
| `name` | `String.t() \| nil` | `nil` | Optional name for the system message source. |

---

#### ToolCall

A tool call the model wants to execute.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Unique ID for this call, used to reference in tool result messages. |
| `call_type` | `ToolType` | — | Tool type (always "function"). |
| `function` | `FunctionCall` | — | Function name and arguments. |

---

#### ToolMessage

Tool execution result returned to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Result of the tool execution. |
| `tool_call_id` | `String.t()` | — | ID of the tool call this result responds to. |
| `name` | `String.t() \| nil` | `nil` | Optional tool/function name. |

---

#### TranscriptionResponse

Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String.t()` | — | The transcribed text. |
| `language` | `String.t() \| nil` | `nil` | Detected language (ISO-639-1 code). |
| `duration` | `float() \| nil` | `nil` | Total audio duration in seconds. |
| `segments` | `list(TranscriptionSegment) \| nil` | `[]` | Detailed segment-level transcription (if response_format is "verbose_json"). |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `integer()` | — | Segment index (0-based). |
| `start` | `float()` | — | Start time in seconds. |
| `end` | `float()` | — | End time in seconds. |
| `text` | `String.t()` | — | Transcribed text for this segment. |

---

#### Usage

Token-usage accounting returned by the provider on each completion / embedding call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_tokens` | `integer()` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `completion_tokens` | `integer()` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `total_tokens` | `integer()` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `prompt_tokens_details` | `PromptTokensDetails \| nil` | `nil` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

User message in the conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `:text` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `name` | `String.t() \| nil` | `nil` | Optional name for the user. |

---

### Enums

#### Message

A chat message in a conversation.

| Value | Description |
|-------|-------------|
| `system` | System — Fields: `0`: `SystemMessage` |
| `user` | User — Fields: `0`: `UserMessage` |
| `assistant` | Assistant — Fields: `0`: `AssistantMessage` |
| `tool` | Tool — Fields: `0`: `ToolMessage` |
| `developer` | Developer — Fields: `0`: `DeveloperMessage` |
| `function` | Deprecated legacy function-role message; retained for API compatibility. — Fields: `0`: `FunctionMessage` |

---

#### UserContent

User message content as either plain text or a list of multimodal parts.

| Value | Description |
|-------|-------------|
| `text` | Plain text content. — Fields: `0`: `String.t()` |
| `parts` | Array of content parts (text, images, documents, audio). — Fields: `0`: `list(ContentPart)` |

---

#### ContentPart

A single content part in a user message — text, image, document, or audio.

| Value | Description |
|-------|-------------|
| `text` | Plain text. — Fields: `text`: `String.t()` |
| `image_url` | Image identified by URL (with optional detail level). — Fields: `image_url`: `ImageUrl` |
| `document` | Document file (PDF, CSV, etc.) as base64 or URL. — Fields: `document`: `DocumentContent` |
| `input_audio` | Audio input as base64. — Fields: `input_audio`: `AudioContent` |

---

#### ImageDetail

Image detail level controlling token cost and processing.

| Value | Description |
|-------|-------------|
| `low` | Low detail: scales image to 512x512, uses fewer tokens. |
| `high` | High detail: processes up to 2x2 grid of tiles, higher token cost. |
| `auto` | Auto: model chooses low or high based on image dimensions. |

---

#### ToolType

The type discriminator for tool/tool-call objects.

Per the OpenAI spec this is always `"function"`. Using an enum enforces
that constraint at the type level and rejects any other value on
deserialization.

| Value | Description |
|-------|-------------|
| `function` | Function |

---

#### ToolChoice

Tool usage mode or a specific tool to call.

| Value | Description |
|-------|-------------|
| `mode` | Predefined mode: auto, required, or none. — Fields: `0`: `ToolChoiceMode` |
| `specific` | Force a specific tool to be called. — Fields: `0`: `SpecificToolChoice` |

---

#### ToolChoiceMode

Tool choice mode.

| Value | Description |
|-------|-------------|
| `auto` | Model may or may not call tools; default behavior. |
| `required` | Model must call at least one tool. |
| `none` | Model must not call any tools. |

---

#### ResponseFormat

Response format constraint.

| Value | Description |
|-------|-------------|
| `text` | Plain text output (default). |
| `json_object` | Output must be valid JSON object (no schema validation). |
| `json_schema` | Output must conform to the specified JSON schema. — Fields: `json_schema`: `JsonSchemaFormat` |

---

#### StopSequence

Stop sequence(s) that cause the model to stop generating.

| Value | Description |
|-------|-------------|
| `single` | Single stop sequence. — Fields: `0`: `String.t()` |
| `multiple` | Multiple stop sequences. — Fields: `0`: `list(String.t())` |

---

#### FinishReason

Why a choice stopped generating tokens.

| Value | Description |
|-------|-------------|
| `stop` | Stop |
| `length` | Length |
| `tool_calls` | Tool calls |
| `content_filter` | Content filter |
| `function_call` | Deprecated legacy finish reason; retained for API compatibility. |
| `other` | Catch-all for unknown finish reasons returned by non-OpenAI providers. Note: this intentionally does **not** carry the original string (e.g. `Other(String)`).  Using `#[serde(other)]` requires a unit variant, and switching to `#[serde(untagged)]` would change deserialization semantics for all variants.  The original value can be recovered by inspecting the raw JSON if needed. |

---

#### ReasoningEffort

Controls how much reasoning effort the model should use.

| Value | Description |
|-------|-------------|
| `low` | Low |
| `medium` | Medium |
| `high` | High |

---

#### EmbeddingFormat

The format in which the embedding vectors are returned.

| Value | Description |
|-------|-------------|
| `float` | 32-bit floating-point numbers (default). |
| `base64` | Base64-encoded string representation of the floats. |

---

#### EmbeddingInput

Text or texts to embed.

| Value | Description |
|-------|-------------|
| `single` | Single text string. — Fields: `0`: `String.t()` |
| `multiple` | Multiple text strings (batch embedding). — Fields: `0`: `list(String.t())` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value | Description |
|-------|-------------|
| `single` | Single text string. — Fields: `0`: `String.t()` |
| `multiple` | Multiple text strings (batch moderation). — Fields: `0`: `list(String.t())` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value | Description |
|-------|-------------|
| `text` | Plain text document content. — Fields: `0`: `String.t()` |
| `object` | Document with explicit text field (may include metadata). — Fields: `text`: `String.t()` |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value | Description |
|-------|-------------|
| `url` | A publicly accessible document URL. — Fields: `url`: `String.t()` |
| `base64` | Inline base64-encoded document data. — Fields: `data`: `String.t()`, `media_type`: `String.t()` |

---

#### FilePurpose

Purpose of an uploaded file.

| Value | Description |
|-------|-------------|
| `assistants` | File for use with Assistants API. |
| `batch` | File for batch processing. |
| `fine_tune` | File for fine-tuning. |
| `vision` | File for vision/image tasks. |

---

#### BatchStatus

Status of a batch job.

| Value | Description |
|-------|-------------|
| `validating` | Validating the input file. |
| `failed` | Job failed. |
| `in_progress` | Job is running. |
| `finalizing` | Finalizing results. |
| `completed` | Job completed successfully. |
| `expired` | Job expired before completion. |
| `cancelling` | Job is being cancelled. |
| `cancelled` | Job has been cancelled. |

---

#### AuthHeaderFormat

How the API key is sent in the HTTP request.

| Value | Description |
|-------|-------------|
| `bearer` | Bearer token: `Authorization: Bearer <key>` |
| `api_key` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `String.t()` |
| `none` | No authentication required. |

---

#### AuthType

Auth scheme used by a provider.

| Value | Description |
|-------|-------------|
| `bearer` | Standard `Authorization: Bearer <key>` header. |
| `api_key` | `x-api-key: <key>` header (also handles `"header"` and `"x-api-key"` aliases). |
| `none` | No authentication header required. |
| `unknown` | Unrecognised auth scheme — falls back to bearer. |

---

#### Enforcement

How budget limits are enforced.

| Value | Description |
|-------|-------------|
| `hard` | Reject requests that would exceed the budget with `LiterLlmError.BudgetExceeded`. |
| `soft` | Allow requests through but emit a `tracing.warn!` when the budget is exceeded. |

---

#### CacheBackend

Storage backend for the response cache.

| Value | Description |
|-------|-------------|
| `memory` | In-memory LRU cache (default). No external dependencies. |
| `open_dal` | OpenDAL-backed storage. Supports 40+ backends (S3, Redis, GCS, local FS, etc.). — Fields: `scheme`: `String.t()`, `config`: `map()` |

---

### Errors

#### LiterLlmError

All errors that can occur when using `liter-llm`.

| Variant | Description |
|---------|-------------|
| `authentication` | `status` preserves the exact HTTP status code received (401 or 403). |
| `rate_limited` | rate limited: {message} |
| `bad_request` | `status` preserves the exact HTTP status code received (400, 405, 413, 422, …). |
| `context_window_exceeded` | context window exceeded: {message} |
| `content_policy` | content policy violation: {message} |
| `not_found` | not found: {message} |
| `server_error` | `status` preserves the exact HTTP status code received (500, or other 5xx not covered by `ServiceUnavailable`). |
| `service_unavailable` | `status` preserves the exact HTTP status code received (502, 503, or 504). |
| `timeout` | request timeout |
| `streaming` | A catch-all for errors that occur during streaming response processing. This variant covers multiple sub-conditions including UTF-8 decoding failures, CRC/checksum mismatches (AWS EventStream), JSON parse errors in individual SSE chunks, and buffer overflow conditions.  The `message` field contains a human-readable description of the specific failure. |
| `endpoint_not_supported` | provider {provider} does not support {endpoint} |
| `invalid_header` | invalid header {name:?}: {reason} |
| `serialization` | serialization error: {0} |
| `budget_exceeded` | budget exceeded: {message} |
| `hook_rejected` | hook rejected: {message} |
| `internal_error` | An internal logic error (e.g. unexpected Tower response variant). This should never surface in normal operation — if it does, it indicates a bug in the library. |

---
