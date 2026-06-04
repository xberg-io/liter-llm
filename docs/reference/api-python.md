---
title: "Python API Reference"
---

## Python API Reference <span class="version-badge">v1.4.0-rc.59</span>

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

```python
def create_client(api_key: str, base_url: str = None, timeout_secs: int = None, max_retries: int = None, model_hint: str = None) -> DefaultClient
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `api_key` | `str` | Yes | The api key |
| `base_url` | `str \| None` | No | The base url |
| `timeout_secs` | `int \| None` | No | The timeout secs |
| `max_retries` | `int \| None` | No | The max retries |
| `model_hint` | `str \| None` | No | The model hint |

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

```python
def create_client_from_json(json: str) -> DefaultClient
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `json` | `str` | Yes | The json |

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

```python
def register_custom_provider(config: CustomProviderConfig) -> None
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `CustomProviderConfig` | Yes | The configuration options |

**Returns:** `None`
**Errors:** Raises `Error`.

---

#### unregister_custom_provider()

Remove a previously registered custom provider by name.

Returns `True` if a provider with the given name was found and removed,
`False` if no such provider existed.

**Errors:**

Returns an error only if the internal lock is poisoned.

**Signature:**

```python
def unregister_custom_provider(name: str) -> bool
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `str` | Yes | The name |

**Returns:** `bool`
**Errors:** Raises `Error`.

---

#### all_providers()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.

**Signature:**

```python
def all_providers() -> list[ProviderConfig]
```

**Returns:** `list[ProviderConfig]`
**Errors:** Raises `Error`.

---

#### complex_provider_names()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```python
def complex_provider_names() -> list[str]
```

**Returns:** `list[str]`
**Errors:** Raises `Error`.

---

#### completion_cost()

Calculate the estimated cost of a completion given a model name and token
counts.

Returns `None` if the model is not present in the embedded pricing registry.
Returns `Some(cost_usd)` otherwise, where the value is in US dollars.

When an exact model name match is not found, progressively shorter prefixes
are tried by stripping from the last `-` or `.` separator. For example,
`gpt-4-0613` will match `gpt-4` if no `gpt-4-0613` entry exists.

**Signature:**

```python
def completion_cost(model: str, prompt_tokens: int, completion_tokens: int) -> float | None
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `str` | Yes | The model |
| `prompt_tokens` | `int` | Yes | The prompt tokens |
| `completion_tokens` | `int` | Yes | The completion tokens |

**Returns:** `float | None`

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

```python
def completion_cost_with_cache(model: str, prompt_tokens: int, cached_tokens: int, completion_tokens: int) -> float | None
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `str` | Yes | The model |
| `prompt_tokens` | `int` | Yes | The prompt tokens |
| `cached_tokens` | `int` | Yes | The cached tokens |
| `completion_tokens` | `int` | Yes | The completion tokens |

**Returns:** `float | None`

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

```python
def count_tokens(model: str, text: str) -> int
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `str` | Yes | The model |
| `text` | `str` | Yes | The text |

**Returns:** `int`
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

```python
def count_request_tokens(model: str, req: ChatCompletionRequest) -> int
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `str` | Yes | The model |
| `req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `int`
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

Windows builds use native-tls (SChannel) via reqwest, so rustls is not
present and no crypto provider installation is needed.

**Signature:**

```python
def ensure_crypto_provider() -> None
```

**Returns:** `None`

---

### Types

#### AssistantMessage

Assistant's response to a user message.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str \| None` | `None` | The assistant's text response. Absent if tool calls are returned instead. |
| `name` | `str \| None` | `None` | Optional name for the assistant. |
| `tool_calls` | `list[ToolCall] \| None` | `[]` | Tool calls the model wants to execute, if any. |
| `refusal` | `str \| None` | `None` | Refusal reason, if the model declined to respond per safety policies. |
| `function_call` | `FunctionCall \| None` | `None` | Deprecated legacy function_call field; retained for API compatibility. |

---

#### AudioContent

Audio content part for speech-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `str` | — | Base64-encoded audio data. |
| `format` | `str` | — | Audio format (e.g., "wav", "mp3", "ogg"). |

---

#### AuthConfig

Auth configuration block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auth_type` | `AuthType` | — | Auth scheme classification. |
| `env_var` | `str \| None` | `None` | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### BatchListQuery

Query parameters for listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `int \| None` | `None` | Maximum number of results to return. Defaults to 20. |
| `after` | `str \| None` | `None` | Pagination cursor: return results after this batch ID. |

---

#### BatchListResponse

Response from listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `str` | — | Object type (always `"list"`). |
| `data` | `list[BatchObject]` | `[]` | List of batch objects. |
| `has_more` | `bool \| None` | `None` | Whether more results are available. |
| `first_id` | `str \| None` | `None` | First batch ID in the result set (for pagination). |
| `last_id` | `str \| None` | `None` | Last batch ID in the result set (for pagination). |

---

#### BatchObject

A batch job object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique batch ID. |
| `object` | `str` | — | Object type (always `"batch"`). |
| `endpoint` | `str` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `input_file_id` | `str` | — | ID of the input file. |
| `completion_window` | `str` | — | Completion window (e.g., `"24h"`). |
| `status` | `BatchStatus` | `BatchStatus.VALIDATING` | Current job status. |
| `output_file_id` | `str \| None` | `None` | ID of the output file (present when completed). |
| `error_file_id` | `str \| None` | `None` | ID of the error file (present if some requests failed). |
| `created_at` | `int` | — | Unix timestamp of batch creation. |
| `completed_at` | `int \| None` | `None` | Unix timestamp of completion (if completed). |
| `failed_at` | `int \| None` | `None` | Unix timestamp of failure (if failed). |
| `expired_at` | `int \| None` | `None` | Unix timestamp of expiration (if expired). |
| `request_counts` | `BatchRequestCounts \| None` | `None` | Request processing counts. |
| `metadata` | `dict[str, Any] \| None` | `None` | Metadata attached to the batch. |

---

#### BatchRequestCounts

Request processing counts for a batch.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total` | `int` | — | Total requests in the batch. |
| `completed` | `int` | — | Completed requests. |
| `failed` | `int` | — | Failed requests. |

---

#### BudgetConfig

Configuration for budget enforcement.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `global_limit` | `float \| None` | `None` | Maximum total spend across all models, in USD.  `None` means unlimited. |
| `model_limits` | `dict[str, float]` | `{}` | Per-model spending limits in USD.  Models not listed here are only constrained by `global_limit`. |
| `enforcement` | `Enforcement` | `Enforcement.HARD` | Whether to reject requests or merely warn when a limit is exceeded. |

### Methods

#### default()

**Signature:**

```python
@staticmethod
def default() -> BudgetConfig
```

---

#### CacheConfig

Configuration for the response cache.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_entries` | `int` | `256` | Maximum number of cached entries. |
| `ttl` | `float` | `300000ms` | Time-to-live for each cached entry. |
| `backend` | `CacheBackend` | `CacheBackend.MEMORY` | Storage backend to use. |

### Methods

#### default()

**Signature:**

```python
@staticmethod
def default() -> CacheConfig
```

---

#### ChatCompletionChunk

A streamed chunk of a chat completion response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier for this stream. |
| `object` | `str` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created` | `int` | — | Unix timestamp of chunk creation. |
| `model` | `str` | — | Model used to generate the chunk. |
| `choices` | `list[StreamChoice]` | `[]` | Streaming choices (delta updates). |
| `usage` | `Usage \| None` | `None` | Token usage (typically only in the final chunk). |
| `system_fingerprint` | `str \| None` | `None` | Fingerprint of the system configuration (OpenAI-specific). |
| `service_tier` | `str \| None` | `None` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`). |
| `messages` | `list[Message]` | `[]` | Conversation history from oldest to newest. |
| `temperature` | `float \| None` | `None` | Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0. |
| `top_p` | `float \| None` | `None` | Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused. |
| `n` | `int \| None` | `None` | Number of chat completions to generate. Defaults to 1. |
| `stream` | `bool \| None` | `None` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `stop` | `StopSequence \| None` | `None` | Stop sequence(s) that halt token generation. |
| `max_tokens` | `int \| None` | `None` | Max output tokens. Different from max_completion_tokens in some providers. |
| `presence_penalty` | `float \| None` | `None` | Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics. |
| `frequency_penalty` | `float \| None` | `None` | Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens. |
| `logit_bias` | `dict[str, float] \| None` | `{}` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user` | `str \| None` | `None` | User identifier for request tracking and abuse detection. |
| `tools` | `list[ChatCompletionTool] \| None` | `[]` | Tools the model can invoke. |
| `tool_choice` | `ToolChoice \| None` | `None` | Tool usage mode (auto, required, none, or specific tool). |
| `parallel_tool_calls` | `bool \| None` | `None` | Whether the model can call multiple tools in parallel. Defaults to true. |
| `response_format` | `ResponseFormat \| None` | `None` | Output format constraint (text, JSON, JSON schema). |
| `stream_options` | `StreamOptions \| None` | `None` | Streaming options (e.g., include_usage). |
| `seed` | `int \| None` | `None` | Random seed for reproducible outputs. Provider support varies. |
| `reasoning_effort` | `ReasoningEffort \| None` | `None` | Reasoning effort level (low, medium, high) for extended-thinking models. |
| `extra_body` | `dict[str, Any] \| None` | `None` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

#### ChatCompletionResponse

Chat completion response from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier for this response. |
| `object` | `str` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `int` | — | Unix timestamp of response creation. |
| `model` | `str` | — | Model used to generate the response. |
| `choices` | `list[Choice]` | `[]` | List of completion choices. |
| `usage` | `Usage \| None` | `None` | Token usage statistics. |
| `system_fingerprint` | `str \| None` | `None` | Fingerprint of the system configuration (OpenAI-specific). |
| `service_tier` | `str \| None` | `None` | Service tier used (OpenAI-specific). |

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
| `index` | `int` | — | Index of this choice in the choices array. |
| `message` | `AssistantMessage` | — | The assistant's message response. |
| `finish_reason` | `FinishReason \| None` | `None` | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

---

#### CreateBatchRequest

Request to create a batch job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_file_id` | `str` | — | ID of the uploaded input file (JSONL format). |
| `endpoint` | `str` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `completion_window` | `str` | — | Completion window (e.g., `"24h"`). |
| `metadata` | `dict[str, Any] \| None` | `None` | Optional metadata to attach to the batch. |

---

#### CreateFileRequest

Request to upload a file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | `str` | — | Base64-encoded file data. |
| `purpose` | `FilePurpose` | `FilePurpose.ASSISTANTS` | Purpose for the file. |
| `filename` | `str \| None` | `None` | Optional filename to associate with the upload. |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt` | `str` | — | Text description of the image to generate. |
| `model` | `str \| None` | `None` | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `n` | `int \| None` | `None` | Number of images to generate. Defaults to 1. |
| `size` | `str \| None` | `None` | Image size (e.g., `"1024x1024"`, `"1792x1024"`). |
| `quality` | `str \| None` | `None` | Image quality: `"standard"` or `"hd"`. |
| `style` | `str \| None` | `None` | Style: `"natural"` or `"vivid"` (DALL-E 3 only). |
| `response_format` | `str \| None` | `None` | Response format: `"url"` or `"b64_json"`. |
| `user` | `str \| None` | `None` | User identifier for request tracking. |

---

#### CreateResponseRequest

Request to create a structured response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model ID. |
| `input` | `dict[str, Any]` | — | Input data to process (e.g., a document to extract from). |
| `instructions` | `str \| None` | `None` | Instructions for processing the input. |
| `tools` | `list[ResponseTool] \| None` | `[]` | Available tools the model can use. |
| `temperature` | `float \| None` | `None` | Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0. |
| `max_output_tokens` | `int \| None` | `None` | Maximum output tokens. |
| `metadata` | `dict[str, Any] \| None` | `None` | Optional metadata. |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`). |
| `input` | `str` | — | Text to synthesize into speech. |
| `voice` | `str` | — | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `response_format` | `str \| None` | `None` | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`). |
| `speed` | `float \| None` | `None` | Playback speed in `[0.25, 4.0]`. Defaults to 1.0. |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model ID (e.g., `"whisper-1"`). |
| `file` | `str` | — | Base64-encoded audio file data. |
| `language` | `str \| None` | `None` | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `prompt` | `str \| None` | `None` | Optional text to guide the model (improves accuracy for domain-specific terms). |
| `response_format` | `str \| None` | `None` | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`). |
| `temperature` | `float \| None` | `None` | Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0. |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | Unique name for this provider (e.g., "my-provider"). |
| `base_url` | `str` | — | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `auth_header` | `AuthHeaderFormat` | — | Authentication header format. |
| `model_prefixes` | `list[str]` | — | Model name prefixes that route to this provider (e.g., `["my-"]`). |

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

```python
def chat(self, req: ChatCompletionRequest) -> ChatCompletionResponse
```

#### chat_stream()

**Signature:**

```python
def chat_stream(self, req: ChatCompletionRequest) -> str
```

#### embed()

**Signature:**

```python
def embed(self, req: EmbeddingRequest) -> EmbeddingResponse
```

#### list_models()

**Signature:**

```python
def list_models(self) -> ModelsListResponse
```

#### image_generate()

**Signature:**

```python
def image_generate(self, req: CreateImageRequest) -> ImagesResponse
```

#### speech()

**Signature:**

```python
def speech(self, req: CreateSpeechRequest) -> bytes
```

#### transcribe()

**Signature:**

```python
def transcribe(self, req: CreateTranscriptionRequest) -> TranscriptionResponse
```

#### moderate()

**Signature:**

```python
def moderate(self, req: ModerationRequest) -> ModerationResponse
```

#### rerank()

**Signature:**

```python
def rerank(self, req: RerankRequest) -> RerankResponse
```

#### search()

**Signature:**

```python
def search(self, req: SearchRequest) -> SearchResponse
```

#### ocr()

**Signature:**

```python
def ocr(self, req: OcrRequest) -> OcrResponse
```

#### create_file()

**Signature:**

```python
def create_file(self, req: CreateFileRequest) -> FileObject
```

#### retrieve_file()

**Signature:**

```python
def retrieve_file(self, file_id: str) -> FileObject
```

#### delete_file()

**Signature:**

```python
def delete_file(self, file_id: str) -> DeleteResponse
```

#### list_files()

**Signature:**

```python
def list_files(self, query: FileListQuery) -> FileListResponse
```

#### file_content()

**Signature:**

```python
def file_content(self, file_id: str) -> bytes
```

#### create_batch()

**Signature:**

```python
def create_batch(self, req: CreateBatchRequest) -> BatchObject
```

#### retrieve_batch()

**Signature:**

```python
def retrieve_batch(self, batch_id: str) -> BatchObject
```

#### list_batches()

**Signature:**

```python
def list_batches(self, query: BatchListQuery) -> BatchListResponse
```

#### cancel_batch()

**Signature:**

```python
def cancel_batch(self, batch_id: str) -> BatchObject
```

#### create_response()

**Signature:**

```python
def create_response(self, req: CreateResponseRequest) -> ResponseObject
```

#### retrieve_response()

**Signature:**

```python
def retrieve_response(self, response_id: str) -> ResponseObject
```

#### cancel_response()

**Signature:**

```python
def cancel_response(self, response_id: str) -> ResponseObject
```

---

#### DeleteResponse

Response from a delete operation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | ID of the deleted resource. |
| `object` | `str` | — | Object type. |
| `deleted` | `bool` | — | Confirmation that the resource was deleted. |

---

#### DeveloperMessage

Developer message (system-like message for Claude models).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | Developer-specific instructions or context. |
| `name` | `str \| None` | `None` | Optional name for the developer message source. |

---

#### DocumentContent

PDF/document content part for vision-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `str` | — | Base64-encoded document data or URL. |
| `media_type` | `str` | — | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### EmbeddingObject

A single embedding vector.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `str` | — | Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `embedding` | `list[float]` | — | The embedding vector. |
| `index` | `int` | — | Index in the batch (corresponds to input order). |

---

#### EmbeddingRequest

Embedding request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model ID (e.g., `"text-embedding-3-small"`). |
| `input` | `EmbeddingInput` | `EmbeddingInput.SINGLE` | Text or texts to embed. |
| `encoding_format` | `EmbeddingFormat \| None` | `None` | Output format: float (native) or base64. |
| `dimensions` | `int \| None` | `None` | Requested embedding dimensions (if supported by the model). |
| `user` | `str \| None` | `None` | User identifier for request tracking. |

---

#### EmbeddingResponse

Embedding response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `str` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `list[EmbeddingObject]` | — | List of embeddings. |
| `model` | `str` | — | Model used to generate embeddings. |
| `usage` | `Usage \| None` | `/* serde(default) */` | Token usage (input tokens only; embeddings have zero output tokens). |

---

#### FileListQuery

Query parameters for listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `purpose` | `str \| None` | `None` | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `limit` | `int \| None` | `None` | Maximum number of results to return. Defaults to 20. |
| `after` | `str \| None` | `None` | Pagination cursor: return results after this file ID. |

---

#### FileListResponse

Response from listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `str` | — | Object type (always `"list"`). |
| `data` | `list[FileObject]` | `[]` | List of file objects. |
| `has_more` | `bool \| None` | `None` | Whether more results are available. |

---

#### FileObject

An uploaded file object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique file ID. |
| `object` | `str` | — | Object type (always `"file"`). |
| `bytes` | `int` | — | File size in bytes. |
| `created_at` | `int` | — | Unix timestamp of file creation. |
| `filename` | `str` | — | Filename. |
| `purpose` | `str` | — | File purpose. |
| `status` | `str \| None` | `None` | Processing status (e.g., `"uploaded"`, `"processed"`). |

---

#### FunctionCall

Function call details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | Function name. |
| `arguments` | `str` | — | Arguments as a JSON string (parse with serde_json.from_str). |

---

#### FunctionDefinition

Function definition exposed to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | Name of the function. Required and must be alphanumeric + underscores. |
| `description` | `str \| None` | `/* serde(default) */` | Human-readable description explaining what the function does. |
| `parameters` | `dict[str, Any] \| None` | `/* serde(default) */` | JSON Schema defining the function's parameters. |
| `strict` | `bool \| None` | `/* serde(default) */` | If true, enforce strict JSON schema validation for arguments. |

---

#### FunctionMessage

Deprecated legacy function-role message body.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | The extracted text content |
| `name` | `str` | — | The name |

---

#### Image

A single generated image, returned as either a URL or base64 data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `str \| None` | `None` | Image URL (if response_format was "url"). |
| `b64_json` | `str \| None` | `None` | Base64-encoded image data (if response_format was "b64_json"). |
| `revised_prompt` | `str \| None` | `None` | The final prompt used to generate the image (DALL-E 3). |

---

#### ImageUrl

An image URL reference with optional detail level for processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `str` | — | URL of the image (data URI or HTTP/HTTPS URL). |
| `detail` | `ImageDetail \| None` | `None` | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### ImagesResponse

Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `created` | `int` | — | Unix timestamp of image creation. |
| `data` | `list[Image]` | `[]` | List of generated images. |

---

#### JsonSchemaFormat

JSON Schema specification for constrained output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | Name of the schema (must be unique in the request). |
| `description` | `str \| None` | `None` | Description of what the schema represents. |
| `schema` | `dict[str, Any]` | — | JSON Schema object defining the output structure. |
| `strict` | `bool \| None` | `None` | If true, enforce strict schema validation. |

---

#### ModelObject

A model available from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`). |
| `object` | `str` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `int` | — | Unix timestamp of model creation (or release date). |
| `owned_by` | `str` | — | Organization or entity that owns the model. |

---

#### ModelsListResponse

Response listing available models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `str` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `list[ModelObject]` | `[]` | List of available models. |

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
| `sexual` | `float` | — | Sexual content score. |
| `hate` | `float` | — | Hate speech score. |
| `harassment` | `float` | — | Harassment score. |
| `self_harm` | `float` | — | Self-harm content score. |
| `sexual_minors` | `float` | — | Sexual content involving minors score. |
| `hate_threatening` | `float` | — | Hate speech that threatens violence score. |
| `violence_graphic` | `float` | — | Graphic violence score. |
| `self_harm_intent` | `float` | — | Intent to self-harm score. |
| `self_harm_instructions` | `float` | — | Instructions for self-harm score. |
| `harassment_threatening` | `float` | — | Harassment that threatens violence score. |
| `violence` | `float` | — | Non-graphic violence score. |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input` | `ModerationInput` | `ModerationInput.SINGLE` | Text or texts to check. |
| `model` | `str \| None` | `None` | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier for this moderation request. |
| `model` | `str` | — | Model used for classification. |
| `results` | `list[ModerationResult]` | — | Results for each input string. |

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
| `id` | `str` | — | Unique image identifier within the document. |
| `image_base64` | `str \| None` | `/* serde(default) */` | Base64-encoded image data (if `include_image_base64` was true). |

---

#### OcrPage

A single page of OCR output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Page index (0-based). |
| `markdown` | `str` | — | Extracted page content as Markdown. |
| `images` | `list[OcrImage] \| None` | `/* serde(default) */` | Embedded images extracted from the page (if `include_image_base64` was true). |
| `dimensions` | `PageDimensions \| None` | `/* serde(default) */` | Page dimensions in pixels, if available. |

---

#### OcrRequest

An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document` | `OcrDocument` | `OcrDocument.URL` | The document to process (URL or base64). |
| `pages` | `list[int] \| None` | `[]` | Specific pages to process (1-indexed). `None` means all pages. |
| `include_image_base64` | `bool \| None` | `None` | Whether to include base64-encoded images of each processed page. |

---

#### OcrResponse

An OCR response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pages` | `list[OcrPage]` | — | Extracted pages in order. |
| `model` | `str` | — | Model/provider used for OCR. |
| `usage` | `Usage \| None` | `/* serde(default) */` | Token usage, if reported by the provider. |

---

#### PageDimensions

Page dimensions in pixels.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `int` | — | Width in pixels. |
| `height` | `int` | — | Height in pixels. |

---

#### PromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage.prompt_tokens` — it is *not* an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cached_tokens` | `int` | — | Cached tokens present in the prompt. Defaults to 0 when absent. |
| `audio_tokens` | `int` | — | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### ProviderConfig

Static configuration for a single provider entry in providers.json.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | Provider identifier (matches the entry key in providers.json). |
| `display_name` | `str \| None` | `None` | Human-readable provider name shown in UIs. |
| `base_url` | `str \| None` | `None` | Base URL used as the default for this provider's HTTP client. |
| `auth` | `AuthConfig \| None` | `None` | Authentication scheme metadata (auth type + env var holding the key). |
| `endpoints` | `list[str] \| None` | `None` | Supported endpoint kinds (e.g. `chat`, `embeddings`). |
| `model_prefixes` | `list[str] \| None` | `None` | Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`). |
| `param_mappings` | `dict[str, str] \| None` | `None` | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`).  Applied automatically by `ConfigDrivenProvider.transform_request`. |

---

#### RateLimitConfig

Configuration for per-model rate limits.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rpm` | `int \| None` | `None` | Maximum requests per window.  `None` means unlimited. |
| `tpm` | `int \| None` | `None` | Maximum tokens per window.  `None` means unlimited. |
| `window` | `float` | `60000ms` | Fixed window duration (defaults to 60 s). |

### Methods

#### default()

**Signature:**

```python
@staticmethod
def default() -> RateLimitConfig
```

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model ID (e.g., `"cohere/rerank-english-v3.0"`). |
| `query` | `str` | — | The search query. |
| `documents` | `list[RerankDocument]` | `[]` | Documents to rerank. |
| `top_n` | `int \| None` | `None` | Return only the top N results. Optional. |
| `return_documents` | `bool \| None` | `None` | Include the document content in results. Defaults to false. |

---

#### RerankResponse

Response from the rerank endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str \| None` | `None` | Unique identifier for this rerank request. |
| `results` | `list[RerankResult]` | — | Reranked documents in order of relevance. |
| `meta` | `dict[str, Any] \| None` | `/* serde(default) */` | Optional metadata about the reranking operation. |

---

#### RerankResult

A single reranked document with its relevance score.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Original document index in the input list. |
| `relevance_score` | `float` | — | Relevance score in `[0, 1]`. Higher indicates more relevant. |
| `document` | `RerankResultDocument \| None` | `/* serde(default) */` | Original document content (if `return_documents` was true). |

---

#### RerankResultDocument

The text content of a reranked document, returned when `return_documents` is true.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `str` | — | Document text. |

---

#### ResponseObject

Response from a structured response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique response ID. |
| `object` | `str` | — | Object type (e.g., `"response"`). |
| `created_at` | `int` | — | Unix timestamp of response creation. |
| `model` | `str` | — | Model used to generate the response. |
| `status` | `str` | — | Status (e.g., `"succeeded"`, `"failed"`). |
| `output` | `list[ResponseOutputItem]` | `[]` | Output items from the response. |
| `usage` | `ResponseUsage \| None` | `None` | Token usage. |
| `error` | `dict[str, Any] \| None` | `None` | Error details (if status is "failed"). |

---

#### ResponseOutputItem

A single output item from the response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `item_type` | `str` | — | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `content` | `dict[str, Any]` | — | Output content (flattened into the object). |

---

#### ResponseTool

A tool available for the response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tool_type` | `str` | — | Tool type (e.g., "extractor", "search"). |
| `config` | `dict[str, Any]` | — | Tool configuration (flattened into the object). |

---

#### ResponseUsage

Token usage for a response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_tokens` | `int` | — | Input tokens used. |
| `output_tokens` | `int` | — | Output tokens used. |
| `total_tokens` | `int` | — | Total tokens used. |

---

#### SearchRequest

A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query` | `str` | — | The search query string. |
| `max_results` | `int \| None` | `None` | Maximum number of results to return. |
| `search_domain_filter` | `list[str] \| None` | `[]` | Domain filter — restrict results to specific domains. |
| `country` | `str \| None` | `None` | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

---

#### SearchResponse

A search response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `results` | `list[SearchResult]` | — | List of search results. |
| `model` | `str` | — | Model/provider that performed the search. |

---

#### SearchResult

An individual search result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `str` | — | Result title. |
| `url` | `str` | — | Result URL. |
| `snippet` | `str` | — | Text snippet or excerpt from the page. |
| `date` | `str \| None` | `/* serde(default) */` | Publication or last-updated date, if available. |

---

#### SpecificFunction

Name of the specific function to invoke.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | Function name. |

---

#### SpecificToolChoice

Directive to call a specific tool.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `choice_type` | `ToolType` | `ToolType.FUNCTION` | Tool type (always "function"). |
| `function` | `SpecificFunction` | — | The specific function to invoke. |

---

#### StreamChoice

A streaming choice with incremental delta.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index of this choice in the choices array. |
| `delta` | `StreamDelta` | — | Incremental update to the message (content, tool calls, etc.). |
| `finish_reason` | `FinishReason \| None` | `None` | Why the stream ended (present only in final chunk). |

---

#### StreamDelta

Incremental delta in a stream chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `role` | `str \| None` | `None` | Role (typically present only in the first chunk). |
| `content` | `str \| None` | `None` | Partial content chunk (e.g., a few words of the response). |
| `tool_calls` | `list[StreamToolCall] \| None` | `[]` | Partial tool calls being streamed. |
| `function_call` | `StreamFunctionCall \| None` | `None` | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal` | `str \| None` | `None` | Partial refusal message. |

---

#### StreamFunctionCall

Partial function call details in a stream.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str \| None` | `None` | Function name (typically in the first chunk). |
| `arguments` | `str \| None` | `None` | Partial JSON arguments chunk. |

---

#### StreamOptions

Options for streaming responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_usage` | `bool \| None` | `None` | If true, include token usage in the final stream chunk. |

---

#### StreamToolCall

A streaming tool call being built incrementally.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index of this tool call in the tool_calls array. |
| `id` | `str \| None` | `None` | Tool call ID (typically in the first chunk for this call). |
| `call_type` | `ToolType \| None` | `None` | Tool type (typically "function"). |
| `function` | `StreamFunctionCall \| None` | `None` | Partial function name and arguments. |

---

#### SystemMessage

System message guiding model behavior for the entire conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | Instructions or context that apply throughout the conversation. |
| `name` | `str \| None` | `None` | Optional name for the system message source. |

---

#### ToolCall

A tool call the model wants to execute.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique ID for this call, used to reference in tool result messages. |
| `call_type` | `ToolType` | — | Tool type (always "function"). |
| `function` | `FunctionCall` | — | Function name and arguments. |

---

#### ToolMessage

Tool execution result returned to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | Result of the tool execution. |
| `tool_call_id` | `str` | — | ID of the tool call this result responds to. |
| `name` | `str \| None` | `None` | Optional tool/function name. |

---

#### TranscriptionResponse

Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `str` | — | The transcribed text. |
| `language` | `str \| None` | `None` | Detected language (ISO-639-1 code). |
| `duration` | `float \| None` | `None` | Total audio duration in seconds. |
| `segments` | `list[TranscriptionSegment] \| None` | `[]` | Detailed segment-level transcription (if response_format is "verbose_json"). |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `int` | — | Segment index (0-based). |
| `start` | `float` | — | Start time in seconds. |
| `end` | `float` | — | End time in seconds. |
| `text` | `str` | — | Transcribed text for this segment. |

---

#### Usage

Token-usage accounting returned by the provider on each completion / embedding call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_tokens` | `int` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `completion_tokens` | `int` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `total_tokens` | `int` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `prompt_tokens_details` | `PromptTokensDetails \| None` | `None` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

User message in the conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `UserContent.TEXT` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `name` | `str \| None` | `None` | Optional name for the user. |

---

### Enums

#### Message

A chat message in a conversation.

| Value | Description |
|-------|-------------|
| `SYSTEM` | System — Fields: `0`: `SystemMessage` |
| `USER` | User — Fields: `0`: `UserMessage` |
| `ASSISTANT` | Assistant — Fields: `0`: `AssistantMessage` |
| `TOOL` | Tool — Fields: `0`: `ToolMessage` |
| `DEVELOPER` | Developer — Fields: `0`: `DeveloperMessage` |
| `FUNCTION` | Deprecated legacy function-role message; retained for API compatibility. — Fields: `0`: `FunctionMessage` |

---

#### UserContent

User message content as either plain text or a list of multimodal parts.

| Value | Description |
|-------|-------------|
| `TEXT` | Plain text content. — Fields: `0`: `str` |
| `PARTS` | Array of content parts (text, images, documents, audio). — Fields: `0`: `list[ContentPart]` |

---

#### ContentPart

A single content part in a user message — text, image, document, or audio.

| Value | Description |
|-------|-------------|
| `TEXT` | Plain text. — Fields: `text`: `str` |
| `IMAGE_URL` | Image identified by URL (with optional detail level). — Fields: `image_url`: `ImageUrl` |
| `DOCUMENT` | Document file (PDF, CSV, etc.) as base64 or URL. — Fields: `document`: `DocumentContent` |
| `INPUT_AUDIO` | Audio input as base64. — Fields: `input_audio`: `AudioContent` |

---

#### ImageDetail

Image detail level controlling token cost and processing.

| Value | Description |
|-------|-------------|
| `LOW` | Low detail: scales image to 512x512, uses fewer tokens. |
| `HIGH` | High detail: processes up to 2x2 grid of tiles, higher token cost. |
| `AUTO` | Auto: model chooses low or high based on image dimensions. |

---

#### ToolType

The type discriminator for tool/tool-call objects.

Per the OpenAI spec this is always `"function"`. Using an enum enforces
that constraint at the type level and rejects any other value on
deserialization.

| Value | Description |
|-------|-------------|
| `FUNCTION` | Function |

---

#### ToolChoice

Tool usage mode or a specific tool to call.

| Value | Description |
|-------|-------------|
| `MODE` | Predefined mode: auto, required, or none. — Fields: `0`: `ToolChoiceMode` |
| `SPECIFIC` | Force a specific tool to be called. — Fields: `0`: `SpecificToolChoice` |

---

#### ToolChoiceMode

Tool choice mode.

| Value | Description |
|-------|-------------|
| `AUTO` | Model may or may not call tools; default behavior. |
| `REQUIRED` | Model must call at least one tool. |
| `NONE` | Model must not call any tools. |

---

#### ResponseFormat

Response format constraint.

| Value | Description |
|-------|-------------|
| `TEXT` | Plain text output (default). |
| `JSON_OBJECT` | Output must be valid JSON object (no schema validation). |
| `JSON_SCHEMA` | Output must conform to the specified JSON schema. — Fields: `json_schema`: `JsonSchemaFormat` |

---

#### StopSequence

Stop sequence(s) that cause the model to stop generating.

| Value | Description |
|-------|-------------|
| `SINGLE` | Single stop sequence. — Fields: `0`: `str` |
| `MULTIPLE` | Multiple stop sequences. — Fields: `0`: `list[str]` |

---

#### FinishReason

Why a choice stopped generating tokens.

| Value | Description |
|-------|-------------|
| `STOP` | Stop |
| `LENGTH` | Length |
| `TOOL_CALLS` | Tool calls |
| `CONTENT_FILTER` | Content filter |
| `FUNCTION_CALL` | Deprecated legacy finish reason; retained for API compatibility. |
| `OTHER` | Catch-all for unknown finish reasons returned by non-OpenAI providers. Note: this intentionally does **not** carry the original string (e.g. `Other(String)`).  Using `#[serde(other)]` requires a unit variant, and switching to `#[serde(untagged)]` would change deserialization semantics for all variants.  The original value can be recovered by inspecting the raw JSON if needed. |

---

#### ReasoningEffort

Controls how much reasoning effort the model should use.

| Value | Description |
|-------|-------------|
| `LOW` | Low |
| `MEDIUM` | Medium |
| `HIGH` | High |

---

#### EmbeddingFormat

The format in which the embedding vectors are returned.

| Value | Description |
|-------|-------------|
| `FLOAT` | 32-bit floating-point numbers (default). |
| `BASE64` | Base64-encoded string representation of the floats. |

---

#### EmbeddingInput

Text or texts to embed.

| Value | Description |
|-------|-------------|
| `SINGLE` | Single text string. — Fields: `0`: `str` |
| `MULTIPLE` | Multiple text strings (batch embedding). — Fields: `0`: `list[str]` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value | Description |
|-------|-------------|
| `SINGLE` | Single text string. — Fields: `0`: `str` |
| `MULTIPLE` | Multiple text strings (batch moderation). — Fields: `0`: `list[str]` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value | Description |
|-------|-------------|
| `TEXT` | Plain text document content. — Fields: `0`: `str` |
| `OBJECT` | Document with explicit text field (may include metadata). — Fields: `text`: `str` |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value | Description |
|-------|-------------|
| `URL` | A publicly accessible document URL. — Fields: `url`: `str` |
| `BASE64` | Inline base64-encoded document data. — Fields: `data`: `str`, `media_type`: `str` |

---

#### FilePurpose

Purpose of an uploaded file.

| Value | Description |
|-------|-------------|
| `ASSISTANTS` | File for use with Assistants API. |
| `BATCH` | File for batch processing. |
| `FINE_TUNE` | File for fine-tuning. |
| `VISION` | File for vision/image tasks. |

---

#### BatchStatus

Status of a batch job.

| Value | Description |
|-------|-------------|
| `VALIDATING` | Validating the input file. |
| `FAILED` | Job failed. |
| `IN_PROGRESS` | Job is running. |
| `FINALIZING` | Finalizing results. |
| `COMPLETED` | Job completed successfully. |
| `EXPIRED` | Job expired before completion. |
| `CANCELLING` | Job is being cancelled. |
| `CANCELLED` | Job has been cancelled. |

---

#### AuthHeaderFormat

How the API key is sent in the HTTP request.

| Value | Description |
|-------|-------------|
| `BEARER` | Bearer token: `Authorization: Bearer <key>` |
| `API_KEY` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `str` |
| `NONE` | No authentication required. |

---

#### AuthType

Auth scheme used by a provider.

| Value | Description |
|-------|-------------|
| `BEARER` | Standard `Authorization: Bearer <key>` header. |
| `API_KEY` | `x-api-key: <key>` header (also handles `"header"` and `"x-api-key"` aliases). |
| `NONE` | No authentication header required. |
| `UNKNOWN` | Unrecognised auth scheme — falls back to bearer. |

---

#### Enforcement

How budget limits are enforced.

| Value | Description |
|-------|-------------|
| `HARD` | Reject requests that would exceed the budget with `LiterLlmError.BudgetExceeded`. |
| `SOFT` | Allow requests through but emit a `tracing.warn!` when the budget is exceeded. |

---

#### CacheBackend

Storage backend for the response cache.

| Value | Description |
|-------|-------------|
| `MEMORY` | In-memory LRU cache (default). No external dependencies. |
| `OPEN_DAL` | OpenDAL-backed storage. Supports 40+ backends (S3, Redis, GCS, local FS, etc.). — Fields: `scheme`: `str`, `config`: `dict[str, str]` |

---

### Errors

#### LiterLlmError

All errors that can occur when using `liter-llm`.

**Base class:** `LiterLlmError(Exception)`

| Exception | Description |
|-----------|-------------|
| `Authentication(LiterLlmError)` | `status` preserves the exact HTTP status code received (401 or 403). |
| `RateLimited(LiterLlmError)` | rate limited: {message} |
| `BadRequest(LiterLlmError)` | `status` preserves the exact HTTP status code received (400, 405, 413, 422, …). |
| `ContextWindowExceeded(LiterLlmError)` | context window exceeded: {message} |
| `ContentPolicy(LiterLlmError)` | content policy violation: {message} |
| `NotFound(LiterLlmError)` | not found: {message} |
| `ServerError(LiterLlmError)` | `status` preserves the exact HTTP status code received (500, or other 5xx not covered by `ServiceUnavailable`). |
| `ServiceUnavailable(LiterLlmError)` | `status` preserves the exact HTTP status code received (502, 503, or 504). |
| `Timeout(LiterLlmError)` | request timeout |
| `Streaming(LiterLlmError)` | A catch-all for errors that occur during streaming response processing. This variant covers multiple sub-conditions including UTF-8 decoding failures, CRC/checksum mismatches (AWS EventStream), JSON parse errors in individual SSE chunks, and buffer overflow conditions.  The `message` field contains a human-readable description of the specific failure. |
| `EndpointNotSupported(LiterLlmError)` | provider {provider} does not support {endpoint} |
| `InvalidHeader(LiterLlmError)` | invalid header {name:?}: {reason} |
| `Serialization(LiterLlmError)` | serialization error: {0} |
| `BudgetExceeded(LiterLlmError)` | budget exceeded: {message} |
| `HookRejected(LiterLlmError)` | hook rejected: {message} |
| `InternalError(LiterLlmError)` | An internal logic error (e.g. unexpected Tower response variant). This should never surface in normal operation — if it does, it indicates a bug in the library. |

---
