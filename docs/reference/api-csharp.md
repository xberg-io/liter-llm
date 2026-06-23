---
title: "C# API Reference"
---

## C# API Reference <span class="version-badge">v1.8.1</span>

### Functions

#### CreateClient()

Create a new LLM client with simple scalar configuration.

This is the primary binding entry-point. All parameters except `api_key`
are optional — omitting them uses the same defaults as
`ClientConfigBuilder`.

**Errors:**

Returns `LiterLlmError` if the underlying HTTP client cannot be
constructed, or if the resolved provider configuration is invalid.

**Signature:**

```csharp
public static DefaultClient CreateClient(string apiKey, string? baseUrl = null, ulong? timeoutSecs = null, uint? maxRetries = null, string? modelHint = null)
```

**Example:**

```csharp
var result = CreateClient("value", "value", 42, 42, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ApiKey` | `string` | Yes | The api key |
| `BaseUrl` | `string?` | No | The base url |
| `TimeoutSecs` | `ulong?` | No | The timeout secs |
| `MaxRetries` | `uint?` | No | The max retries |
| `ModelHint` | `string?` | No | The model hint |

**Returns:** `DefaultClient`

**Errors:** Throws `Error`.

---

#### CreateClientFromJson()

Create a new LLM client from a JSON string.

The JSON object accepts the same fields as `liter-llm.toml` (snake_case).

**Errors:**

Returns `LiterLlmError.BadRequest` if `json` is not valid JSON or
contains unknown fields.

**Signature:**

```csharp
public static DefaultClient CreateClientFromJson(string json)
```

**Example:**

```csharp
var result = CreateClientFromJson("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Json` | `string` | Yes | The json |

**Returns:** `DefaultClient`

**Errors:** Throws `Error`.

---

#### EncodeDataUrl()

Encode bytes as a base64 data URL: `data:<mime>;base64,<b64>`.

`mime` defaults to `IMAGE_PNG` when `null`.

**Signature:**

```csharp
public static string EncodeDataUrl(byte[] bytes, string? mime = null)
```

**Example:**

```csharp
var result = EncodeDataUrl(System.Text.Encoding.UTF8.GetBytes("data"), "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Bytes` | `byte\[\]` | Yes | The bytes |
| `Mime` | `string?` | No | The mime |

**Returns:** `string`

---

#### DecodeDataUrl()

Decode a base64 data URL into `DecodedDataUrl`.

Returns `null` for:

- Non-data URLs (strings that do not start with `"data:"`).
- Malformed prefixes (missing `";base64,"` marker).
- Invalid base64 payloads.

The returned MIME string is extracted verbatim from the URL prefix —
it is not validated or normalised.

**Signature:**

```csharp
public static DecodedDataUrl? DecodeDataUrl(string url)
```

**Example:**

```csharp
var result = DecodeDataUrl("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Url` | `string` | Yes | The URL to fetch |

**Returns:** `DecodedDataUrl?`

---

#### RegisterCustomProvider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```csharp
public static void RegisterCustomProvider(CustomProviderConfig config)
```

**Example:**

```csharp
RegisterCustomProvider(new CustomProviderConfig());
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Config` | `CustomProviderConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Throws `Error`.

---

#### UnregisterCustomProvider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error if the custom-provider registry cannot be updated.

**Signature:**

```csharp
public static bool UnregisterCustomProvider(string name)
```

**Example:**

```csharp
var result = UnregisterCustomProvider("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The name |

**Returns:** `bool`

**Errors:** Throws `Error`.

---

#### Capabilities()

Return the capability flags for a named provider.

Performs an O(n) linear scan over the embedded registry (143 entries).
Returns an owned value so bindings can pass capability data without
borrowing registry internals.

For unknown `provider_name` values the function returns an all-`false`
sentinel so callers never need to handle `Option`.

**Signature:**

```csharp
public static ProviderCapabilities Capabilities(string providerName)
```

**Example:**

```csharp
var result = Capabilities("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ProviderName` | `string` | Yes | The provider name |

**Returns:** `ProviderCapabilities`

---

#### AllProviders()

Return all provider configs from the registry.

Useful for tooling, documentation generation, or runtime enumeration.
Returns the public `ProviderConfig` slice (without capability flags).
To query capability flags for a specific provider use `capabilities`.

**Signature:**

```csharp
public static List<ProviderConfig> AllProviders()
```

**Example:**

```csharp
var result = AllProviders();
```

**Returns:** `List<ProviderConfig>`

**Errors:** Throws `Error`.

---

#### ComplexProviderNames()

Return the set of complex provider names.

Complex providers require custom auth/routing logic beyond simple bearer
tokens (e.g. AWS Bedrock SigV4, Vertex AI OAuth2).

The returned reference points into the static registry — no allocation.

**Signature:**

```csharp
public static List<string> ComplexProviderNames()
```

**Example:**

```csharp
var result = ComplexProviderNames();
```

**Returns:** `List<string>`

**Errors:** Throws `Error`.

---

#### CompletionCost()

Calculate the estimated cost of a completion given a model name and token
counts.

Returns `null` if the model is not present in the embedded pricing registry.
Returns `Some(cost_usd)` otherwise, where the value is in US dollars.

When an exact model name match is not found, progressively shorter prefixes
are tried by stripping from the last `-` or `.` separator. For example,
`gpt-4-0613` will match `gpt-4` if no `gpt-4-0613` entry exists.

**Signature:**

```csharp
public static double? CompletionCost(string model, ulong promptTokens, ulong completionTokens)
```

**Example:**

```csharp
var result = CompletionCost("value", 42, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Model` | `string` | Yes | The model |
| `PromptTokens` | `ulong` | Yes | The prompt tokens |
| `CompletionTokens` | `ulong` | Yes | The completion tokens |

**Returns:** `double?`

---

#### CompletionCostWithCache()

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

```csharp
public static double? CompletionCostWithCache(string model, ulong promptTokens, ulong cachedTokens, ulong completionTokens)
```

**Example:**

```csharp
var result = CompletionCostWithCache("value", 42, 42, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Model` | `string` | Yes | The model |
| `PromptTokens` | `ulong` | Yes | The prompt tokens |
| `CachedTokens` | `ulong` | Yes | The cached tokens |
| `CompletionTokens` | `ulong` | Yes | The completion tokens |

**Returns:** `double?`

---

#### Clear()

Remove all guardrails from the global registry.

Primarily useful in tests to reset state between test cases.

**Panics:**

Panics if the global registry lock is poisoned.

**Signature:**

```csharp
public static void Clear()
```

**Example:**

```csharp
Clear();
```

**Returns:** No return value.

---

#### CountTokens()

Count tokens in a text string using the tokenizer for the given model.

The tokenizer is resolved from the model name prefix (e.g. `"gpt-4o"` maps
to the `Xenova/gpt-4o` HuggingFace tokenizer). Tokenizers are cached after
first load.

**Errors:**

Returns `LiterLlmError.BadRequest` if the tokenizer cannot be loaded
(e.g. network failure on first use) or if tokenization itself fails.

**Signature:**

```csharp
public static nuint CountTokens(string model, string text)
```

**Example:**

```csharp
var result = CountTokens("value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Model` | `string` | Yes | The model |
| `Text` | `string` | Yes | The text |

**Returns:** `nuint`

**Errors:** Throws `Error`.

---

#### CountRequestTokens()

Count tokens for a full `ChatCompletionRequest`.

Sums tokens across all message text contents plus a per-message overhead
of ~4 tokens (for role, separators, and formatting metadata). Tool
definitions and multimodal content parts (images, audio, documents) are
not counted — only textual content contributes to the token total.

**Errors:**

Returns `LiterLlmError.BadRequest` if the tokenizer cannot be loaded or
if tokenization fails for any message.

**Signature:**

```csharp
public static nuint CountRequestTokens(string model, ChatCompletionRequest req)
```

**Example:**

```csharp
var result = CountRequestTokens("value", new ChatCompletionRequest());
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Model` | `string` | Yes | The model |
| `Req` | `ChatCompletionRequest` | Yes | The chat completion request |

**Returns:** `nuint`

**Errors:** Throws `Error`.

---

#### CheckBound()

Assert that `current_len + incoming` does not exceed `limit`.

Call this before appending `incoming` bytes to any buffer that must
stay below `limit`. Returns `Err(LiterLlmError.Streaming)` on overflow
and emits a `tracing.warn!` with context.

**Signature:**

```csharp
public static void CheckBound(string context, nuint currentLen, nuint incoming, nuint limit)
```

**Example:**

```csharp
CheckBound("value", 42, 42, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Context` | `string` | Yes | The context |
| `CurrentLen` | `nuint` | Yes | The current len |
| `Incoming` | `nuint` | Yes | The incoming |
| `Limit` | `nuint` | Yes | The limit |

**Returns:** No return value.

**Errors:** Throws `Error`.

---

#### EnsureCryptoProvider()

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

```csharp
public static void EnsureCryptoProvider()
```

**Example:**

```csharp
EnsureCryptoProvider();
```

**Returns:** No return value.

---

#### EnsureCryptoProvider()

No-op on Windows: reqwest uses native-tls (SChannel), so no rustls provider
installation is needed. All callers use the same call site regardless of
platform.

**Signature:**

```csharp
public static void EnsureCryptoProvider()
```

**Example:**

```csharp
EnsureCryptoProvider();
```

**Returns:** No return value.

---

### Types

#### AssistantMessage

Assistant's response to a user message.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `AssistantContent?` | `null` | The assistant's response: plain text, structured parts, or absent. `null` is valid when the model replies with tool calls only. |
| `Name` | `string?` | `null` | Optional name for the assistant. |
| `ToolCalls` | `List<ToolCall>?` | `new List<ToolCall>()` | Tool calls the model wants to execute, if any. |
| `Refusal` | `string?` | `null` | Refusal reason, if the model declined to respond per safety policies. |
| `FunctionCall` | `FunctionCall?` | `null` | Deprecated legacy function_call field; retained for API compatibility. |

##### Methods

###### Text()

Return the assistant's textual response, concatenating all `Text` parts
if the content is structured.

Returns `null` for `Refusal`-only or `OutputImage`-only responses.

**Signature:**

```csharp
public string? Text()
```

**Example:**

```csharp
var result = instance.Text();
```

**Returns:** `string?`

###### RefusalText()

Return the refusal message, if the model declined to respond.

Checks both the top-level `refusal` field and any `Refusal` parts
inside a structured `content`.

**Signature:**

```csharp
public string? RefusalText()
```

**Example:**

```csharp
var result = instance.RefusalText();
```

**Returns:** `string?`

###### OutputImages()

Return all `AssistantPart.OutputImage` parts in the response.

**Signature:**

```csharp
public List<ImageUrl> OutputImages()
```

**Example:**

```csharp
var result = instance.OutputImages();
```

**Returns:** `List<ImageUrl>`

###### OutputAudio()

Return all `AssistantPart.OutputAudio` parts in the response.

**Signature:**

```csharp
public List<AudioContent> OutputAudio()
```

**Example:**

```csharp
var result = instance.OutputAudio();
```

**Returns:** `List<AudioContent>`

---

#### AudioContent

Audio content part for speech-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Data` | `string` | — | Base64-encoded audio data. |
| `Format` | `string` | — | Audio format (e.g., "wav", "mp3", "ogg"). |

---

#### AuthConfig

Auth configuration block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `AuthType` | `AuthType` | — | Auth scheme classification. |
| `EnvVar` | `string?` | `null` | Name of the environment variable that holds the API key (e.g. `"OPENAI_API_KEY"`). Holds the variable name, never the secret value. |

---

#### BatchListQuery

Query parameters for listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Limit` | `uint?` | `null` | Maximum number of results to return. Defaults to 20. |
| `After` | `string?` | `null` | Pagination cursor: return results after this batch ID. |

---

#### BatchListResponse

Response from listing batches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Object` | `string` | — | Object type (always `"list"`). |
| `Data` | `List<BatchObject>` | `new List<BatchObject>()` | List of batch objects. |
| `HasMore` | `bool?` | `null` | Whether more results are available. |
| `FirstId` | `string?` | `null` | First batch ID in the result set (for pagination). |
| `LastId` | `string?` | `null` | Last batch ID in the result set (for pagination). |

---

#### BatchObject

A batch job object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique batch ID. |
| `Object` | `string` | — | Object type (always `"batch"`). |
| `Endpoint` | `string` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `InputFileId` | `string` | — | ID of the input file. |
| `CompletionWindow` | `string` | — | Completion window (e.g., `"24h"`). |
| `Status` | `BatchStatus` | `BatchStatus.Validating` | Current job status. |
| `OutputFileId` | `string?` | `null` | ID of the output file (present when completed). |
| `ErrorFileId` | `string?` | `null` | ID of the error file (present if some requests failed). |
| `CreatedAt` | `ulong` | — | Unix timestamp of batch creation. |
| `CompletedAt` | `ulong?` | `null` | Unix timestamp of completion (if completed). |
| `FailedAt` | `ulong?` | `null` | Unix timestamp of failure (if failed). |
| `ExpiredAt` | `ulong?` | `null` | Unix timestamp of expiration (if expired). |
| `RequestCounts` | `BatchRequestCounts?` | `null` | Request processing counts. |
| `Metadata` | `object?` | `null` | Metadata attached to the batch. |

---

#### BatchRequestCounts

Request processing counts for a batch.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Total` | `ulong` | — | Total requests in the batch. |
| `Completed` | `ulong` | — | Completed requests. |
| `Failed` | `ulong` | — | Failed requests. |

---

#### BudgetConfig

Configuration for budget enforcement.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `GlobalLimit` | `double?` | `null` | Maximum total spend across all models, in USD.  `null` means unlimited. |
| `ModelLimits` | `Dictionary<string, double>` | `new Dictionary<string, double>()` | Per-model spending limits in USD.  Models not listed here are only constrained by `global_limit`. |
| `Enforcement` | `Enforcement` | `Enforcement.Hard` | Whether to reject requests or merely warn when a limit is exceeded. |

##### Methods

###### CreateDefault()

**Signature:**

```csharp
public BudgetConfig CreateDefault()
```

**Example:**

```csharp
var result = BudgetConfig.CreateDefault();
```

**Returns:** `BudgetConfig`

---

#### CacheConfig

Configuration for the response cache.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MaxEntries` | `nuint` | `256` | Maximum number of cached entries. |
| `Ttl` | `TimeSpan` | `300000ms` | Time-to-live for each cached entry. |
| `Backend` | `CacheBackend` | `CacheBackend.Memory` | Storage backend to use. |

##### Methods

###### CreateDefault()

**Signature:**

```csharp
public CacheConfig CreateDefault()
```

**Example:**

```csharp
var result = CacheConfig.CreateDefault();
```

**Returns:** `CacheConfig`

---

#### ChatCompletionChunk

A streamed chunk of a chat completion response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique identifier for this stream. |
| `Object` | `string` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `Created` | `ulong` | — | Unix timestamp of chunk creation. |
| `Model` | `string` | — | Model used to generate the chunk. |
| `Choices` | `List<StreamChoice>` | `new List<StreamChoice>()` | Streaming choices (delta updates). |
| `Usage` | `Usage?` | `null` | Token usage (typically only in the final chunk). |
| `SystemFingerprint` | `string?` | `null` | Fingerprint of the system configuration (OpenAI-specific). |
| `ServiceTier` | `string?` | `null` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionRequest

Chat completion request (compatible with OpenAI and similar APIs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`). |
| `Messages` | `List<Message>` | `new List<Message>()` | Conversation history from oldest to newest. |
| `Temperature` | `double?` | `null` | Sampling temperature in `\[0.0, 2.0\]`. Higher increases randomness. Defaults to 1.0. |
| `TopP` | `double?` | `null` | Nucleus sampling parameter in `\[0.0, 1.0\]`. Lower is more focused. |
| `N` | `uint?` | `null` | Number of chat completions to generate. Defaults to 1. |
| `Stream` | `bool?` | `null` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `Stop` | `StopSequence?` | `null` | Stop sequence(s) that halt token generation. |
| `MaxTokens` | `ulong?` | `null` | Max output tokens. Different from max_completion_tokens in some providers. |
| `PresencePenalty` | `double?` | `null` | Presence penalty in `\[-2.0, 2.0\]`. Positive discourages repeated topics. |
| `FrequencyPenalty` | `double?` | `null` | Frequency penalty in `\[-2.0, 2.0\]`. Positive discourages repeated tokens. |
| `LogitBias` | `Dictionary<string, double>?` | `new Dictionary<string, double>()` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `User` | `string?` | `null` | User identifier for request tracking and abuse detection. |
| `Tools` | `List<ChatCompletionTool>?` | `new List<ChatCompletionTool>()` | Tools the model can invoke. |
| `ToolChoice` | `ToolChoice?` | `null` | Tool usage mode (auto, required, none, or specific tool). |
| `ParallelToolCalls` | `bool?` | `null` | Whether the model can call multiple tools in parallel. Defaults to true. |
| `ResponseFormat` | `ResponseFormat?` | `null` | Output format constraint (text, JSON, JSON schema). |
| `StreamOptions` | `StreamOptions?` | `null` | Streaming options (e.g., include_usage). |
| `Seed` | `long?` | `null` | Random seed for reproducible outputs. Provider support varies. |
| `ReasoningEffort` | `ReasoningEffort?` | `null` | Reasoning effort level (low, medium, high) for extended-thinking models. |
| `Modalities` | `List<Modality>?` | `new List<Modality>()` | Output modalities to request from the model. For OpenAI audio models, pass `\["text", "audio"\]`. Vertex AI / Gemini translates these to `generationConfig.responseModalities` (uppercase). |
| `ExtraBody` | `object?` | `null` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

#### ChatCompletionResponse

Chat completion response from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique identifier for this response. |
| `Object` | `string` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `Created` | `ulong` | — | Unix timestamp of response creation. |
| `Model` | `string` | — | Model used to generate the response. |
| `Choices` | `List<Choice>` | `new List<Choice>()` | List of completion choices. |
| `Usage` | `Usage?` | `null` | Token usage statistics. |
| `SystemFingerprint` | `string?` | `null` | Fingerprint of the system configuration (OpenAI-specific). |
| `ServiceTier` | `string?` | `null` | Service tier used (OpenAI-specific). |

---

#### ChatCompletionTool

A tool the model can invoke (currently, all tools are functions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ToolType` | `ToolType` | — | Tool type (always "function" in OpenAI spec). |
| `Function` | `FunctionDefinition` | — | Function definition with name, description, and JSON schema parameters. |

---

#### Choice

A single completion choice.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Index` | `uint` | — | Index of this choice in the choices array. |
| `Message` | `AssistantMessage` | — | The assistant's message response. |
| `FinishReason` | `FinishReason?` | `null` | Why the model stopped generating (stop, length, tool_calls, content_filter, etc.). |

---

#### ChunkMiddleware

A per-chunk transformation in the `StreamPipeline`.

Each middleware receives a typed chunk and returns `Ok(Some(chunk))`
to pass it through (optionally modified), `Ok(None)` to drop the chunk,
or `Err(e)` to propagate a stream error.

The trait is object-safe so multiple middleware implementations can be
chained inside `StreamPipeline`.

##### Methods

###### Process()

Process a single chunk.

- `Ok(Some(chunk))` — emit (possibly transformed) chunk.
- `Ok(None)` — drop this chunk silently.
- `Err(e)` — propagate as a stream error.

**Signature:**

```csharp
public ChatCompletionChunk? Process(ChatCompletionChunk chunk)
```

**Example:**

```csharp
var result = instance.Process(new ChatCompletionChunk());
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Chunk` | `ChatCompletionChunk` | Yes | The chat completion chunk |

**Returns:** `ChatCompletionChunk?`

**Errors:** Throws `Error`.

---

#### CreateBatchRequest

Request to create a batch job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `InputFileId` | `string` | — | ID of the uploaded input file (JSONL format). |
| `Endpoint` | `string` | — | API endpoint (e.g., `"/v1/chat/completions"`). |
| `CompletionWindow` | `string` | — | Completion window (e.g., `"24h"`). |
| `Metadata` | `object?` | `null` | Optional metadata to attach to the batch. |

---

#### CreateFileRequest

Request to upload a file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `File` | `string` | — | Base64-encoded file data. |
| `Purpose` | `FilePurpose` | `FilePurpose.Assistants` | Purpose for the file. |
| `Filename` | `string?` | `null` | Optional filename to associate with the upload. |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Prompt` | `string` | — | Text description of the image to generate. |
| `Model` | `string?` | `null` | Model ID (e.g., `"dall-e-3"`). Optional; API may use default if unset. |
| `N` | `uint?` | `null` | Number of images to generate. Defaults to 1. |
| `Size` | `string?` | `null` | Image size (e.g., `"1024x1024"`, `"1792x1024"`). |
| `Quality` | `string?` | `null` | Image quality: `"standard"` or `"hd"`. |
| `Style` | `string?` | `null` | Style: `"natural"` or `"vivid"` (DALL-E 3 only). |
| `ResponseFormat` | `string?` | `null` | Response format: `"url"` or `"b64_json"`. |
| `User` | `string?` | `null` | User identifier for request tracking. |

---

#### CreateResponseRequest

Request to create a structured response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | Model ID. |
| `Input` | `object` | — | Input data to process (e.g., a document to extract from). |
| `Instructions` | `string?` | `null` | Instructions for processing the input. |
| `Tools` | `List<ResponseTool>?` | `new List<ResponseTool>()` | Available tools the model can use. |
| `Temperature` | `double?` | `null` | Sampling temperature in `\[0.0, 2.0\]`. Defaults to 1.0. |
| `MaxOutputTokens` | `ulong?` | `null` | Maximum output tokens. |
| `Metadata` | `object?` | `null` | Optional metadata. |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | Model ID (e.g., `"tts-1"`, `"tts-1-hd"`). |
| `Input` | `string` | — | Text to synthesize into speech. |
| `Voice` | `string` | — | Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`). |
| `ResponseFormat` | `string?` | `null` | Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`). |
| `Speed` | `double?` | `null` | Playback speed in `\[0.25, 4.0\]`. Defaults to 1.0. |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | Model ID (e.g., `"whisper-1"`). |
| `File` | `string` | — | Base64-encoded audio file data. |
| `Language` | `string?` | `null` | Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects. |
| `Prompt` | `string?` | `null` | Optional text to guide the model (improves accuracy for domain-specific terms). |
| `ResponseFormat` | `string?` | `null` | Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`). |
| `Temperature` | `double?` | `null` | Sampling temperature in `\[0.0, 1.0\]`. Higher increases variability. Defaults to 0. |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Unique name for this provider (e.g., "my-provider"). |
| `BaseUrl` | `string` | — | Base URL for the provider's API (e.g., `<https://api.my-provider.com/v1>`). |
| `AuthHeader` | `AuthHeaderFormat` | — | Authentication header format. |
| `ModelPrefixes` | `List<string>` | — | Model name prefixes that route to this provider (e.g., `\["my-"\]`). |

---

#### DecodedDataUrl

Result of decoding a `data:` URL — MIME type and the decoded byte payload.

Named struct (rather than a tuple) so polyglot bindings can extract
`decode_data_url` with a typed return rather than a sanitized scalar.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Mime` | `string` | — | MIME type extracted from the URL prefix (verbatim, not normalised). |
| `Data` | `byte\[\]` | — | Decoded base64 payload. |

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

###### FetchBatchForPolling()

**Signature:**

```csharp
public async Task<BatchObject> FetchBatchForPollingAsync(string batchId)
```

**Example:**

```csharp
var result = await instance.FetchBatchForPolling("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `BatchId` | `string` | Yes | The batch id |

**Returns:** `BatchObject`

**Errors:** Throws `Error`.

###### WaitForBatch()

Poll a batch until it reaches a terminal status (Completed, Failed, Expired, Cancelled).

Uses exponential backoff with configurable initial interval, maximum interval, and backoff multiplier.
Optionally supports a timeout that aborts polling if exceeded.

**Errors:**

Returns `BatchWaitError.Failed` if the batch reaches a failure terminal status.
Returns `BatchWaitError.Timeout` if the configured timeout is exceeded.
Returns `BatchWaitError.Client` for underlying client errors.

**Signature:**

```csharp
public async Task<BatchObject> WaitForBatchAsync(string batchId, WaitForBatchConfig config)
```

**Example:**

```csharp
var result = await instance.WaitForBatch("value", new WaitForBatchConfig());
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `BatchId` | `string` | Yes | The batch id |
| `Config` | `WaitForBatchConfig` | Yes | The configuration options |

**Returns:** `BatchObject`

**Errors:** Throws `BatchWaitError`.

---

#### DeleteResponse

Response from a delete operation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | ID of the deleted resource. |
| `Object` | `string` | — | Object type. |
| `Deleted` | `bool` | — | Confirmation that the resource was deleted. |

---

#### DeveloperMessage

Developer message (system-like message for Claude models).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Developer-specific instructions or context. |
| `Name` | `string?` | `null` | Optional name for the developer message source. |

---

#### DocumentContent

PDF/document content part for vision-capable models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Data` | `string` | — | Base64-encoded document data or URL. |
| `MediaType` | `string` | — | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### EmbeddingObject

A single embedding vector.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Object` | `string` | — | Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `Embedding` | `List<double>` | — | The embedding vector. |
| `Index` | `uint` | — | Index in the batch (corresponds to input order). |

---

#### EmbeddingRequest

Embedding request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | Model ID (e.g., `"text-embedding-3-small"`). |
| `Input` | `EmbeddingInput` | `EmbeddingInput.Single` | Text or texts to embed. |
| `EncodingFormat` | `EmbeddingFormat?` | `null` | Output format: float (native) or base64. |
| `Dimensions` | `uint?` | `null` | Requested embedding dimensions (if supported by the model). |
| `User` | `string?` | `null` | User identifier for request tracking. |

---

#### EmbeddingResponse

Embedding response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Object` | `string` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `Data` | `List<EmbeddingObject>` | — | List of embeddings. |
| `Model` | `string` | — | Model used to generate embeddings. |
| `Usage` | `Usage?` | `/* serde(default) */` | Token usage (input tokens only; embeddings have zero output tokens). |

---

#### FileListQuery

Query parameters for listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Purpose` | `string?` | `null` | Filter by file purpose (e.g., `"batch"`, `"fine-tune"`). |
| `Limit` | `uint?` | `null` | Maximum number of results to return. Defaults to 20. |
| `After` | `string?` | `null` | Pagination cursor: return results after this file ID. |

---

#### FileListResponse

Response from listing files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Object` | `string` | — | Object type (always `"list"`). |
| `Data` | `List<FileObject>` | `new List<FileObject>()` | List of file objects. |
| `HasMore` | `bool?` | `null` | Whether more results are available. |

---

#### FileObject

An uploaded file object.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique file ID. |
| `Object` | `string` | — | Object type (always `"file"`). |
| `Bytes` | `ulong` | — | File size in bytes. |
| `CreatedAt` | `ulong` | — | Unix timestamp of file creation. |
| `Filename` | `string` | — | Filename. |
| `Purpose` | `string` | — | File purpose. |
| `Status` | `string?` | `null` | Processing status (e.g., `"uploaded"`, `"processed"`). |

---

#### FunctionCall

Function call details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Function name. |
| `Arguments` | `string` | — | Arguments as a JSON string (parse with serde_json.from_str). |

---

#### FunctionDefinition

Function definition exposed to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Name of the function. Required and must be alphanumeric + underscores. |
| `Description` | `string?` | `/* serde(default) */` | Human-readable description explaining what the function does. |
| `Parameters` | `object?` | `/* serde(default) */` | JSON Schema defining the function's parameters. |
| `Strict` | `bool?` | `/* serde(default) */` | If true, enforce strict JSON schema validation for arguments. |

---

#### FunctionMessage

Deprecated legacy function-role message body.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | The extracted text content |
| `Name` | `string` | — | The name |

---

#### HealthChecker

Abstraction over a health probe strategy.

Implementors issue a lightweight probe against `upstream` (typically a
provider base URL or named identifier) and report `HealthStatus`.

##### Methods

###### Check()

Probe `upstream` and return its current `HealthStatus`.

The parameter is taken by value (`String`) so that implementations can
move it into the returned future without a clone, making the
`'static + Send` bound on the future trivially satisfiable.

**Signature:**

```csharp
public async Task<HealthStatus> CheckAsync(string upstream)
```

**Example:**

```csharp
var result = await instance.Check("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Upstream` | `string` | Yes | The upstream |

**Returns:** `HealthStatus`

---

#### Image

A single generated image, returned as either a URL or base64 data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Url` | `string?` | `null` | Image URL (if response_format was "url"). |
| `B64Json` | `string?` | `null` | Base64-encoded image data (if response_format was "b64_json"). |
| `RevisedPrompt` | `string?` | `null` | The final prompt used to generate the image (DALL-E 3). |

---

#### ImageUrl

An image URL reference with optional detail level for processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Url` | `string` | — | URL of the image (data URI or HTTP/HTTPS URL). |
| `Detail` | `ImageDetail?` | `null` | Detail level: low (512x512), high (2x2 tiles), or auto (model-selected). |

---

#### ImagesResponse

Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Created` | `ulong` | — | Unix timestamp of image creation. |
| `Data` | `List<Image>` | `new List<Image>()` | List of generated images. |

---

#### IntentPrototype

An intent prototype: `(intent_name, prototype_embedding, target_model_id)`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Human-readable name for the intent (used in logs/metrics). |
| `Embedding` | `List<double>` | — | Pre-computed embedding vector for this intent. |
| `Model` | `string` | — | Model to route to when this intent is detected. |

---

#### JsonSchemaFormat

JSON Schema specification for constrained output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Name of the schema (must be unique in the request). |
| `Description` | `string?` | `null` | Description of what the schema represents. |
| `Schema` | `object` | — | JSON Schema object defining the output structure. |
| `Strict` | `bool?` | `null` | If true, enforce strict schema validation. |

---

#### ModelObject

A model available from the API.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Model ID (e.g., `"gpt-4o"`, `"claude-3-5-sonnet"`). |
| `Object` | `string` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `Created` | `ulong` | — | Unix timestamp of model creation (or release date). |
| `OwnedBy` | `string` | — | Organization or entity that owns the model. |

---

#### ModelsListResponse

Response listing available models.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Object` | `string` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `Data` | `List<ModelObject>` | `new List<ModelObject>()` | List of available models. |

---

#### ModerationCategories

Boolean flags for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Sexual` | `bool` | — | Sexual content. |
| `Hate` | `bool` | — | Hate speech. |
| `Harassment` | `bool` | — | Harassment. |
| `SelfHarm` | `bool` | — | Self-harm content. |
| `SexualMinors` | `bool` | — | Sexual content involving minors. |
| `HateThreatening` | `bool` | — | Hate speech that threatens violence. |
| `ViolenceGraphic` | `bool` | — | Graphic violence. |
| `SelfHarmIntent` | `bool` | — | Intent to self-harm. |
| `SelfHarmInstructions` | `bool` | — | Instructions for self-harm. |
| `HarassmentThreatening` | `bool` | — | Harassment that threatens violence. |
| `Violence` | `bool` | — | Non-graphic violence. |

---

#### ModerationCategoryScores

Confidence scores for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Sexual` | `double` | — | Sexual content score. |
| `Hate` | `double` | — | Hate speech score. |
| `Harassment` | `double` | — | Harassment score. |
| `SelfHarm` | `double` | — | Self-harm content score. |
| `SexualMinors` | `double` | — | Sexual content involving minors score. |
| `HateThreatening` | `double` | — | Hate speech that threatens violence score. |
| `ViolenceGraphic` | `double` | — | Graphic violence score. |
| `SelfHarmIntent` | `double` | — | Intent to self-harm score. |
| `SelfHarmInstructions` | `double` | — | Instructions for self-harm score. |
| `HarassmentThreatening` | `double` | — | Harassment that threatens violence score. |
| `Violence` | `double` | — | Non-graphic violence score. |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Input` | `ModerationInput` | `ModerationInput.Single` | Text or texts to check. |
| `Model` | `string?` | `null` | Model ID (e.g., `"text-moderation-latest"`). Optional; API uses default if unset. |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique identifier for this moderation request. |
| `Model` | `string` | — | Model used for classification. |
| `Results` | `List<ModerationResult>` | — | Results for each input string. |

---

#### ModerationResult

A single moderation classification result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Flagged` | `bool` | — | True if any category was flagged. |
| `Categories` | `ModerationCategories` | — | Boolean flags for each moderation category. |
| `CategoryScores` | `ModerationCategoryScores` | — | Confidence scores for each category. |

---

#### OcrImage

An image extracted from an OCR page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique image identifier within the document. |
| `ImageBase64` | `string?` | `/* serde(default) */` | Base64-encoded image data (if `include_image_base64` was true). |

---

#### OcrPage

A single page of OCR output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Index` | `uint` | — | Page index (0-based). |
| `Markdown` | `string` | — | Extracted page content as Markdown. |
| `Images` | `List<OcrImage>?` | `/* serde(default) */` | Embedded images extracted from the page (if `include_image_base64` was true). |
| `Dimensions` | `PageDimensions?` | `/* serde(default) */` | Page dimensions in pixels, if available. |

---

#### OcrRequest

An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `Document` | `OcrDocument` | `OcrDocument.Url` | The document to process (URL or base64). |
| `Pages` | `List<uint>?` | `new List<uint>()` | Specific pages to process (1-indexed). `null` means all pages. |
| `IncludeImageBase64` | `bool?` | `null` | Whether to include base64-encoded images of each processed page. |

---

#### OcrResponse

An OCR response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Pages` | `List<OcrPage>` | — | Extracted pages in order. |
| `Model` | `string` | — | Model/provider used for OCR. |
| `Usage` | `Usage?` | `/* serde(default) */` | Token usage, if reported by the provider. |

---

#### PageDimensions

Page dimensions in pixels.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Width` | `uint` | — | Width in pixels. |
| `Height` | `uint` | — | Height in pixels. |

---

#### PromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage.prompt_tokens` — it is *not* an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `CachedTokens` | `ulong` | — | Cached tokens present in the prompt. Defaults to 0 when absent. |
| `AudioTokens` | `ulong` | — | Audio input tokens present in the prompt. Defaults to 0 when absent. |

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
| `Vision` | `bool` | — | The provider accepts image input in chat messages. |
| `Reasoning` | `bool` | — | The provider supports extended-thinking / reasoning tokens. |
| `StructuredOutput` | `bool` | — | The provider supports JSON-mode or `response_format` structured output. |
| `FunctionCalling` | `bool` | — | The provider supports tool / function calling. |
| `AudioIn` | `bool` | — | The provider accepts audio as input. |
| `AudioOut` | `bool` | — | The provider can generate audio / TTS output. |
| `VideoIn` | `bool` | — | The provider accepts video as input. |

---

#### ProviderConfig

Static configuration for a single provider entry in providers.json.

This struct deliberately does not include capability flags or streaming
format, which are accessed via the `capabilities` function.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Provider identifier (matches the entry key in providers.json). |
| `DisplayName` | `string?` | `null` | Human-readable provider name shown in UIs. |
| `BaseUrl` | `string?` | `null` | Base URL used as the default for this provider's HTTP client. |
| `Auth` | `AuthConfig?` | `null` | Authentication scheme metadata (auth type + env var holding the key). |
| `Endpoints` | `List<string>?` | `null` | Supported endpoint kinds (e.g. `chat`, `embeddings`). |
| `ModelPrefixes` | `List<string>?` | `null` | Model-name prefixes claimed by this provider (e.g. `\["gpt-", "o1-"\]`). |
| `ParamMappings` | `Dictionary<string, string>?` | `null` | Parameter key renaming for this provider. Each entry maps an OpenAI-spec field name (e.g. `"max_completion_tokens"`) to the name this provider expects (e.g. `"max_tokens"`).  Applied automatically by `ConfigDrivenProvider.transform_request`. |

---

#### RateLimitConfig

Configuration for per-model rate limits.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Rpm` | `uint?` | `null` | Maximum requests per window.  `null` means unlimited. |
| `Tpm` | `ulong?` | `null` | Maximum tokens per window.  `null` means unlimited. |
| `Window` | `TimeSpan` | `60000ms` | Fixed window duration (defaults to 60 s). |

##### Methods

###### CreateDefault()

**Signature:**

```csharp
public RateLimitConfig CreateDefault()
```

**Example:**

```csharp
var result = RateLimitConfig.CreateDefault();
```

**Returns:** `RateLimitConfig`

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | Model ID (e.g., `"cohere/rerank-english-v3.0"`). |
| `Query` | `string` | — | The search query. |
| `Documents` | `List<RerankDocument>` | `new List<RerankDocument>()` | Documents to rerank. |
| `TopN` | `uint?` | `null` | Return only the top N results. Optional. |
| `ReturnDocuments` | `bool?` | `null` | Include the document content in results. Defaults to false. |

---

#### RerankResponse

Response from the rerank endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string?` | `null` | Unique identifier for this rerank request. |
| `Results` | `List<RerankResult>` | — | Reranked documents in order of relevance. |
| `Meta` | `object?` | `/* serde(default) */` | Optional metadata about the reranking operation. |

---

#### RerankResult

A single reranked document with its relevance score.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Index` | `uint` | — | Original document index in the input list. |
| `RelevanceScore` | `double` | — | Relevance score in `\[0, 1\]`. Higher indicates more relevant. |
| `Document` | `RerankResultDocument?` | `/* serde(default) */` | Original document content (if `return_documents` was true). |

---

#### RerankResultDocument

The text content of a reranked document, returned when `return_documents` is true.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | Document text. |

---

#### ResponseObject

Response from a structured response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique response ID. |
| `Object` | `string` | — | Object type (e.g., `"response"`). |
| `CreatedAt` | `ulong` | — | Unix timestamp of response creation. |
| `Model` | `string` | — | Model used to generate the response. |
| `Status` | `string` | — | Status (e.g., `"succeeded"`, `"failed"`). |
| `Output` | `List<ResponseOutputItem>` | `new List<ResponseOutputItem>()` | Output items from the response. |
| `Usage` | `ResponseUsage?` | `null` | Token usage. |
| `Error` | `object?` | `null` | Error details (if status is "failed"). |

---

#### ResponseOutputItem

A single output item from the response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ItemType` | `string` | — | Output type (e.g., `"text"`, `"object"`, `"error"`). |
| `Content` | `object` | — | Output content (flattened into the object). |

---

#### ResponseTool

A tool available for the response request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ToolType` | `string` | — | Tool type (e.g., "extractor", "search"). |
| `Config` | `object` | — | Tool configuration (flattened into the object). |

---

#### ResponseUsage

Token usage for a response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `InputTokens` | `ulong` | — | Input tokens used. |
| `OutputTokens` | `ulong` | — | Output tokens used. |
| `TotalTokens` | `ulong` | — | Total tokens used. |

---

#### SearchRequest

A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `Query` | `string` | — | The search query string. |
| `MaxResults` | `uint?` | `null` | Maximum number of results to return. |
| `SearchDomainFilter` | `List<string>?` | `new List<string>()` | Domain filter — restrict results to specific domains. |
| `Country` | `string?` | `null` | Country code for localized results (ISO 3166-1 alpha-2, e.g., `"US"`, `"FR"`). |

---

#### SearchResponse

A search response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Results` | `List<SearchResult>` | — | List of search results. |
| `Model` | `string` | — | Model/provider that performed the search. |

---

#### SearchResult

An individual search result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Title` | `string` | — | Result title. |
| `Url` | `string` | — | Result URL. |
| `Snippet` | `string` | — | Text snippet or excerpt from the page. |
| `Date` | `string?` | `/* serde(default) */` | Publication or last-updated date, if available. |

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
| `Name` | `string` | — | Function name. |

---

#### SpecificToolChoice

Directive to call a specific tool.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ChoiceType` | `ToolType` | `ToolType.Function` | Tool type (always "function"). |
| `Function` | `SpecificFunction` | — | The specific function to invoke. |

---

#### StreamChoice

A streaming choice with incremental delta.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Index` | `uint` | — | Index of this choice in the choices array. |
| `Delta` | `StreamDelta` | — | Incremental update to the message (content, tool calls, etc.). |
| `FinishReason` | `FinishReason?` | `null` | Why the stream ended (present only in final chunk). |

---

#### StreamDelta

Incremental delta in a stream chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Role` | `string?` | `null` | Role (typically present only in the first chunk). |
| `Content` | `string?` | `null` | Partial content chunk (e.g., a few words of the response). |
| `ToolCalls` | `List<StreamToolCall>?` | `new List<StreamToolCall>()` | Partial tool calls being streamed. |
| `FunctionCall` | `StreamFunctionCall?` | `null` | Deprecated legacy function_call delta; retained for API compatibility. |
| `Refusal` | `string?` | `null` | Partial refusal message. |

---

#### StreamFunctionCall

Partial function call details in a stream.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string?` | `null` | Function name (typically in the first chunk). |
| `Arguments` | `string?` | `null` | Partial JSON arguments chunk. |

---

#### StreamOptions

Options for streaming responses.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `IncludeUsage` | `bool?` | `null` | If true, include token usage in the final stream chunk. |

---

#### StreamToolCall

A streaming tool call being built incrementally.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Index` | `uint` | — | Index of this tool call in the tool_calls array. |
| `Id` | `string?` | `null` | Tool call ID (typically in the first chunk for this call). |
| `CallType` | `ToolType?` | `null` | Tool type (typically "function"). |
| `Function` | `StreamFunctionCall?` | `null` | Partial function name and arguments. |

---

#### SystemMessage

System message guiding model behavior for the entire conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `UserContent` | `UserContent.Text` | Instructions or context that apply throughout the conversation. Accepts either a plain text string or an array of content parts, mirroring `UserContent` so that `Message.system_with_parts` works. |
| `Name` | `string?` | `null` | Optional name for the system message source. |

---

#### ToolCall

A tool call the model wants to execute.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique ID for this call, used to reference in tool result messages. |
| `CallType` | `ToolType` | — | Tool type (always "function"). |
| `Function` | `FunctionCall` | — | Function name and arguments. |

---

#### ToolMessage

Tool execution result returned to the model.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Result of the tool execution. |
| `ToolCallId` | `string` | — | ID of the tool call this result responds to. |
| `Name` | `string?` | `null` | Optional tool/function name. |

---

#### TranscriptionResponse

Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | The transcribed text. |
| `Language` | `string?` | `null` | Detected language (ISO-639-1 code). |
| `Duration` | `double?` | `null` | Total audio duration in seconds. |
| `Segments` | `List<TranscriptionSegment>?` | `new List<TranscriptionSegment>()` | Detailed segment-level transcription (if response_format is "verbose_json"). |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `uint` | — | Segment index (0-based). |
| `Start` | `double` | — | Start time in seconds. |
| `End` | `double` | — | End time in seconds. |
| `Text` | `string` | — | Transcribed text for this segment. |

---

#### Usage

Token-usage accounting returned by the provider on each completion / embedding call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PromptTokens` | `ulong` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `CompletionTokens` | `ulong` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `TotalTokens` | `ulong` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `PromptTokensDetails` | `PromptTokensDetails?` | `null` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

User message in the conversation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `UserContent` | `UserContent.Text` | Message content as plain text or array of content parts (text, images, documents, audio). |
| `Name` | `string?` | `null` | Optional name for the user. |

---

#### WaitForBatchConfig

Configuration for polling a batch until terminal status.

All time values are in seconds as `f64` so the struct bridges across FFI
boundaries without requiring a `Duration` shim.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `InitialIntervalSecs` | `double` | `5` | Initial interval between polls, in seconds. |
| `MaxIntervalSecs` | `double` | `60` | Maximum interval between polls (backoff plateau), in seconds. |
| `BackoffMultiplier` | `float` | `1.5` | Exponential backoff multiplier (e.g., 1.5 increases delay by 50% each poll). |
| `TimeoutSecs` | `double?` | `null` | Optional timeout in seconds — polling fails if this duration is exceeded. |

##### Methods

###### CreateDefault()

**Signature:**

```csharp
public WaitForBatchConfig CreateDefault()
```

**Example:**

```csharp
var result = WaitForBatchConfig.CreateDefault();
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
| `Text` | Plain text content. — Fields: `0`: `string` |
| `Parts` | Array of content parts (text, images, documents, audio). — Fields: `0`: `List<ContentPart>` |

---

#### ContentPart

A single content part in a user message — text, image, document, or audio.

| Value | Description |
|-------|-------------|
| `Text` | Plain text. — Fields: `Text`: `string` |
| `ImageUrl` | Image identified by URL (with optional detail level). — Fields: `ImageUrl`: `ImageUrl` |
| `Document` | Document file (PDF, CSV, etc.) as base64 or URL. — Fields: `Document`: `DocumentContent` |
| `InputAudio` | Audio input as base64. — Fields: `InputAudio`: `AudioContent` |

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
| `Text` | Plain text response (the common case for text-only models). — Fields: `0`: `string` |
| `Parts` | Structured parts — text, refusals, output images, output audio. — Fields: `0`: `List<AssistantPart>` |

---

#### AssistantPart

One part of a structured assistant response.

`#[serde(tag = "type", rename_all = "snake_case")]` matches OpenAI's
parts-spec discriminator (`"type": "text"`, `"type": "output_image"`, …).

| Value | Description |
|-------|-------------|
| `Text` | A text segment of the response. — Fields: `Text`: `string` |
| `Refusal` | A refusal — the model declined to respond. — Fields: `Refusal`: `string` |
| `OutputImage` | An image produced by the model (e.g. `gpt-image-1`, Gemini Imagen). — Fields: `ImageUrl`: `ImageUrl` |
| `OutputAudio` | Audio produced by the model (e.g. `gpt-4o-audio-preview`). — Fields: `Audio`: `AudioContent` |

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
| `JsonSchema` | Output must conform to the specified JSON schema. — Fields: `JsonSchema`: `JsonSchemaFormat` |

---

#### StopSequence

Stop sequence(s) that cause the model to stop generating.

| Value | Description |
|-------|-------------|
| `Single` | Single stop sequence. — Fields: `0`: `string` |
| `Multiple` | Multiple stop sequences. — Fields: `0`: `List<string>` |

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
| `Single` | Single text string. — Fields: `0`: `string` |
| `Multiple` | Multiple text strings (batch embedding). — Fields: `0`: `List<string>` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value | Description |
|-------|-------------|
| `Single` | Single text string. — Fields: `0`: `string` |
| `Multiple` | Multiple text strings (batch moderation). — Fields: `0`: `List<string>` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value | Description |
|-------|-------------|
| `Text` | Plain text document content. — Fields: `0`: `string` |
| `Object` | Document with explicit text field (may include metadata). — Fields: `Text`: `string` |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value | Description |
|-------|-------------|
| `Url` | A publicly accessible document URL. — Fields: `Url`: `string` |
| `Base64` | Inline base64-encoded document data. — Fields: `Data`: `string`, `MediaType`: `string` |

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
| `OpenDal` | OpenDAL-backed storage. Supports 40+ backends (S3, Redis, GCS, local FS, etc.). — Fields: `Scheme`: `string`, `Config`: `Dictionary<string, string>` |

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
