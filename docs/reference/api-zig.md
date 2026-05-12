---
title: "Zig API Reference"
---

## Zig API Reference <span class="version-badge">v1.4.0-rc.27</span>

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

```zig
// Phase 1: zig backend signature generation
```

**Parameters:**

| Name          | Type            | Required | Description      |
| ------------- | --------------- | -------- | ---------------- |
| `apiKey`      | `[:0]const u8`  | Yes      | The api key      |
| `baseUrl`     | `[:0]const u8?` | No       | The base url     |
| `timeoutSecs` | `u64?`          | No       | The timeout secs |
| `maxRetries`  | `u32?`          | No       | The max retries  |
| `modelHint`   | `[:0]const u8?` | No       | The model hint   |

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

```zig
// Phase 1: zig backend signature generation
```

**Parameters:**

| Name   | Type           | Required | Description |
| ------ | -------------- | -------- | ----------- |
| `json` | `[:0]const u8` | Yes      | The json    |

**Returns:** `DefaultClient`
**Errors:** Throws `Error`.

---

#### registerCustomProvider()

Register a custom provider in the global runtime registry.

The provider will be checked **before** all built-in providers during model
detection. If a provider with the same `name` already exists it is replaced.

**Errors:**

Returns an error if the config is invalid (empty name, empty base_url, or
no model prefixes).

**Signature:**

```zig
// Phase 1: zig backend signature generation
```

**Parameters:**

| Name     | Type                   | Required | Description               |
| -------- | ---------------------- | -------- | ------------------------- |
| `config` | `CustomProviderConfig` | Yes      | The configuration options |

**Returns:** `void`
**Errors:** Throws `Error`.

---

#### unregisterCustomProvider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error only if the internal lock is poisoned.

**Signature:**

```zig
// Phase 1: zig backend signature generation
```

**Parameters:**

| Name   | Type           | Required | Description |
| ------ | -------------- | -------- | ----------- |
| `name` | `[:0]const u8` | Yes      | The name    |

**Returns:** `bool`
**Errors:** Throws `Error`.

---

### Types

#### AssistantMessage

| Field          | Type                | Default | Description                                                            |
| -------------- | ------------------- | ------- | ---------------------------------------------------------------------- |
| `content`      | `[:0]const u8?`     | `null`  | The extracted text content                                             |
| `name`         | `[:0]const u8?`     | `null`  | The name                                                               |
| `toolCalls`    | `[]const ToolCall?` | `[]`    | Tool calls                                                             |
| `refusal`      | `[:0]const u8?`     | `null`  | Refusal                                                                |
| `functionCall` | `FunctionCall?`     | `null`  | Deprecated legacy function_call field; retained for API compatibility. |

---

#### AudioContent

| Field    | Type           | Default | Description                               |
| -------- | -------------- | ------- | ----------------------------------------- |
| `data`   | `[:0]const u8` | —       | Base64-encoded audio data.                |
| `format` | `[:0]const u8` | —       | Audio format (e.g., "wav", "mp3", "ogg"). |

---

#### BatchListQuery

| Field   | Type            | Default | Description |
| ------- | --------------- | ------- | ----------- |
| `limit` | `u32?`          | `null`  | Limit       |
| `after` | `[:0]const u8?` | `null`  | After       |

---

#### BatchListResponse

| Field     | Type                  | Default | Description  |
| --------- | --------------------- | ------- | ------------ |
| `object`  | `[:0]const u8`        | —       | Object       |
| `data`    | `[]const BatchObject` | `[]`    | Data         |
| `hasMore` | `bool?`               | `null`  | Whether more |
| `firstId` | `[:0]const u8?`       | `null`  | First id     |
| `lastId`  | `[:0]const u8?`       | `null`  | Last id      |

---

#### BatchObject

| Field              | Type                  | Default                  | Description                           |
| ------------------ | --------------------- | ------------------------ | ------------------------------------- |
| `id`               | `[:0]const u8`        | —                        | Unique identifier                     |
| `object`           | `[:0]const u8`        | —                        | Object                                |
| `endpoint`         | `[:0]const u8`        | —                        | Endpoint                              |
| `inputFileId`      | `[:0]const u8`        | —                        | Input file id                         |
| `completionWindow` | `[:0]const u8`        | —                        | Completion window                     |
| `status`           | `BatchStatus`         | `BatchStatus.Validating` | Status (batch status)                 |
| `outputFileId`     | `[:0]const u8?`       | `null`                   | Output file id                        |
| `errorFileId`      | `[:0]const u8?`       | `null`                   | Error file id                         |
| `createdAt`        | `u64`                 | —                        | Created at                            |
| `completedAt`      | `u64?`                | `null`                   | Completed at                          |
| `failedAt`         | `u64?`                | `null`                   | Failed at                             |
| `expiredAt`        | `u64?`                | `null`                   | Expired at                            |
| `requestCounts`    | `BatchRequestCounts?` | `null`                   | Request counts (batch request counts) |
| `metadata`         | `[:0]const u8?`       | `null`                   | Document metadata                     |

---

#### BatchRequestCounts

| Field       | Type  | Default | Description |
| ----------- | ----- | ------- | ----------- |
| `total`     | `u64` | —       | Total       |
| `completed` | `u64` | —       | Completed   |
| `failed`    | `u64` | —       | Failed      |

---

#### ChatCompletionChunk

| Field               | Type                   | Default | Description                                                                                                                                   |
| ------------------- | ---------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `id`                | `[:0]const u8`         | —       | Unique identifier                                                                                                                             |
| `object`            | `[:0]const u8`         | —       | Always `"chat.completion.chunk"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created`           | `u64`                  | —       | Created                                                                                                                                       |
| `model`             | `[:0]const u8`         | —       | Model                                                                                                                                         |
| `choices`           | `[]const StreamChoice` | `[]`    | Choices                                                                                                                                       |
| `usage`             | `Usage?`               | `null`  | Usage (usage)                                                                                                                                 |
| `systemFingerprint` | `[:0]const u8?`        | `null`  | System fingerprint                                                                                                                            |
| `serviceTier`       | `[:0]const u8?`        | `null`  | Service tier                                                                                                                                  |

---

#### ChatCompletionRequest

| Field               | Type                          | Default | Description                                                                                                                       |
| ------------------- | ----------------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `model`             | `[:0]const u8`                | —       | Model                                                                                                                             |
| `messages`          | `[]const Message`             | `[]`    | Messages                                                                                                                          |
| `temperature`       | `f64?`                        | `null`  | Temperature                                                                                                                       |
| `topP`              | `f64?`                        | `null`  | Top p                                                                                                                             |
| `n`                 | `u32?`                        | `null`  | N                                                                                                                                 |
| `stream`            | `bool?`                       | `null`  | Whether to stream the response. Managed by the client layer — do not set directly.                                                |
| `stop`              | `StopSequence?`               | `null`  | Stop (stop sequence)                                                                                                              |
| `maxTokens`         | `u64?`                        | `null`  | Maximum tokens                                                                                                                    |
| `presencePenalty`   | `f64?`                        | `null`  | Presence penalty                                                                                                                  |
| `frequencyPenalty`  | `f64?`                        | `null`  | Frequency penalty                                                                                                                 |
| `logitBias`         | `std.StringHashMap(f64)?`     | `{}`    | Token bias map. Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user`              | `[:0]const u8?`               | `null`  | User                                                                                                                              |
| `tools`             | `[]const ChatCompletionTool?` | `[]`    | Tools                                                                                                                             |
| `toolChoice`        | `ToolChoice?`                 | `null`  | Tool choice (tool choice)                                                                                                         |
| `parallelToolCalls` | `bool?`                       | `null`  | Parallel tool calls                                                                                                               |
| `responseFormat`    | `ResponseFormat?`             | `null`  | Response format (response format)                                                                                                 |
| `streamOptions`     | `StreamOptions?`              | `null`  | Stream options (stream options)                                                                                                   |
| `seed`              | `i64?`                        | `null`  | Seed                                                                                                                              |
| `reasoningEffort`   | `ReasoningEffort?`            | `null`  | Reasoning effort (reasoning effort)                                                                                               |
| `extraBody`         | `[:0]const u8?`               | `null`  | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc.      |

---

#### ChatCompletionResponse

| Field               | Type             | Default | Description                                                                                                                                      |
| ------------------- | ---------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `id`                | `[:0]const u8`   | —       | Unique identifier                                                                                                                                |
| `object`            | `[:0]const u8`   | —       | Always `"chat.completion"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created`           | `u64`            | —       | Created                                                                                                                                          |
| `model`             | `[:0]const u8`   | —       | Model                                                                                                                                            |
| `choices`           | `[]const Choice` | `[]`    | Choices                                                                                                                                          |
| `usage`             | `Usage?`         | `null`  | Usage (usage)                                                                                                                                    |
| `systemFingerprint` | `[:0]const u8?`  | `null`  | System fingerprint                                                                                                                               |
| `serviceTier`       | `[:0]const u8?`  | `null`  | Service tier                                                                                                                                     |

---

#### ChatCompletionTool

| Field      | Type                 | Default | Description                    |
| ---------- | -------------------- | ------- | ------------------------------ |
| `toolType` | `ToolType`           | —       | Tool type (tool type)          |
| `function` | `FunctionDefinition` | —       | Function (function definition) |

---

#### Choice

| Field          | Type               | Default | Description                   |
| -------------- | ------------------ | ------- | ----------------------------- |
| `index`        | `u32`              | —       | Index                         |
| `message`      | `AssistantMessage` | —       | Message (assistant message)   |
| `finishReason` | `FinishReason?`    | `null`  | Finish reason (finish reason) |

---

#### CreateBatchRequest

| Field              | Type            | Default | Description       |
| ------------------ | --------------- | ------- | ----------------- |
| `inputFileId`      | `[:0]const u8`  | —       | Input file id     |
| `endpoint`         | `[:0]const u8`  | —       | Endpoint          |
| `completionWindow` | `[:0]const u8`  | —       | Completion window |
| `metadata`         | `[:0]const u8?` | `null`  | Document metadata |

---

#### CreateFileRequest

| Field      | Type            | Default                  | Description               |
| ---------- | --------------- | ------------------------ | ------------------------- |
| `file`     | `[:0]const u8`  | —                        | Base64-encoded file data. |
| `purpose`  | `FilePurpose`   | `FilePurpose.Assistants` | Purpose (file purpose)    |
| `filename` | `[:0]const u8?` | `null`                   | Filename                  |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field            | Type            | Default | Description     |
| ---------------- | --------------- | ------- | --------------- |
| `prompt`         | `[:0]const u8`  | —       | Prompt          |
| `model`          | `[:0]const u8?` | `null`  | Model           |
| `n`              | `u32?`          | `null`  | N               |
| `size`           | `[:0]const u8?` | `null`  | Size in bytes   |
| `quality`        | `[:0]const u8?` | `null`  | Quality         |
| `style`          | `[:0]const u8?` | `null`  | Style           |
| `responseFormat` | `[:0]const u8?` | `null`  | Response format |
| `user`           | `[:0]const u8?` | `null`  | User            |

---

#### CreateResponseRequest

| Field             | Type                    | Default | Description           |
| ----------------- | ----------------------- | ------- | --------------------- |
| `model`           | `[:0]const u8`          | —       | Model                 |
| `input`           | `[:0]const u8`          | —       | Input                 |
| `instructions`    | `[:0]const u8?`         | `null`  | Instructions          |
| `tools`           | `[]const ResponseTool?` | `[]`    | Tools                 |
| `temperature`     | `f64?`                  | `null`  | Temperature           |
| `maxOutputTokens` | `u64?`                  | `null`  | Maximum output tokens |
| `metadata`        | `[:0]const u8?`         | `null`  | Document metadata     |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field            | Type            | Default | Description     |
| ---------------- | --------------- | ------- | --------------- |
| `model`          | `[:0]const u8`  | —       | Model           |
| `input`          | `[:0]const u8`  | —       | Input           |
| `voice`          | `[:0]const u8`  | —       | Voice           |
| `responseFormat` | `[:0]const u8?` | `null`  | Response format |
| `speed`          | `f64?`          | `null`  | Speed           |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field            | Type            | Default | Description                     |
| ---------------- | --------------- | ------- | ------------------------------- |
| `model`          | `[:0]const u8`  | —       | Model                           |
| `file`           | `[:0]const u8`  | —       | Base64-encoded audio file data. |
| `language`       | `[:0]const u8?` | `null`  | Language                        |
| `prompt`         | `[:0]const u8?` | `null`  | Prompt                          |
| `responseFormat` | `[:0]const u8?` | `null`  | Response format                 |
| `temperature`    | `f64?`          | `null`  | Temperature                     |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field           | Type                   | Default | Description                                                                 |
| --------------- | ---------------------- | ------- | --------------------------------------------------------------------------- |
| `name`          | `[:0]const u8`         | —       | Unique name for this provider (e.g., "my-provider").                        |
| `baseUrl`       | `[:0]const u8`         | —       | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `authHeader`    | `AuthHeaderFormat`     | —       | Authentication header format.                                               |
| `modelPrefixes` | `[]const [:0]const u8` | —       | Model name prefixes that route to this provider (e.g., ["my-"]).            |

---

#### DefaultClient

Default client implementation backed by `reqwest`.

The provider is resolved at construction time from `model_hint` (or
defaults to OpenAI). However, individual requests can override the
provider when their model string contains a prefix that clearly
identifies a different provider (e.g. `"anthropic/claude-3"` will
route to Anthropic even if the client was built without a hint).

When the model prefix does not match any known provider, the
construction-time provider is used as the fallback.

The provider is stored behind an `Arc` so it can be shared cheaply into
async closures and streaming tasks that must be `'static`.

##### Methods

###### chat()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### chatStream()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### embed()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### listModels()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### imageGenerate()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### speech()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### transcribe()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### moderate()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### rerank()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### search()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### ocr()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### createFile()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### retrieveFile()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### deleteFile()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### listFiles()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### fileContent()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### createBatch()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### retrieveBatch()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### listBatches()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### cancelBatch()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### createResponse()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### retrieveResponse()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

###### cancelResponse()

**Signature:**

```zig
// Phase 1: zig backend method signature generation
```

---

#### DeleteResponse

| Field     | Type           | Default | Description       |
| --------- | -------------- | ------- | ----------------- |
| `id`      | `[:0]const u8` | —       | Unique identifier |
| `object`  | `[:0]const u8` | —       | Object            |
| `deleted` | `bool`         | —       | Deleted           |

---

#### DeveloperMessage

| Field     | Type            | Default | Description                |
| --------- | --------------- | ------- | -------------------------- |
| `content` | `[:0]const u8`  | —       | The extracted text content |
| `name`    | `[:0]const u8?` | `null`  | The name                   |

---

#### DocumentContent

| Field       | Type           | Default | Description                                      |
| ----------- | -------------- | ------- | ------------------------------------------------ |
| `data`      | `[:0]const u8` | —       | Base64-encoded document data or URL.             |
| `mediaType` | `[:0]const u8` | —       | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### EmbeddingObject

| Field       | Type           | Default | Description                                                                                                                                |
| ----------- | -------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `object`    | `[:0]const u8` | —       | Always `"embedding"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `embedding` | `[]const f64`  | —       | Embedding                                                                                                                                  |
| `index`     | `u32`          | —       | Index                                                                                                                                      |

---

#### EmbeddingRequest

| Field            | Type               | Default                 | Description                        |
| ---------------- | ------------------ | ----------------------- | ---------------------------------- |
| `model`          | `[:0]const u8`     | —                       | Model                              |
| `input`          | `EmbeddingInput`   | `EmbeddingInput.Single` | Input (embedding input)            |
| `encodingFormat` | `EmbeddingFormat?` | `null`                  | Encoding format (embedding format) |
| `dimensions`     | `u32?`             | `null`                  | Dimensions                         |
| `user`           | `[:0]const u8?`    | `null`                  | User                               |

---

#### EmbeddingResponse

| Field    | Type                      | Default | Description                                                                                                                           |
| -------- | ------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `object` | `[:0]const u8`            | —       | Always `"list"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data`   | `[]const EmbeddingObject` | —       | Data                                                                                                                                  |
| `model`  | `[:0]const u8`            | —       | Model                                                                                                                                 |
| `usage`  | `Usage?`                  | `null`  | Usage (usage)                                                                                                                         |

---

#### FileListQuery

| Field     | Type            | Default | Description |
| --------- | --------------- | ------- | ----------- |
| `purpose` | `[:0]const u8?` | `null`  | Purpose     |
| `limit`   | `u32?`          | `null`  | Limit       |
| `after`   | `[:0]const u8?` | `null`  | After       |

---

#### FileListResponse

| Field     | Type                 | Default | Description  |
| --------- | -------------------- | ------- | ------------ |
| `object`  | `[:0]const u8`       | —       | Object       |
| `data`    | `[]const FileObject` | `[]`    | Data         |
| `hasMore` | `bool?`              | `null`  | Whether more |

---

#### FileObject

| Field       | Type            | Default | Description       |
| ----------- | --------------- | ------- | ----------------- |
| `id`        | `[:0]const u8`  | —       | Unique identifier |
| `object`    | `[:0]const u8`  | —       | Object            |
| `bytes`     | `u64`           | —       | Bytes             |
| `createdAt` | `u64`           | —       | Created at        |
| `filename`  | `[:0]const u8`  | —       | Filename          |
| `purpose`   | `[:0]const u8`  | —       | Purpose           |
| `status`    | `[:0]const u8?` | `null`  | Status            |

---

#### FunctionCall

| Field       | Type           | Default | Description |
| ----------- | -------------- | ------- | ----------- |
| `name`      | `[:0]const u8` | —       | The name    |
| `arguments` | `[:0]const u8` | —       | Arguments   |

---

#### FunctionDefinition

| Field         | Type            | Default | Description                |
| ------------- | --------------- | ------- | -------------------------- |
| `name`        | `[:0]const u8`  | —       | The name                   |
| `description` | `[:0]const u8?` | `null`  | Human-readable description |
| `parameters`  | `[:0]const u8?` | `null`  | Parameters                 |
| `strict`      | `bool?`         | `null`  | Strict                     |

---

#### FunctionMessage

Deprecated legacy function-role message body.

| Field     | Type           | Default | Description                |
| --------- | -------------- | ------- | -------------------------- |
| `content` | `[:0]const u8` | —       | The extracted text content |
| `name`    | `[:0]const u8` | —       | The name                   |

---

#### Image

A single generated image, returned as either a URL or base64 data.

| Field           | Type            | Default | Description    |
| --------------- | --------------- | ------- | -------------- |
| `url`           | `[:0]const u8?` | `null`  | Url            |
| `b64Json`       | `[:0]const u8?` | `null`  | B64 json       |
| `revisedPrompt` | `[:0]const u8?` | `null`  | Revised prompt |

---

#### ImageUrl

| Field    | Type           | Default | Description           |
| -------- | -------------- | ------- | --------------------- |
| `url`    | `[:0]const u8` | —       | Url                   |
| `detail` | `ImageDetail?` | `null`  | Detail (image detail) |

---

#### ImagesResponse

Response containing generated images.

| Field     | Type            | Default | Description |
| --------- | --------------- | ------- | ----------- |
| `created` | `u64`           | —       | Created     |
| `data`    | `[]const Image` | `[]`    | Data        |

---

#### JsonSchemaFormat

| Field         | Type            | Default | Description                |
| ------------- | --------------- | ------- | -------------------------- |
| `name`        | `[:0]const u8`  | —       | The name                   |
| `description` | `[:0]const u8?` | `null`  | Human-readable description |
| `schema`      | `[:0]const u8`  | —       | Schema                     |
| `strict`      | `bool?`         | `null`  | Strict                     |

---

#### ModelObject

| Field     | Type           | Default | Description                                                                                                                            |
| --------- | -------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `id`      | `[:0]const u8` | —       | Unique identifier                                                                                                                      |
| `object`  | `[:0]const u8` | —       | Always `"model"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `u64`          | —       | Created                                                                                                                                |
| `ownedBy` | `[:0]const u8` | —       | Owned by                                                                                                                               |

---

#### ModelsListResponse

| Field    | Type                  | Default | Description                                                                                                                           |
| -------- | --------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `object` | `[:0]const u8`        | —       | Always `"list"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data`   | `[]const ModelObject` | `[]`    | Data                                                                                                                                  |

---

#### ModerationCategories

Boolean flags for each moderation category.

| Field                   | Type   | Default | Description            |
| ----------------------- | ------ | ------- | ---------------------- |
| `sexual`                | `bool` | —       | Sexual                 |
| `hate`                  | `bool` | —       | Hate                   |
| `harassment`            | `bool` | —       | Harassment             |
| `selfHarm`              | `bool` | —       | Self harm              |
| `sexualMinors`          | `bool` | —       | Sexual minors          |
| `hateThreatening`       | `bool` | —       | Hate threatening       |
| `violenceGraphic`       | `bool` | —       | Violence graphic       |
| `selfHarmIntent`        | `bool` | —       | Self harm intent       |
| `selfHarmInstructions`  | `bool` | —       | Self harm instructions |
| `harassmentThreatening` | `bool` | —       | Harassment threatening |
| `violence`              | `bool` | —       | Violence               |

---

#### ModerationCategoryScores

Confidence scores for each moderation category.

| Field                   | Type  | Default | Description            |
| ----------------------- | ----- | ------- | ---------------------- |
| `sexual`                | `f64` | —       | Sexual                 |
| `hate`                  | `f64` | —       | Hate                   |
| `harassment`            | `f64` | —       | Harassment             |
| `selfHarm`              | `f64` | —       | Self harm              |
| `sexualMinors`          | `f64` | —       | Sexual minors          |
| `hateThreatening`       | `f64` | —       | Hate threatening       |
| `violenceGraphic`       | `f64` | —       | Violence graphic       |
| `selfHarmIntent`        | `f64` | —       | Self harm intent       |
| `selfHarmInstructions`  | `f64` | —       | Self harm instructions |
| `harassmentThreatening` | `f64` | —       | Harassment threatening |
| `violence`              | `f64` | —       | Violence               |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field   | Type              | Default                  | Description              |
| ------- | ----------------- | ------------------------ | ------------------------ |
| `input` | `ModerationInput` | `ModerationInput.Single` | Input (moderation input) |
| `model` | `[:0]const u8?`   | `null`                   | Model                    |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field     | Type                       | Default | Description       |
| --------- | -------------------------- | ------- | ----------------- |
| `id`      | `[:0]const u8`             | —       | Unique identifier |
| `model`   | `[:0]const u8`             | —       | Model             |
| `results` | `[]const ModerationResult` | —       | Results           |

---

#### ModerationResult

A single moderation classification result.

| Field            | Type                       | Default | Description                                  |
| ---------------- | -------------------------- | ------- | -------------------------------------------- |
| `flagged`        | `bool`                     | —       | Flagged                                      |
| `categories`     | `ModerationCategories`     | —       | Categories (moderation categories)           |
| `categoryScores` | `ModerationCategoryScores` | —       | Category scores (moderation category scores) |

---

#### OcrImage

An image extracted from an OCR page.

| Field         | Type            | Default | Description                |
| ------------- | --------------- | ------- | -------------------------- |
| `id`          | `[:0]const u8`  | —       | Unique image identifier.   |
| `imageBase64` | `[:0]const u8?` | `null`  | Base64-encoded image data. |

---

#### OcrPage

A single page of OCR output.

| Field        | Type                | Default | Description                                          |
| ------------ | ------------------- | ------- | ---------------------------------------------------- |
| `index`      | `u32`               | —       | Page index (0-based).                                |
| `markdown`   | `[:0]const u8`      | —       | Extracted content as Markdown.                       |
| `images`     | `[]const OcrImage?` | `null`  | Extracted images, if `include_image_base64` was set. |
| `dimensions` | `PageDimensions?`   | `null`  | Page dimensions in pixels, if available.             |

---

#### OcrRequest

An OCR request.

| Field                | Type           | Default           | Description                                                      |
| -------------------- | -------------- | ----------------- | ---------------------------------------------------------------- |
| `model`              | `[:0]const u8` | —                 | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document`           | `OcrDocument`  | `OcrDocument.Url` | The document to process.                                         |
| `pages`              | `[]const u32?` | `[]`              | Specific pages to process (1-indexed). `null` means all pages.   |
| `includeImageBase64` | `bool?`        | `null`            | Whether to include base64-encoded images of each page.           |

---

#### OcrResponse

An OCR response.

| Field   | Type              | Default | Description                               |
| ------- | ----------------- | ------- | ----------------------------------------- |
| `pages` | `[]const OcrPage` | —       | Extracted pages.                          |
| `model` | `[:0]const u8`    | —       | The model used.                           |
| `usage` | `Usage?`          | `null`  | Token usage, if reported by the provider. |

---

#### PageDimensions

Page dimensions in pixels.

| Field    | Type  | Default | Description       |
| -------- | ----- | ------- | ----------------- |
| `width`  | `u32` | —       | Width in pixels.  |
| `height` | `u32` | —       | Height in pixels. |

---

#### PromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage.prompt_tokens` — it is _not_ an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field          | Type  | Default | Description                                                          |
| -------------- | ----- | ------- | -------------------------------------------------------------------- |
| `cachedTokens` | `u64` | —       | Cached tokens present in the prompt. Defaults to 0 when absent.      |
| `audioTokens`  | `u64` | —       | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field             | Type                     | Default | Description      |
| ----------------- | ------------------------ | ------- | ---------------- |
| `model`           | `[:0]const u8`           | —       | Model            |
| `query`           | `[:0]const u8`           | —       | Query            |
| `documents`       | `[]const RerankDocument` | `[]`    | Documents        |
| `topN`            | `u32?`                   | `null`  | Top n            |
| `returnDocuments` | `bool?`                  | `null`  | Return documents |

---

#### RerankResponse

Response from the rerank endpoint.

| Field     | Type                   | Default | Description       |
| --------- | ---------------------- | ------- | ----------------- |
| `id`      | `[:0]const u8?`        | `null`  | Unique identifier |
| `results` | `[]const RerankResult` | —       | Results           |
| `meta`    | `[:0]const u8?`        | `null`  | Meta              |

---

#### RerankResult

A single reranked document with its relevance score.

| Field            | Type                    | Default | Description                       |
| ---------------- | ----------------------- | ------- | --------------------------------- |
| `index`          | `u32`                   | —       | Index                             |
| `relevanceScore` | `f64`                   | —       | Relevance score                   |
| `document`       | `RerankResultDocument?` | `null`  | Document (rerank result document) |

---

#### RerankResultDocument

The text content of a reranked document, returned when `return_documents` is true.

| Field  | Type           | Default | Description |
| ------ | -------------- | ------- | ----------- |
| `text` | `[:0]const u8` | —       | Text        |

---

#### ResponseObject

| Field       | Type                         | Default | Description            |
| ----------- | ---------------------------- | ------- | ---------------------- |
| `id`        | `[:0]const u8`               | —       | Unique identifier      |
| `object`    | `[:0]const u8`               | —       | Object                 |
| `createdAt` | `u64`                        | —       | Created at             |
| `model`     | `[:0]const u8`               | —       | Model                  |
| `status`    | `[:0]const u8`               | —       | Status                 |
| `output`    | `[]const ResponseOutputItem` | `[]`    | Output                 |
| `usage`     | `ResponseUsage?`             | `null`  | Usage (response usage) |
| `error`     | `[:0]const u8?`              | `null`  | Error                  |

---

#### ResponseOutputItem

| Field      | Type           | Default | Description                |
| ---------- | -------------- | ------- | -------------------------- |
| `itemType` | `[:0]const u8` | —       | Item type                  |
| `content`  | `[:0]const u8` | —       | The extracted text content |

---

#### ResponseTool

| Field      | Type           | Default | Description |
| ---------- | -------------- | ------- | ----------- |
| `toolType` | `[:0]const u8` | —       | Tool type   |
| `config`   | `[:0]const u8` | —       | Config      |

---

#### ResponseUsage

| Field          | Type  | Default | Description   |
| -------------- | ----- | ------- | ------------- |
| `inputTokens`  | `u64` | —       | Input tokens  |
| `outputTokens` | `u64` | —       | Output tokens |
| `totalTokens`  | `u64` | —       | Total tokens  |

---

#### SearchRequest

A search request.

| Field                | Type                    | Default | Description                                                               |
| -------------------- | ----------------------- | ------- | ------------------------------------------------------------------------- |
| `model`              | `[:0]const u8`          | —       | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query`              | `[:0]const u8`          | —       | The search query.                                                         |
| `maxResults`         | `u32?`                  | `null`  | Maximum number of results to return.                                      |
| `searchDomainFilter` | `[]const [:0]const u8?` | `[]`    | Domain filter — restrict results to specific domains.                     |
| `country`            | `[:0]const u8?`         | `null`  | Country code for localized results (ISO 3166-1 alpha-2).                  |

---

#### SearchResponse

A search response.

| Field     | Type                   | Default | Description         |
| --------- | ---------------------- | ------- | ------------------- |
| `results` | `[]const SearchResult` | —       | The search results. |
| `model`   | `[:0]const u8`         | —       | The model used.     |

---

#### SearchResult

An individual search result.

| Field     | Type            | Default | Description                                     |
| --------- | --------------- | ------- | ----------------------------------------------- |
| `title`   | `[:0]const u8`  | —       | Title of the result.                            |
| `url`     | `[:0]const u8`  | —       | URL of the result.                              |
| `snippet` | `[:0]const u8`  | —       | Text snippet / excerpt.                         |
| `date`    | `[:0]const u8?` | `null`  | Publication or last-updated date, if available. |

---

#### SpecificFunction

| Field  | Type           | Default | Description |
| ------ | -------------- | ------- | ----------- |
| `name` | `[:0]const u8` | —       | The name    |

---

#### SpecificToolChoice

| Field        | Type               | Default             | Description                  |
| ------------ | ------------------ | ------------------- | ---------------------------- |
| `choiceType` | `ToolType`         | `ToolType.Function` | Choice type (tool type)      |
| `function`   | `SpecificFunction` | —                   | Function (specific function) |

---

#### StreamChoice

| Field          | Type            | Default | Description                   |
| -------------- | --------------- | ------- | ----------------------------- |
| `index`        | `u32`           | —       | Index                         |
| `delta`        | `StreamDelta`   | —       | Delta (stream delta)          |
| `finishReason` | `FinishReason?` | `null`  | Finish reason (finish reason) |

---

#### StreamDelta

| Field          | Type                      | Default | Description                                                            |
| -------------- | ------------------------- | ------- | ---------------------------------------------------------------------- |
| `role`         | `[:0]const u8?`           | `null`  | Role                                                                   |
| `content`      | `[:0]const u8?`           | `null`  | The extracted text content                                             |
| `toolCalls`    | `[]const StreamToolCall?` | `[]`    | Tool calls                                                             |
| `functionCall` | `StreamFunctionCall?`     | `null`  | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal`      | `[:0]const u8?`           | `null`  | Refusal                                                                |

---

#### StreamFunctionCall

| Field       | Type            | Default | Description |
| ----------- | --------------- | ------- | ----------- |
| `name`      | `[:0]const u8?` | `null`  | The name    |
| `arguments` | `[:0]const u8?` | `null`  | Arguments   |

---

#### StreamOptions

| Field          | Type    | Default | Description   |
| -------------- | ------- | ------- | ------------- |
| `includeUsage` | `bool?` | `null`  | Include usage |

---

#### StreamToolCall

| Field      | Type                  | Default | Description                     |
| ---------- | --------------------- | ------- | ------------------------------- |
| `index`    | `u32`                 | —       | Index                           |
| `id`       | `[:0]const u8?`       | `null`  | Unique identifier               |
| `callType` | `ToolType?`           | `null`  | Call type (tool type)           |
| `function` | `StreamFunctionCall?` | `null`  | Function (stream function call) |

---

#### SystemMessage

| Field     | Type            | Default | Description                |
| --------- | --------------- | ------- | -------------------------- |
| `content` | `[:0]const u8`  | —       | The extracted text content |
| `name`    | `[:0]const u8?` | `null`  | The name                   |

---

#### ToolCall

| Field      | Type           | Default | Description              |
| ---------- | -------------- | ------- | ------------------------ |
| `id`       | `[:0]const u8` | —       | Unique identifier        |
| `callType` | `ToolType`     | —       | Call type (tool type)    |
| `function` | `FunctionCall` | —       | Function (function call) |

---

#### ToolMessage

| Field        | Type            | Default | Description                |
| ------------ | --------------- | ------- | -------------------------- |
| `content`    | `[:0]const u8`  | —       | The extracted text content |
| `toolCallId` | `[:0]const u8`  | —       | Tool call id               |
| `name`       | `[:0]const u8?` | `null`  | The name                   |

---

#### TranscriptionResponse

Response from a transcription request.

| Field      | Type                            | Default | Description |
| ---------- | ------------------------------- | ------- | ----------- |
| `text`     | `[:0]const u8`                  | —       | Text        |
| `language` | `[:0]const u8?`                 | `null`  | Language    |
| `duration` | `f64?`                          | `null`  | Duration    |
| `segments` | `[]const TranscriptionSegment?` | `[]`    | Segments    |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field   | Type           | Default | Description       |
| ------- | -------------- | ------- | ----------------- |
| `id`    | `u32`          | —       | Unique identifier |
| `start` | `f64`          | —       | Start             |
| `end`   | `f64`          | —       | End               |
| `text`  | `[:0]const u8` | —       | Text              |

---

#### Usage

| Field                 | Type                   | Default | Description                                                                                                                                                                         |
| --------------------- | ---------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `promptTokens`        | `u64`                  | —       | Prompt tokens used. Defaults to 0 when absent (some providers omit this).                                                                                                           |
| `completionTokens`    | `u64`                  | —       | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses).                                                                                                       |
| `totalTokens`         | `u64`                  | —       | Total tokens used. Defaults to 0 when absent (some providers omit this).                                                                                                            |
| `promptTokensDetails` | `PromptTokensDetails?` | `null`  | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

| Field     | Type            | Default            | Description                |
| --------- | --------------- | ------------------ | -------------------------- |
| `content` | `UserContent`   | `UserContent.Text` | The extracted text content |
| `name`    | `[:0]const u8?` | `null`             | The name                   |

---

### Enums

#### Message

A chat message in a conversation.

| Value       | Description                                                                                               |
| ----------- | --------------------------------------------------------------------------------------------------------- |
| `System`    | System — Fields: `0`: `SystemMessage`                                                                     |
| `User`      | User — Fields: `0`: `UserMessage`                                                                         |
| `Assistant` | Assistant — Fields: `0`: `AssistantMessage`                                                               |
| `Tool`      | Tool — Fields: `0`: `ToolMessage`                                                                         |
| `Developer` | Developer — Fields: `0`: `DeveloperMessage`                                                               |
| `Function`  | Deprecated legacy function-role message; retained for API compatibility. — Fields: `0`: `FunctionMessage` |

---

#### UserContent

| Value   | Description                                |
| ------- | ------------------------------------------ |
| `Text`  | Text format — Fields: `0`: `[:0]const u8`  |
| `Parts` | Parts — Fields: `0`: `[]const ContentPart` |

---

#### ContentPart

| Value        | Description                                        |
| ------------ | -------------------------------------------------- |
| `Text`       | Text format — Fields: `text`: `[:0]const u8`       |
| `ImageUrl`   | Image url — Fields: `imageUrl`: `ImageUrl`         |
| `Document`   | Document — Fields: `document`: `DocumentContent`   |
| `InputAudio` | Input audio — Fields: `inputAudio`: `AudioContent` |

---

#### ImageDetail

| Value  | Description |
| ------ | ----------- |
| `Low`  | Low         |
| `High` | High        |
| `Auto` | Auto        |

---

#### ToolType

The type discriminator for tool/tool-call objects.

Per the OpenAI spec this is always `"function"`. Using an enum enforces
that constraint at the type level and rejects any other value on
deserialization.

| Value      | Description |
| ---------- | ----------- |
| `Function` | Function    |

---

#### ToolChoice

| Value      | Description                                  |
| ---------- | -------------------------------------------- |
| `Mode`     | Mode — Fields: `0`: `ToolChoiceMode`         |
| `Specific` | Specific — Fields: `0`: `SpecificToolChoice` |

---

#### ToolChoiceMode

| Value      | Description |
| ---------- | ----------- |
| `Auto`     | Auto        |
| `Required` | Required    |
| `None`     | None        |

---

#### ResponseFormat

| Value        | Description                                            |
| ------------ | ------------------------------------------------------ |
| `Text`       | Text format                                            |
| `JsonObject` | Json object                                            |
| `JsonSchema` | Json schema — Fields: `jsonSchema`: `JsonSchemaFormat` |

---

#### StopSequence

| Value      | Description                                    |
| ---------- | ---------------------------------------------- |
| `Single`   | Single — Fields: `0`: `[:0]const u8`           |
| `Multiple` | Multiple — Fields: `0`: `[]const [:0]const u8` |

---

#### FinishReason

Why a choice stopped generating tokens.

| Value           | Description                                                                                                                                                                                                                                                                                                                                                                              |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Stop`          | Stop                                                                                                                                                                                                                                                                                                                                                                                     |
| `Length`        | Length                                                                                                                                                                                                                                                                                                                                                                                   |
| `ToolCalls`     | Tool calls                                                                                                                                                                                                                                                                                                                                                                               |
| `ContentFilter` | Content filter                                                                                                                                                                                                                                                                                                                                                                           |
| `FunctionCall`  | Deprecated legacy finish reason; retained for API compatibility.                                                                                                                                                                                                                                                                                                                         |
| `Other`         | Catch-all for unknown finish reasons returned by non-OpenAI providers. Note: this intentionally does **not** carry the original string (e.g. `Other(String)`). Using `#[serde(other)]` requires a unit variant, and switching to `#[serde(untagged)]` would change deserialization semantics for all variants. The original value can be recovered by inspecting the raw JSON if needed. |

---

#### ReasoningEffort

Controls how much reasoning effort the model should use.

| Value    | Description |
| -------- | ----------- |
| `Low`    | Low         |
| `Medium` | Medium      |
| `High`   | High        |

---

#### EmbeddingFormat

The format in which the embedding vectors are returned.

| Value    | Description                                         |
| -------- | --------------------------------------------------- |
| `Float`  | 32-bit floating-point numbers (default).            |
| `Base64` | Base64-encoded string representation of the floats. |

---

#### EmbeddingInput

| Value      | Description                                    |
| ---------- | ---------------------------------------------- |
| `Single`   | Single — Fields: `0`: `[:0]const u8`           |
| `Multiple` | Multiple — Fields: `0`: `[]const [:0]const u8` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value      | Description                                    |
| ---------- | ---------------------------------------------- |
| `Single`   | Single — Fields: `0`: `[:0]const u8`           |
| `Multiple` | Multiple — Fields: `0`: `[]const [:0]const u8` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value    | Description                               |
| -------- | ----------------------------------------- |
| `Text`   | Text format — Fields: `0`: `[:0]const u8` |
| `Object` | Object — Fields: `text`: `[:0]const u8`   |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value    | Description                                                                                        |
| -------- | -------------------------------------------------------------------------------------------------- |
| `Url`    | A publicly accessible document URL. — Fields: `url`: `[:0]const u8`                                |
| `Base64` | Inline base64-encoded document data. — Fields: `data`: `[:0]const u8`, `mediaType`: `[:0]const u8` |

---

#### FilePurpose

| Value        | Description |
| ------------ | ----------- |
| `Assistants` | Assistants  |
| `Batch`      | Batch       |
| `FineTune`   | Fine tune   |
| `Vision`     | Vision      |

---

#### BatchStatus

| Value        | Description |
| ------------ | ----------- |
| `Validating` | Validating  |
| `Failed`     | Failed      |
| `InProgress` | In progress |
| `Finalizing` | Finalizing  |
| `Completed`  | Completed   |
| `Expired`    | Expired     |
| `Cancelling` | Cancelling  |
| `Cancelled`  | Cancelled   |

---

#### AuthHeaderFormat

How the API key is sent in the HTTP request.

| Value    | Description                                                           |
| -------- | --------------------------------------------------------------------- |
| `Bearer` | Bearer token: `Authorization: Bearer <key>`                           |
| `ApiKey` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `[:0]const u8` |
| `None`   | No authentication required.                                           |

---

### Errors

#### LiterLlmError

All errors that can occur when using `liter-llm`.

| Variant                 | Description                                                                                                                                                                                                                                                                                                                                                      |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Authentication`        | authentication failed: {message}                                                                                                                                                                                                                                                                                                                                 |
| `RateLimited`           | rate limited: {message}                                                                                                                                                                                                                                                                                                                                          |
| `BadRequest`            | bad request: {message}                                                                                                                                                                                                                                                                                                                                           |
| `ContextWindowExceeded` | context window exceeded: {message}                                                                                                                                                                                                                                                                                                                               |
| `ContentPolicy`         | content policy violation: {message}                                                                                                                                                                                                                                                                                                                              |
| `NotFound`              | not found: {message}                                                                                                                                                                                                                                                                                                                                             |
| `ServerError`           | server error: {message}                                                                                                                                                                                                                                                                                                                                          |
| `ServiceUnavailable`    | service unavailable: {message}                                                                                                                                                                                                                                                                                                                                   |
| `Timeout`               | request timeout                                                                                                                                                                                                                                                                                                                                                  |
| `Streaming`             | A catch-all for errors that occur during streaming response processing. This variant covers multiple sub-conditions including UTF-8 decoding failures, CRC/checksum mismatches (AWS EventStream), JSON parse errors in individual SSE chunks, and buffer overflow conditions. The `message` field contains a human-readable description of the specific failure. |
| `EndpointNotSupported`  | provider {provider} does not support {endpoint}                                                                                                                                                                                                                                                                                                                  |
| `InvalidHeader`         | invalid header {name:?}: {reason}                                                                                                                                                                                                                                                                                                                                |
| `Serialization`         | serialization error: {0}                                                                                                                                                                                                                                                                                                                                         |
| `BudgetExceeded`        | budget exceeded: {message}                                                                                                                                                                                                                                                                                                                                       |
| `HookRejected`          | hook rejected: {message}                                                                                                                                                                                                                                                                                                                                         |
| `InternalError`         | An internal logic error (e.g. unexpected Tower response variant). This should never surface in normal operation — if it does, it indicates a bug in the library.                                                                                                                                                                                                 |

---
