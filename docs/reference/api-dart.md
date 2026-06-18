---
title: "Dart API Reference"
---

## Dart API Reference <span class="version-badge">v1.7.1</span>

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

```dart
DefaultClient createClient(String apiKey, [String? baseUrl, int? timeoutSecs, int? maxRetries, String? modelHint])
```

**Example:**

```dart
final result = createClient("value", "value", 42, 42, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `apiKey` | `String` | Yes | The api key |
| `baseUrl` | `String?` | No | The base url |
| `timeoutSecs` | `int?` | No | The timeout secs |
| `maxRetries` | `int?` | No | The max retries |
| `modelHint` | `String?` | No | The model hint |

**Returns:** `DefaultClient`

**Errors:** Throws `Error`.

---

#### createClientFromJson()

Create a new LLM client from a JSON string.

The JSON object accepts the same fields as `liter-llm.toml` (snake_case).

**Errors:**

Returns `LiterLlmError.BadRequest` if `json` is not valid JSON or
contains unknown fields.

**Signature:**

```dart
DefaultClient createClientFromJson(String json)
```

**Example:**

```dart
final result = createClientFromJson("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `json` | `String` | Yes | The json |

**Returns:** `DefaultClient`

**Errors:** Throws `Error`.

---

#### encodeDataUrl()

Encode bytes as a base64 data URL: `data:<mime>;base64,<b64>`.

`mime` defaults to `IMAGE_PNG` when `null`.

**Signature:**

```dart
String encodeDataUrl(Uint8List bytes, [String? mime])
```

**Example:**

```dart
final result = encodeDataUrl(Uint8List.fromList([100, 97, 116, 97]), "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `bytes` | `Uint8List` | Yes | The bytes |
| `mime` | `String?` | No | The mime |

**Returns:** `String`

---

#### decodeDataUrl()

Decode a base64 data URL into `DecodedDataUrl`.

Returns `null` for:

- Non-data URLs (strings that do not start with `"data:"`).
- Malformed prefixes (missing `";base64,"` marker).
- Invalid base64 payloads.

The returned MIME string is extracted verbatim from the URL prefix —
it is not validated or normalised.

**Signature:**

```dart
DecodedDataUrl? decodeDataUrl(String url)
```

**Example:**

```dart
final result = decodeDataUrl("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `url` | `String` | Yes | The URL to fetch |

**Returns:** `DecodedDataUrl?`

---

#### registerCustomProvider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```dart
void registerCustomProvider(CustomProviderConfig config)
```

**Example:**

```dart
registerCustomProvider(CustomProviderConfig());
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `CustomProviderConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Throws `Error`.

---

#### unregisterCustomProvider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error if the custom-provider registry cannot be updated.

**Signature:**

```dart
bool unregisterCustomProvider(String name)
```

**Example:**

```dart
final result = unregisterCustomProvider("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `bool`

**Errors:** Throws `Error`.

---

#### capabilities()

Return the capability flags for a named provider.

Performs an O(n) linear scan over the embedded registry (143 entries).
Returns an owned value so bindings can pass capability data without
borrowing registry internals.

For unknown `provider_name` values the function returns an all-`false`
sentinel so callers never need to handle `Option`.

**Signature:**

```dart
ProviderCapabilities capabilities(String providerName)
```

**Example:**

```dart
final result = capabilities("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `providerName` | `String` | Yes | The provider name |

**Returns:** `ProviderCapabilities`

---

#### allProviders()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.
Returns the public `ProviderConfig` slice (without capability flags).
To query capability flags for a specific provider use `capabilities`.

**Signature:**

```dart
List<ProviderConfig> allProviders()
```

**Example:**

```dart
final result = allProviders();
```

**Returns:** `List<ProviderConfig>`

**Errors:** Throws `Error`.

---

#### complexProviderNames()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```dart
List<String> complexProviderNames()
```

**Example:**

```dart
final result = complexProviderNames();
```

**Returns:** `List<String>`

**Errors:** Throws `Error`.

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

```dart
double? completionCost(String model, int promptTokens, int completionTokens)
```

**Example:**

```dart
final result = completionCost("value", 42, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `promptTokens` | `int` | Yes | The prompt tokens |
| `completionTokens` | `int` | Yes | The completion tokens |

**Returns:** `double?`

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

```dart
double? completionCostWithCache(String model, int promptTokens, int cachedTokens, int completionTokens)
```

**Example:**

```dart
final result = completionCostWithCache("value", 42, 42, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `promptTokens` | `int` | Yes | The prompt tokens |
| `cachedTokens` | `int` | Yes | The cached tokens |
| `completionTokens` | `int` | Yes | The completion tokens |

**Returns:** `double?`

---

#### clear()

Remove all guardrails from the global registry.

Primarily useful in tests to reset state between test cases.

**Panics:**

Panics if the global registry lock is poisoned.

**Signature:**

```dart
void clear()
```

**Example:**

```dart
clear();
```

**Returns:** No return value.

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

```dart
int countTokens(String model, String text)
```

**Example:**

```dart
final result = countTokens("value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `text` | `String` | Yes | The text |

**Returns:** `int`

**Errors:** Throws `Error`.

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

```dart
int countRequestTokens(String model, ChatCompletionRequest req)
```

**Example:**

```dart
final result = countRequestTokens("value", ChatCompletionRequest());
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model |
| `req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `int`

**Errors:** Throws `Error`.

---

#### checkBound()

Assert that `current_len + incoming` does not exceed `limit`.

Call this before appending `incoming` bytes to any buffer that must
stay below `limit`. Returns `Err(LiterLlmError.Streaming)` on overflow
and emits a `tracing.warn!` with context.

**Signature:**

```dart
void checkBound(String context, int currentLen, int incoming, int limit)
```

**Example:**

```dart
checkBound("value", 42, 42, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `context` | `String` | Yes | The context |
| `currentLen` | `int` | Yes | The current len |
| `incoming` | `int` | Yes | The incoming |
| `limit` | `int` | Yes | The limit |

**Returns:** No return value.

**Errors:** Throws `Error`.

---

#### ensureCryptoProvider()

Install the `ring` crypto provider as the rustls process default, idempotently.

rustls 0.23+ removed the implicit default provider. This function installs
`ring` once per process. Subsequent calls are no-ops. Calling it after
another rustls crypto provider has already been installed is safe: the
`Err` from `install_default()` is silently ignored.

Called automatically by every internal `reqwest.Client` constructor
(auth providers, default HTTP client). Bindings and downstream consumers
reach those constructors transitively, so no manual init is required.

WASM builds are exempt — the WASM target uses the browser/Node.js fetch
API instead of rustls, so no crypto provider is needed.

Windows builds use native-tls (SChannel) via reqwest, so rustls is not
present and no crypto provider installation is needed.

**Signature:**

```dart
void ensureCryptoProvider()
```

**Example:**

```dart
ensureCryptoProvider();
```

**Returns:** No return value.

---

#### ensureCryptoProvider()

No-op on Windows: reqwest uses native-tls (SChannel), so no rustls provider
installation is needed. All callers use the same call site regardless of
platform.

**Signature:**

```dart
void ensureCryptoProvider()
```

**Example:**

```dart
ensureCryptoProvider();
```

**Returns:** No return value.

---

### Types

#### AssistantMessage

Assistant's response to a user message.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `AssistantContent?` | `null` | The assistant's response: plain text, structured parts, or absent. `null` is valid when the model replies with tool calls only. |
| `name` | `String?` | `null` | Optional name for the assistant. |
| `toolCalls` | `List<ToolCall>?` | `\[\]` | Tool calls the model wants to execute, if any. |
| `refusal` | `String?` | `null` | Refusal reason, if the model declined to respond per safety policies. |
| `functionCall` | `FunctionCall?` | `null` | Deprecated legacy function_call field; retained for API compatibility. |

##### Methods

###### text()

Return the assistant's textual response, concatenating all `Text` parts
if the content is structured.

Returns `null` for `Refusal`-only or `OutputImage`-only responses.

**Signature:**

```dart
String? text()
```

**Example:**

```dart
final result = instance.text();
```

**Returns:** `String?`

###### refusalText()

Return the refusal message, if the model declined to respond.

Checks both the top-level `refusal` field and any `Refusal` parts
inside a structured `content`.

**Signature:**

```dart
String? refusalText()
```

**Example:**

```dart
final result = instance.refusalText();
```

**Returns:** `String?`

###### outputImages()

Return all `AssistantPart.OutputImage` parts in the response.

**Signature:**

```dart
List<ImageUrl> outputImages()
```

**Example:**

```dart
final result = instance.outputImages();
```

**Returns:** `List<ImageUrl>`

###### outputAudio()

Return all `AssistantPart.OutputAudio` parts in the response.

**Signature:**

```dart
List<AudioContent> outputAudio()
```

**Example:**

```dart
final result = instance.outputAudio();
```

**Returns:** `List<AudioContent>`

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
| `envVar` | `String?` | `null` | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### BatchListQuery

Query parameters for listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `int?` | `null` | Maximum number of results to return. Defaults to 20. |
| `after` | `String?` | `null` | Pagination cursor: return results after this batch ID. |

---

#### BatchListResponse

Response from listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Object type (always `"list"`). |
| `data` | `List<BatchObject>` | `\[\]` | List of batch objects. |
| `hasMore` | `bool?` | `null` | Whether more results are available. |
| `firstId` | `String?` | `null` | First batch ID in the result set (for pagination). |
| `lastId` | `String?` | `null` | Last batch ID in the result set (for pagination). |

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
| `status` | `BatchStatus` | `BatchStatus.Validating` | Current job status. |
| `outputFileId` | `String?` | `null` | ID of the output file (present when completed). |
| `errorFileId` | `String?` | `null` | ID of the error file (present if some requests failed). |
| `createdAt` | `int` | — | Unix timestamp of batch creation. |
| `completedAt` | `int?` | `null` | Unix timestamp of completion (if completed). |
| `failedAt` | `int?` | `null` | Unix timestamp of failure (if failed). |
| `expiredAt` | `int?` | `null` | Unix timestamp of expiration (if expired). |
| `requestCounts` | `BatchRequestCounts?` | `null` | Request processing counts. |
| `metadata` | `String?` | `null` | Metadata attached to the batch. |

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
| `globalLimit` | `double?` | `null` | Maximum total spend across all models, in USD.  `null` means unlimited. |
| `modelLimits` | `Map<String, double>` | `{}` | Per-model spending limits in USD.  Models not listed here are only constrained by `global_limit`. |
| `enforcement` | `Enforcement` | `Enforcement.Hard` | Whether to reject requests or merely warn when a limit is exceeded. |

##### Methods

###### default()

**Signature:**

```dart
static BudgetConfig default()
```

**Example:**

```dart
final result = BudgetConfig.default();
```

**Returns:** `BudgetConfig`

---

#### CacheConfig

Configuration for the response cache.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `maxEntries` | `int` | `256` | Maximum number of cached entries. |
| `ttl` | `Duration` | `300000ms` | Time-to-live for each cached entry. |
| `backend` | `CacheBackend` | `CacheBackend.Memory` | Storage backend to use. |

##### Methods

###### default()

**Signature:**

```dart
static CacheConfig default()
```

**Example:**

```dart
final result = CacheConfig.default();
```

**Returns:** `CacheConfig`

---

#### ChatCompletionChunk

A streamed chunk of a chat completion response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this stream. |
| `object` | `String` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created` | `int` | — | Unix timestamp of chunk creation. |
| `model` | `String` | — | Model used to generate the chunk. |
| `choices` | `List<StreamChoice>` | `\[\]` | Streaming choices (delta updates). |
| `usage` | `Usage?` | `null` | Token usage (typically only in the final chunk). |
| `systemFingerprint` | `String?` | `null` | Fingerprint of the system configuration (OpenAI-specific). |
| `serviceTier` | `String?` | `null` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`). |
| `messages` | `List<Message>` | `\[\]` | Conversation history from oldest to newest. |
| `temperature` | `double?` | `null` | Sampling temperature in `\[0.0, 2.0\]`. Higher increases randomness. Defaults to 1.0. |
| `topP` | `double?` | `null` | Nucleus sampling parameter in `\[0.0, 1.0\]`. Lower is more focused. |
| `n` | `int?` | `null` | Number of chat completions to generate. Defaults to 1. |
| `stream` | `bool?` | `null` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `stop` | `StopSequence?` | `null` | Stop sequence(s) that halt token generation. |
| `maxTokens` | `int?` | `null` | Max output tokens. Different from max_completion_tokens in some providers. |
| `presencePenalty` | `double?` | `null` | Presence penalty in `\[-2.0, 2.0\]`. Positive discourages repeated topics. |
| `frequencyPenalty` | `double?` | `null` | Frequency penalty in `\[-2.0, 2.0\]`. Positive discourages repeated tokens. |
| `logitBias` | `Map<String, double>?` | `{}` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user` | `String?` | `null` | User identifier for request tracking and abuse detection. |
| `tools` | `List<ChatCompletionTool>?` | `\[\]` | Tools the model can invoke. |
| `toolChoice` | `ToolChoice?` | `null` | Tool usage mode (auto, required, none, or specific tool). |
| `parallelToolCalls` | `bool?` | `null` | Whether the model can call multiple tools in parallel. Defaults to true. |
| `responseFormat` | `ResponseFormat?` | `null` | Output format constraint (text, JSON, JSON schema). |
| `streamOptions` | `StreamOptions?` | `null` | Streaming options (e.g., include_usage). |
| `seed` | `int?` | `null` | Random seed for reproducible outputs. Provider support varies. |
| `reasoningEffort` | `ReasoningEffort?` | `null` | Reasoning effort level (low, medium, high) for extended-thinking models. |
| `modalities` | `List<Modality>?` | `\[\]` | Output modalities to request from the model. For OpenAI audio models, pass `\["text", "audio"\]`. Vertex AI / Gemini translates these to `generationConfig.responseModalities` (uppercase). |
| `extraBody` | `String?` | `null` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

#### ChatCompletionResponse

Chat completion response from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier for this response. |
| `object` | `String` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `int` | — | Unix timestamp of response creation. |
| `model` | `String` | — | Model used to generate the response. |
| `choices` | `List<Choice>` | `\[\]` | List of completion choices. |
| `usage` | `Usage?` | `null` | Token usage statistics. |
| `systemFingerprint` | `String?` | `null` | Fingerprint of the system configuration (OpenAI-specific). |
| `serviceTier` | `String?` | `null` | Service tier used (OpenAI-specific). |

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
| `finishReason` | `FinishReason?` | `null` | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

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

```dart
ChatCompletionChunk? process(ChatCompletionChunk chunk)
```

**Example:**

```dart
final result = instance.process(ChatCompletionChunk());
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `chunk` | `ChatCompletionChunk` | Yes | The chat completion chunk |

**Returns:** `ChatCompletionChunk?`

**Errors:** Throws `Error`.

---

#### CreateBatchRequest

Request to create a batch job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inputFileId` | `String` | — | ID of the uploaded input file (JSONL format). |
| `endpoint` | `String` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `completionWindow` | `String` | — | Completion window (e.g., `"24h"`). |
| `metadata` | `String?` | `null` | Optional metadata to attach to the batch. |

---

#### CreateFileRequest

Request to upload a file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | `String` | — | Base64-encoded file data. |
| `purpose` | `FilePurpose` | `FilePurpose.Assistants` | Purpose for the file. |
| `filename` | `String?` | `null` | Optional filename to associate with the upload. |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt` | `String` | — | Text description of the image to generate. |
| `model` | `String?` | `null` | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `n` | `int?` | `null` | Number of images to generate. Defaults to 1. |
| `size` | `String?` | `null` | Image size (e.g., `"1024x1024"`, `"1792x1024"`). |
| `quality` | `String?` | `null` | Image quality: `"standard"` or `"hd"`. |
| `style` | `String?` | `null` | Style: `"natural"` or `"vivid"` (DALL-E 3 only). |
| `responseFormat` | `String?` | `null` | Response format: `"url"` or `"b64_json"`. |
| `user` | `String?` | `null` | User identifier for request tracking. |

---

#### CreateResponseRequest

Request to create a structured response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID. |
| `input` | `String` | — | Input data to process (e.g., a document to extract from). |
| `instructions` | `String?` | `null` | Instructions for processing the input. |
| `tools` | `List<ResponseTool>?` | `\[\]` | Available tools the model can use. |
| `temperature` | `double?` | `null` | Sampling temperature in `\[0.0, 2.0\]`. Defaults to 1.0. |
| `maxOutputTokens` | `int?` | `null` | Maximum output tokens. |
| `metadata` | `String?` | `null` | Optional metadata. |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`). |
| `input` | `String` | — | Text to synthesize into speech. |
| `voice` | `String` | — | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `responseFormat` | `String?` | `null` | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`). |
| `speed` | `double?` | `null` | Playback speed in `\[0.25, 4.0\]`. Defaults to 1.0. |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"whisper-1"`). |
| `file` | `String` | — | Base64-encoded audio file data. |
| `language` | `String?` | `null` | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `prompt` | `String?` | `null` | Optional text to guide the model (improves accuracy for domain-specific terms). |
| `responseFormat` | `String?` | `null` | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`). |
| `temperature` | `double?` | `null` | Sampling temperature in `\[0.0, 1.0\]`. Higher increases variability. Defaults to 0. |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Unique name for this provider (e.g., "my-provider"). |
| `baseUrl` | `String` | — | Base URL for the provider's API (e.g., `<https://api.my-provider.com/v1>`). |
| `authHeader` | `AuthHeaderFormat` | — | Authentication header format. |
| `modelPrefixes` | `List<String>` | — | Model name prefixes that route to this provider (e.g., `\["my-"\]`). |

---

#### DecodedDataUrl

Result of decoding a `data:` URL — MIME type and the decoded byte payload.

Named struct (rather than a tuple) so polyglot bindings can extract
`decode_data_url` with a typed return rather than a sanitized scalar.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime` | `String` | — | MIME type extracted from the URL prefix (verbatim, not normalised). |
| `data` | `Uint8List` | — | Decoded base64 payload. |

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

###### fetchBatchForPolling()

**Signature:**

```dart
BatchObject fetchBatchForPolling(String batchId)
```

**Example:**

```dart
final result = await instance.fetchBatchForPolling("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `batchId` | `String` | Yes | The batch id |

**Returns:** `BatchObject`

**Errors:** Throws `Error`.

###### waitForBatch()

Poll a batch until it reaches a terminal status (Completed, Failed, Expired, Cancelled).

Uses exponential backoff with configurable initial interval, maximum interval, and backoff multiplier.
Optionally supports a timeout that aborts polling if exceeded.

**Errors:**

Returns `BatchWaitError.Failed` if the batch reaches a failure terminal status.
Returns `BatchWaitError.Timeout` if the configured timeout is exceeded.
Returns `BatchWaitError.Client` for underlying client errors.

**Signature:**

```dart
BatchObject waitForBatch(String batchId, WaitForBatchConfig config)
```

**Example:**

```dart
final result = await instance.waitForBatch("value", WaitForBatchConfig());
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `batchId` | `String` | Yes | The batch id |
| `config` | `WaitForBatchConfig` | Yes | The configuration options |

**Returns:** `BatchObject`

**Errors:** Throws `BatchWaitError`.

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
| `name` | `String?` | `null` | Optional name for the developer message source. |

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
| `embedding` | `List<double>` | — | The embedding vector. |
| `index` | `int` | — | Index in the batch (corresponds to input order). |

---

#### EmbeddingRequest

Embedding request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"text-embedding-3-small"`). |
| `input` | `EmbeddingInput` | `EmbeddingInput.Single` | Text or texts to embed. |
| `encodingFormat` | `EmbeddingFormat?` | `null` | Output format: float (native) or base64. |
| `dimensions` | `int?` | `null` | Requested embedding dimensions (if supported by the model). |
| `user` | `String?` | `null` | User identifier for request tracking. |

---

#### EmbeddingResponse

Embedding response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `List<EmbeddingObject>` | — | List of embeddings. |
| `model` | `String` | — | Model used to generate embeddings. |
| `usage` | `Usage?` | `/* serde(default) */` | Token usage (input tokens only; embeddings have zero output tokens). |

---

#### FileListQuery

Query parameters for listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `purpose` | `String?` | `null` | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `limit` | `int?` | `null` | Maximum number of results to return. Defaults to 20. |
| `after` | `String?` | `null` | Pagination cursor: return results after this file ID. |

---

#### FileListResponse

Response from listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Object type (always `"list"`). |
| `data` | `List<FileObject>` | `\[\]` | List of file objects. |
| `hasMore` | `bool?` | `null` | Whether more results are available. |

---

#### FileObject

An uploaded file object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique file ID. |
| `object` | `String` | — | Object type (always `"file"`). |
| `bytes` | `int` | — | File size in bytes. |
| `createdAt` | `int` | — | Unix timestamp of file creation. |
| `filename` | `String` | — | Filename. |
| `purpose` | `String` | — | File purpose. |
| `status` | `String?` | `null` | Processing status (e.g., `"uploaded"`, `"processed"`). |

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
| `parameters` | `String?` | `/* serde(default) */` | JSON Schema defining the function's parameters. |
| `strict` | `bool?` | `/* serde(default) */` | If true, enforce strict JSON schema validation for arguments. |

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

```dart
HealthStatus check(String upstream)
```

**Example:**

```dart
final result = await instance.check("value");
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
| `url` | `String?` | `null` | Image URL (if response_format was "url"). |
| `b64Json` | `String?` | `null` | Base64-encoded image data (if response_format was "b64_json"). |
| `revisedPrompt` | `String?` | `null` | The final prompt used to generate the image (DALL-E 3). |

---

#### ImageUrl

An image URL reference with optional detail level for processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | URL of the image (data URI or HTTP/HTTPS URL). |
| `detail` | `ImageDetail?` | `null` | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### ImagesResponse

Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `created` | `int` | — | Unix timestamp of image creation. |
| `data` | `List<Image>` | `\[\]` | List of generated images. |

---

#### IntentPrototype

An intent prototype: `(intent_name, prototype_embedding, target_model_id)`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Human-readable name for the intent (used in logs/metrics). |
| `embedding` | `List<double>` | — | Pre-computed embedding vector for this intent. |
| `model` | `String` | — | Model to route to when this intent is detected. |

---

#### JsonSchemaFormat

JSON Schema specification for constrained output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Name of the schema (must be unique in the request). |
| `description` | `String?` | `null` | Description of what the schema represents. |
| `schema` | `String` | — | JSON Schema object defining the output structure. |
| `strict` | `bool?` | `null` | If true, enforce strict schema validation. |

---

#### ModelObject

A model available from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`). |
| `object` | `String` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `int` | — | Unix timestamp of model creation (or release date). |
| `ownedBy` | `String` | — | Organization or entity that owns the model. |

---

#### ModelsListResponse

Response listing available models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `String` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `List<ModelObject>` | `\[\]` | List of available models. |

---

#### ModerationCategories

Boolean flags for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `bool` | — | Sexual content. |
| `hate` | `bool` | — | Hate speech. |
| `harassment` | `bool` | — | Harassment. |
| `selfHarm` | `bool` | — | Self-harm content. |
| `sexualMinors` | `bool` | — | Sexual content involving minors. |
| `hateThreatening` | `bool` | — | Hate speech that threatens violence. |
| `violenceGraphic` | `bool` | — | Graphic violence. |
| `selfHarmIntent` | `bool` | — | Intent to self-harm. |
| `selfHarmInstructions` | `bool` | — | Instructions for self-harm. |
| `harassmentThreatening` | `bool` | — | Harassment that threatens violence. |
| `violence` | `bool` | — | Non-graphic violence. |

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
| `input` | `ModerationInput` | `ModerationInput.Single` | Text or texts to check. |
| `model` | `String?` | `null` | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

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
| `flagged` | `bool` | — | True if any category was flagged. |
| `categories` | `ModerationCategories` | — | Boolean flags for each moderation category. |
| `categoryScores` | `ModerationCategoryScores` | — | Confidence scores for each category. |

---

#### OcrImage

An image extracted from an OCR page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique image identifier within the document. |
| `imageBase64` | `String?` | `/* serde(default) */` | Base64-encoded image data (if `include_image_base64` was true). |

---

#### OcrPage

A single page of OCR output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Page index (0-based). |
| `markdown` | `String` | — | Extracted page content as Markdown. |
| `images` | `List<OcrImage>?` | `/* serde(default) */` | Embedded images extracted from the page (if `include_image_base64` was true). |
| `dimensions` | `PageDimensions?` | `/* serde(default) */` | Page dimensions in pixels, if available. |

---

#### OcrRequest

An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document` | `OcrDocument` | `OcrDocument.Url` | The document to process (URL or base64). |
| `pages` | `List<int>?` | `\[\]` | Specific pages to process (1-indexed). `null` means all pages. |
| `includeImageBase64` | `bool?` | `null` | Whether to include base64-encoded images of each processed page. |

---

#### OcrResponse

An OCR response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pages` | `List<OcrPage>` | — | Extracted pages in order. |
| `model` | `String` | — | Model/provider used for OCR. |
| `usage` | `Usage?` | `/* serde(default) */` | Token usage, if reported by the provider. |

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
| `cachedTokens` | `int` | — | Cached tokens present in the prompt. Defaults to 0 when absent. |
| `audioTokens` | `int` | — | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### ProviderCapabilities

Static capability flags for a provider.

Each flag indicates whether the provider's models *generally* support that
feature. For providers that aggregate many underlying models (e.g. Bedrock,
OpenRouter, vLLM) the flags reflect the superset of available model
capabilities — a flag being `true` means at least one model supports the
feature, not every model.

All flags default to `false` so that newly added providers are safe.

Access via the crate-level `capabilities` function:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `vision` | `bool` | — | The provider accepts image input in chat messages. |
| `reasoning` | `bool` | — | The provider supports extended-thinking / reasoning tokens. |
| `structuredOutput` | `bool` | — | The provider supports JSON-mode or `response_format` structured output. |
| `functionCalling` | `bool` | — | The provider supports tool / function calling. |
| `audioIn` | `bool` | — | The provider accepts audio as input. |
| `audioOut` | `bool` | — | The provider can generate audio / TTS output. |
| `videoIn` | `bool` | — | The provider accepts video as input. |

---

#### ProviderConfig

Static configuration for a single provider entry in providers.json.

This struct deliberately does not include capability flags or streaming
format, which are accessed via the `capabilities` function.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Provider identifier (matches the entry key in providers.json). |
| `displayName` | `String?` | `null` | Human-readable provider name shown in UIs. |
| `baseUrl` | `String?` | `null` | Base URL used as the default for this provider's HTTP client. |
| `auth` | `AuthConfig?` | `null` | Authentication scheme metadata (auth type + env var holding the key). |
| `endpoints` | `List<String>?` | `null` | Supported endpoint kinds (e.g. `chat`, `embeddings`). |
| `modelPrefixes` | `List<String>?` | `null` | Model-name prefixes claimed by this provider (e.g. `\["gpt-", "o1-"\]`). |
| `paramMappings` | `Map<String, String>?` | `null` | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`).  Applied automatically by `ConfigDrivenProvider.transform_request`. |

---

#### RateLimitConfig

Configuration for per-model rate limits.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rpm` | `int?` | `null` | Maximum requests per window.  `null` means unlimited. |
| `tpm` | `int?` | `null` | Maximum tokens per window.  `null` means unlimited. |
| `window` | `Duration` | `60000ms` | Fixed window duration (defaults to 60 s). |

##### Methods

###### default()

**Signature:**

```dart
static RateLimitConfig default()
```

**Example:**

```dart
final result = RateLimitConfig.default();
```

**Returns:** `RateLimitConfig`

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Model ID (e.g., `"cohere/rerank-english-v3.0"`). |
| `query` | `String` | — | The search query. |
| `documents` | `List<RerankDocument>` | `\[\]` | Documents to rerank. |
| `topN` | `int?` | `null` | Return only the top N results. Optional. |
| `returnDocuments` | `bool?` | `null` | Include the document content in results. Defaults to false. |

---

#### RerankResponse

Response from the rerank endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String?` | `null` | Unique identifier for this rerank request. |
| `results` | `List<RerankResult>` | — | Reranked documents in order of relevance. |
| `meta` | `String?` | `/* serde(default) */` | Optional metadata about the reranking operation. |

---

#### RerankResult

A single reranked document with its relevance score.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Original document index in the input list. |
| `relevanceScore` | `double` | — | Relevance score in `\[0, 1\]`. Higher indicates more relevant. |
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
| `createdAt` | `int` | — | Unix timestamp of response creation. |
| `model` | `String` | — | Model used to generate the response. |
| `status` | `String` | — | Status (e.g., `"succeeded"`, `"failed"`). |
| `output` | `List<ResponseOutputItem>` | `\[\]` | Output items from the response. |
| `usage` | `ResponseUsage?` | `null` | Token usage. |
| `error` | `String?` | `null` | Error details (if status is "failed"). |

---

#### ResponseOutputItem

A single output item from the response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `itemType` | `String` | — | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `content` | `String` | — | Output content (flattened into the object). |

---

#### ResponseTool

A tool available for the response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `toolType` | `String` | — | Tool type (e.g., "extractor", "search"). |
| `config` | `String` | — | Tool configuration (flattened into the object). |

---

#### ResponseUsage

Token usage for a response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inputTokens` | `int` | — | Input tokens used. |
| `outputTokens` | `int` | — | Output tokens used. |
| `totalTokens` | `int` | — | Total tokens used. |

---

#### SearchRequest

A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query` | `String` | — | The search query string. |
| `maxResults` | `int?` | `null` | Maximum number of results to return. |
| `searchDomainFilter` | `List<String>?` | `\[\]` | Domain filter — restrict results to specific domains. |
| `country` | `String?` | `null` | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

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
| `date` | `String?` | `/* serde(default) */` | Publication or last-updated date, if available. |

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
| `choiceType` | `ToolType` | `ToolType.Function` | Tool type (always "function"). |
| `function` | `SpecificFunction` | — | The specific function to invoke. |

---

#### StreamChoice

A streaming choice with incremental delta.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index of this choice in the choices array. |
| `delta` | `StreamDelta` | — | Incremental update to the message (content, tool calls, etc.). |
| `finishReason` | `FinishReason?` | `null` | Why the stream ended (present only in final chunk). |

---

#### StreamDelta

Incremental delta in a stream chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `role` | `String?` | `null` | Role (typically present only in the first chunk). |
| `content` | `String?` | `null` | Partial content chunk (e.g., a few words of the response). |
| `toolCalls` | `List<StreamToolCall>?` | `\[\]` | Partial tool calls being streamed. |
| `functionCall` | `StreamFunctionCall?` | `null` | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal` | `String?` | `null` | Partial refusal message. |

---

#### StreamFunctionCall

Partial function call details in a stream.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String?` | `null` | Function name (typically in the first chunk). |
| `arguments` | `String?` | `null` | Partial JSON arguments chunk. |

---

#### StreamOptions

Options for streaming responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `includeUsage` | `bool?` | `null` | If true, include token usage in the final stream chunk. |

---

#### StreamToolCall

A streaming tool call being built incrementally.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index of this tool call in the tool_calls array. |
| `id` | `String?` | `null` | Tool call ID (typically in the first chunk for this call). |
| `callType` | `ToolType?` | `null` | Tool type (typically "function"). |
| `function` | `StreamFunctionCall?` | `null` | Partial function name and arguments. |

---

#### SystemMessage

System message guiding model behavior for the entire conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `UserContent.Text` | Instructions or context that apply throughout the conversation. Accepts either a plain text string or an array of content parts, mirroring `UserContent` so that `Message.system_with_parts` works. |
| `name` | `String?` | `null` | Optional name for the system message source. |

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
| `name` | `String?` | `null` | Optional tool/function name. |

---

#### TranscriptionResponse

Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The transcribed text. |
| `language` | `String?` | `null` | Detected language (ISO-639-1 code). |
| `duration` | `double?` | `null` | Total audio duration in seconds. |
| `segments` | `List<TranscriptionSegment>?` | `\[\]` | Detailed segment-level transcription (if response_format is "verbose_json"). |

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
| `promptTokens` | `int` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `completionTokens` | `int` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `totalTokens` | `int` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `promptTokensDetails` | `PromptTokensDetails?` | `null` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

User message in the conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `UserContent.Text` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `name` | `String?` | `null` | Optional name for the user. |

---

#### WaitForBatchConfig

Configuration for polling a batch until terminal status.

All time values are in seconds as `f64` so the struct bridges across FFI
boundaries without requiring a `Duration` shim.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `initialIntervalSecs` | `double` | `5` | Initial interval between polls, in seconds. |
| `maxIntervalSecs` | `double` | `60` | Maximum interval between polls (backoff plateau), in seconds. |
| `backoffMultiplier` | `double` | `1.5` | Exponential backoff multiplier (e.g., 1.5 increases delay by 50% each poll). |
| `timeoutSecs` | `double?` | `null` | Optional timeout in seconds — polling fails if this duration is exceeded. |

##### Methods

###### default()

**Signature:**

```dart
static WaitForBatchConfig default()
```

**Example:**

```dart
final result = WaitForBatchConfig.default();
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
| `Parts` | Array of content parts (text, images, documents, audio). — Fields: `0`: `List<ContentPart>` |

---

#### ContentPart

A single content part in a user message — text, image, document, or audio.

| Value | Description |
|-------|-------------|
| `Text` | Plain text. — Fields: `text`: `String` |
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

#### AssistantContent

Content shape for assistant messages.

`#[serde(untagged)]` means providers returning a plain scalar string for the
`content` field still deserialise correctly into `AssistantContent.Text(_)`.
Providers returning an array of typed parts (e.g. after an image-generation
or audio-synthesis request) deserialise into `AssistantContent.Parts(_)`.

| Value | Description |
|-------|-------------|
| `Text` | Plain text response (the common case for text-only models). — Fields: `0`: `String` |
| `Parts` | Structured parts — text, refusals, output images, output audio. — Fields: `0`: `List<AssistantPart>` |

---

#### AssistantPart

One part of a structured assistant response.

`#[serde(tag = "type", rename_all = "snake_case")]` matches OpenAI's
parts-spec discriminator (`"type": "text"`, `"type": "output_image"`, …).

| Value | Description |
|-------|-------------|
| `Text` | A text segment of the response. — Fields: `text`: `String` |
| `Refusal` | A refusal — the model declined to respond. — Fields: `refusal`: `String` |
| `OutputImage` | An image produced by the model (e.g. `gpt-image-1`, Gemini Imagen). — Fields: `imageUrl`: `ImageUrl` |
| `OutputAudio` | Audio produced by the model (e.g. `gpt-4o-audio-preview`). — Fields: `audio`: `AudioContent` |

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

Wire format for the chat completions `response_format` field.

### Provider mapping

- **OpenAI** (and OpenAI-compatible providers): emitted verbatim as
  `{"type": "json_schema", "json_schema": {...}}` per the
  chat-completions spec.

- **Gemini / Vertex AI**: translated to
  `generationConfig.responseMimeType = "application/json"` and
  `generationConfig.responseSchema = <schema>`. The `name`,
  `description`, and `strict` fields are dropped — Gemini's
  structured-output API does not consume them.

- **Anthropic**: no native JSON mode. A system instruction is
  prepended asking the model to respond with valid JSON.
  `strict` is advisory only; callers should still validate the
  returned JSON if the schema is load-bearing.

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
| `Single` | Single stop sequence. — Fields: `0`: `String` |
| `Multiple` | Multiple stop sequences. — Fields: `0`: `List<String>` |

---

#### Modality

Output modality requested from the model.

Passed as `modalities: ["text", "audio"]` (OpenAI) or translated to
`generationConfig.responseModalities` (Gemini / Vertex AI).

| Value | Description |
|-------|-------------|
| `Text` | Text output (the default for all providers). |
| `Audio` | Audio / speech output. |
| `Image` | Image output (Gemini Imagen, gpt-image-1). |

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
| `Other` | Catch-all for unknown finish reasons returned by non-OpenAI providers. Note: this intentionally does **not** carry the original string (e.g. `Other(String)`).  Using `#\[serde(other)\]` requires a unit variant, and switching to `#\[serde(untagged)\]` would change deserialization semantics for all variants.  The original value can be recovered by inspecting the raw JSON if needed. |

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
| `Multiple` | Multiple text strings (batch embedding). — Fields: `0`: `List<String>` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value | Description |
|-------|-------------|
| `Single` | Single text string. — Fields: `0`: `String` |
| `Multiple` | Multiple text strings (batch moderation). — Fields: `0`: `List<String>` |

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
| `Base64` | Inline base64-encoded document data. — Fields: `data`: `String`, `mediaType`: `String` |

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

Most providers use standard Server-Sent Events (SSE). AWS Bedrock uses
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
| `Hard` | Reject requests that would exceed the budget with `LiterLlmError.BudgetExceeded`. |
| `Soft` | Allow requests through but emit a `tracing.warn!` when the budget is exceeded. |

---

#### CacheBackend

Storage backend for the response cache.

| Value | Description |
|-------|-------------|
| `Memory` | In-memory LRU cache (default). No external dependencies. |
| `OpenDal` | OpenDAL-backed storage. Supports 40+ backends (S3, Redis, GCS, local FS, etc.). — Fields: `scheme`: `String`, `config`: `Map<String, String>` |

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
