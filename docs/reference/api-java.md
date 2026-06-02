---
title: "Java API Reference"
---

## Java API Reference <span class="version-badge">v1.4.0-rc.48</span>

### Functions

#### createClient()

Create a new LLM client with simple scalar configuration.

This is the primary binding entry-point. All parameters except `api_key`
are optional — omitting them uses the same defaults as
`ClientConfigBuilder`.

**Errors:**

Returns `LiterLlmError` if the underlying HTTP client cannot be
constructed, or if the resolved provider configuration is invalid.

**Signature:**

```java
public static DefaultClient createClient(String apiKey, String baseUrl, long timeoutSecs, int maxRetries, String modelHint) throws Error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `apiKey` | `String` | Yes | The api key |
| `baseUrl` | `Optional<String>` | No | The base url |
| `timeoutSecs` | `Optional<Long>` | No | The timeout secs |
| `maxRetries` | `Optional<Integer>` | No | The max retries |
| `modelHint` | `Optional<String>` | No | The model hint |

**Returns:** `DefaultClient`
**Errors:** Throws `ErrorException`.

---

#### createClientFromJson()

Create a new LLM client from a JSON string.

The JSON object accepts the same fields as `liter-llm.toml` (snake_case).

**Errors:**

Returns `LiterLlmError.BadRequest` if `json` is not valid JSON or
contains unknown fields.

**Signature:**

```java
public static DefaultClient createClientFromJson(String json) throws Error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `json` | `String` | Yes | The json |

**Returns:** `DefaultClient`
**Errors:** Throws `ErrorException`.

---

#### registerCustomProvider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```java
public static void registerCustomProvider(CustomProviderConfig config) throws Error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `CustomProviderConfig` | Yes | The configuration options |

**Returns:** `void`
**Errors:** Throws `ErrorException`.

---

#### unregisterCustomProvider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error only if the internal lock is poisoned.

**Signature:**

```java
public static boolean unregisterCustomProvider(String name) throws Error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `boolean`
**Errors:** Throws `ErrorException`.

---

#### allProviders()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.

**Signature:**

```java
public static List<ProviderConfig> allProviders() throws Error
```

**Returns:** `List<ProviderConfig>`
**Errors:** Throws `ErrorException`.

---

#### complexProviderNames()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```java
public static List<String> complexProviderNames() throws Error
```

**Returns:** `List<String>`
**Errors:** Throws `ErrorException`.

---

#### completionCost()

Calculate the estimated cost of a completion given a model name and token
counts.

Returns `null` if the model is not present in the embedded pricing registry.
Returns `Some(cost_usd)` otherwise, where the value is in US dollars.

When an exact model name match is not found, progressively shorter prefixes
are tried by stripping from the last `-` or `.` separator. For example,
`gpt-4-0613` will match `gpt-4` if no `gpt-4-0613` entry exists.

**Signature:**

```java
public static Optional<Double> completionCost(String model, long promptTokens, long completionTokens)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `promptTokens` | `long` | Yes | The prompt tokens |
| `completionTokens` | `long` | Yes | The completion tokens |

**Returns:** `Optional<Double>`

---

#### completionCostWithCache()

Calculate the estimated cost of a completion, accounting for cached
(cache-hit) prompt tokens billed at the provider's discounted rate.

`cached_tokens` is the count of prompt tokens served from the provider's
prompt cache. It must be `<= prompt_tokens` (cached tokens are a subset of
the prompt). The non-cached portion is billed at `input_cost_per_token`
and the cached portion at `cache_read_input_token_cost` when the model
has cache pricing; otherwise the entire prompt is billed at the regular
input rate.

Returns `null` if the model is not present in the embedded pricing
registry, mirroring `completion_cost`.

**Signature:**

```java
public static Optional<Double> completionCostWithCache(String model, long promptTokens, long cachedTokens, long completionTokens)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `promptTokens` | `long` | Yes | The prompt tokens |
| `cachedTokens` | `long` | Yes | The cached tokens |
| `completionTokens` | `long` | Yes | The completion tokens |

**Returns:** `Optional<Double>`

---

#### countTokens()

Count tokens in a text string using the tokenizer for the given model.

The tokenizer is resolved from the model name prefix (e.g. `"gpt-4o"` maps
to the `Xenova/gpt-4o` HuggingFace tokenizer). Tokenizers are cached after
first load.

**Errors:**

Returns `LiterLlmError.BadRequest` if the tokenizer cannot be loaded
(e.g. network failure on first use) or if tokenization itself fails.

**Signature:**

```java
public static long countTokens(String model, String text) throws Error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `text` | `String` | Yes | The text |

**Returns:** `long`
**Errors:** Throws `ErrorException`.

---

#### countRequestTokens()

Count tokens for a full `ChatCompletionRequest`.

Sums tokens across all message text contents plus a per-message overhead
of ~4 tokens (for role, separators, and formatting metadata). Tool
definitions and multimodal content parts (images, audio, documents) are
not counted — only textual content contributes to the token total.

**Errors:**

Returns `LiterLlmError.BadRequest` if the tokenizer cannot be loaded or
if tokenization fails for any message.

**Signature:**

```java
public static long countRequestTokens(String model, ChatCompletionRequest req) throws Error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `long`
**Errors:** Throws `ErrorException`.

---

#### ensureCryptoProvider()

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

```java
public static void ensureCryptoProvider()
```

**Returns:** `void`

---

### Types

#### AssistantMessage

Assistant's response to a user message.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `Optional<String>` | `null` | The assistant's text response. Absent if tool calls are returned instead. |
| `name` | `Optional<String>` | `null` | Optional name for the assistant. |
| `toolCalls` | `Optional<List<ToolCall>>` | `Collections.emptyList()` | Tool calls the model wants to execute, if any. |
| `refusal` | `Optional<String>` | `null` | Refusal reason, if the model declined to respond per safety policies. |
| `functionCall` | `Optional<FunctionCall>` | `null` | Deprecated legacy function_call field; retained for API compatibility. |

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
| `authType` | `AuthType` | — | Auth scheme classification. |
| `envVar` | `Optional<String>` | `null` | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### BatchListQuery

Query parameters for listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `Optional<Integer>` | `null` | Maximum number of results to return. Defaults to 20. |
| `after` | `Optional<String>` | `null` | Pagination cursor: return results after this batch ID. |

---

#### BatchListResponse

Response from listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Object type (always `"list"`). |
| `data` | `List<BatchObject>` | `Collections.emptyList()` | List of batch objects. |
| `hasMore` | `Optional<Boolean>` | `null` | Whether more results are available. |
| `firstId` | `Optional<String>` | `null` | First batch ID in the result set (for pagination). |
| `lastId` | `Optional<String>` | `null` | Last batch ID in the result set (for pagination). |

---

#### BatchObject

A batch job object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique batch ID. |
| `object` | `String` | — | Object type (always `"batch"`). |
| `endpoint` | `String` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `inputFileId` | `String` | — | ID of the input file. |
| `completionWindow` | `String` | — | Completion window (e.g., `"24h"`). |
| `status` | `BatchStatus` | `BatchStatus.VALIDATING` | Current job status. |
| `outputFileId` | `Optional<String>` | `null` | ID of the output file (present when completed). |
| `errorFileId` | `Optional<String>` | `null` | ID of the error file (present if some requests failed). |
| `createdAt` | `long` | — | Unix timestamp of batch creation. |
| `completedAt` | `Optional<Long>` | `null` | Unix timestamp of completion (if completed). |
| `failedAt` | `Optional<Long>` | `null` | Unix timestamp of failure (if failed). |
| `expiredAt` | `Optional<Long>` | `null` | Unix timestamp of expiration (if expired). |
| `requestCounts` | `Optional<BatchRequestCounts>` | `null` | Request processing counts. |
| `metadata` | `Optional<Object>` | `null` | Metadata attached to the batch. |

---

#### BatchRequestCounts

Request processing counts for a batch.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total` | `long` | — | Total requests in the batch. |
| `completed` | `long` | — | Completed requests. |
| `failed` | `long` | — | Failed requests. |

---

#### BudgetConfig

Configuration for budget enforcement.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `globalLimit` | `Optional<Double>` | `null` | Maximum total spend across all models, in USD.  `null` means unlimited. |
| `modelLimits` | `Map<String, Double>` | `Collections.emptyMap()` | Per-model spending limits in USD.  Models not listed here are only constrained by `global_limit`. |
| `enforcement` | `Enforcement` | `Enforcement.HARD` | Whether to reject requests or merely warn when a limit is exceeded. |

### Methods

#### defaultOptions()

**Signature:**

```java
public static BudgetConfig defaultOptions()
```

---

#### CacheConfig

Configuration for the response cache.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `maxEntries` | `long` | `256` | Maximum number of cached entries. |
| `ttl` | `Duration` | `300000ms` | Time-to-live for each cached entry. |
| `backend` | `CacheBackend` | `CacheBackend.MEMORY` | Storage backend to use. |

### Methods

#### defaultOptions()

**Signature:**

```java
public static CacheConfig defaultOptions()
```

---

#### ChatCompletionChunk

A streamed chunk of a chat completion response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this stream. |
| `object` | `String` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created` | `long` | — | Unix timestamp of chunk creation. |
| `model` | `String` | — | Model used to generate the chunk. |
| `choices` | `List<StreamChoice>` | `Collections.emptyList()` | Streaming choices (delta updates). |
| `usage` | `Optional<Usage>` | `null` | Token usage (typically only in the final chunk). |
| `systemFingerprint` | `Optional<String>` | `null` | Fingerprint of the system configuration (OpenAI-specific). |
| `serviceTier` | `Optional<String>` | `null` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`). |
| `messages` | `List<Message>` | `Collections.emptyList()` | Conversation history from oldest to newest. |
| `temperature` | `Optional<Double>` | `null` | Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0. |
| `topP` | `Optional<Double>` | `null` | Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused. |
| `n` | `Optional<Integer>` | `null` | Number of chat completions to generate. Defaults to 1. |
| `stream` | `Optional<Boolean>` | `null` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `stop` | `Optional<StopSequence>` | `null` | Stop sequence(s) that halt token generation. |
| `maxTokens` | `Optional<Long>` | `null` | Max output tokens. Different from max_completion_tokens in some providers. |
| `presencePenalty` | `Optional<Double>` | `null` | Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics. |
| `frequencyPenalty` | `Optional<Double>` | `null` | Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens. |
| `logitBias` | `Optional<Map<String, Double>>` | `Collections.emptyMap()` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user` | `Optional<String>` | `null` | User identifier for request tracking and abuse detection. |
| `tools` | `Optional<List<ChatCompletionTool>>` | `Collections.emptyList()` | Tools the model can invoke. |
| `toolChoice` | `Optional<ToolChoice>` | `null` | Tool usage mode (auto, required, none, or specific tool). |
| `parallelToolCalls` | `Optional<Boolean>` | `null` | Whether the model can call multiple tools in parallel. Defaults to true. |
| `responseFormat` | `Optional<ResponseFormat>` | `null` | Output format constraint (text, JSON, JSON schema). |
| `streamOptions` | `Optional<StreamOptions>` | `null` | Streaming options (e.g., include_usage). |
| `seed` | `Optional<Long>` | `null` | Random seed for reproducible outputs. Provider support varies. |
| `reasoningEffort` | `Optional<ReasoningEffort>` | `null` | Reasoning effort level (low, medium, high) for extended-thinking models. |
| `extraBody` | `Optional<Object>` | `null` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

#### ChatCompletionResponse

Chat completion response from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this response. |
| `object` | `String` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `long` | — | Unix timestamp of response creation. |
| `model` | `String` | — | Model used to generate the response. |
| `choices` | `List<Choice>` | `Collections.emptyList()` | List of completion choices. |
| `usage` | `Optional<Usage>` | `null` | Token usage statistics. |
| `systemFingerprint` | `Optional<String>` | `null` | Fingerprint of the system configuration (OpenAI-specific). |
| `serviceTier` | `Optional<String>` | `null` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionTool

A tool the model can invoke (currently, all tools are functions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `toolType` | `ToolType` | — | Tool type (always "function" in OpenAI spec). |
| `function` | `FunctionDefinition` | — | Function definition with name, description, and JSON schema parameters. |

---

#### Choice

A single completion choice.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index of this choice in the choices array. |
| `message` | `AssistantMessage` | — | The assistant's message response. |
| `finishReason` | `Optional<FinishReason>` | `null` | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

---

#### CreateBatchRequest

Request to create a batch job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inputFileId` | `String` | — | ID of the uploaded input file (JSONL format). |
| `endpoint` | `String` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `completionWindow` | `String` | — | Completion window (e.g., `"24h"`). |
| `metadata` | `Optional<Object>` | `null` | Optional metadata to attach to the batch. |

---

#### CreateFileRequest

Request to upload a file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | `String` | — | Base64-encoded file data. |
| `purpose` | `FilePurpose` | `FilePurpose.ASSISTANTS` | Purpose for the file. |
| `filename` | `Optional<String>` | `null` | Optional filename to associate with the upload. |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt` | `String` | — | Text description of the image to generate. |
| `model` | `Optional<String>` | `null` | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `n` | `Optional<Integer>` | `null` | Number of images to generate. Defaults to 1. |
| `size` | `Optional<String>` | `null` | Image size (e.g., `"1024x1024"`, `"1792x1024"`). |
| `quality` | `Optional<String>` | `null` | Image quality: `"standard"` or `"hd"`. |
| `style` | `Optional<String>` | `null` | Style: `"natural"` or `"vivid"` (DALL-E 3 only). |
| `responseFormat` | `Optional<String>` | `null` | Response format: `"url"` or `"b64_json"`. |
| `user` | `Optional<String>` | `null` | User identifier for request tracking. |

---

#### CreateResponseRequest

Request to create a structured response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID. |
| `input` | `Object` | — | Input data to process (e.g., a document to extract from). |
| `instructions` | `Optional<String>` | `null` | Instructions for processing the input. |
| `tools` | `Optional<List<ResponseTool>>` | `Collections.emptyList()` | Available tools the model can use. |
| `temperature` | `Optional<Double>` | `null` | Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0. |
| `maxOutputTokens` | `Optional<Long>` | `null` | Maximum output tokens. |
| `metadata` | `Optional<Object>` | `null` | Optional metadata. |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`). |
| `input` | `String` | — | Text to synthesize into speech. |
| `voice` | `String` | — | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `responseFormat` | `Optional<String>` | `null` | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`). |
| `speed` | `Optional<Double>` | `null` | Playback speed in `[0.25, 4.0]`. Defaults to 1.0. |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"whisper-1"`). |
| `file` | `String` | — | Base64-encoded audio file data. |
| `language` | `Optional<String>` | `null` | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `prompt` | `Optional<String>` | `null` | Optional text to guide the model (improves accuracy for domain-specific terms). |
| `responseFormat` | `Optional<String>` | `null` | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`). |
| `temperature` | `Optional<Double>` | `null` | Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0. |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Unique name for this provider (e.g., "my-provider"). |
| `baseUrl` | `String` | — | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `authHeader` | `AuthHeaderFormat` | — | Authentication header format. |
| `modelPrefixes` | `List<String>` | — | Model name prefixes that route to this provider (e.g., `["my-"]`). |

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

```java
public ChatCompletionResponse chat(ChatCompletionRequest req) throws Error
```

#### chatStream()

**Signature:**

```java
public String chatStream(ChatCompletionRequest req) throws Error
```

#### embed()

**Signature:**

```java
public EmbeddingResponse embed(EmbeddingRequest req) throws Error
```

#### listModels()

**Signature:**

```java
public ModelsListResponse listModels() throws Error
```

#### imageGenerate()

**Signature:**

```java
public ImagesResponse imageGenerate(CreateImageRequest req) throws Error
```

#### speech()

**Signature:**

```java
public byte[] speech(CreateSpeechRequest req) throws Error
```

#### transcribe()

**Signature:**

```java
public TranscriptionResponse transcribe(CreateTranscriptionRequest req) throws Error
```

#### moderate()

**Signature:**

```java
public ModerationResponse moderate(ModerationRequest req) throws Error
```

#### rerank()

**Signature:**

```java
public RerankResponse rerank(RerankRequest req) throws Error
```

#### search()

**Signature:**

```java
public SearchResponse search(SearchRequest req) throws Error
```

#### ocr()

**Signature:**

```java
public OcrResponse ocr(OcrRequest req) throws Error
```

#### createFile()

**Signature:**

```java
public FileObject createFile(CreateFileRequest req) throws Error
```

#### retrieveFile()

**Signature:**

```java
public FileObject retrieveFile(String fileId) throws Error
```

#### deleteFile()

**Signature:**

```java
public DeleteResponse deleteFile(String fileId) throws Error
```

#### listFiles()

**Signature:**

```java
public FileListResponse listFiles(FileListQuery query) throws Error
```

#### fileContent()

**Signature:**

```java
public byte[] fileContent(String fileId) throws Error
```

#### createBatch()

**Signature:**

```java
public BatchObject createBatch(CreateBatchRequest req) throws Error
```

#### retrieveBatch()

**Signature:**

```java
public BatchObject retrieveBatch(String batchId) throws Error
```

#### listBatches()

**Signature:**

```java
public BatchListResponse listBatches(BatchListQuery query) throws Error
```

#### cancelBatch()

**Signature:**

```java
public BatchObject cancelBatch(String batchId) throws Error
```

#### createResponse()

**Signature:**

```java
public ResponseObject createResponse(CreateResponseRequest req) throws Error
```

#### retrieveResponse()

**Signature:**

```java
public ResponseObject retrieveResponse(String responseId) throws Error
```

#### cancelResponse()

**Signature:**

```java
public ResponseObject cancelResponse(String responseId) throws Error
```

---

#### DeleteResponse

Response from a delete operation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | ID of the deleted resource. |
| `object` | `String` | — | Object type. |
| `deleted` | `boolean` | — | Confirmation that the resource was deleted. |

---

#### DeveloperMessage

Developer message (system-like message for Claude models).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Developer-specific instructions or context. |
| `name` | `Optional<String>` | `null` | Optional name for the developer message source. |

---

#### DocumentContent

PDF/document content part for vision-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `String` | — | Base64-encoded document data or URL. |
| `mediaType` | `String` | — | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### EmbeddingObject

A single embedding vector.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `embedding` | `List<Double>` | — | The embedding vector. |
| `index` | `int` | — | Index in the batch (corresponds to input order). |

---

#### EmbeddingRequest

Embedding request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"text-embedding-3-small"`). |
| `input` | `EmbeddingInput` | `EmbeddingInput.SINGLE` | Text or texts to embed. |
| `encodingFormat` | `Optional<EmbeddingFormat>` | `null` | Output format: float (native) or base64. |
| `dimensions` | `Optional<Integer>` | `null` | Requested embedding dimensions (if supported by the model). |
| `user` | `Optional<String>` | `null` | User identifier for request tracking. |

---

#### EmbeddingResponse

Embedding response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `List<EmbeddingObject>` | — | List of embeddings. |
| `model` | `String` | — | Model used to generate embeddings. |
| `usage` | `Optional<Usage>` | `/* serde(default) */` | Token usage (input tokens only; embeddings have zero output tokens). |

---

#### FileListQuery

Query parameters for listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `purpose` | `Optional<String>` | `null` | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `limit` | `Optional<Integer>` | `null` | Maximum number of results to return. Defaults to 20. |
| `after` | `Optional<String>` | `null` | Pagination cursor: return results after this file ID. |

---

#### FileListResponse

Response from listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Object type (always `"list"`). |
| `data` | `List<FileObject>` | `Collections.emptyList()` | List of file objects. |
| `hasMore` | `Optional<Boolean>` | `null` | Whether more results are available. |

---

#### FileObject

An uploaded file object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique file ID. |
| `object` | `String` | — | Object type (always `"file"`). |
| `bytes` | `long` | — | File size in bytes. |
| `createdAt` | `long` | — | Unix timestamp of file creation. |
| `filename` | `String` | — | Filename. |
| `purpose` | `String` | — | File purpose. |
| `status` | `Optional<String>` | `null` | Processing status (e.g., `"uploaded"`, `"processed"`). |

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
| `description` | `Optional<String>` | `/* serde(default) */` | Human-readable description explaining what the function does. |
| `parameters` | `Optional<Object>` | `/* serde(default) */` | JSON Schema defining the function's parameters. |
| `strict` | `Optional<Boolean>` | `/* serde(default) */` | If true, enforce strict JSON schema validation for arguments. |

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
| `url` | `Optional<String>` | `null` | Image URL (if response_format was "url"). |
| `b64Json` | `Optional<String>` | `null` | Base64-encoded image data (if response_format was "b64_json"). |
| `revisedPrompt` | `Optional<String>` | `null` | The final prompt used to generate the image (DALL-E 3). |

---

#### ImageUrl

An image URL reference with optional detail level for processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | URL of the image (data URI or HTTP/HTTPS URL). |
| `detail` | `Optional<ImageDetail>` | `null` | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### ImagesResponse

Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `created` | `long` | — | Unix timestamp of image creation. |
| `data` | `List<Image>` | `Collections.emptyList()` | List of generated images. |

---

#### JsonSchemaFormat

JSON Schema specification for constrained output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Name of the schema (must be unique in the request). |
| `description` | `Optional<String>` | `null` | Description of what the schema represents. |
| `schema` | `Object` | — | JSON Schema object defining the output structure. |
| `strict` | `Optional<Boolean>` | `null` | If true, enforce strict schema validation. |

---

#### ModelObject

A model available from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`). |
| `object` | `String` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `long` | — | Unix timestamp of model creation (or release date). |
| `ownedBy` | `String` | — | Organization or entity that owns the model. |

---

#### ModelsListResponse

Response listing available models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `List<ModelObject>` | `Collections.emptyList()` | List of available models. |

---

#### ModerationCategories

Boolean flags for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `boolean` | — | Sexual content. |
| `hate` | `boolean` | — | Hate speech. |
| `harassment` | `boolean` | — | Harassment. |
| `selfHarm` | `boolean` | — | Self-harm content. |
| `sexualMinors` | `boolean` | — | Sexual content involving minors. |
| `hateThreatening` | `boolean` | — | Hate speech that threatens violence. |
| `violenceGraphic` | `boolean` | — | Graphic violence. |
| `selfHarmIntent` | `boolean` | — | Intent to self-harm. |
| `selfHarmInstructions` | `boolean` | — | Instructions for self-harm. |
| `harassmentThreatening` | `boolean` | — | Harassment that threatens violence. |
| `violence` | `boolean` | — | Non-graphic violence. |

---

#### ModerationCategoryScores

Confidence scores for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `double` | — | Sexual content score. |
| `hate` | `double` | — | Hate speech score. |
| `harassment` | `double` | — | Harassment score. |
| `selfHarm` | `double` | — | Self-harm content score. |
| `sexualMinors` | `double` | — | Sexual content involving minors score. |
| `hateThreatening` | `double` | — | Hate speech that threatens violence score. |
| `violenceGraphic` | `double` | — | Graphic violence score. |
| `selfHarmIntent` | `double` | — | Intent to self-harm score. |
| `selfHarmInstructions` | `double` | — | Instructions for self-harm score. |
| `harassmentThreatening` | `double` | — | Harassment that threatens violence score. |
| `violence` | `double` | — | Non-graphic violence score. |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input` | `ModerationInput` | `ModerationInput.SINGLE` | Text or texts to check. |
| `model` | `Optional<String>` | `null` | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this moderation request. |
| `model` | `String` | — | Model used for classification. |
| `results` | `List<ModerationResult>` | — | Results for each input string. |

---

#### ModerationResult

A single moderation classification result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `flagged` | `boolean` | — | True if any category was flagged. |
| `categories` | `ModerationCategories` | — | Boolean flags for each moderation category. |
| `categoryScores` | `ModerationCategoryScores` | — | Confidence scores for each category. |

---

#### OcrImage

An image extracted from an OCR page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique image identifier within the document. |
| `imageBase64` | `Optional<String>` | `/* serde(default) */` | Base64-encoded image data (if `include_image_base64` was true). |

---

#### OcrPage

A single page of OCR output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Page index (0-based). |
| `markdown` | `String` | — | Extracted page content as Markdown. |
| `images` | `Optional<List<OcrImage>>` | `/* serde(default) */` | Embedded images extracted from the page (if `include_image_base64` was true). |
| `dimensions` | `Optional<PageDimensions>` | `/* serde(default) */` | Page dimensions in pixels, if available. |

---

#### OcrRequest

An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document` | `OcrDocument` | `OcrDocument.URL` | The document to process (URL or base64). |
| `pages` | `Optional<List<Integer>>` | `Collections.emptyList()` | Specific pages to process (1-indexed). `null` means all pages. |
| `includeImageBase64` | `Optional<Boolean>` | `null` | Whether to include base64-encoded images of each processed page. |

---

#### OcrResponse

An OCR response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pages` | `List<OcrPage>` | — | Extracted pages in order. |
| `model` | `String` | — | Model/provider used for OCR. |
| `usage` | `Optional<Usage>` | `/* serde(default) */` | Token usage, if reported by the provider. |

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
| `cachedTokens` | `long` | — | Cached tokens present in the prompt. Defaults to 0 when absent. |
| `audioTokens` | `long` | — | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### ProviderConfig

Static configuration for a single provider entry in providers.json.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Provider identifier (matches the entry key in providers.json). |
| `displayName` | `Optional<String>` | `null` | Human-readable provider name shown in UIs. |
| `baseUrl` | `Optional<String>` | `null` | Base URL used as the default for this provider's HTTP client. |
| `auth` | `Optional<AuthConfig>` | `null` | Authentication scheme metadata (auth type + env var holding the key). |
| `endpoints` | `Optional<List<String>>` | `null` | Supported endpoint kinds (e.g. `chat`, `embeddings`). |
| `modelPrefixes` | `Optional<List<String>>` | `null` | Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`). |
| `paramMappings` | `Optional<Map<String, String>>` | `null` | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`).  Applied automatically by `ConfigDrivenProvider.transform_request`. |

---

#### RateLimitConfig

Configuration for per-model rate limits.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rpm` | `Optional<Integer>` | `null` | Maximum requests per window.  `null` means unlimited. |
| `tpm` | `Optional<Long>` | `null` | Maximum tokens per window.  `null` means unlimited. |
| `window` | `Duration` | `60000ms` | Fixed window duration (defaults to 60 s). |

### Methods

#### defaultOptions()

**Signature:**

```java
public static RateLimitConfig defaultOptions()
```

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"cohere/rerank-english-v3.0"`). |
| `query` | `String` | — | The search query. |
| `documents` | `List<RerankDocument>` | `Collections.emptyList()` | Documents to rerank. |
| `topN` | `Optional<Integer>` | `null` | Return only the top N results. Optional. |
| `returnDocuments` | `Optional<Boolean>` | `null` | Include the document content in results. Defaults to false. |

---

#### RerankResponse

Response from the rerank endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `Optional<String>` | `null` | Unique identifier for this rerank request. |
| `results` | `List<RerankResult>` | — | Reranked documents in order of relevance. |
| `meta` | `Optional<Object>` | `/* serde(default) */` | Optional metadata about the reranking operation. |

---

#### RerankResult

A single reranked document with its relevance score.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Original document index in the input list. |
| `relevanceScore` | `double` | — | Relevance score in `[0, 1]`. Higher indicates more relevant. |
| `document` | `Optional<RerankResultDocument>` | `/* serde(default) */` | Original document content (if `return_documents` was true). |

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
| `createdAt` | `long` | — | Unix timestamp of response creation. |
| `model` | `String` | — | Model used to generate the response. |
| `status` | `String` | — | Status (e.g., `"succeeded"`, `"failed"`). |
| `output` | `List<ResponseOutputItem>` | `Collections.emptyList()` | Output items from the response. |
| `usage` | `Optional<ResponseUsage>` | `null` | Token usage. |
| `error` | `Optional<Object>` | `null` | Error details (if status is "failed"). |

---

#### ResponseOutputItem

A single output item from the response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `itemType` | `String` | — | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `content` | `Object` | — | Output content (flattened into the object). |

---

#### ResponseTool

A tool available for the response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `toolType` | `String` | — | Tool type (e.g., "extractor", "search"). |
| `config` | `Object` | — | Tool configuration (flattened into the object). |

---

#### ResponseUsage

Token usage for a response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inputTokens` | `long` | — | Input tokens used. |
| `outputTokens` | `long` | — | Output tokens used. |
| `totalTokens` | `long` | — | Total tokens used. |

---

#### SearchRequest

A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query` | `String` | — | The search query string. |
| `maxResults` | `Optional<Integer>` | `null` | Maximum number of results to return. |
| `searchDomainFilter` | `Optional<List<String>>` | `Collections.emptyList()` | Domain filter — restrict results to specific domains. |
| `country` | `Optional<String>` | `null` | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

---

#### SearchResponse

A search response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `results` | `List<SearchResult>` | — | List of search results. |
| `model` | `String` | — | Model/provider that performed the search. |

---

#### SearchResult

An individual search result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String` | — | Result title. |
| `url` | `String` | — | Result URL. |
| `snippet` | `String` | — | Text snippet or excerpt from the page. |
| `date` | `Optional<String>` | `/* serde(default) */` | Publication or last-updated date, if available. |

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
| `choiceType` | `ToolType` | `ToolType.FUNCTION` | Tool type (always "function"). |
| `function` | `SpecificFunction` | — | The specific function to invoke. |

---

#### StreamChoice

A streaming choice with incremental delta.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index of this choice in the choices array. |
| `delta` | `StreamDelta` | — | Incremental update to the message (content, tool calls, etc.). |
| `finishReason` | `Optional<FinishReason>` | `null` | Why the stream ended (present only in final chunk). |

---

#### StreamDelta

Incremental delta in a stream chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `role` | `Optional<String>` | `null` | Role (typically present only in the first chunk). |
| `content` | `Optional<String>` | `null` | Partial content chunk (e.g., a few words of the response). |
| `toolCalls` | `Optional<List<StreamToolCall>>` | `Collections.emptyList()` | Partial tool calls being streamed. |
| `functionCall` | `Optional<StreamFunctionCall>` | `null` | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal` | `Optional<String>` | `null` | Partial refusal message. |

---

#### StreamFunctionCall

Partial function call details in a stream.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `Optional<String>` | `null` | Function name (typically in the first chunk). |
| `arguments` | `Optional<String>` | `null` | Partial JSON arguments chunk. |

---

#### StreamOptions

Options for streaming responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `includeUsage` | `Optional<Boolean>` | `null` | If true, include token usage in the final stream chunk. |

---

#### StreamToolCall

A streaming tool call being built incrementally.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index of this tool call in the tool_calls array. |
| `id` | `Optional<String>` | `null` | Tool call ID (typically in the first chunk for this call). |
| `callType` | `Optional<ToolType>` | `null` | Tool type (typically "function"). |
| `function` | `Optional<StreamFunctionCall>` | `null` | Partial function name and arguments. |

---

#### SystemMessage

System message guiding model behavior for the entire conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Instructions or context that apply throughout the conversation. |
| `name` | `Optional<String>` | `null` | Optional name for the system message source. |

---

#### ToolCall

A tool call the model wants to execute.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique ID for this call, used to reference in tool result messages. |
| `callType` | `ToolType` | — | Tool type (always "function"). |
| `function` | `FunctionCall` | — | Function name and arguments. |

---

#### ToolMessage

Tool execution result returned to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Result of the tool execution. |
| `toolCallId` | `String` | — | ID of the tool call this result responds to. |
| `name` | `Optional<String>` | `null` | Optional tool/function name. |

---

#### TranscriptionResponse

Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The transcribed text. |
| `language` | `Optional<String>` | `null` | Detected language (ISO-639-1 code). |
| `duration` | `Optional<Double>` | `null` | Total audio duration in seconds. |
| `segments` | `Optional<List<TranscriptionSegment>>` | `Collections.emptyList()` | Detailed segment-level transcription (if response_format is "verbose_json"). |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `int` | — | Segment index (0-based). |
| `start` | `double` | — | Start time in seconds. |
| `end` | `double` | — | End time in seconds. |
| `text` | `String` | — | Transcribed text for this segment. |

---

#### Usage

Token-usage accounting returned by the provider on each completion / embedding call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `promptTokens` | `long` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `completionTokens` | `long` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `totalTokens` | `long` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `promptTokensDetails` | `Optional<PromptTokensDetails>` | `null` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

User message in the conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `UserContent.TEXT` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `name` | `Optional<String>` | `null` | Optional name for the user. |

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
| `TEXT` | Plain text content. — Fields: `0`: `String` |
| `PARTS` | Array of content parts (text, images, documents, audio). — Fields: `0`: `List<ContentPart>` |

---

#### ContentPart

A single content part in a user message — text, image, document, or audio.

| Value | Description |
|-------|-------------|
| `TEXT` | Plain text. — Fields: `text`: `String` |
| `IMAGE_URL` | Image identified by URL (with optional detail level). — Fields: `imageUrl`: `ImageUrl` |
| `DOCUMENT` | Document file (PDF, CSV, etc.) as base64 or URL. — Fields: `document`: `DocumentContent` |
| `INPUT_AUDIO` | Audio input as base64. — Fields: `inputAudio`: `AudioContent` |

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
| `JSON_SCHEMA` | Output must conform to the specified JSON schema. — Fields: `jsonSchema`: `JsonSchemaFormat` |

---

#### StopSequence

Stop sequence(s) that cause the model to stop generating.

| Value | Description |
|-------|-------------|
| `SINGLE` | Single stop sequence. — Fields: `0`: `String` |
| `MULTIPLE` | Multiple stop sequences. — Fields: `0`: `List<String>` |

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
| `SINGLE` | Single text string. — Fields: `0`: `String` |
| `MULTIPLE` | Multiple text strings (batch embedding). — Fields: `0`: `List<String>` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value | Description |
|-------|-------------|
| `SINGLE` | Single text string. — Fields: `0`: `String` |
| `MULTIPLE` | Multiple text strings (batch moderation). — Fields: `0`: `List<String>` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value | Description |
|-------|-------------|
| `TEXT` | Plain text document content. — Fields: `0`: `String` |
| `OBJECT` | Document with explicit text field (may include metadata). — Fields: `text`: `String` |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value | Description |
|-------|-------------|
| `URL` | A publicly accessible document URL. — Fields: `url`: `String` |
| `BASE64` | Inline base64-encoded document data. — Fields: `data`: `String`, `mediaType`: `String` |

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
| `API_KEY` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `String` |
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
| `OPEN_DAL` | OpenDAL-backed storage. Supports 40+ backends (S3, Redis, GCS, local FS, etc.). — Fields: `scheme`: `String`, `config`: `Map<String, String>` |

---

### Errors

#### LiterLlmError

All errors that can occur when using `liter-llm`.

| Variant | Description |
|---------|-------------|
| `AUTHENTICATION` | `status` preserves the exact HTTP status code received (401 or 403). |
| `RATE_LIMITED` | rate limited: {message} |
| `BAD_REQUEST` | `status` preserves the exact HTTP status code received (400, 405, 413, 422, …). |
| `CONTEXT_WINDOW_EXCEEDED` | context window exceeded: {message} |
| `CONTENT_POLICY` | content policy violation: {message} |
| `NOT_FOUND` | not found: {message} |
| `SERVER_ERROR` | `status` preserves the exact HTTP status code received (500, or other 5xx not covered by `ServiceUnavailable`). |
| `SERVICE_UNAVAILABLE` | `status` preserves the exact HTTP status code received (502, 503, or 504). |
| `TIMEOUT` | request timeout |
| `STREAMING` | A catch-all for errors that occur during streaming response processing. This variant covers multiple sub-conditions including UTF-8 decoding failures, CRC/checksum mismatches (AWS EventStream), JSON parse errors in individual SSE chunks, and buffer overflow conditions.  The `message` field contains a human-readable description of the specific failure. |
| `ENDPOINT_NOT_SUPPORTED` | provider {provider} does not support {endpoint} |
| `INVALID_HEADER` | invalid header {name:?}: {reason} |
| `SERIALIZATION` | serialization error: {0} |
| `BUDGET_EXCEEDED` | budget exceeded: {message} |
| `HOOK_REJECTED` | hook rejected: {message} |
| `INTERNAL_ERROR` | An internal logic error (e.g. unexpected Tower response variant). This should never surface in normal operation — if it does, it indicates a bug in the library. |

---
