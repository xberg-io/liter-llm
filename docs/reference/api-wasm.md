---
title: "WebAssembly API Reference"
---

## WebAssembly API Reference <span class="version-badge">v1.4.0-rc.36</span>

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

```typescript
function createClient(apiKey: string, baseUrl?: string, timeoutSecs?: number, maxRetries?: number, modelHint?: string): DefaultClient
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `apiKey` | `string` | Yes | The api key |
| `baseUrl` | `string \| null` | No | The base url |
| `timeoutSecs` | `number \| null` | No | The timeout secs |
| `maxRetries` | `number \| null` | No | The max retries |
| `modelHint` | `string \| null` | No | The model hint |

**Returns:** `DefaultClient`
**Errors:** Throws `Error` with a descriptive message.

---

#### createClientFromJson()

Create a new LLM client from a JSON string.

The JSON object accepts the same fields as `liter-llm.toml` (snake_case).

**Errors:**

Returns `LiterLlmError.BadRequest` if `json` is not valid JSON or
contains unknown fields.

**Signature:**

```typescript
function createClientFromJson(json: string): DefaultClient
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `json` | `string` | Yes | The json |

**Returns:** `DefaultClient`
**Errors:** Throws `Error` with a descriptive message.

---

#### registerCustomProvider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```typescript
function registerCustomProvider(config: CustomProviderConfig): void
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `CustomProviderConfig` | Yes | The configuration options |

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### unregisterCustomProvider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error only if the internal lock is poisoned.

**Signature:**

```typescript
function unregisterCustomProvider(name: string): boolean
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `string` | Yes | The name |

**Returns:** `boolean`
**Errors:** Throws `Error` with a descriptive message.

---

#### allProviders()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.

**Signature:**

```typescript
function allProviders(): Array<ProviderConfig>
```

**Returns:** `Array<ProviderConfig>`
**Errors:** Throws `Error` with a descriptive message.

---

#### complexProviderNames()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```typescript
function complexProviderNames(): Array<string>
```

**Returns:** `Array<string>`
**Errors:** Throws `Error` with a descriptive message.

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

```typescript
function completionCost(model: string, promptTokens: number, completionTokens: number): number | null
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `string` | Yes | The model |
| `promptTokens` | `number` | Yes | The prompt tokens |
| `completionTokens` | `number` | Yes | The completion tokens |

**Returns:** `number | null`

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

```typescript
function completionCostWithCache(model: string, promptTokens: number, cachedTokens: number, completionTokens: number): number | null
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `string` | Yes | The model |
| `promptTokens` | `number` | Yes | The prompt tokens |
| `cachedTokens` | `number` | Yes | The cached tokens |
| `completionTokens` | `number` | Yes | The completion tokens |

**Returns:** `number | null`

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

```typescript
function ensureCryptoProvider(): void
```

**Returns:** `void`

---

### Types

#### AssistantMessage

Assistant's response to a user message.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string \| null` | `null` | The assistant's text response. Absent if tool calls are returned instead. |
| `name` | `string \| null` | `null` | Optional name for the assistant. |
| `toolCalls` | `Array<ToolCall> \| null` | `[]` | Tool calls the model wants to execute, if any. |
| `refusal` | `string \| null` | `null` | Refusal reason, if the model declined to respond per safety policies. |
| `functionCall` | `FunctionCall \| null` | `null` | Deprecated legacy function_call field; retained for API compatibility. |

---

#### AudioContent

Audio content part for speech-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `string` | — | Base64-encoded audio data. |
| `format` | `string` | — | Audio format (e.g., "wav", "mp3", "ogg"). |

---

#### AuthConfig

Auth configuration block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `authType` | `AuthType` | — | Auth scheme classification. |
| `envVar` | `string \| null` | `null` | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### BatchListQuery

Query parameters for listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `number \| null` | `null` | Maximum number of results to return. Defaults to 20. |
| `after` | `string \| null` | `null` | Pagination cursor: return results after this batch ID. |

---

#### BatchListResponse

Response from listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `string` | — | Object type (always `"list"`). |
| `data` | `Array<BatchObject>` | `[]` | List of batch objects. |
| `hasMore` | `boolean \| null` | `null` | Whether more results are available. |
| `firstId` | `string \| null` | `null` | First batch ID in the result set (for pagination). |
| `lastId` | `string \| null` | `null` | Last batch ID in the result set (for pagination). |

---

#### BatchObject

A batch job object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Unique batch ID. |
| `object` | `string` | — | Object type (always `"batch"`). |
| `endpoint` | `string` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `inputFileId` | `string` | — | ID of the input file. |
| `completionWindow` | `string` | — | Completion window (e.g., `"24h"`). |
| `status` | `BatchStatus` | `BatchStatus.Validating` | Current job status. |
| `outputFileId` | `string \| null` | `null` | ID of the output file (present when completed). |
| `errorFileId` | `string \| null` | `null` | ID of the error file (present if some requests failed). |
| `createdAt` | `number` | — | Unix timestamp of batch creation. |
| `completedAt` | `number \| null` | `null` | Unix timestamp of completion (if completed). |
| `failedAt` | `number \| null` | `null` | Unix timestamp of failure (if failed). |
| `expiredAt` | `number \| null` | `null` | Unix timestamp of expiration (if expired). |
| `requestCounts` | `BatchRequestCounts \| null` | `null` | Request processing counts. |
| `metadata` | `unknown \| null` | `null` | Metadata attached to the batch. |

---

#### BatchRequestCounts

Request processing counts for a batch.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total` | `number` | — | Total requests in the batch. |
| `completed` | `number` | — | Completed requests. |
| `failed` | `number` | — | Failed requests. |

---

#### ChatCompletionChunk

A streamed chunk of a chat completion response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Unique identifier for this stream. |
| `object` | `string` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created` | `number` | — | Unix timestamp of chunk creation. |
| `model` | `string` | — | Model used to generate the chunk. |
| `choices` | `Array<StreamChoice>` | `[]` | Streaming choices (delta updates). |
| `usage` | `Usage \| null` | `null` | Token usage (typically only in the final chunk). |
| `systemFingerprint` | `string \| null` | `null` | Fingerprint of the system configuration (OpenAI-specific). |
| `serviceTier` | `string \| null` | `null` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`). |
| `messages` | `Array<Message>` | `[]` | Conversation history from oldest to newest. |
| `temperature` | `number \| null` | `null` | Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0. |
| `topP` | `number \| null` | `null` | Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused. |
| `n` | `number \| null` | `null` | Number of chat completions to generate. Defaults to 1. |
| `stream` | `boolean \| null` | `null` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `stop` | `StopSequence \| null` | `null` | Stop sequence(s) that halt token generation. |
| `maxTokens` | `number \| null` | `null` | Max output tokens. Different from max_completion_tokens in some providers. |
| `presencePenalty` | `number \| null` | `null` | Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics. |
| `frequencyPenalty` | `number \| null` | `null` | Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens. |
| `logitBias` | `Record<string, number> \| null` | `{}` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user` | `string \| null` | `null` | User identifier for request tracking and abuse detection. |
| `tools` | `Array<ChatCompletionTool> \| null` | `[]` | Tools the model can invoke. |
| `toolChoice` | `ToolChoice \| null` | `null` | Tool usage mode (auto, required, none, or specific tool). |
| `parallelToolCalls` | `boolean \| null` | `null` | Whether the model can call multiple tools in parallel. Defaults to true. |
| `responseFormat` | `ResponseFormat \| null` | `null` | Output format constraint (text, JSON, JSON schema). |
| `streamOptions` | `StreamOptions \| null` | `null` | Streaming options (e.g., include_usage). |
| `seed` | `number \| null` | `null` | Random seed for reproducible outputs. Provider support varies. |
| `reasoningEffort` | `ReasoningEffort \| null` | `null` | Reasoning effort level (low, medium, high) for extended-thinking models. |
| `extraBody` | `unknown \| null` | `null` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

#### ChatCompletionResponse

Chat completion response from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Unique identifier for this response. |
| `object` | `string` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `number` | — | Unix timestamp of response creation. |
| `model` | `string` | — | Model used to generate the response. |
| `choices` | `Array<Choice>` | `[]` | List of completion choices. |
| `usage` | `Usage \| null` | `null` | Token usage statistics. |
| `systemFingerprint` | `string \| null` | `null` | Fingerprint of the system configuration (OpenAI-specific). |
| `serviceTier` | `string \| null` | `null` | Service tier used (OpenAI-specific). |

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
| `index` | `number` | — | Index of this choice in the choices array. |
| `message` | `AssistantMessage` | — | The assistant's message response. |
| `finishReason` | `FinishReason \| null` | `null` | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

---

#### CreateBatchRequest

Request to create a batch job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inputFileId` | `string` | — | ID of the uploaded input file (JSONL format). |
| `endpoint` | `string` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `completionWindow` | `string` | — | Completion window (e.g., `"24h"`). |
| `metadata` | `unknown \| null` | `null` | Optional metadata to attach to the batch. |

---

#### CreateFileRequest

Request to upload a file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | `string` | — | Base64-encoded file data. |
| `purpose` | `FilePurpose` | `FilePurpose.Assistants` | Purpose for the file. |
| `filename` | `string \| null` | `null` | Optional filename to associate with the upload. |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt` | `string` | — | Text description of the image to generate. |
| `model` | `string \| null` | `null` | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `n` | `number \| null` | `null` | Number of images to generate. Defaults to 1. |
| `size` | `string \| null` | `null` | Image size (e.g., `"1024x1024"`, `"1792x1024"`). |
| `quality` | `string \| null` | `null` | Image quality: `"standard"` or `"hd"`. |
| `style` | `string \| null` | `null` | Style: `"natural"` or `"vivid"` (DALL-E 3 only). |
| `responseFormat` | `string \| null` | `null` | Response format: `"url"` or `"b64_json"`. |
| `user` | `string \| null` | `null` | User identifier for request tracking. |

---

#### CreateResponseRequest

Request to create a structured response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | Model ID. |
| `input` | `unknown` | — | Input data to process (e.g., a document to extract from). |
| `instructions` | `string \| null` | `null` | Instructions for processing the input. |
| `tools` | `Array<ResponseTool> \| null` | `[]` | Available tools the model can use. |
| `temperature` | `number \| null` | `null` | Sampling temperature in `[0.0, 2.0]`. Defaults to 1.0. |
| `maxOutputTokens` | `number \| null` | `null` | Maximum output tokens. |
| `metadata` | `unknown \| null` | `null` | Optional metadata. |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`). |
| `input` | `string` | — | Text to synthesize into speech. |
| `voice` | `string` | — | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `responseFormat` | `string \| null` | `null` | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`). |
| `speed` | `number \| null` | `null` | Playback speed in `[0.25, 4.0]`. Defaults to 1.0. |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | Model ID (e.g., `"whisper-1"`). |
| `file` | `string` | — | Base64-encoded audio file data. |
| `language` | `string \| null` | `null` | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `prompt` | `string \| null` | `null` | Optional text to guide the model (improves accuracy for domain-specific terms). |
| `responseFormat` | `string \| null` | `null` | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`). |
| `temperature` | `number \| null` | `null` | Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0. |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Unique name for this provider (e.g., "my-provider"). |
| `baseUrl` | `string` | — | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `authHeader` | `AuthHeaderFormat` | — | Authentication header format. |
| `modelPrefixes` | `Array<string>` | — | Model name prefixes that route to this provider (e.g., `["my-"]`). |

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

```typescript
chat(req: ChatCompletionRequest): ChatCompletionResponse
```

#### chatStream()

**Signature:**

```typescript
chatStream(req: ChatCompletionRequest): string
```

#### embed()

**Signature:**

```typescript
embed(req: EmbeddingRequest): EmbeddingResponse
```

#### listModels()

**Signature:**

```typescript
listModels(): ModelsListResponse
```

#### imageGenerate()

**Signature:**

```typescript
imageGenerate(req: CreateImageRequest): ImagesResponse
```

#### speech()

**Signature:**

```typescript
speech(req: CreateSpeechRequest): Buffer
```

#### transcribe()

**Signature:**

```typescript
transcribe(req: CreateTranscriptionRequest): TranscriptionResponse
```

#### moderate()

**Signature:**

```typescript
moderate(req: ModerationRequest): ModerationResponse
```

#### rerank()

**Signature:**

```typescript
rerank(req: RerankRequest): RerankResponse
```

#### search()

**Signature:**

```typescript
search(req: SearchRequest): SearchResponse
```

#### ocr()

**Signature:**

```typescript
ocr(req: OcrRequest): OcrResponse
```

#### createFile()

**Signature:**

```typescript
createFile(req: CreateFileRequest): FileObject
```

#### retrieveFile()

**Signature:**

```typescript
retrieveFile(fileId: string): FileObject
```

#### deleteFile()

**Signature:**

```typescript
deleteFile(fileId: string): DeleteResponse
```

#### listFiles()

**Signature:**

```typescript
listFiles(query: FileListQuery): FileListResponse
```

#### fileContent()

**Signature:**

```typescript
fileContent(fileId: string): Buffer
```

#### createBatch()

**Signature:**

```typescript
createBatch(req: CreateBatchRequest): BatchObject
```

#### retrieveBatch()

**Signature:**

```typescript
retrieveBatch(batchId: string): BatchObject
```

#### listBatches()

**Signature:**

```typescript
listBatches(query: BatchListQuery): BatchListResponse
```

#### cancelBatch()

**Signature:**

```typescript
cancelBatch(batchId: string): BatchObject
```

#### createResponse()

**Signature:**

```typescript
createResponse(req: CreateResponseRequest): ResponseObject
```

#### retrieveResponse()

**Signature:**

```typescript
retrieveResponse(responseId: string): ResponseObject
```

#### cancelResponse()

**Signature:**

```typescript
cancelResponse(responseId: string): ResponseObject
```

---

#### DeleteResponse

Response from a delete operation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | ID of the deleted resource. |
| `object` | `string` | — | Object type. |
| `deleted` | `boolean` | — | Confirmation that the resource was deleted. |

---

#### DeveloperMessage

Developer message (system-like message for Claude models).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Developer-specific instructions or context. |
| `name` | `string \| null` | `null` | Optional name for the developer message source. |

---

#### DocumentContent

PDF/document content part for vision-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `string` | — | Base64-encoded document data or URL. |
| `mediaType` | `string` | — | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### EmbeddingObject

A single embedding vector.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `string` | — | Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `embedding` | `Array<number>` | — | The embedding vector. |
| `index` | `number` | — | Index in the batch (corresponds to input order). |

---

#### EmbeddingRequest

Embedding request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | Model ID (e.g., `"text-embedding-3-small"`). |
| `input` | `EmbeddingInput` | `EmbeddingInput.Single` | Text or texts to embed. |
| `encodingFormat` | `EmbeddingFormat \| null` | `null` | Output format: float (native) or base64. |
| `dimensions` | `number \| null` | `null` | Requested embedding dimensions (if supported by the model). |
| `user` | `string \| null` | `null` | User identifier for request tracking. |

---

#### EmbeddingResponse

Embedding response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `string` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `Array<EmbeddingObject>` | — | List of embeddings. |
| `model` | `string` | — | Model used to generate embeddings. |
| `usage` | `Usage \| null` | `/* serde(default) */` | Token usage (input tokens only; embeddings have zero output tokens). |

---

#### FileListQuery

Query parameters for listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `purpose` | `string \| null` | `null` | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `limit` | `number \| null` | `null` | Maximum number of results to return. Defaults to 20. |
| `after` | `string \| null` | `null` | Pagination cursor: return results after this file ID. |

---

#### FileListResponse

Response from listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `string` | — | Object type (always `"list"`). |
| `data` | `Array<FileObject>` | `[]` | List of file objects. |
| `hasMore` | `boolean \| null` | `null` | Whether more results are available. |

---

#### FileObject

An uploaded file object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Unique file ID. |
| `object` | `string` | — | Object type (always `"file"`). |
| `bytes` | `number` | — | File size in bytes. |
| `createdAt` | `number` | — | Unix timestamp of file creation. |
| `filename` | `string` | — | Filename. |
| `purpose` | `string` | — | File purpose. |
| `status` | `string \| null` | `null` | Processing status (e.g., `"uploaded"`, `"processed"`). |

---

#### FunctionCall

Function call details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Function name. |
| `arguments` | `string` | — | Arguments as a JSON string (parse with serde_json.from_str). |

---

#### FunctionDefinition

Function definition exposed to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Name of the function. Required and must be alphanumeric + underscores. |
| `description` | `string \| null` | `/* serde(default) */` | Human-readable description explaining what the function does. |
| `parameters` | `unknown \| null` | `/* serde(default) */` | JSON Schema defining the function's parameters. |
| `strict` | `boolean \| null` | `/* serde(default) */` | If true, enforce strict JSON schema validation for arguments. |

---

#### FunctionMessage

Deprecated legacy function-role message body.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | The extracted text content |
| `name` | `string` | — | The name |

---

#### Image

A single generated image, returned as either a URL or base64 data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `string \| null` | `null` | Image URL (if response_format was "url"). |
| `b64Json` | `string \| null` | `null` | Base64-encoded image data (if response_format was "b64_json"). |
| `revisedPrompt` | `string \| null` | `null` | The final prompt used to generate the image (DALL-E 3). |

---

#### ImageUrl

An image URL reference with optional detail level for processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `string` | — | URL of the image (data URI or HTTP/HTTPS URL). |
| `detail` | `ImageDetail \| null` | `null` | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### ImagesResponse

Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `created` | `number` | — | Unix timestamp of image creation. |
| `data` | `Array<Image>` | `[]` | List of generated images. |

---

#### JsonSchemaFormat

JSON Schema specification for constrained output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Name of the schema (must be unique in the request). |
| `description` | `string \| null` | `null` | Description of what the schema represents. |
| `schema` | `unknown` | — | JSON Schema object defining the output structure. |
| `strict` | `boolean \| null` | `null` | If true, enforce strict schema validation. |

---

#### ModelObject

A model available from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`). |
| `object` | `string` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `number` | — | Unix timestamp of model creation (or release date). |
| `ownedBy` | `string` | — | Organization or entity that owns the model. |

---

#### ModelsListResponse

Response listing available models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `string` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `Array<ModelObject>` | `[]` | List of available models. |

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
| `sexual` | `number` | — | Sexual content score. |
| `hate` | `number` | — | Hate speech score. |
| `harassment` | `number` | — | Harassment score. |
| `selfHarm` | `number` | — | Self-harm content score. |
| `sexualMinors` | `number` | — | Sexual content involving minors score. |
| `hateThreatening` | `number` | — | Hate speech that threatens violence score. |
| `violenceGraphic` | `number` | — | Graphic violence score. |
| `selfHarmIntent` | `number` | — | Intent to self-harm score. |
| `selfHarmInstructions` | `number` | — | Instructions for self-harm score. |
| `harassmentThreatening` | `number` | — | Harassment that threatens violence score. |
| `violence` | `number` | — | Non-graphic violence score. |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input` | `ModerationInput` | `ModerationInput.Single` | Text or texts to check. |
| `model` | `string \| null` | `null` | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Unique identifier for this moderation request. |
| `model` | `string` | — | Model used for classification. |
| `results` | `Array<ModerationResult>` | — | Results for each input string. |

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
| `id` | `string` | — | Unique image identifier within the document. |
| `imageBase64` | `string \| null` | `/* serde(default) */` | Base64-encoded image data (if `include_image_base64` was true). |

---

#### OcrPage

A single page of OCR output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `number` | — | Page index (0-based). |
| `markdown` | `string` | — | Extracted page content as Markdown. |
| `images` | `Array<OcrImage> \| null` | `/* serde(default) */` | Embedded images extracted from the page (if `include_image_base64` was true). |
| `dimensions` | `PageDimensions \| null` | `/* serde(default) */` | Page dimensions in pixels, if available. |

---

#### OcrRequest

An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document` | `OcrDocument` | `OcrDocument.Url` | The document to process (URL or base64). |
| `pages` | `Array<number> \| null` | `[]` | Specific pages to process (1-indexed). `null` means all pages. |
| `includeImageBase64` | `boolean \| null` | `null` | Whether to include base64-encoded images of each processed page. |

---

#### OcrResponse

An OCR response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pages` | `Array<OcrPage>` | — | Extracted pages in order. |
| `model` | `string` | — | Model/provider used for OCR. |
| `usage` | `Usage \| null` | `/* serde(default) */` | Token usage, if reported by the provider. |

---

#### PageDimensions

Page dimensions in pixels.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `number` | — | Width in pixels. |
| `height` | `number` | — | Height in pixels. |

---

#### PromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage.prompt_tokens` — it is *not* an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cachedTokens` | `number` | — | Cached tokens present in the prompt. Defaults to 0 when absent. |
| `audioTokens` | `number` | — | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### ProviderConfig

Static configuration for a single provider entry in providers.json.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Provider identifier (matches the entry key in providers.json). |
| `displayName` | `string \| null` | `null` | Human-readable provider name shown in UIs. |
| `baseUrl` | `string \| null` | `null` | Base URL used as the default for this provider's HTTP client. |
| `auth` | `AuthConfig \| null` | `null` | Authentication scheme metadata (auth type + env var holding the key). |
| `endpoints` | `Array<string> \| null` | `null` | Supported endpoint kinds (e.g. `chat`, `embeddings`). |
| `modelPrefixes` | `Array<string> \| null` | `null` | Model-name prefixes claimed by this provider (e.g. `["gpt-", "o1-"]`). |
| `paramMappings` | `Record<string, string> \| null` | `null` | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`).  Applied automatically by `ConfigDrivenProvider.transform_request`. |

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | Model ID (e.g., `"cohere/rerank-english-v3.0"`). |
| `query` | `string` | — | The search query. |
| `documents` | `Array<RerankDocument>` | `[]` | Documents to rerank. |
| `topN` | `number \| null` | `null` | Return only the top N results. Optional. |
| `returnDocuments` | `boolean \| null` | `null` | Include the document content in results. Defaults to false. |

---

#### RerankResponse

Response from the rerank endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string \| null` | `null` | Unique identifier for this rerank request. |
| `results` | `Array<RerankResult>` | — | Reranked documents in order of relevance. |
| `meta` | `unknown \| null` | `/* serde(default) */` | Optional metadata about the reranking operation. |

---

#### RerankResult

A single reranked document with its relevance score.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `number` | — | Original document index in the input list. |
| `relevanceScore` | `number` | — | Relevance score in `[0, 1]`. Higher indicates more relevant. |
| `document` | `RerankResultDocument \| null` | `/* serde(default) */` | Original document content (if `return_documents` was true). |

---

#### RerankResultDocument

The text content of a reranked document, returned when `return_documents` is true.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `string` | — | Document text. |

---

#### ResponseObject

Response from a structured response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Unique response ID. |
| `object` | `string` | — | Object type (e.g., `"response"`). |
| `createdAt` | `number` | — | Unix timestamp of response creation. |
| `model` | `string` | — | Model used to generate the response. |
| `status` | `string` | — | Status (e.g., `"succeeded"`, `"failed"`). |
| `output` | `Array<ResponseOutputItem>` | `[]` | Output items from the response. |
| `usage` | `ResponseUsage \| null` | `null` | Token usage. |
| `error` | `unknown \| null` | `null` | Error details (if status is "failed"). |

---

#### ResponseOutputItem

A single output item from the response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `itemType` | `string` | — | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `content` | `unknown` | — | Output content (flattened into the object). |

---

#### ResponseTool

A tool available for the response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `toolType` | `string` | — | Tool type (e.g., "extractor", "search"). |
| `config` | `unknown` | — | Tool configuration (flattened into the object). |

---

#### ResponseUsage

Token usage for a response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inputTokens` | `number` | — | Input tokens used. |
| `outputTokens` | `number` | — | Output tokens used. |
| `totalTokens` | `number` | — | Total tokens used. |

---

#### SearchRequest

A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query` | `string` | — | The search query string. |
| `maxResults` | `number \| null` | `null` | Maximum number of results to return. |
| `searchDomainFilter` | `Array<string> \| null` | `[]` | Domain filter — restrict results to specific domains. |
| `country` | `string \| null` | `null` | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

---

#### SearchResponse

A search response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `results` | `Array<SearchResult>` | — | List of search results. |
| `model` | `string` | — | Model/provider that performed the search. |

---

#### SearchResult

An individual search result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `string` | — | Result title. |
| `url` | `string` | — | Result URL. |
| `snippet` | `string` | — | Text snippet or excerpt from the page. |
| `date` | `string \| null` | `/* serde(default) */` | Publication or last-updated date, if available. |

---

#### SpecificFunction

Name of the specific function to invoke.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Function name. |

---

#### SpecificToolChoice

Directive to call a specific tool.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `choiceType` | `ToolType` | `ToolType.Function` | Tool type (always "function"). |
| `function` | `SpecificFunction` | — | The specific function to invoke. |

---

#### StreamChoice

A streaming choice with incremental delta.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `number` | — | Index of this choice in the choices array. |
| `delta` | `StreamDelta` | — | Incremental update to the message (content, tool calls, etc.). |
| `finishReason` | `FinishReason \| null` | `null` | Why the stream ended (present only in final chunk). |

---

#### StreamDelta

Incremental delta in a stream chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `role` | `string \| null` | `null` | Role (typically present only in the first chunk). |
| `content` | `string \| null` | `null` | Partial content chunk (e.g., a few words of the response). |
| `toolCalls` | `Array<StreamToolCall> \| null` | `[]` | Partial tool calls being streamed. |
| `functionCall` | `StreamFunctionCall \| null` | `null` | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal` | `string \| null` | `null` | Partial refusal message. |

---

#### StreamFunctionCall

Partial function call details in a stream.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string \| null` | `null` | Function name (typically in the first chunk). |
| `arguments` | `string \| null` | `null` | Partial JSON arguments chunk. |

---

#### StreamOptions

Options for streaming responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `includeUsage` | `boolean \| null` | `null` | If true, include token usage in the final stream chunk. |

---

#### StreamToolCall

A streaming tool call being built incrementally.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `number` | — | Index of this tool call in the tool_calls array. |
| `id` | `string \| null` | `null` | Tool call ID (typically in the first chunk for this call). |
| `callType` | `ToolType \| null` | `null` | Tool type (typically "function"). |
| `function` | `StreamFunctionCall \| null` | `null` | Partial function name and arguments. |

---

#### SystemMessage

System message guiding model behavior for the entire conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Instructions or context that apply throughout the conversation. |
| `name` | `string \| null` | `null` | Optional name for the system message source. |

---

#### ToolCall

A tool call the model wants to execute.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Unique ID for this call, used to reference in tool result messages. |
| `callType` | `ToolType` | — | Tool type (always "function"). |
| `function` | `FunctionCall` | — | Function name and arguments. |

---

#### ToolMessage

Tool execution result returned to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Result of the tool execution. |
| `toolCallId` | `string` | — | ID of the tool call this result responds to. |
| `name` | `string \| null` | `null` | Optional tool/function name. |

---

#### TranscriptionResponse

Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `string` | — | The transcribed text. |
| `language` | `string \| null` | `null` | Detected language (ISO-639-1 code). |
| `duration` | `number \| null` | `null` | Total audio duration in seconds. |
| `segments` | `Array<TranscriptionSegment> \| null` | `[]` | Detailed segment-level transcription (if response_format is "verbose_json"). |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `number` | — | Segment index (0-based). |
| `start` | `number` | — | Start time in seconds. |
| `end` | `number` | — | End time in seconds. |
| `text` | `string` | — | Transcribed text for this segment. |

---

#### Usage

Token-usage accounting returned by the provider on each completion / embedding call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `promptTokens` | `number` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `completionTokens` | `number` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `totalTokens` | `number` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `promptTokensDetails` | `PromptTokensDetails \| null` | `null` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

User message in the conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `UserContent.Text` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `name` | `string \| null` | `null` | Optional name for the user. |

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
| `Text` | Plain text content. — Fields: `0`: `string` |
| `Parts` | Array of content parts (text, images, documents, audio). — Fields: `0`: `Array<ContentPart>` |

---

#### ContentPart

A single content part in a user message — text, image, document, or audio.

| Value | Description |
|-------|-------------|
| `Text` | Plain text. — Fields: `text`: `string` |
| `ImageUrl` | Image identified by URL (with optional detail level). — Fields: `imageUrl`: `ImageUrl` |
| `Document` | Document file (PDF, CSV, etc.) as base64 or URL. — Fields: `document`: `DocumentContent` |
| `InputAudio` | Audio input as base64. — Fields: `inputAudio`: `AudioContent` |

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
| `JsonSchema` | Output must conform to the specified JSON schema. — Fields: `jsonSchema`: `JsonSchemaFormat` |

---

#### StopSequence

Stop sequence(s) that cause the model to stop generating.

| Value | Description |
|-------|-------------|
| `Single` | Single stop sequence. — Fields: `0`: `string` |
| `Multiple` | Multiple stop sequences. — Fields: `0`: `Array<string>` |

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
| `Single` | Single text string. — Fields: `0`: `string` |
| `Multiple` | Multiple text strings (batch embedding). — Fields: `0`: `Array<string>` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value | Description |
|-------|-------------|
| `Single` | Single text string. — Fields: `0`: `string` |
| `Multiple` | Multiple text strings (batch moderation). — Fields: `0`: `Array<string>` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value | Description |
|-------|-------------|
| `Text` | Plain text document content. — Fields: `0`: `string` |
| `Object` | Document with explicit text field (may include metadata). — Fields: `text`: `string` |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value | Description |
|-------|-------------|
| `Url` | A publicly accessible document URL. — Fields: `url`: `string` |
| `Base64` | Inline base64-encoded document data. — Fields: `data`: `string`, `mediaType`: `string` |

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
| `ApiKey` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `string` |
| `None` | No authentication required. |

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

### Errors

#### LiterLlmError

All errors that can occur when using `liter-llm`.

Errors are thrown as plain `Error` objects with descriptive messages.

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

---
