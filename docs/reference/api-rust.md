---
title: "Rust API Reference"
---

## Rust API Reference <span class="version-badge">v1.6.3</span>

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

```rust
pub fn create_client(api_key: &str, base_url: Option<String>, timeout_secs: Option<u64>, max_retries: Option<u32>, model_hint: Option<String>) -> Result<DefaultClient, Error>
```

**Example:**

```rust
let result = create_client("value", "value", 42, 42, "value")?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `api_key` | `String` | Yes | The api key |
| `base_url` | `Option<String>` | No | The base url |
| `timeout_secs` | `Option<u64>` | No | The timeout secs |
| `max_retries` | `Option<u32>` | No | The max retries |
| `model_hint` | `Option<String>` | No | The model hint |

**Returns:** `DefaultClient`

**Errors:** Returns `Err(Error)`.

---

#### create_client_from_json()

Create a new LLM client from a JSON string.

The JSON object accepts the same fields as `liter-llm.toml` (snake_case).

**Errors:**

Returns `LiterLlmError::BadRequest` if `json` is not valid JSON or
contains unknown fields.

**Signature:**

```rust
pub fn create_client_from_json(json: &str) -> Result<DefaultClient, Error>
```

**Example:**

```rust
let result = create_client_from_json("value")?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `json` | `String` | Yes | The json |

**Returns:** `DefaultClient`

**Errors:** Returns `Err(Error)`.

---

#### register_custom_provider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```rust
pub fn register_custom_provider(config: CustomProviderConfig) -> Result<(), Error>
```

**Example:**

```rust
register_custom_provider(CustomProviderConfig::default())?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `CustomProviderConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Returns `Err(Error)`.

---

#### unregister_custom_provider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error if the custom-provider registry cannot be updated.

**Signature:**

```rust
pub fn unregister_custom_provider(name: &str) -> Result<bool, Error>
```

**Example:**

```rust
let result = unregister_custom_provider("value")?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `bool`

**Errors:** Returns `Err(Error)`.

---

#### capabilities()

Return the capability flags for a named provider.

Performs an O(n) linear scan over the embedded registry (143 entries).
Returns an owned value so bindings can pass capability data without
borrowing registry internals.

For unknown `provider_name` values the function returns an all-`false`
sentinel so callers never need to handle `Option`.

**Signature:**

```rust
pub fn capabilities(provider_name: &str) -> ProviderCapabilities
```

**Example:**

```rust
let result = capabilities("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `provider_name` | `String` | Yes | The provider name |

**Returns:** `ProviderCapabilities`

---

#### all_providers()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.
Returns the public `ProviderConfig` slice (without capability flags).
To query capability flags for a specific provider use `capabilities`.

**Signature:**

```rust
pub fn all_providers() -> Result<Vec<ProviderConfig>, Error>
```

**Example:**

```rust
let result = all_providers()?;
```

**Returns:** `Vec<ProviderConfig>`

**Errors:** Returns `Err(Error)`.

---

#### complex_provider_names()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```rust
pub fn complex_provider_names() -> Result<Vec<String>, Error>
```

**Example:**

```rust
let result = complex_provider_names()?;
```

**Returns:** `Vec<String>`

**Errors:** Returns `Err(Error)`.

---

#### completion_cost()

Calculate the estimated cost of a completion given a model name and token
counts.

Returns `None` if the model is not present in the embedded pricing registry.
Returns `Some(cost_usd)` otherwise, where the value is in US dollars.

When an exact model name match is not found, progressively shorter prefixes
are tried by stripping from the last `-` or `.` separator.  For example,
`gpt-4-0613` will match `gpt-4` if no `gpt-4-0613` entry exists.

**Signature:**

```rust
pub fn completion_cost(model: &str, prompt_tokens: u64, completion_tokens: u64) -> Option<f64>
```

**Example:**

```rust
use liter_llm::cost;

let usd = cost::completion_cost("gpt-4o", 1_000, 500).expect("gpt-4o is a known model");
// 1000 * 0.0000025 + 500 * 0.00001 = 0.0025 + 0.005 = 0.0075
assert!((usd - 0.0075).abs() < 1e-9);
```rust

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `prompt_tokens` | `u64` | Yes | The prompt tokens |
| `completion_tokens` | `u64` | Yes | The completion tokens |

**Returns:** `Option<f64>`

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

Returns `None` if the model is not present in the embedded pricing
registry, mirroring `completion_cost`.

**Signature:**

```rust
pub fn completion_cost_with_cache(model: &str, prompt_tokens: u64, cached_tokens: u64, completion_tokens: u64) -> Option<f64>
```

**Example:**

```rust
let result = completion_cost_with_cache("value", 42, 42, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `prompt_tokens` | `u64` | Yes | The prompt tokens |
| `cached_tokens` | `u64` | Yes | The cached tokens |
| `completion_tokens` | `u64` | Yes | The completion tokens |

**Returns:** `Option<f64>`

---

#### clear()

Remove all guardrails from the global registry.

Primarily useful in tests to reset state between test cases.

**Panics:**

Panics if the global registry lock is poisoned.

**Signature:**

```rust
pub fn clear()
```

**Example:**

```rust
clear();
```

**Returns:** No return value.

---

#### count_tokens()

Count tokens in a text string using the tokenizer for the given model.

The tokenizer is resolved from the model name prefix (e.g. `"gpt-4o"` maps
to the `Xenova/gpt-4o` HuggingFace tokenizer). Tokenizers are cached after
first load.

**Errors:**

Returns `LiterLlmError::BadRequest` if the tokenizer cannot be loaded
(e.g. network failure on first use) or if tokenization itself fails.

**Signature:**

```rust
pub fn count_tokens(model: &str, text: &str) -> Result<usize, Error>
```

**Example:**

```rust
let result = count_tokens("value", "value")?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `text` | `String` | Yes | The text |

**Returns:** `usize`

**Errors:** Returns `Err(Error)`.

---

#### count_request_tokens()

Count tokens for a full `ChatCompletionRequest`.

Sums tokens across all message text contents plus a per-message overhead
of ~4 tokens (for role, separators, and formatting metadata). Tool
definitions and multimodal content parts (images, audio, documents) are
not counted — only textual content contributes to the token total.

**Errors:**

Returns `LiterLlmError::BadRequest` if the tokenizer cannot be loaded or
if tokenization fails for any message.

**Signature:**

```rust
pub fn count_request_tokens(model: &str, req: ChatCompletionRequest) -> Result<usize, Error>
```

**Example:**

```rust
let result = count_request_tokens("value", ChatCompletionRequest::default())?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `usize`

**Errors:** Returns `Err(Error)`.

---

#### check_bound()

Assert that `current_len + incoming` does not exceed `limit`.

Call this before appending `incoming` bytes to any buffer that must
stay below `limit`.  Returns `Err(LiterLlmError::Streaming)` on overflow
and emits a `tracing::warn!` with context.

**Signature:**

```rust
pub fn check_bound(context: &str, current_len: usize, incoming: usize, limit: usize) -> Result<(), Error>
```

**Example:**

```rust
check_bound("SSE buffer", buffer.len(), chunk.len(), SSE_BUFFER_MAX_BYTES)?;
buffer.push_str(chunk_str);
```rust

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `context` | `String` | Yes | The context |
| `current_len` | `usize` | Yes | The current len |
| `incoming` | `usize` | Yes | The incoming |
| `limit` | `usize` | Yes | The limit |

**Returns:** No return value.

**Errors:** Returns `Err(Error)`.

---

#### ensure_crypto_provider()

Install the `ring` crypto provider as the rustls process default, idempotently.

rustls 0.23+ removed the implicit default provider. This function installs
`ring` once per process. Subsequent calls are no-ops. Calling it after
another rustls crypto provider has already been installed is safe: the
`Err` from `install_default()` is silently ignored.

Called automatically by every internal `reqwest::Client` constructor
(auth providers, default HTTP client). Bindings and downstream consumers
reach those constructors transitively, so no manual init is required.

WASM builds are exempt — the WASM target uses the browser/Node.js fetch
API instead of rustls, so no crypto provider is needed.

Windows builds use native-tls (SChannel) via reqwest, so rustls is not
present and no crypto provider installation is needed.

**Signature:**

```rust
pub fn ensure_crypto_provider()
```

**Example:**

```rust
ensure_crypto_provider();
```

**Returns:** No return value.

---

### Types

#### AssistantMessage

Assistant's response to a user message.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `Option<String>` | `Default::default()` | The assistant's text response. Absent if tool calls are returned instead. |
| `name` | `Option<String>` | `Default::default()` | Optional name for the assistant. |
| `tool_calls` | `Option<Vec<ToolCall>>` | `vec![]` | Tool calls the model wants to execute, if any. |
| `refusal` | `Option<String>` | `Default::default()` | Refusal reason, if the model declined to respond per safety policies. |
| `function_call` | `Option<FunctionCall>` | `Default::default()` | Deprecated legacy function_call field; retained for API compatibility. |

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
| `env_var` | `Option<String>` | `None` | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### BatchListQuery

Query parameters for listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `Option<u32>` | `Default::default()` | Maximum number of results to return. Defaults to 20. |
| `after` | `Option<String>` | `Default::default()` | Pagination cursor: return results after this batch ID. |

---

#### BatchListResponse

Response from listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Object type (always `"list"`). |
| `data` | `Vec<BatchObject>` | `vec![]` | List of batch objects. |
| `has_more` | `Option<bool>` | `Default::default()` | Whether more results are available. |
| `first_id` | `Option<String>` | `Default::default()` | First batch ID in the result set (for pagination). |
| `last_id` | `Option<String>` | `Default::default()` | Last batch ID in the result set (for pagination). |

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
| `status` | `BatchStatus` | `BatchStatus::Validating` | Current job status. |
| `output_file_id` | `Option<String>` | `Default::default()` | ID of the output file (present when completed). |
| `error_file_id` | `Option<String>` | `Default::default()` | ID of the error file (present if some requests failed). |
| `created_at` | `u64` | — | Unix timestamp of batch creation. |
| `completed_at` | `Option<u64>` | `Default::default()` | Unix timestamp of completion (if completed). |
| `failed_at` | `Option<u64>` | `Default::default()` | Unix timestamp of failure (if failed). |
| `expired_at` | `Option<u64>` | `Default::default()` | Unix timestamp of expiration (if expired). |
| `request_counts` | `Option<BatchRequestCounts>` | `Default::default()` | Request processing counts. |
| `metadata` | `Option<serde_json::Value>` | `Default::default()` | Metadata attached to the batch. |

---

#### BatchRequestCounts

Request processing counts for a batch.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total` | `u64` | — | Total requests in the batch. |
| `completed` | `u64` | — | Completed requests. |
| `failed` | `u64` | — | Failed requests. |

---

#### BudgetConfig

Configuration for budget enforcement.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `global_limit` | `Option<f64>` | `None` | Maximum total spend across all models, in USD.  `None` means unlimited. |
| `model_limits` | `HashMap<String, f64>` | `HashMap::new()` | Per-model spending limits in USD.  Models not listed here are only constrained by `global_limit`. |
| `enforcement` | `Enforcement` | `Enforcement::Hard` | Whether to reject requests or merely warn when a limit is exceeded. |

##### Methods

###### default()

**Signature:**

```rust
pub fn default() -> BudgetConfig
```

**Example:**

```rust
let result = BudgetConfig::default();
```

**Returns:** `BudgetConfig`

---

#### CacheConfig

Configuration for the response cache.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_entries` | `usize` | `256` | Maximum number of cached entries. |
| `ttl` | `std::time::Duration` | `300000ms` | Time-to-live for each cached entry. |
| `backend` | `CacheBackend` | `CacheBackend::Memory` | Storage backend to use. |

##### Methods

###### default()

**Signature:**

```rust
pub fn default() -> CacheConfig
```

**Example:**

```rust
let result = CacheConfig::default();
```

**Returns:** `CacheConfig`

---

#### ChatCompletionChunk

A streamed chunk of a chat completion response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this stream. |
| `object` | `String` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created` | `u64` | — | Unix timestamp of chunk creation. |
| `model` | `String` | — | Model used to generate the chunk. |
| `choices` | `Vec<StreamChoice>` | `vec![]` | Streaming choices (delta updates). |
| `usage` | `Option<Usage>` | `Default::default()` | Token usage (typically only in the final chunk). |
| `system_fingerprint` | `Option<String>` | `Default::default()` | Fingerprint of the system configuration (OpenAI-specific). |
| `service_tier` | `Option<String>` | `Default::default()` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`). |
| `messages` | `Vec<Message>` | `vec![]` | Conversation history from oldest to newest. |
| `temperature` | `Option<f64>` | `Default::default()` | Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0. |
| `top_p` | `Option<f64>` | `Default::default()` | Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused. |
| `n` | `Option<u32>` | `Default::default()` | Number of chat completions to generate. Defaults to 1. |
| `stream` | `Option<bool>` | `Default::default()` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `stop` | `Option<StopSequence>` | `Default::default()` | Stop sequence(s) that halt token generation. |
| `max_tokens` | `Option<u64>` | `Default::default()` | Max output tokens. Different from max_completion_tokens in some providers. |
| `presence_penalty` | `Option<f64>` | `Default::default()` | Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics. |
| `frequency_penalty` | `Option<f64>` | `Default::default()` | Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens. |
| `logit_bias` | `Option<HashMap<String, f64>>` | `HashMap::new()` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user` | `Option<String>` | `Default::default()` | User identifier for request tracking and abuse detection. |
| `tools` | `Option<Vec<ChatCompletionTool>>` | `vec![]` | Tools the model can invoke. |
| `tool_choice` | `Option<ToolChoice>` | `Default::default()` | Tool usage mode (auto, required, none, or specific tool). |
| `parallel_tool_calls` | `Option<bool>` | `Default::default()` | Whether the model can call multiple tools in parallel. Defaults to true. |
| `response_format` | `Option<ResponseFormat>` | `Default::default()` | Output format constraint (text, JSON, JSON schema). |
| `stream_options` | `Option<StreamOptions>` | `Default::default()` | Streaming options (e.g., include_usage). |
| `seed` | `Option<i64>` | `Default::default()` | Random seed for reproducible outputs. Provider support varies. |
| `reasoning_effort` | `Option<ReasoningEffort>` | `Default::default()` | Reasoning effort level (low, medium, high) for extended-thinking models. |
| `extra_body` | `Option<serde_json::Value>` | `Default::default()` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

#### ChatCompletionResponse

Chat completion response from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this response. |
| `object` | `String` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `u64` | — | Unix timestamp of response creation. |
| `model` | `String` | — | Model used to generate the response. |
| `choices` | `Vec<Choice>` | `vec![]` | List of completion choices. |
| `usage` | `Option<Usage>` | `Default::default()` | Token usage statistics. |
| `system_fingerprint` | `Option<String>` | `Default::default()` | Fingerprint of the system configuration (OpenAI-specific). |
| `service_tier` | `Option<String>` | `Default::default()` | Service tier used (OpenAI-specific). |

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
| `index` | `u32` | — | Index of this choice in the choices array. |
| `message` | `AssistantMessage` | — | The assistant's message response. |
| `finish_reason` | `Option<FinishReason>` | `Default::default()` | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

---

#### ChunkMiddleware

A per-chunk transformation in the `StreamPipeline`.

Each middleware receives a typed chunk and returns `Ok(Some(chunk))`
to pass it through (optionally modified), `Ok(None)` to drop the chunk,
or `Err(e)` to propagate a stream error.

The trait is object-safe so multiple middleware implementations can be
chained inside `StreamPipeline`.

##### Methods

###### process()

Process a single chunk.

- `Ok(Some(chunk))` — emit (possibly transformed) chunk.
- `Ok(None)` — drop this chunk silently.
- `Err(e)` — propagate as a stream error.

**Signature:**

```rust
pub fn process(&self, chunk: ChatCompletionChunk) -> Result<Option<ChatCompletionChunk>, Error>
```

**Example:**

```rust
let result = instance.process(ChatCompletionChunk::default())?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `chunk` | `ChatCompletionChunk` | Yes | The chat completion chunk |

**Returns:** `Option<ChatCompletionChunk>`

**Errors:** Returns `Err(Error)`.

---

#### CreateBatchRequest

Request to create a batch job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_file_id` | `String` | — | ID of the uploaded input file (JSONL format). |
| `endpoint` | `String` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `completion_window` | `String` | — | Completion window (e.g., `"24h"`). |
| `metadata` | `Option<serde_json::Value>` | `Default::default()` | Optional metadata to attach to the batch. |

---

#### CreateFileRequest

Request to upload a file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | `String` | — | Base64-encoded file data. |
| `purpose` | `FilePurpose` | `FilePurpose::Assistants` | Purpose for the file. |
| `filename` | `Option<String>` | `Default::default()` | Optional filename to associate with the upload. |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt` | `String` | — | Text description of the image to generate. |
| `model` | `Option<String>` | `Default::default()` | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `n` | `Option<u32>` | `Default::default()` | Number of images to generate. Defaults to 1. |
| `size` | `Option<String>` | `Default::default()` | Image size (e.g., `"1024x1024"`, `"1792x1024"`). |
| `quality` | `Option<String>` | `Default::default()` | Image quality: `"standard"` or `"hd"`. |
| `style` | `Option<String>` | `Default::default()` | Style: `"natural"` or `"vivid"` (DALL-E 3 only). |
| `response_format` | `Option<String>` | `Default::default()` | Response format: `"url"` or `"b64_json"`. |
| `user` | `Option<String>` | `Default::default()` | User identifier for request tracking. |

---

#### CreateResponseRequest

Request to create a structured response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID. |
| `input` | `serde_json::Value` | — | Input data to process (e.g., a document to extract from). |
| `instructions` | `Option<String>` | `Default::default()` | Instructions for processing the input. |
| `tools` | `Option<Vec<ResponseTool>>` | `vec![]` | Available tools the model can use. |
| `temperature` | `Option<f64>` | `Default::default()` | Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0. |
| `max_output_tokens` | `Option<u64>` | `Default::default()` | Maximum output tokens. |
| `metadata` | `Option<serde_json::Value>` | `Default::default()` | Optional metadata. |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`). |
| `input` | `String` | — | Text to synthesize into speech. |
| `voice` | `String` | — | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `response_format` | `Option<String>` | `Default::default()` | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`). |
| `speed` | `Option<f64>` | `Default::default()` | Playback speed in `[0.25, 4.0]`. Defaults to 1.0. |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"whisper-1"`). |
| `file` | `String` | — | Base64-encoded audio file data. |
| `language` | `Option<String>` | `Default::default()` | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `prompt` | `Option<String>` | `Default::default()` | Optional text to guide the model (improves accuracy for domain-specific terms). |
| `response_format` | `Option<String>` | `Default::default()` | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`). |
| `temperature` | `Option<f64>` | `Default::default()` | Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0. |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Unique name for this provider (e.g., "my-provider"). |
| `base_url` | `String` | — | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `auth_header` | `AuthHeaderFormat` | — | Authentication header format. |
| `model_prefixes` | `Vec<String>` | — | Model name prefixes that route to this provider (e.g., `["my-"]`). |

---

#### DefaultClient

Default client implementation backed by `reqwest`.

Sends requests to 143 LLM providers with automatic provider detection
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

##### Methods

###### chat()

**Signature:**

```rust
pub async fn chat(&self, req: ChatCompletionRequest) -> Result<ChatCompletionResponse, Error>
```

**Example:**

```rust
let result = instance.chat(ChatCompletionRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `ChatCompletionResponse`

**Errors:** Returns `Err(Error)`.

###### chat_stream()

**Signature:**

```rust
fn chat_stream(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>>
```

**Example:**

```rust
let mut stream = instance.chat_stream(ChatCompletionRequest::default()).await?;
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    println!("{chunk:?}");
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>>`

**Errors:** Returns `Err(Error)`.

###### embed()

**Signature:**

```rust
pub async fn embed(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, Error>
```

**Example:**

```rust
let result = instance.embed(EmbeddingRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `EmbeddingRequest` | Yes | The embedding request |

**Returns:** `EmbeddingResponse`

**Errors:** Returns `Err(Error)`.

###### list_models()

**Signature:**

```rust
pub async fn list_models(&self) -> Result<ModelsListResponse, Error>
```

**Example:**

```rust
let result = instance.list_models().await?;
```

**Returns:** `ModelsListResponse`

**Errors:** Returns `Err(Error)`.

###### image_generate()

**Signature:**

```rust
pub async fn image_generate(&self, req: CreateImageRequest) -> Result<ImagesResponse, Error>
```

**Example:**

```rust
let result = instance.image_generate(CreateImageRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `CreateImageRequest` | Yes | The create image request |

**Returns:** `ImagesResponse`

**Errors:** Returns `Err(Error)`.

###### speech()

**Signature:**

```rust
pub async fn speech(&self, req: CreateSpeechRequest) -> Result<Vec<u8>, Error>
```

**Example:**

```rust
let result = instance.speech(CreateSpeechRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `CreateSpeechRequest` | Yes | The create speech request |

**Returns:** `Vec<u8>`

**Errors:** Returns `Err(Error)`.

###### transcribe()

**Signature:**

```rust
pub async fn transcribe(&self, req: CreateTranscriptionRequest) -> Result<TranscriptionResponse, Error>
```

**Example:**

```rust
let result = instance.transcribe(CreateTranscriptionRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `CreateTranscriptionRequest` | Yes | The create transcription request |

**Returns:** `TranscriptionResponse`

**Errors:** Returns `Err(Error)`.

###### moderate()

**Signature:**

```rust
pub async fn moderate(&self, req: ModerationRequest) -> Result<ModerationResponse, Error>
```

**Example:**

```rust
let result = instance.moderate(ModerationRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `ModerationRequest` | Yes | The moderation request |

**Returns:** `ModerationResponse`

**Errors:** Returns `Err(Error)`.

###### rerank()

**Signature:**

```rust
pub async fn rerank(&self, req: RerankRequest) -> Result<RerankResponse, Error>
```

**Example:**

```rust
let result = instance.rerank(RerankRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `RerankRequest` | Yes | The rerank request |

**Returns:** `RerankResponse`

**Errors:** Returns `Err(Error)`.

###### search()

**Signature:**

```rust
pub async fn search(&self, req: SearchRequest) -> Result<SearchResponse, Error>
```

**Example:**

```rust
let result = instance.search(SearchRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `SearchRequest` | Yes | The search request |

**Returns:** `SearchResponse`

**Errors:** Returns `Err(Error)`.

###### ocr()

**Signature:**

```rust
pub async fn ocr(&self, req: OcrRequest) -> Result<OcrResponse, Error>
```

**Example:**

```rust
let result = instance.ocr(OcrRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `OcrRequest` | Yes | The ocr request |

**Returns:** `OcrResponse`

**Errors:** Returns `Err(Error)`.

###### create_file()

**Signature:**

```rust
pub async fn create_file(&self, req: CreateFileRequest) -> Result<FileObject, Error>
```

**Example:**

```rust
let result = instance.create_file(CreateFileRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `CreateFileRequest` | Yes | The create file request |

**Returns:** `FileObject`

**Errors:** Returns `Err(Error)`.

###### retrieve_file()

**Signature:**

```rust
pub async fn retrieve_file(&self, file_id: &str) -> Result<FileObject, Error>
```

**Example:**

```rust
let result = instance.retrieve_file("value").await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `file_id` | `String` | Yes | The file id |

**Returns:** `FileObject`

**Errors:** Returns `Err(Error)`.

###### delete_file()

**Signature:**

```rust
pub async fn delete_file(&self, file_id: &str) -> Result<DeleteResponse, Error>
```

**Example:**

```rust
let result = instance.delete_file("value").await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `file_id` | `String` | Yes | The file id |

**Returns:** `DeleteResponse`

**Errors:** Returns `Err(Error)`.

###### list_files()

**Signature:**

```rust
pub async fn list_files(&self, query: Option<FileListQuery>) -> Result<FileListResponse, Error>
```

**Example:**

```rust
let result = instance.list_files(FileListQuery::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `Option<FileListQuery>` | No | The file list query |

**Returns:** `FileListResponse`

**Errors:** Returns `Err(Error)`.

###### file_content()

**Signature:**

```rust
pub async fn file_content(&self, file_id: &str) -> Result<Vec<u8>, Error>
```

**Example:**

```rust
let result = instance.file_content("value").await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `file_id` | `String` | Yes | The file id |

**Returns:** `Vec<u8>`

**Errors:** Returns `Err(Error)`.

###### create_batch()

**Signature:**

```rust
pub async fn create_batch(&self, req: CreateBatchRequest) -> Result<BatchObject, Error>
```

**Example:**

```rust
let result = instance.create_batch(CreateBatchRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `CreateBatchRequest` | Yes | The create batch request |

**Returns:** `BatchObject`

**Errors:** Returns `Err(Error)`.

###### retrieve_batch()

**Signature:**

```rust
pub async fn retrieve_batch(&self, batch_id: &str) -> Result<BatchObject, Error>
```

**Example:**

```rust
let result = instance.retrieve_batch("value").await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `batch_id` | `String` | Yes | The batch id |

**Returns:** `BatchObject`

**Errors:** Returns `Err(Error)`.

###### list_batches()

**Signature:**

```rust
pub async fn list_batches(&self, query: Option<BatchListQuery>) -> Result<BatchListResponse, Error>
```

**Example:**

```rust
let result = instance.list_batches(BatchListQuery::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `Option<BatchListQuery>` | No | The batch list query |

**Returns:** `BatchListResponse`

**Errors:** Returns `Err(Error)`.

###### cancel_batch()

**Signature:**

```rust
pub async fn cancel_batch(&self, batch_id: &str) -> Result<BatchObject, Error>
```

**Example:**

```rust
let result = instance.cancel_batch("value").await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `batch_id` | `String` | Yes | The batch id |

**Returns:** `BatchObject`

**Errors:** Returns `Err(Error)`.

###### fetch_batch_for_polling()

**Signature:**

```rust
pub async fn fetch_batch_for_polling(&self, batch_id: &str) -> Result<BatchObject, Error>
```

**Example:**

```rust
let result = instance.fetch_batch_for_polling("value").await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `batch_id` | `String` | Yes | The batch id |

**Returns:** `BatchObject`

**Errors:** Returns `Err(Error)`.

###### wait_for_batch()

Poll a batch until it reaches a terminal status (Completed, Failed, Expired, Cancelled).

Uses exponential backoff with configurable initial interval, maximum interval, and backoff multiplier.
Optionally supports a timeout that aborts polling if exceeded.

**Errors:**

Returns `BatchWaitError::Failed` if the batch reaches a failure terminal status.
Returns `BatchWaitError::Timeout` if the configured timeout is exceeded.
Returns `BatchWaitError::Client` for underlying client errors.

**Signature:**

```rust
pub async fn wait_for_batch(&self, batch_id: &str, config: WaitForBatchConfig) -> Result<BatchObject, BatchWaitError>
```

**Example:**

```rust
# use liter_llm::client::{DefaultClient, ClientConfig, WaitForBatchConfig};
# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let client = DefaultClient::new(ClientConfig::new("api-key"), None)?;
let batch = client.wait_for_batch("b-123", WaitForBatchConfig::default()).await?;
println!("Batch completed: {:?}", batch.status);
# Ok(())
# }
```rust

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `batch_id` | `String` | Yes | The batch id |
| `config` | `WaitForBatchConfig` | Yes | The configuration options |

**Returns:** `BatchObject`

**Errors:** Returns `Err(BatchWaitError)`.

###### create_response()

**Signature:**

```rust
pub async fn create_response(&self, req: CreateResponseRequest) -> Result<ResponseObject, Error>
```

**Example:**

```rust
let result = instance.create_response(CreateResponseRequest::default()).await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `req` | `CreateResponseRequest` | Yes | The create response request |

**Returns:** `ResponseObject`

**Errors:** Returns `Err(Error)`.

###### retrieve_response()

**Signature:**

```rust
pub async fn retrieve_response(&self, response_id: &str) -> Result<ResponseObject, Error>
```

**Example:**

```rust
let result = instance.retrieve_response("value").await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `response_id` | `String` | Yes | The response id |

**Returns:** `ResponseObject`

**Errors:** Returns `Err(Error)`.

###### cancel_response()

**Signature:**

```rust
pub async fn cancel_response(&self, response_id: &str) -> Result<ResponseObject, Error>
```

**Example:**

```rust
let result = instance.cancel_response("value").await?;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `response_id` | `String` | Yes | The response id |

**Returns:** `ResponseObject`

**Errors:** Returns `Err(Error)`.

---

#### DeleteResponse

Response from a delete operation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | ID of the deleted resource. |
| `object` | `String` | — | Object type. |
| `deleted` | `bool` | — | Confirmation that the resource was deleted. |

---

#### DeveloperMessage

Developer message (system-like message for Claude models).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Developer-specific instructions or context. |
| `name` | `Option<String>` | `Default::default()` | Optional name for the developer message source. |

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
| `embedding` | `Vec<f64>` | — | The embedding vector. |
| `index` | `u32` | — | Index in the batch (corresponds to input order). |

---

#### EmbeddingRequest

Embedding request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"text-embedding-3-small"`). |
| `input` | `EmbeddingInput` | `EmbeddingInput::Single` | Text or texts to embed. |
| `encoding_format` | `Option<EmbeddingFormat>` | `Default::default()` | Output format: float (native) or base64. |
| `dimensions` | `Option<u32>` | `Default::default()` | Requested embedding dimensions (if supported by the model). |
| `user` | `Option<String>` | `Default::default()` | User identifier for request tracking. |

---

#### EmbeddingResponse

Embedding response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `Vec<EmbeddingObject>` | — | List of embeddings. |
| `model` | `String` | — | Model used to generate embeddings. |
| `usage` | `Option<Usage>` | language default | Token usage (input tokens only; embeddings have zero output tokens). |

---

#### FileListQuery

Query parameters for listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `purpose` | `Option<String>` | `Default::default()` | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `limit` | `Option<u32>` | `Default::default()` | Maximum number of results to return. Defaults to 20. |
| `after` | `Option<String>` | `Default::default()` | Pagination cursor: return results after this file ID. |

---

#### FileListResponse

Response from listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Object type (always `"list"`). |
| `data` | `Vec<FileObject>` | `vec![]` | List of file objects. |
| `has_more` | `Option<bool>` | `Default::default()` | Whether more results are available. |

---

#### FileObject

An uploaded file object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique file ID. |
| `object` | `String` | — | Object type (always `"file"`). |
| `bytes` | `u64` | — | File size in bytes. |
| `created_at` | `u64` | — | Unix timestamp of file creation. |
| `filename` | `String` | — | Filename. |
| `purpose` | `String` | — | File purpose. |
| `status` | `Option<String>` | `Default::default()` | Processing status (e.g., `"uploaded"`, `"processed"`). |

---

#### FunctionCall

Function call details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Function name. |
| `arguments` | `String` | — | Arguments as a JSON string (parse with serde_json::from_str). |

---

#### FunctionDefinition

Function definition exposed to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Name of the function. Required and must be alphanumeric + underscores. |
| `description` | `Option<String>` | language default | Human-readable description explaining what the function does. |
| `parameters` | `Option<serde_json::Value>` | language default | JSON Schema defining the function's parameters. |
| `strict` | `Option<bool>` | language default | If true, enforce strict JSON schema validation for arguments. |

---

#### FunctionMessage

Deprecated legacy function-role message body.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | The extracted text content |
| `name` | `String` | — | The name |

---

#### HealthChecker

Abstraction over a health probe strategy.

Implementors issue a lightweight probe against `upstream` (typically a
provider base URL or named identifier) and report `HealthStatus`.

##### Methods

###### check()

Probe `upstream` and return its current `HealthStatus`.

The parameter is taken by value (`String`) so that implementations can
move it into the returned future without a clone, making the
`'static + Send` bound on the future trivially satisfiable.

**Signature:**

```rust
pub async fn check(&self, upstream: &str) -> HealthStatus
```

**Example:**

```rust
let result = instance.check("value").await;
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `upstream` | `String` | Yes | The upstream |

**Returns:** `HealthStatus`

---

#### Image

A single generated image, returned as either a URL or base64 data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `Option<String>` | `Default::default()` | Image URL (if response_format was "url"). |
| `b64_json` | `Option<String>` | `Default::default()` | Base64-encoded image data (if response_format was "b64_json"). |
| `revised_prompt` | `Option<String>` | `Default::default()` | The final prompt used to generate the image (DALL-E 3). |

---

#### ImageUrl

An image URL reference with optional detail level for processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | URL of the image (data URI or HTTP/HTTPS URL). |
| `detail` | `Option<ImageDetail>` | `Default::default()` | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### ImagesResponse

Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `created` | `u64` | — | Unix timestamp of image creation. |
| `data` | `Vec<Image>` | `vec![]` | List of generated images. |

---

#### IntentPrototype

An intent prototype: `(intent_name, prototype_embedding, target_model_id)`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Human-readable name for the intent (used in logs/metrics). |
| `embedding` | `Vec<f64>` | — | Pre-computed embedding vector for this intent. |
| `model` | `String` | — | Model to route to when this intent is detected. |

---

#### JsonSchemaFormat

JSON Schema specification for constrained output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Name of the schema (must be unique in the request). |
| `description` | `Option<String>` | `Default::default()` | Description of what the schema represents. |
| `schema` | `serde_json::Value` | — | JSON Schema object defining the output structure. |
| `strict` | `Option<bool>` | `Default::default()` | If true, enforce strict schema validation. |

---

#### ModelObject

A model available from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`). |
| `object` | `String` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `u64` | — | Unix timestamp of model creation (or release date). |
| `owned_by` | `String` | — | Organization or entity that owns the model. |

---

#### ModelsListResponse

Response listing available models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `Vec<ModelObject>` | `vec![]` | List of available models. |

---

#### ModerationCategories

Boolean flags for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `bool` | — | Sexual content. |
| `hate` | `bool` | — | Hate speech. |
| `harassment` | `bool` | — | Harassment. |
| `self_harm` | `bool` | — | Self-harm content. |
| `sexual_minors` | `bool` | — | Sexual content involving minors. |
| `hate_threatening` | `bool` | — | Hate speech that threatens violence. |
| `violence_graphic` | `bool` | — | Graphic violence. |
| `self_harm_intent` | `bool` | — | Intent to self-harm. |
| `self_harm_instructions` | `bool` | — | Instructions for self-harm. |
| `harassment_threatening` | `bool` | — | Harassment that threatens violence. |
| `violence` | `bool` | — | Non-graphic violence. |

---

#### ModerationCategoryScores

Confidence scores for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `f64` | — | Sexual content score. |
| `hate` | `f64` | — | Hate speech score. |
| `harassment` | `f64` | — | Harassment score. |
| `self_harm` | `f64` | — | Self-harm content score. |
| `sexual_minors` | `f64` | — | Sexual content involving minors score. |
| `hate_threatening` | `f64` | — | Hate speech that threatens violence score. |
| `violence_graphic` | `f64` | — | Graphic violence score. |
| `self_harm_intent` | `f64` | — | Intent to self-harm score. |
| `self_harm_instructions` | `f64` | — | Instructions for self-harm score. |
| `harassment_threatening` | `f64` | — | Harassment that threatens violence score. |
| `violence` | `f64` | — | Non-graphic violence score. |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input` | `ModerationInput` | `ModerationInput::Single` | Text or texts to check. |
| `model` | `Option<String>` | `Default::default()` | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this moderation request. |
| `model` | `String` | — | Model used for classification. |
| `results` | `Vec<ModerationResult>` | — | Results for each input string. |

---

#### ModerationResult

A single moderation classification result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `flagged` | `bool` | — | True if any category was flagged. |
| `categories` | `ModerationCategories` | — | Boolean flags for each moderation category. |
| `category_scores` | `ModerationCategoryScores` | — | Confidence scores for each category. |

---

#### OcrImage

An image extracted from an OCR page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique image identifier within the document. |
| `image_base64` | `Option<String>` | language default | Base64-encoded image data (if `include_image_base64` was true). |

---

#### OcrPage

A single page of OCR output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `u32` | — | Page index (0-based). |
| `markdown` | `String` | — | Extracted page content as Markdown. |
| `images` | `Option<Vec<OcrImage>>` | language default | Embedded images extracted from the page (if `include_image_base64` was true). |
| `dimensions` | `Option<PageDimensions>` | language default | Page dimensions in pixels, if available. |

---

#### OcrRequest

An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document` | `OcrDocument` | `OcrDocument::Url` | The document to process (URL or base64). |
| `pages` | `Option<Vec<u32>>` | `vec![]` | Specific pages to process (1-indexed). `None` means all pages. |
| `include_image_base64` | `Option<bool>` | `Default::default()` | Whether to include base64-encoded images of each processed page. |

---

#### OcrResponse

An OCR response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pages` | `Vec<OcrPage>` | — | Extracted pages in order. |
| `model` | `String` | — | Model/provider used for OCR. |
| `usage` | `Option<Usage>` | language default | Token usage, if reported by the provider. |

---

#### PageDimensions

Page dimensions in pixels.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `u32` | — | Width in pixels. |
| `height` | `u32` | — | Height in pixels. |

---

#### PromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage::prompt_tokens` — it is *not* an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cached_tokens` | `u64` | — | Cached tokens present in the prompt. Defaults to 0 when absent. |
| `audio_tokens` | `u64` | — | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### ProviderCapabilities

Static capability flags for a provider.

Each flag indicates whether the provider's models *generally* support that
feature.  For providers that aggregate many underlying models (e.g. Bedrock,
OpenRouter, vLLM) the flags reflect the superset of available model
capabilities — a flag being `true` means at least one model supports the
feature, not every model.

All flags default to `false` so that newly added providers are safe.

Access via the crate-level `capabilities` function:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `vision` | `bool` | — | The provider accepts image input in chat messages. |
| `reasoning` | `bool` | — | The provider supports extended-thinking / reasoning tokens. |
| `structured_output` | `bool` | — | The provider supports JSON-mode or `response_format` structured output. |
| `function_calling` | `bool` | — | The provider supports tool / function calling. |
| `audio_in` | `bool` | — | The provider accepts audio as input. |
| `audio_out` | `bool` | — | The provider can generate audio / TTS output. |
| `video_in` | `bool` | — | The provider accepts video as input. |

---

#### ProviderConfig

Static configuration for a single provider entry in providers.json.

This struct deliberately does not include capability flags or streaming
format, which are accessed via the `capabilities` function.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Provider identifier (matches the entry key in providers.json). |
| `display_name` | `Option<String>` | `None` | Human-readable provider name shown in UIs. |
| `base_url` | `Option<String>` | `None` | Base URL used as the default for this provider's HTTP client. |
| `auth` | `Option<AuthConfig>` | `None` | Authentication scheme metadata (auth type + env var holding the key). |
| `endpoints` | `Option<Vec<String>>` | `None` | Supported endpoint kinds (e.g. `chat`, `embeddings`). |
| `model_prefixes` | `Option<Vec<String>>` | `None` | Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`). |
| `param_mappings` | `Option<HashMap<String, String>>` | `None` | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`).  Applied automatically by `ConfigDrivenProvider::transform_request`. |

---

#### RateLimitConfig

Configuration for per-model rate limits.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rpm` | `Option<u32>` | `None` | Maximum requests per window.  `None` means unlimited. |
| `tpm` | `Option<u64>` | `None` | Maximum tokens per window.  `None` means unlimited. |
| `window` | `std::time::Duration` | `60000ms` | Fixed window duration (defaults to 60 s). |

##### Methods

###### default()

**Signature:**

```rust
pub fn default() -> RateLimitConfig
```

**Example:**

```rust
let result = RateLimitConfig::default();
```

**Returns:** `RateLimitConfig`

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"cohere/rerank-english-v3.0"`). |
| `query` | `String` | — | The search query. |
| `documents` | `Vec<RerankDocument>` | `vec![]` | Documents to rerank. |
| `top_n` | `Option<u32>` | `Default::default()` | Return only the top N results. Optional. |
| `return_documents` | `Option<bool>` | `Default::default()` | Include the document content in results. Defaults to false. |

---

#### RerankResponse

Response from the rerank endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `Option<String>` | `None` | Unique identifier for this rerank request. |
| `results` | `Vec<RerankResult>` | — | Reranked documents in order of relevance. |
| `meta` | `Option<serde_json::Value>` | language default | Optional metadata about the reranking operation. |

---

#### RerankResult

A single reranked document with its relevance score.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `u32` | — | Original document index in the input list. |
| `relevance_score` | `f64` | — | Relevance score in `[0, 1]`. Higher indicates more relevant. |
| `document` | `Option<RerankResultDocument>` | language default | Original document content (if `return_documents` was true). |

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
| `created_at` | `u64` | — | Unix timestamp of response creation. |
| `model` | `String` | — | Model used to generate the response. |
| `status` | `String` | — | Status (e.g., `"succeeded"`, `"failed"`). |
| `output` | `Vec<ResponseOutputItem>` | `vec![]` | Output items from the response. |
| `usage` | `Option<ResponseUsage>` | `Default::default()` | Token usage. |
| `error` | `Option<serde_json::Value>` | `Default::default()` | Error details (if status is "failed"). |

---

#### ResponseOutputItem

A single output item from the response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `item_type` | `String` | — | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `content` | `serde_json::Value` | — | Output content (flattened into the object). |

---

#### ResponseTool

A tool available for the response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tool_type` | `String` | — | Tool type (e.g., "extractor", "search"). |
| `config` | `serde_json::Value` | — | Tool configuration (flattened into the object). |

---

#### ResponseUsage

Token usage for a response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_tokens` | `u64` | — | Input tokens used. |
| `output_tokens` | `u64` | — | Output tokens used. |
| `total_tokens` | `u64` | — | Total tokens used. |

---

#### SearchRequest

A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query` | `String` | — | The search query string. |
| `max_results` | `Option<u32>` | `Default::default()` | Maximum number of results to return. |
| `search_domain_filter` | `Option<Vec<String>>` | `vec![]` | Domain filter — restrict results to specific domains. |
| `country` | `Option<String>` | `Default::default()` | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

---

#### SearchResponse

A search response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `results` | `Vec<SearchResult>` | — | List of search results. |
| `model` | `String` | — | Model/provider that performed the search. |

---

#### SearchResult

An individual search result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String` | — | Result title. |
| `url` | `String` | — | Result URL. |
| `snippet` | `String` | — | Text snippet or excerpt from the page. |
| `date` | `Option<String>` | language default | Publication or last-updated date, if available. |

---

#### SingleflightResult

The value broadcast from a singleflight leader to all followers.

The error value is shared so every follower receives the same upstream
failure without cloning the underlying error.

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
| `choice_type` | `ToolType` | `ToolType::Function` | Tool type (always "function"). |
| `function` | `SpecificFunction` | — | The specific function to invoke. |

---

#### StreamChoice

A streaming choice with incremental delta.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `u32` | — | Index of this choice in the choices array. |
| `delta` | `StreamDelta` | — | Incremental update to the message (content, tool calls, etc.). |
| `finish_reason` | `Option<FinishReason>` | `Default::default()` | Why the stream ended (present only in final chunk). |

---

#### StreamDelta

Incremental delta in a stream chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `role` | `Option<String>` | `Default::default()` | Role (typically present only in the first chunk). |
| `content` | `Option<String>` | `Default::default()` | Partial content chunk (e.g., a few words of the response). |
| `tool_calls` | `Option<Vec<StreamToolCall>>` | `vec![]` | Partial tool calls being streamed. |
| `function_call` | `Option<StreamFunctionCall>` | `Default::default()` | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal` | `Option<String>` | `Default::default()` | Partial refusal message. |

---

#### StreamFunctionCall

Partial function call details in a stream.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `Option<String>` | `Default::default()` | Function name (typically in the first chunk). |
| `arguments` | `Option<String>` | `Default::default()` | Partial JSON arguments chunk. |

---

#### StreamOptions

Options for streaming responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_usage` | `Option<bool>` | `Default::default()` | If true, include token usage in the final stream chunk. |

---

#### StreamToolCall

A streaming tool call being built incrementally.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `u32` | — | Index of this tool call in the tool_calls array. |
| `id` | `Option<String>` | `Default::default()` | Tool call ID (typically in the first chunk for this call). |
| `call_type` | `Option<ToolType>` | `Default::default()` | Tool type (typically "function"). |
| `function` | `Option<StreamFunctionCall>` | `Default::default()` | Partial function name and arguments. |

---

#### SystemMessage

System message guiding model behavior for the entire conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Instructions or context that apply throughout the conversation. |
| `name` | `Option<String>` | `Default::default()` | Optional name for the system message source. |

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
| `name` | `Option<String>` | `Default::default()` | Optional tool/function name. |

---

#### TranscriptionResponse

Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The transcribed text. |
| `language` | `Option<String>` | `Default::default()` | Detected language (ISO-639-1 code). |
| `duration` | `Option<f64>` | `Default::default()` | Total audio duration in seconds. |
| `segments` | `Option<Vec<TranscriptionSegment>>` | `vec![]` | Detailed segment-level transcription (if response_format is "verbose_json"). |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `u32` | — | Segment index (0-based). |
| `start` | `f64` | — | Start time in seconds. |
| `end` | `f64` | — | End time in seconds. |
| `text` | `String` | — | Transcribed text for this segment. |

---

#### Usage

Token-usage accounting returned by the provider on each completion / embedding call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_tokens` | `u64` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `completion_tokens` | `u64` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `total_tokens` | `u64` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `prompt_tokens_details` | `Option<PromptTokensDetails>` | `Default::default()` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

User message in the conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `UserContent::Text` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `name` | `Option<String>` | `Default::default()` | Optional name for the user. |

---

#### WaitForBatchConfig

Configuration for polling a batch until terminal status.

All time values are in seconds as `f64` so the struct bridges across FFI
boundaries without requiring a `Duration` shim.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `initial_interval_secs` | `f64` | `5` | Initial interval between polls, in seconds. |
| `max_interval_secs` | `f64` | `60` | Maximum interval between polls (backoff plateau), in seconds. |
| `backoff_multiplier` | `f32` | `1.5` | Exponential backoff multiplier (e.g., 1.5 increases delay by 50% each poll). |
| `timeout_secs` | `Option<f64>` | `None` | Optional timeout in seconds — polling fails if this duration is exceeded. |

##### Methods

###### default()

**Signature:**

```rust
pub fn default() -> WaitForBatchConfig
```

**Example:**

```rust
let result = WaitForBatchConfig::default();
```

**Returns:** `WaitForBatchConfig`

---

### Enums

#### Message

A chat message in a conversation.

| Value | Description |
|-------|-------------|
| `System` | System — Fields: `0`: `SystemMessage` |
| `User` | User — Fields: `0`: `UserMessage` |
| `Assistant` | Assistant — Fields: `0`: `AssistantMessage` |
| `Tool` | Tool — Fields: `0`: `ToolMessage` |
| `Developer` | Developer — Fields: `0`: `DeveloperMessage` |
| `Function` | Deprecated legacy function-role message; retained for API compatibility. — Fields: `0`: `FunctionMessage` |

---

#### UserContent

User message content as either plain text or a list of multimodal parts.

| Value | Description |
|-------|-------------|
| `Text` | Plain text content. — Fields: `0`: `String` |
| `Parts` | Array of content parts (text, images, documents, audio). — Fields: `0`: `Vec<ContentPart>` |

---

#### ContentPart

A single content part in a user message — text, image, document, or audio.

| Value | Description |
|-------|-------------|
| `Text` | Plain text. — Fields: `text`: `String` |
| `ImageUrl` | Image identified by URL (with optional detail level). — Fields: `image_url`: `ImageUrl` |
| `Document` | Document file (PDF, CSV, etc.) as base64 or URL. — Fields: `document`: `DocumentContent` |
| `InputAudio` | Audio input as base64. — Fields: `input_audio`: `AudioContent` |

---

#### ImageDetail

Image detail level controlling token cost and processing.

| Value | Description |
|-------|-------------|
| `Low` | Low detail: scales image to 512x512, uses fewer tokens. |
| `High` | High detail: processes up to 2x2 grid of tiles, higher token cost. |
| `Auto` | Auto: model chooses low or high based on image dimensions. |

---

#### ToolType

The type discriminator for tool/tool-call objects.

Per the OpenAI spec this is always `"function"`. Using an enum enforces
that constraint at the type level and rejects any other value on
deserialization.

| Value | Description |
|-------|-------------|
| `Function` | Function |

---

#### ToolChoice

Tool usage mode or a specific tool to call.

| Value | Description |
|-------|-------------|
| `Mode` | Predefined mode: auto, required, or none. — Fields: `0`: `ToolChoiceMode` |
| `Specific` | Force a specific tool to be called. — Fields: `0`: `SpecificToolChoice` |

---

#### ToolChoiceMode

Tool choice mode.

| Value | Description |
|-------|-------------|
| `Auto` | Model may or may not call tools; default behavior. |
| `Required` | Model must call at least one tool. |
| `None` | Model must not call any tools. |

---

#### ResponseFormat

Response format constraint.

| Value | Description |
|-------|-------------|
| `Text` | Plain text output (default). |
| `JsonObject` | Output must be valid JSON object (no schema validation). |
| `JsonSchema` | Output must conform to the specified JSON schema. — Fields: `json_schema`: `JsonSchemaFormat` |

---

#### StopSequence

Stop sequence(s) that cause the model to stop generating.

| Value | Description |
|-------|-------------|
| `Single` | Single stop sequence. — Fields: `0`: `String` |
| `Multiple` | Multiple stop sequences. — Fields: `0`: `Vec<String>` |

---

#### FinishReason

Why a choice stopped generating tokens.

| Value | Description |
|-------|-------------|
| `Stop` | Stop |
| `Length` | Length |
| `ToolCalls` | Tool calls |
| `ContentFilter` | Content filter |
| `FunctionCall` | Deprecated legacy finish reason; retained for API compatibility. |
| `Other` | Catch-all for unknown finish reasons returned by non-OpenAI providers. Note: this intentionally does **not** carry the original string (e.g. `Other(String)`).  Using `#[serde(other)]` requires a unit variant, and switching to `#[serde(untagged)]` would change deserialization semantics for all variants.  The original value can be recovered by inspecting the raw JSON if needed. |

---

#### ReasoningEffort

Controls how much reasoning effort the model should use.

| Value | Description |
|-------|-------------|
| `Low` | Low |
| `Medium` | Medium |
| `High` | High |

---

#### EmbeddingFormat

The format in which the embedding vectors are returned.

| Value | Description |
|-------|-------------|
| `Float` | 32-bit floating-point numbers (default). |
| `Base64` | Base64-encoded string representation of the floats. |

---

#### EmbeddingInput

Text or texts to embed.

| Value | Description |
|-------|-------------|
| `Single` | Single text string. — Fields: `0`: `String` |
| `Multiple` | Multiple text strings (batch embedding). — Fields: `0`: `Vec<String>` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value | Description |
|-------|-------------|
| `Single` | Single text string. — Fields: `0`: `String` |
| `Multiple` | Multiple text strings (batch moderation). — Fields: `0`: `Vec<String>` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value | Description |
|-------|-------------|
| `Text` | Plain text document content. — Fields: `0`: `String` |
| `Object` | Document with explicit text field (may include metadata). — Fields: `text`: `String` |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value | Description |
|-------|-------------|
| `Url` | A publicly accessible document URL. — Fields: `url`: `String` |
| `Base64` | Inline base64-encoded document data. — Fields: `data`: `String`, `media_type`: `String` |

---

#### FilePurpose

Purpose of an uploaded file.

| Value | Description |
|-------|-------------|
| `Assistants` | File for use with Assistants API. |
| `Batch` | File for batch processing. |
| `FineTune` | File for fine-tuning. |
| `Vision` | File for vision/image tasks. |

---

#### BatchStatus

Status of a batch job.

| Value | Description |
|-------|-------------|
| `Validating` | Validating the input file. |
| `Failed` | Job failed. |
| `InProgress` | Job is running. |
| `Finalizing` | Finalizing results. |
| `Completed` | Job completed successfully. |
| `Expired` | Job expired before completion. |
| `Cancelling` | Job is being cancelled. |
| `Cancelled` | Job has been cancelled. |

---

#### AuthHeaderFormat

How the API key is sent in the HTTP request.

| Value | Description |
|-------|-------------|
| `Bearer` | Bearer token: `Authorization: Bearer <key>` |
| `ApiKey` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `String` |
| `None` | No authentication required. |

---

#### StreamFormat

The streaming wire format a provider uses for its response stream.

Most providers use standard Server-Sent Events (SSE).  AWS Bedrock uses
a proprietary binary EventStream framing.

Deserialized from the `streaming_format` JSON field via `serde`.

| Value | Description |
|-------|-------------|
| `Sse` | Standard Server-Sent Events (text/event-stream). |
| `AwsEventStream` | AWS EventStream binary framing (application/vnd.amazon.eventstream). |

---

#### AuthType

Auth scheme used by a provider.

| Value | Description |
|-------|-------------|
| `Bearer` | Standard `Authorization: Bearer <key>` header. |
| `ApiKey` | `x-api-key: <key>` header (also handles `"header"` and `"x-api-key"` aliases). |
| `None` | No authentication header required. |
| `Unknown` | Unrecognised auth scheme — falls back to bearer. |

---

#### Enforcement

How budget limits are enforced.

| Value | Description |
|-------|-------------|
| `Hard` | Reject requests that would exceed the budget with `LiterLlmError::BudgetExceeded`. |
| `Soft` | Allow requests through but emit a `tracing::warn!` when the budget is exceeded. |

---

#### CacheBackend

Storage backend for the response cache.

| Value | Description |
|-------|-------------|
| `Memory` | In-memory LRU cache (default). No external dependencies. |
| `OpenDal` | OpenDAL-backed storage. Supports 40+ backends (S3, Redis, GCS, local FS, etc.). — Fields: `scheme`: `String`, `config`: `HashMap<String, String>` |

---

#### CircuitState

Observable state of a circuit breaker.

| Value | Description |
|-------|-------------|
| `Closed` | Requests flow through normally. |
| `Open` | All requests are rejected; the circuit is waiting for the backoff to elapse. |
| `HalfOpen` | One probe request is allowed through to test service health. |

---

#### HealthStatus

The result of a single health probe.

| Value | Description |
|-------|-------------|
| `Healthy` | The probe succeeded; the upstream is reachable. |
| `Unhealthy` | The probe failed; the upstream may be down. |

---

### Errors

#### LiterLlmError

All errors that can occur when using `liter-llm`.

| Variant | Description |
|---------|-------------|
| `Authentication` | `status` preserves the exact HTTP status code received (401 or 403). |
| `RateLimited` | rate limited: {message} |
| `BadRequest` | `status` preserves the exact HTTP status code received (400, 405, 413, 422, …). |
| `ContextWindowExceeded` | context window exceeded: {message} |
| `ContentPolicy` | content policy violation: {message} |
| `NotFound` | not found: {message} |
| `ServerError` | `status` preserves the exact HTTP status code received (500, or other 5xx not covered by `ServiceUnavailable`). |
| `ServiceUnavailable` | `status` preserves the exact HTTP status code received (502, 503, or 504). |
| `Timeout` | request timeout |
| `Streaming` | A catch-all for errors that occur during streaming response processing. This variant covers multiple sub-conditions including UTF-8 decoding failures, CRC/checksum mismatches (AWS EventStream), JSON parse errors in individual SSE chunks, and buffer overflow conditions.  The `message` field contains a human-readable description of the specific failure. |
| `EndpointNotSupported` | provider {provider} does not support {endpoint} |
| `InvalidHeader` | invalid header {name:?}: {reason} |
| `Serialization` | serialization error: {0} |
| `BudgetExceeded` | budget exceeded: {message} |
| `HookRejected` | hook rejected: {message} |
| `InternalError` | An internal logic error (e.g. unexpected Tower response variant). This should never surface in normal operation — if it does, it indicates a bug in the library. |
| `OutboundForbidden` | An outbound request was blocked by the active `OutboundPolicy`. Returned when `register_custom_provider` is called with a `base_url` that violates the policy (e.g. a private-range IP under `DenyPrivate`), or when the per-connection DNS resolver detects a forbidden address at connect time. |
| `IdempotencyConflict` | A different request body was submitted for an existing `Idempotency-Key`. Per the OpenAI `Idempotency-Key` convention, once a key is used with a particular request body, subsequent requests using the same key must carry an identical body.  A body mismatch is a hard error (not retryable). HTTP equivalent: 409 Conflict. |
| `IdempotencyInFlight` | The same `Idempotency-Key` is already in-flight (another request with the same key is currently being processed). The caller should wait briefly and retry.  The response is not yet available, and this request has been short-circuited to avoid running the operation twice. HTTP equivalent: 409 Conflict (retryable after a brief delay). |

---
