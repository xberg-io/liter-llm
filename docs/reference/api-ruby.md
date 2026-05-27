---
title: "Ruby API Reference"
---

## Ruby API Reference <span class="version-badge">v1.4.0-rc.35</span>

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

```ruby
def self.create_client(api_key, base_url: nil, timeout_secs: nil, max_retries: nil, model_hint: nil)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `api_key` | `String` | Yes | The api key |
| `base_url` | `String?` | No | The base url |
| `timeout_secs` | `Integer?` | No | The timeout secs |
| `max_retries` | `Integer?` | No | The max retries |
| `model_hint` | `String?` | No | The model hint |

**Returns:** `DefaultClient`
**Errors:** Raises `Error`.

---

#### create_client_from_json()

Create a new LLM client from a JSON string.

The JSON object accepts the same fields as `liter-llm.toml` (snake_case).

**Errors:**

Returns `LiterLlmError.BadRequest` if `json` is not valid JSON or
contains unknown fields.

**Signature:**

```ruby
def self.create_client_from_json(json)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `json` | `String` | Yes | The json |

**Returns:** `DefaultClient`
**Errors:** Raises `Error`.

---

#### register_custom_provider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```ruby
def self.register_custom_provider(config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `CustomProviderConfig` | Yes | The configuration options |

**Returns:** `nil`
**Errors:** Raises `Error`.

---

#### unregister_custom_provider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error only if the internal lock is poisoned.

**Signature:**

```ruby
def self.unregister_custom_provider(name)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `Boolean`
**Errors:** Raises `Error`.

---

#### all_providers()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.

**Signature:**

```ruby
def self.all_providers()
```

**Returns:** `Array<ProviderConfig>`
**Errors:** Raises `Error`.

---

#### complex_provider_names()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```ruby
def self.complex_provider_names()
```

**Returns:** `Array<String>`
**Errors:** Raises `Error`.

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

```ruby
def self.completion_cost(model, prompt_tokens, completion_tokens)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `prompt_tokens` | `Integer` | Yes | The prompt tokens |
| `completion_tokens` | `Integer` | Yes | The completion tokens |

**Returns:** `Float?`

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

```ruby
def self.completion_cost_with_cache(model, prompt_tokens, cached_tokens, completion_tokens)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `prompt_tokens` | `Integer` | Yes | The prompt tokens |
| `cached_tokens` | `Integer` | Yes | The cached tokens |
| `completion_tokens` | `Integer` | Yes | The completion tokens |

**Returns:** `Float?`

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

```ruby
def self.count_tokens(model, text)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `text` | `String` | Yes | The text |

**Returns:** `Integer`
**Errors:** Raises `Error`.

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

```ruby
def self.count_request_tokens(model, req)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `Integer`
**Errors:** Raises `Error`.

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

```ruby
def self.ensure_crypto_provider()
```

**Returns:** `nil`

---

### Types

#### AssistantMessage

Assistant's response to a user message.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String?` | `nil` | The assistant's text response. Absent if tool calls are returned instead. |
| `name` | `String?` | `nil` | Optional name for the assistant. |
| `tool_calls` | `Array<ToolCall>?` | `[]` | Tool calls the model wants to execute, if any. |
| `refusal` | `String?` | `nil` | Refusal reason, if the model declined to respond per safety policies. |
| `function_call` | `FunctionCall?` | `nil` | Deprecated legacy function_call field; retained for API compatibility. |

---

#### AudioContent

Audio content part for speech-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `String` | — | Base64-encoded audio data. |
| `format` | `String` | — | Audio format (e.g., "wav", "mp3", "ogg"). |

---

#### AuthConfig

Auth configuration block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auth_type` | `AuthType` | — | Auth scheme classification. |
| `env_var` | `String?` | `nil` | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### BatchListQuery

Query parameters for listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `Integer?` | `nil` | Maximum number of results to return. Defaults to 20. |
| `after` | `String?` | `nil` | Pagination cursor: return results after this batch ID. |

---

#### BatchListResponse

Response from listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Object type (always `"list"`). |
| `data` | `Array<BatchObject>` | `[]` | List of batch objects. |
| `has_more` | `Boolean?` | `nil` | Whether more results are available. |
| `first_id` | `String?` | `nil` | First batch ID in the result set (for pagination). |
| `last_id` | `String?` | `nil` | Last batch ID in the result set (for pagination). |

---

#### BatchObject

A batch job object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique batch ID. |
| `object` | `String` | — | Object type (always `"batch"`). |
| `endpoint` | `String` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `input_file_id` | `String` | — | ID of the input file. |
| `completion_window` | `String` | — | Completion window (e.g., `"24h"`). |
| `status` | `BatchStatus` | `:validating` | Current job status. |
| `output_file_id` | `String?` | `nil` | ID of the output file (present when completed). |
| `error_file_id` | `String?` | `nil` | ID of the error file (present if some requests failed). |
| `created_at` | `Integer` | — | Unix timestamp of batch creation. |
| `completed_at` | `Integer?` | `nil` | Unix timestamp of completion (if completed). |
| `failed_at` | `Integer?` | `nil` | Unix timestamp of failure (if failed). |
| `expired_at` | `Integer?` | `nil` | Unix timestamp of expiration (if expired). |
| `request_counts` | `BatchRequestCounts?` | `nil` | Request processing counts. |
| `metadata` | `Object?` | `nil` | Metadata attached to the batch. |

---

#### BatchRequestCounts

Request processing counts for a batch.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total` | `Integer` | — | Total requests in the batch. |
| `completed` | `Integer` | — | Completed requests. |
| `failed` | `Integer` | — | Failed requests. |

---

#### BudgetConfig

Configuration for budget enforcement.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `global_limit` | `Float?` | `nil` | Maximum total spend across all models, in USD.  `nil` means unlimited. |
| `model_limits` | `Hash{String=>Float}` | `{}` | Per-model spending limits in USD.  Models not listed here are only constrained by `global_limit`. |
| `enforcement` | `Enforcement` | `:hard` | Whether to reject requests or merely warn when a limit is exceeded. |

### Methods

#### default()

**Signature:**

```ruby
def self.default()
```

---

#### ChatCompletionChunk

A streamed chunk of a chat completion response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this stream. |
| `object` | `String` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created` | `Integer` | — | Unix timestamp of chunk creation. |
| `model` | `String` | — | Model used to generate the chunk. |
| `choices` | `Array<StreamChoice>` | `[]` | Streaming choices (delta updates). |
| `usage` | `Usage?` | `nil` | Token usage (typically only in the final chunk). |
| `system_fingerprint` | `String?` | `nil` | Fingerprint of the system configuration (OpenAI-specific). |
| `service_tier` | `String?` | `nil` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`). |
| `messages` | `Array<Message>` | `[]` | Conversation history from oldest to newest. |
| `temperature` | `Float?` | `nil` | Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0. |
| `top_p` | `Float?` | `nil` | Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused. |
| `n` | `Integer?` | `nil` | Number of chat completions to generate. Defaults to 1. |
| `stream` | `Boolean?` | `nil` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `stop` | `StopSequence?` | `nil` | Stop sequence(s) that halt token generation. |
| `max_tokens` | `Integer?` | `nil` | Max output tokens. Different from max_completion_tokens in some providers. |
| `presence_penalty` | `Float?` | `nil` | Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics. |
| `frequency_penalty` | `Float?` | `nil` | Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens. |
| `logit_bias` | `Hash{String=>Float}?` | `{}` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user` | `String?` | `nil` | User identifier for request tracking and abuse detection. |
| `tools` | `Array<ChatCompletionTool>?` | `[]` | Tools the model can invoke. |
| `tool_choice` | `ToolChoice?` | `nil` | Tool usage mode (auto, required, none, or specific tool). |
| `parallel_tool_calls` | `Boolean?` | `nil` | Whether the model can call multiple tools in parallel. Defaults to true. |
| `response_format` | `ResponseFormat?` | `nil` | Output format constraint (text, JSON, JSON schema). |
| `stream_options` | `StreamOptions?` | `nil` | Streaming options (e.g., include_usage). |
| `seed` | `Integer?` | `nil` | Random seed for reproducible outputs. Provider support varies. |
| `reasoning_effort` | `ReasoningEffort?` | `nil` | Reasoning effort level (low, medium, high) for extended-thinking models. |
| `extra_body` | `Object?` | `nil` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

#### ChatCompletionResponse

Chat completion response from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this response. |
| `object` | `String` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `Integer` | — | Unix timestamp of response creation. |
| `model` | `String` | — | Model used to generate the response. |
| `choices` | `Array<Choice>` | `[]` | List of completion choices. |
| `usage` | `Usage?` | `nil` | Token usage statistics. |
| `system_fingerprint` | `String?` | `nil` | Fingerprint of the system configuration (OpenAI-specific). |
| `service_tier` | `String?` | `nil` | Service tier used (OpenAI-specific). |

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
| `index` | `Integer` | — | Index of this choice in the choices array. |
| `message` | `AssistantMessage` | — | The assistant's message response. |
| `finish_reason` | `FinishReason?` | `nil` | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

---

#### CreateBatchRequest

Request to create a batch job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_file_id` | `String` | — | ID of the uploaded input file (JSONL format). |
| `endpoint` | `String` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `completion_window` | `String` | — | Completion window (e.g., `"24h"`). |
| `metadata` | `Object?` | `nil` | Optional metadata to attach to the batch. |

---

#### CreateFileRequest

Request to upload a file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | `String` | — | Base64-encoded file data. |
| `purpose` | `FilePurpose` | `:assistants` | Purpose for the file. |
| `filename` | `String?` | `nil` | Optional filename to associate with the upload. |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt` | `String` | — | Text description of the image to generate. |
| `model` | `String?` | `nil` | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `n` | `Integer?` | `nil` | Number of images to generate. Defaults to 1. |
| `size` | `String?` | `nil` | Image size (e.g., `"1024x1024"`, `"1792x1024"`). |
| `quality` | `String?` | `nil` | Image quality: `"standard"` or `"hd"`. |
| `style` | `String?` | `nil` | Style: `"natural"` or `"vivid"` (DALL-E 3 only). |
| `response_format` | `String?` | `nil` | Response format: `"url"` or `"b64_json"`. |
| `user` | `String?` | `nil` | User identifier for request tracking. |

---

#### CreateResponseRequest

Request to create a structured response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID. |
| `input` | `Object` | — | Input data to process (e.g., a document to extract from). |
| `instructions` | `String?` | `nil` | Instructions for processing the input. |
| `tools` | `Array<ResponseTool>?` | `[]` | Available tools the model can use. |
| `temperature` | `Float?` | `nil` | Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0. |
| `max_output_tokens` | `Integer?` | `nil` | Maximum output tokens. |
| `metadata` | `Object?` | `nil` | Optional metadata. |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`). |
| `input` | `String` | — | Text to synthesize into speech. |
| `voice` | `String` | — | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `response_format` | `String?` | `nil` | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`). |
| `speed` | `Float?` | `nil` | Playback speed in `[0.25, 4.0]`. Defaults to 1.0. |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"whisper-1"`). |
| `file` | `String` | — | Base64-encoded audio file data. |
| `language` | `String?` | `nil` | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `prompt` | `String?` | `nil` | Optional text to guide the model (improves accuracy for domain-specific terms). |
| `response_format` | `String?` | `nil` | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`). |
| `temperature` | `Float?` | `nil` | Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0. |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Unique name for this provider (e.g., "my-provider"). |
| `base_url` | `String` | — | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `auth_header` | `AuthHeaderFormat` | — | Authentication header format. |
| `model_prefixes` | `Array<String>` | — | Model name prefixes that route to this provider (e.g., `["my-"]`). |

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

### Methods

#### chat()

**Signature:**

```ruby
def chat(req)
```

#### chat_stream()

**Signature:**

```ruby
def chat_stream(req)
```

#### embed()

**Signature:**

```ruby
def embed(req)
```

#### list_models()

**Signature:**

```ruby
def list_models()
```

#### image_generate()

**Signature:**

```ruby
def image_generate(req)
```

#### speech()

**Signature:**

```ruby
def speech(req)
```

#### transcribe()

**Signature:**

```ruby
def transcribe(req)
```

#### moderate()

**Signature:**

```ruby
def moderate(req)
```

#### rerank()

**Signature:**

```ruby
def rerank(req)
```

#### search()

**Signature:**

```ruby
def search(req)
```

#### ocr()

**Signature:**

```ruby
def ocr(req)
```

#### create_file()

**Signature:**

```ruby
def create_file(req)
```

#### retrieve_file()

**Signature:**

```ruby
def retrieve_file(file_id)
```

#### delete_file()

**Signature:**

```ruby
def delete_file(file_id)
```

#### list_files()

**Signature:**

```ruby
def list_files(query)
```

#### file_content()

**Signature:**

```ruby
def file_content(file_id)
```

#### create_batch()

**Signature:**

```ruby
def create_batch(req)
```

#### retrieve_batch()

**Signature:**

```ruby
def retrieve_batch(batch_id)
```

#### list_batches()

**Signature:**

```ruby
def list_batches(query)
```

#### cancel_batch()

**Signature:**

```ruby
def cancel_batch(batch_id)
```

#### create_response()

**Signature:**

```ruby
def create_response(req)
```

#### retrieve_response()

**Signature:**

```ruby
def retrieve_response(response_id)
```

#### cancel_response()

**Signature:**

```ruby
def cancel_response(response_id)
```

---

#### DeleteResponse

Response from a delete operation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | ID of the deleted resource. |
| `object` | `String` | — | Object type. |
| `deleted` | `Boolean` | — | Confirmation that the resource was deleted. |

---

#### DeveloperMessage

Developer message (system-like message for Claude models).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Developer-specific instructions or context. |
| `name` | `String?` | `nil` | Optional name for the developer message source. |

---

#### DocumentContent

PDF/document content part for vision-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `String` | — | Base64-encoded document data or URL. |
| `media_type` | `String` | — | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### EmbeddingObject

A single embedding vector.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `embedding` | `Array<Float>` | — | The embedding vector. |
| `index` | `Integer` | — | Index in the batch (corresponds to input order). |

---

#### EmbeddingRequest

Embedding request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"text-embedding-3-small"`). |
| `input` | `EmbeddingInput` | `:single` | Text or texts to embed. |
| `encoding_format` | `EmbeddingFormat?` | `nil` | Output format: float (native) or base64. |
| `dimensions` | `Integer?` | `nil` | Requested embedding dimensions (if supported by the model). |
| `user` | `String?` | `nil` | User identifier for request tracking. |

---

#### EmbeddingResponse

Embedding response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `Array<EmbeddingObject>` | — | List of embeddings. |
| `model` | `String` | — | Model used to generate embeddings. |
| `usage` | `Usage?` | `/* serde(default) */` | Token usage (input tokens only; embeddings have zero output tokens). |

---

#### FileListQuery

Query parameters for listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `purpose` | `String?` | `nil` | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `limit` | `Integer?` | `nil` | Maximum number of results to return. Defaults to 20. |
| `after` | `String?` | `nil` | Pagination cursor: return results after this file ID. |

---

#### FileListResponse

Response from listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Object type (always `"list"`). |
| `data` | `Array<FileObject>` | `[]` | List of file objects. |
| `has_more` | `Boolean?` | `nil` | Whether more results are available. |

---

#### FileObject

An uploaded file object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique file ID. |
| `object` | `String` | — | Object type (always `"file"`). |
| `bytes` | `Integer` | — | File size in bytes. |
| `created_at` | `Integer` | — | Unix timestamp of file creation. |
| `filename` | `String` | — | Filename. |
| `purpose` | `String` | — | File purpose. |
| `status` | `String?` | `nil` | Processing status (e.g., `"uploaded"`, `"processed"`). |

---

#### FunctionCall

Function call details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Function name. |
| `arguments` | `String` | — | Arguments as a JSON string (parse with serde_json.from_str). |

---

#### FunctionDefinition

Function definition exposed to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Name of the function. Required and must be alphanumeric + underscores. |
| `description` | `String?` | `/* serde(default) */` | Human-readable description explaining what the function does. |
| `parameters` | `Object?` | `/* serde(default) */` | JSON Schema defining the function's parameters. |
| `strict` | `Boolean?` | `/* serde(default) */` | If true, enforce strict JSON schema validation for arguments. |

---

#### FunctionMessage

Deprecated legacy function-role message body.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | The extracted text content |
| `name` | `String` | — | The name |

---

#### Image

A single generated image, returned as either a URL or base64 data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String?` | `nil` | Image URL (if response_format was "url"). |
| `b64_json` | `String?` | `nil` | Base64-encoded image data (if response_format was "b64_json"). |
| `revised_prompt` | `String?` | `nil` | The final prompt used to generate the image (DALL-E 3). |

---

#### ImageUrl

An image URL reference with optional detail level for processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | URL of the image (data URI or HTTP/HTTPS URL). |
| `detail` | `ImageDetail?` | `nil` | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### ImagesResponse

Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `created` | `Integer` | — | Unix timestamp of image creation. |
| `data` | `Array<Image>` | `[]` | List of generated images. |

---

#### JsonSchemaFormat

JSON Schema specification for constrained output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Name of the schema (must be unique in the request). |
| `description` | `String?` | `nil` | Description of what the schema represents. |
| `schema` | `Object` | — | JSON Schema object defining the output structure. |
| `strict` | `Boolean?` | `nil` | If true, enforce strict schema validation. |

---

#### ModelObject

A model available from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`). |
| `object` | `String` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `Integer` | — | Unix timestamp of model creation (or release date). |
| `owned_by` | `String` | — | Organization or entity that owns the model. |

---

#### ModelsListResponse

Response listing available models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `Array<ModelObject>` | `[]` | List of available models. |

---

#### ModerationCategories

Boolean flags for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `Boolean` | — | Sexual content. |
| `hate` | `Boolean` | — | Hate speech. |
| `harassment` | `Boolean` | — | Harassment. |
| `self_harm` | `Boolean` | — | Self-harm content. |
| `sexual_minors` | `Boolean` | — | Sexual content involving minors. |
| `hate_threatening` | `Boolean` | — | Hate speech that threatens violence. |
| `violence_graphic` | `Boolean` | — | Graphic violence. |
| `self_harm_intent` | `Boolean` | — | Intent to self-harm. |
| `self_harm_instructions` | `Boolean` | — | Instructions for self-harm. |
| `harassment_threatening` | `Boolean` | — | Harassment that threatens violence. |
| `violence` | `Boolean` | — | Non-graphic violence. |

---

#### ModerationCategoryScores

Confidence scores for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `Float` | — | Sexual content score. |
| `hate` | `Float` | — | Hate speech score. |
| `harassment` | `Float` | — | Harassment score. |
| `self_harm` | `Float` | — | Self-harm content score. |
| `sexual_minors` | `Float` | — | Sexual content involving minors score. |
| `hate_threatening` | `Float` | — | Hate speech that threatens violence score. |
| `violence_graphic` | `Float` | — | Graphic violence score. |
| `self_harm_intent` | `Float` | — | Intent to self-harm score. |
| `self_harm_instructions` | `Float` | — | Instructions for self-harm score. |
| `harassment_threatening` | `Float` | — | Harassment that threatens violence score. |
| `violence` | `Float` | — | Non-graphic violence score. |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input` | `ModerationInput` | `:single` | Text or texts to check. |
| `model` | `String?` | `nil` | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this moderation request. |
| `model` | `String` | — | Model used for classification. |
| `results` | `Array<ModerationResult>` | — | Results for each input string. |

---

#### ModerationResult

A single moderation classification result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `flagged` | `Boolean` | — | True if any category was flagged. |
| `categories` | `ModerationCategories` | — | Boolean flags for each moderation category. |
| `category_scores` | `ModerationCategoryScores` | — | Confidence scores for each category. |

---

#### OcrImage

An image extracted from an OCR page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique image identifier within the document. |
| `image_base64` | `String?` | `/* serde(default) */` | Base64-encoded image data (if `include_image_base64` was true). |

---

#### OcrPage

A single page of OCR output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `Integer` | — | Page index (0-based). |
| `markdown` | `String` | — | Extracted page content as Markdown. |
| `images` | `Array<OcrImage>?` | `/* serde(default) */` | Embedded images extracted from the page (if `include_image_base64` was true). |
| `dimensions` | `PageDimensions?` | `/* serde(default) */` | Page dimensions in pixels, if available. |

---

#### OcrRequest

An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document` | `OcrDocument` | `:url` | The document to process (URL or base64). |
| `pages` | `Array<Integer>?` | `[]` | Specific pages to process (1-indexed). `nil` means all pages. |
| `include_image_base64` | `Boolean?` | `nil` | Whether to include base64-encoded images of each processed page. |

---

#### OcrResponse

An OCR response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pages` | `Array<OcrPage>` | — | Extracted pages in order. |
| `model` | `String` | — | Model/provider used for OCR. |
| `usage` | `Usage?` | `/* serde(default) */` | Token usage, if reported by the provider. |

---

#### PageDimensions

Page dimensions in pixels.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `Integer` | — | Width in pixels. |
| `height` | `Integer` | — | Height in pixels. |

---

#### PromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage.prompt_tokens` — it is *not* an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cached_tokens` | `Integer` | — | Cached tokens present in the prompt. Defaults to 0 when absent. |
| `audio_tokens` | `Integer` | — | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### ProviderConfig

Static configuration for a single provider entry in providers.json.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Provider identifier (matches the entry key in providers.json). |
| `display_name` | `String?` | `nil` | Human-readable provider name shown in UIs. |
| `base_url` | `String?` | `nil` | Base URL used as the default for this provider's HTTP client. |
| `auth` | `AuthConfig?` | `nil` | Authentication scheme metadata (auth type + env var holding the key). |
| `endpoints` | `Array<String>?` | `nil` | Supported endpoint kinds (e.g. `chat`, `embeddings`). |
| `model_prefixes` | `Array<String>?` | `nil` | Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`). |
| `param_mappings` | `Hash{String=>String}?` | `nil` | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`).  Applied automatically by `ConfigDrivenProvider.transform_request`. |

---

#### RateLimitConfig

Configuration for per-model rate limits.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rpm` | `Integer?` | `nil` | Maximum requests per window.  `nil` means unlimited. |
| `tpm` | `Integer?` | `nil` | Maximum tokens per window.  `nil` means unlimited. |
| `window` | `Float` | `60000ms` | Fixed window duration (defaults to 60 s). |

### Methods

#### default()

**Signature:**

```ruby
def self.default()
```

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"cohere/rerank-english-v3.0"`). |
| `query` | `String` | — | The search query. |
| `documents` | `Array<RerankDocument>` | `[]` | Documents to rerank. |
| `top_n` | `Integer?` | `nil` | Return only the top N results. Optional. |
| `return_documents` | `Boolean?` | `nil` | Include the document content in results. Defaults to false. |

---

#### RerankResponse

Response from the rerank endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String?` | `nil` | Unique identifier for this rerank request. |
| `results` | `Array<RerankResult>` | — | Reranked documents in order of relevance. |
| `meta` | `Object?` | `/* serde(default) */` | Optional metadata about the reranking operation. |

---

#### RerankResult

A single reranked document with its relevance score.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `Integer` | — | Original document index in the input list. |
| `relevance_score` | `Float` | — | Relevance score in `[0, 1]`. Higher indicates more relevant. |
| `document` | `RerankResultDocument?` | `/* serde(default) */` | Original document content (if `return_documents` was true). |

---

#### RerankResultDocument

The text content of a reranked document, returned when `return_documents` is true.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | Document text. |

---

#### ResponseObject

Response from a structured response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique response ID. |
| `object` | `String` | — | Object type (e.g., `"response"`). |
| `created_at` | `Integer` | — | Unix timestamp of response creation. |
| `model` | `String` | — | Model used to generate the response. |
| `status` | `String` | — | Status (e.g., `"succeeded"`, `"failed"`). |
| `output` | `Array<ResponseOutputItem>` | `[]` | Output items from the response. |
| `usage` | `ResponseUsage?` | `nil` | Token usage. |
| `error` | `Object?` | `nil` | Error details (if status is "failed"). |

---

#### ResponseOutputItem

A single output item from the response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `item_type` | `String` | — | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `content` | `Object` | — | Output content (flattened into the object). |

---

#### ResponseTool

A tool available for the response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tool_type` | `String` | — | Tool type (e.g., "extractor", "search"). |
| `config` | `Object` | — | Tool configuration (flattened into the object). |

---

#### ResponseUsage

Token usage for a response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_tokens` | `Integer` | — | Input tokens used. |
| `output_tokens` | `Integer` | — | Output tokens used. |
| `total_tokens` | `Integer` | — | Total tokens used. |

---

#### SearchRequest

A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query` | `String` | — | The search query string. |
| `max_results` | `Integer?` | `nil` | Maximum number of results to return. |
| `search_domain_filter` | `Array<String>?` | `[]` | Domain filter — restrict results to specific domains. |
| `country` | `String?` | `nil` | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

---

#### SearchResponse

A search response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `results` | `Array<SearchResult>` | — | List of search results. |
| `model` | `String` | — | Model/provider that performed the search. |

---

#### SearchResult

An individual search result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String` | — | Result title. |
| `url` | `String` | — | Result URL. |
| `snippet` | `String` | — | Text snippet or excerpt from the page. |
| `date` | `String?` | `/* serde(default) */` | Publication or last-updated date, if available. |

---

#### SpecificFunction

Name of the specific function to invoke.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Function name. |

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
| `index` | `Integer` | — | Index of this choice in the choices array. |
| `delta` | `StreamDelta` | — | Incremental update to the message (content, tool calls, etc.). |
| `finish_reason` | `FinishReason?` | `nil` | Why the stream ended (present only in final chunk). |

---

#### StreamDelta

Incremental delta in a stream chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `role` | `String?` | `nil` | Role (typically present only in the first chunk). |
| `content` | `String?` | `nil` | Partial content chunk (e.g., a few words of the response). |
| `tool_calls` | `Array<StreamToolCall>?` | `[]` | Partial tool calls being streamed. |
| `function_call` | `StreamFunctionCall?` | `nil` | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal` | `String?` | `nil` | Partial refusal message. |

---

#### StreamFunctionCall

Partial function call details in a stream.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String?` | `nil` | Function name (typically in the first chunk). |
| `arguments` | `String?` | `nil` | Partial JSON arguments chunk. |

---

#### StreamOptions

Options for streaming responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_usage` | `Boolean?` | `nil` | If true, include token usage in the final stream chunk. |

---

#### StreamToolCall

A streaming tool call being built incrementally.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `Integer` | — | Index of this tool call in the tool_calls array. |
| `id` | `String?` | `nil` | Tool call ID (typically in the first chunk for this call). |
| `call_type` | `ToolType?` | `nil` | Tool type (typically "function"). |
| `function` | `StreamFunctionCall?` | `nil` | Partial function name and arguments. |

---

#### SystemMessage

System message guiding model behavior for the entire conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Instructions or context that apply throughout the conversation. |
| `name` | `String?` | `nil` | Optional name for the system message source. |

---

#### ToolCall

A tool call the model wants to execute.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique ID for this call, used to reference in tool result messages. |
| `call_type` | `ToolType` | — | Tool type (always "function"). |
| `function` | `FunctionCall` | — | Function name and arguments. |

---

#### ToolMessage

Tool execution result returned to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Result of the tool execution. |
| `tool_call_id` | `String` | — | ID of the tool call this result responds to. |
| `name` | `String?` | `nil` | Optional tool/function name. |

---

#### TranscriptionResponse

Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The transcribed text. |
| `language` | `String?` | `nil` | Detected language (ISO-639-1 code). |
| `duration` | `Float?` | `nil` | Total audio duration in seconds. |
| `segments` | `Array<TranscriptionSegment>?` | `[]` | Detailed segment-level transcription (if response_format is "verbose_json"). |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `Integer` | — | Segment index (0-based). |
| `start` | `Float` | — | Start time in seconds. |
| `end` | `Float` | — | End time in seconds. |
| `text` | `String` | — | Transcribed text for this segment. |

---

#### Usage

Token-usage accounting returned by the provider on each completion / embedding call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_tokens` | `Integer` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `completion_tokens` | `Integer` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `total_tokens` | `Integer` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `prompt_tokens_details` | `PromptTokensDetails?` | `nil` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

User message in the conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `:text` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `name` | `String?` | `nil` | Optional name for the user. |

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
| `text` | Plain text content. — Fields: `0`: `String` |
| `parts` | Array of content parts (text, images, documents, audio). — Fields: `0`: `Array<ContentPart>` |

---

#### ContentPart

A single content part in a user message — text, image, document, or audio.

| Value | Description |
|-------|-------------|
| `text` | Plain text. — Fields: `text`: `String` |
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
| `single` | Single stop sequence. — Fields: `0`: `String` |
| `multiple` | Multiple stop sequences. — Fields: `0`: `Array<String>` |

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
| `single` | Single text string. — Fields: `0`: `String` |
| `multiple` | Multiple text strings (batch embedding). — Fields: `0`: `Array<String>` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value | Description |
|-------|-------------|
| `single` | Single text string. — Fields: `0`: `String` |
| `multiple` | Multiple text strings (batch moderation). — Fields: `0`: `Array<String>` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value | Description |
|-------|-------------|
| `text` | Plain text document content. — Fields: `0`: `String` |
| `object` | Document with explicit text field (may include metadata). — Fields: `text`: `String` |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value | Description |
|-------|-------------|
| `url` | A publicly accessible document URL. — Fields: `url`: `String` |
| `base64` | Inline base64-encoded document data. — Fields: `data`: `String`, `media_type`: `String` |

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
| `api_key` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `String` |
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
