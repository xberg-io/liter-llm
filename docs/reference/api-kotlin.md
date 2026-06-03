---
title: "Kotlin API Reference"
---

## Kotlin API Reference <span class="version-badge">v1.4.0-rc.53</span>

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name          | Type      | Required | Description      |
| ------------- | --------- | -------- | ---------------- |
| `apiKey`      | `String`  | Yes      | The api key      |
| `baseUrl`     | `String?` | No       | The base url     |
| `timeoutSecs` | `Long?`   | No       | The timeout secs |
| `maxRetries`  | `Int?`    | No       | The max retries  |
| `modelHint`   | `String?` | No       | The model hint   |

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name   | Type     | Required | Description |
| ------ | -------- | -------- | ----------- |
| `json` | `String` | Yes      | The json    |

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name     | Type                   | Required | Description               |
| -------- | ---------------------- | -------- | ------------------------- |
| `config` | `CustomProviderConfig` | Yes      | The configuration options |

**Returns:** `Unit`
**Errors:** Throws `Error`.

---

#### unregisterCustomProvider()

Remove a previously registered custom provider by name.

Returns `true` if a provider with the given name was found and removed,
`false` if no such provider existed.

**Errors:**

Returns an error only if the internal lock is poisoned.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name   | Type     | Required | Description |
| ------ | -------- | -------- | ----------- |
| `name` | `String` | Yes      | The name    |

**Returns:** `Boolean`
**Errors:** Throws `Error`.

---

### Types

#### AssistantMessage

| Field          | Type              | Default | Description                                                            |
| -------------- | ----------------- | ------- | ---------------------------------------------------------------------- |
| `content`      | `String?`         | `null`  | The extracted text content                                             |
| `name`         | `String?`         | `null`  | The name                                                               |
| `toolCalls`    | `List<ToolCall>?` | `[]`    | Tool calls                                                             |
| `refusal`      | `String?`         | `null`  | Refusal                                                                |
| `functionCall` | `FunctionCall?`   | `null`  | Deprecated legacy function_call field; retained for API compatibility. |

---

#### AudioContent

| Field    | Type     | Default | Description                               |
| -------- | -------- | ------- | ----------------------------------------- |
| `data`   | `String` | —       | Base64-encoded audio data.                |
| `format` | `String` | —       | Audio format (e.g., "wav", "mp3", "ogg"). |

---

#### BatchListQuery

| Field   | Type      | Default | Description |
| ------- | --------- | ------- | ----------- |
| `limit` | `Int?`    | `null`  | Limit       |
| `after` | `String?` | `null`  | After       |

---

#### BatchListResponse

| Field     | Type                | Default | Description  |
| --------- | ------------------- | ------- | ------------ |
| `object`  | `String`            | —       | Object       |
| `data`    | `List<BatchObject>` | `[]`    | Data         |
| `hasMore` | `Boolean?`          | `null`  | Whether more |
| `firstId` | `String?`           | `null`  | First id     |
| `lastId`  | `String?`           | `null`  | Last id      |

---

#### BatchObject

| Field              | Type                  | Default                  | Description                           |
| ------------------ | --------------------- | ------------------------ | ------------------------------------- |
| `id`               | `String`              | —                        | Unique identifier                     |
| `object`           | `String`              | —                        | Object                                |
| `endpoint`         | `String`              | —                        | Endpoint                              |
| `inputFileId`      | `String`              | —                        | Input file id                         |
| `completionWindow` | `String`              | —                        | Completion window                     |
| `status`           | `BatchStatus`         | `BatchStatus.Validating` | Status (batch status)                 |
| `outputFileId`     | `String?`             | `null`                   | Output file id                        |
| `errorFileId`      | `String?`             | `null`                   | Error file id                         |
| `createdAt`        | `Long`                | —                        | Created at                            |
| `completedAt`      | `Long?`               | `null`                   | Completed at                          |
| `failedAt`         | `Long?`               | `null`                   | Failed at                             |
| `expiredAt`        | `Long?`               | `null`                   | Expired at                            |
| `requestCounts`    | `BatchRequestCounts?` | `null`                   | Request counts (batch request counts) |
| `metadata`         | `Any?`                | `null`                   | Document metadata                     |

---

#### BatchRequestCounts

| Field       | Type   | Default | Description |
| ----------- | ------ | ------- | ----------- |
| `total`     | `Long` | —       | Total       |
| `completed` | `Long` | —       | Completed   |
| `failed`    | `Long` | —       | Failed      |

---

#### ChatCompletionChunk

| Field               | Type                 | Default | Description                                                                                                                                   |
| ------------------- | -------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `id`                | `String`             | —       | Unique identifier                                                                                                                             |
| `object`            | `String`             | —       | Always `"chat.completion.chunk"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created`           | `Long`               | —       | Created                                                                                                                                       |
| `model`             | `String`             | —       | Model                                                                                                                                         |
| `choices`           | `List<StreamChoice>` | `[]`    | Choices                                                                                                                                       |
| `usage`             | `Usage?`             | `null`  | Usage (usage)                                                                                                                                 |
| `systemFingerprint` | `String?`            | `null`  | System fingerprint                                                                                                                            |
| `serviceTier`       | `String?`            | `null`  | Service tier                                                                                                                                  |

---

#### ChatCompletionRequest

| Field               | Type                        | Default | Description                                                                                                                       |
| ------------------- | --------------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `model`             | `String`                    | —       | Model                                                                                                                             |
| `messages`          | `List<Message>`             | `[]`    | Messages                                                                                                                          |
| `temperature`       | `Double?`                   | `null`  | Temperature                                                                                                                       |
| `topP`              | `Double?`                   | `null`  | Top p                                                                                                                             |
| `n`                 | `Int?`                      | `null`  | N                                                                                                                                 |
| `stream`            | `Boolean?`                  | `null`  | Whether to stream the response. Managed by the client layer — do not set directly.                                                |
| `stop`              | `StopSequence?`             | `null`  | Stop (stop sequence)                                                                                                              |
| `maxTokens`         | `Long?`                     | `null`  | Maximum tokens                                                                                                                    |
| `presencePenalty`   | `Double?`                   | `null`  | Presence penalty                                                                                                                  |
| `frequencyPenalty`  | `Double?`                   | `null`  | Frequency penalty                                                                                                                 |
| `logitBias`         | `Map<String, Double>?`      | `{}`    | Token bias map. Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user`              | `String?`                   | `null`  | User                                                                                                                              |
| `tools`             | `List<ChatCompletionTool>?` | `[]`    | Tools                                                                                                                             |
| `toolChoice`        | `ToolChoice?`               | `null`  | Tool choice (tool choice)                                                                                                         |
| `parallelToolCalls` | `Boolean?`                  | `null`  | Parallel tool calls                                                                                                               |
| `responseFormat`    | `ResponseFormat?`           | `null`  | Response format (response format)                                                                                                 |
| `streamOptions`     | `StreamOptions?`            | `null`  | Stream options (stream options)                                                                                                   |
| `seed`              | `Long?`                     | `null`  | Seed                                                                                                                              |
| `reasoningEffort`   | `ReasoningEffort?`          | `null`  | Reasoning effort (reasoning effort)                                                                                               |
| `extraBody`         | `Any?`                      | `null`  | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc.      |

---

#### ChatCompletionResponse

| Field               | Type           | Default | Description                                                                                                                                      |
| ------------------- | -------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `id`                | `String`       | —       | Unique identifier                                                                                                                                |
| `object`            | `String`       | —       | Always `"chat.completion"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created`           | `Long`         | —       | Created                                                                                                                                          |
| `model`             | `String`       | —       | Model                                                                                                                                            |
| `choices`           | `List<Choice>` | `[]`    | Choices                                                                                                                                          |
| `usage`             | `Usage?`       | `null`  | Usage (usage)                                                                                                                                    |
| `systemFingerprint` | `String?`      | `null`  | System fingerprint                                                                                                                               |
| `serviceTier`       | `String?`      | `null`  | Service tier                                                                                                                                     |

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
| `index`        | `Int`              | —       | Index                         |
| `message`      | `AssistantMessage` | —       | Message (assistant message)   |
| `finishReason` | `FinishReason?`    | `null`  | Finish reason (finish reason) |

---

#### CreateBatchRequest

| Field              | Type     | Default | Description       |
| ------------------ | -------- | ------- | ----------------- |
| `inputFileId`      | `String` | —       | Input file id     |
| `endpoint`         | `String` | —       | Endpoint          |
| `completionWindow` | `String` | —       | Completion window |
| `metadata`         | `Any?`   | `null`  | Document metadata |

---

#### CreateFileRequest

| Field      | Type          | Default                  | Description               |
| ---------- | ------------- | ------------------------ | ------------------------- |
| `file`     | `String`      | —                        | Base64-encoded file data. |
| `purpose`  | `FilePurpose` | `FilePurpose.Assistants` | Purpose (file purpose)    |
| `filename` | `String?`     | `null`                   | Filename                  |

---

#### CreateImageRequest

Request to create images from a text prompt.

| Field            | Type      | Default | Description     |
| ---------------- | --------- | ------- | --------------- |
| `prompt`         | `String`  | —       | Prompt          |
| `model`          | `String?` | `null`  | Model           |
| `n`              | `Int?`    | `null`  | N               |
| `size`           | `String?` | `null`  | Size in bytes   |
| `quality`        | `String?` | `null`  | Quality         |
| `style`          | `String?` | `null`  | Style           |
| `responseFormat` | `String?` | `null`  | Response format |
| `user`           | `String?` | `null`  | User            |

---

#### CreateResponseRequest

| Field             | Type                  | Default | Description           |
| ----------------- | --------------------- | ------- | --------------------- |
| `model`           | `String`              | —       | Model                 |
| `input`           | `Any`                 | —       | Input                 |
| `instructions`    | `String?`             | `null`  | Instructions          |
| `tools`           | `List<ResponseTool>?` | `[]`    | Tools                 |
| `temperature`     | `Double?`             | `null`  | Temperature           |
| `maxOutputTokens` | `Long?`               | `null`  | Maximum output tokens |
| `metadata`        | `Any?`                | `null`  | Document metadata     |

---

#### CreateSpeechRequest

Request to generate speech audio from text.

| Field            | Type      | Default | Description     |
| ---------------- | --------- | ------- | --------------- |
| `model`          | `String`  | —       | Model           |
| `input`          | `String`  | —       | Input           |
| `voice`          | `String`  | —       | Voice           |
| `responseFormat` | `String?` | `null`  | Response format |
| `speed`          | `Double?` | `null`  | Speed           |

---

#### CreateTranscriptionRequest

Request to transcribe audio into text.

| Field            | Type      | Default | Description                     |
| ---------------- | --------- | ------- | ------------------------------- |
| `model`          | `String`  | —       | Model                           |
| `file`           | `String`  | —       | Base64-encoded audio file data. |
| `language`       | `String?` | `null`  | Language                        |
| `prompt`         | `String?` | `null`  | Prompt                          |
| `responseFormat` | `String?` | `null`  | Response format                 |
| `temperature`    | `Double?` | `null`  | Temperature                     |

---

#### CustomProviderConfig

Configuration for registering a custom LLM provider at runtime.

| Field           | Type               | Default | Description                                                                 |
| --------------- | ------------------ | ------- | --------------------------------------------------------------------------- |
| `name`          | `String`           | —       | Unique name for this provider (e.g., "my-provider").                        |
| `baseUrl`       | `String`           | —       | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `authHeader`    | `AuthHeaderFormat` | —       | Authentication header format.                                               |
| `modelPrefixes` | `List<String>`     | —       | Model name prefixes that route to this provider (e.g., `["my-"]`).          |

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

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### chatStream()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### embed()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### listModels()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### imageGenerate()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### speech()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### transcribe()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### moderate()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### rerank()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### search()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### ocr()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### createFile()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### retrieveFile()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### deleteFile()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### listFiles()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### fileContent()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### createBatch()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### retrieveBatch()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### listBatches()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### cancelBatch()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### createResponse()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### retrieveResponse()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### cancelResponse()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

---

#### DeleteResponse

| Field     | Type      | Default | Description       |
| --------- | --------- | ------- | ----------------- |
| `id`      | `String`  | —       | Unique identifier |
| `object`  | `String`  | —       | Object            |
| `deleted` | `Boolean` | —       | Deleted           |

---

#### DeveloperMessage

| Field     | Type      | Default | Description                |
| --------- | --------- | ------- | -------------------------- |
| `content` | `String`  | —       | The extracted text content |
| `name`    | `String?` | `null`  | The name                   |

---

#### DocumentContent

| Field       | Type     | Default | Description                                      |
| ----------- | -------- | ------- | ------------------------------------------------ |
| `data`      | `String` | —       | Base64-encoded document data or URL.             |
| `mediaType` | `String` | —       | MIME type (e.g., "application/pdf", "text/csv"). |

---

#### EmbeddingObject

| Field       | Type           | Default | Description                                                                                                                                |
| ----------- | -------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `object`    | `String`       | —       | Always `"embedding"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `embedding` | `List<Double>` | —       | Embedding                                                                                                                                  |
| `index`     | `Int`          | —       | Index                                                                                                                                      |

---

#### EmbeddingRequest

| Field            | Type               | Default                 | Description                        |
| ---------------- | ------------------ | ----------------------- | ---------------------------------- |
| `model`          | `String`           | —                       | Model                              |
| `input`          | `EmbeddingInput`   | `EmbeddingInput.Single` | Input (embedding input)            |
| `encodingFormat` | `EmbeddingFormat?` | `null`                  | Encoding format (embedding format) |
| `dimensions`     | `Int?`             | `null`                  | Dimensions                         |
| `user`           | `String?`          | `null`                  | User                               |

---

#### EmbeddingResponse

| Field    | Type                    | Default | Description                                                                                                                           |
| -------- | ----------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `object` | `String`                | —       | Always `"list"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data`   | `List<EmbeddingObject>` | —       | Data                                                                                                                                  |
| `model`  | `String`                | —       | Model                                                                                                                                 |
| `usage`  | `Usage?`                | `null`  | Usage (usage)                                                                                                                         |

---

#### FileListQuery

| Field     | Type      | Default | Description |
| --------- | --------- | ------- | ----------- |
| `purpose` | `String?` | `null`  | Purpose     |
| `limit`   | `Int?`    | `null`  | Limit       |
| `after`   | `String?` | `null`  | After       |

---

#### FileListResponse

| Field     | Type               | Default | Description  |
| --------- | ------------------ | ------- | ------------ |
| `object`  | `String`           | —       | Object       |
| `data`    | `List<FileObject>` | `[]`    | Data         |
| `hasMore` | `Boolean?`         | `null`  | Whether more |

---

#### FileObject

| Field       | Type      | Default | Description       |
| ----------- | --------- | ------- | ----------------- |
| `id`        | `String`  | —       | Unique identifier |
| `object`    | `String`  | —       | Object            |
| `bytes`     | `Long`    | —       | Bytes             |
| `createdAt` | `Long`    | —       | Created at        |
| `filename`  | `String`  | —       | Filename          |
| `purpose`   | `String`  | —       | Purpose           |
| `status`    | `String?` | `null`  | Status            |

---

#### FunctionCall

| Field       | Type     | Default | Description |
| ----------- | -------- | ------- | ----------- |
| `name`      | `String` | —       | The name    |
| `arguments` | `String` | —       | Arguments   |

---

#### FunctionDefinition

| Field         | Type       | Default | Description                |
| ------------- | ---------- | ------- | -------------------------- |
| `name`        | `String`   | —       | The name                   |
| `description` | `String?`  | `null`  | Human-readable description |
| `parameters`  | `Any?`     | `null`  | Parameters                 |
| `strict`      | `Boolean?` | `null`  | Strict                     |

---

#### FunctionMessage

Deprecated legacy function-role message body.

| Field     | Type     | Default | Description                |
| --------- | -------- | ------- | -------------------------- |
| `content` | `String` | —       | The extracted text content |
| `name`    | `String` | —       | The name                   |

---

#### Image

A single generated image, returned as either a URL or base64 data.

| Field           | Type      | Default | Description    |
| --------------- | --------- | ------- | -------------- |
| `url`           | `String?` | `null`  | Url            |
| `b64Json`       | `String?` | `null`  | B64 json       |
| `revisedPrompt` | `String?` | `null`  | Revised prompt |

---

#### ImageUrl

| Field    | Type           | Default | Description           |
| -------- | -------------- | ------- | --------------------- |
| `url`    | `String`       | —       | Url                   |
| `detail` | `ImageDetail?` | `null`  | Detail (image detail) |

---

#### ImagesResponse

Response containing generated images.

| Field     | Type          | Default | Description |
| --------- | ------------- | ------- | ----------- |
| `created` | `Long`        | —       | Created     |
| `data`    | `List<Image>` | `[]`    | Data        |

---

#### JsonSchemaFormat

| Field         | Type       | Default | Description                |
| ------------- | ---------- | ------- | -------------------------- |
| `name`        | `String`   | —       | The name                   |
| `description` | `String?`  | `null`  | Human-readable description |
| `schema`      | `Any`      | —       | Schema                     |
| `strict`      | `Boolean?` | `null`  | Strict                     |

---

#### ModelObject

| Field     | Type     | Default | Description                                                                                                                            |
| --------- | -------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `id`      | `String` | —       | Unique identifier                                                                                                                      |
| `object`  | `String` | —       | Always `"model"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `Long`   | —       | Created                                                                                                                                |
| `ownedBy` | `String` | —       | Owned by                                                                                                                               |

---

#### ModelsListResponse

| Field    | Type                | Default | Description                                                                                                                           |
| -------- | ------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `object` | `String`            | —       | Always `"list"` from OpenAI-compatible APIs. Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data`   | `List<ModelObject>` | `[]`    | Data                                                                                                                                  |

---

#### ModerationCategories

Boolean flags for each moderation category.

| Field                   | Type      | Default | Description            |
| ----------------------- | --------- | ------- | ---------------------- |
| `sexual`                | `Boolean` | —       | Sexual                 |
| `hate`                  | `Boolean` | —       | Hate                   |
| `harassment`            | `Boolean` | —       | Harassment             |
| `selfHarm`              | `Boolean` | —       | Self harm              |
| `sexualMinors`          | `Boolean` | —       | Sexual minors          |
| `hateThreatening`       | `Boolean` | —       | Hate threatening       |
| `violenceGraphic`       | `Boolean` | —       | Violence graphic       |
| `selfHarmIntent`        | `Boolean` | —       | Self harm intent       |
| `selfHarmInstructions`  | `Boolean` | —       | Self harm instructions |
| `harassmentThreatening` | `Boolean` | —       | Harassment threatening |
| `violence`              | `Boolean` | —       | Violence               |

---

#### ModerationCategoryScores

Confidence scores for each moderation category.

| Field                   | Type     | Default | Description            |
| ----------------------- | -------- | ------- | ---------------------- |
| `sexual`                | `Double` | —       | Sexual                 |
| `hate`                  | `Double` | —       | Hate                   |
| `harassment`            | `Double` | —       | Harassment             |
| `selfHarm`              | `Double` | —       | Self harm              |
| `sexualMinors`          | `Double` | —       | Sexual minors          |
| `hateThreatening`       | `Double` | —       | Hate threatening       |
| `violenceGraphic`       | `Double` | —       | Violence graphic       |
| `selfHarmIntent`        | `Double` | —       | Self harm intent       |
| `selfHarmInstructions`  | `Double` | —       | Self harm instructions |
| `harassmentThreatening` | `Double` | —       | Harassment threatening |
| `violence`              | `Double` | —       | Violence               |

---

#### ModerationRequest

Request to classify content for policy violations.

| Field   | Type              | Default                  | Description              |
| ------- | ----------------- | ------------------------ | ------------------------ |
| `input` | `ModerationInput` | `ModerationInput.Single` | Input (moderation input) |
| `model` | `String?`         | `null`                   | Model                    |

---

#### ModerationResponse

Response from the moderation endpoint.

| Field     | Type                     | Default | Description       |
| --------- | ------------------------ | ------- | ----------------- |
| `id`      | `String`                 | —       | Unique identifier |
| `model`   | `String`                 | —       | Model             |
| `results` | `List<ModerationResult>` | —       | Results           |

---

#### ModerationResult

A single moderation classification result.

| Field            | Type                       | Default | Description                                  |
| ---------------- | -------------------------- | ------- | -------------------------------------------- |
| `flagged`        | `Boolean`                  | —       | Flagged                                      |
| `categories`     | `ModerationCategories`     | —       | Categories (moderation categories)           |
| `categoryScores` | `ModerationCategoryScores` | —       | Category scores (moderation category scores) |

---

#### OcrImage

An image extracted from an OCR page.

| Field         | Type      | Default | Description                |
| ------------- | --------- | ------- | -------------------------- |
| `id`          | `String`  | —       | Unique image identifier.   |
| `imageBase64` | `String?` | `null`  | Base64-encoded image data. |

---

#### OcrPage

A single page of OCR output.

| Field        | Type              | Default | Description                                          |
| ------------ | ----------------- | ------- | ---------------------------------------------------- |
| `index`      | `Int`             | —       | Page index (0-based).                                |
| `markdown`   | `String`          | —       | Extracted content as Markdown.                       |
| `images`     | `List<OcrImage>?` | `null`  | Extracted images, if `include_image_base64` was set. |
| `dimensions` | `PageDimensions?` | `null`  | Page dimensions in pixels, if available.             |

---

#### OcrRequest

An OCR request.

| Field                | Type          | Default           | Description                                                      |
| -------------------- | ------------- | ----------------- | ---------------------------------------------------------------- |
| `model`              | `String`      | —                 | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document`           | `OcrDocument` | `OcrDocument.Url` | The document to process.                                         |
| `pages`              | `List<Int>?`  | `[]`              | Specific pages to process (1-indexed). `null` means all pages.   |
| `includeImageBase64` | `Boolean?`    | `null`            | Whether to include base64-encoded images of each page.           |

---

#### OcrResponse

An OCR response.

| Field   | Type            | Default | Description                               |
| ------- | --------------- | ------- | ----------------------------------------- |
| `pages` | `List<OcrPage>` | —       | Extracted pages.                          |
| `model` | `String`        | —       | The model used.                           |
| `usage` | `Usage?`        | `null`  | Token usage, if reported by the provider. |

---

#### PageDimensions

Page dimensions in pixels.

| Field    | Type  | Default | Description       |
| -------- | ----- | ------- | ----------------- |
| `width`  | `Int` | —       | Width in pixels.  |
| `height` | `Int` | —       | Height in pixels. |

---

#### PromptTokensDetails

Breakdown of tokens used in the prompt portion of a request.

`cached_tokens` is included in `Usage.prompt_tokens` — it is _not_ an
additional charge on top of the prompt token count. When pricing supports
a `cache_read_input_token_cost`, the cached portion is billed at the
discounted rate and the remainder at the regular input rate.

| Field          | Type   | Default | Description                                                          |
| -------------- | ------ | ------- | -------------------------------------------------------------------- |
| `cachedTokens` | `Long` | —       | Cached tokens present in the prompt. Defaults to 0 when absent.      |
| `audioTokens`  | `Long` | —       | Audio input tokens present in the prompt. Defaults to 0 when absent. |

---

#### RerankRequest

Request to rerank documents by relevance to a query.

| Field             | Type                   | Default | Description      |
| ----------------- | ---------------------- | ------- | ---------------- |
| `model`           | `String`               | —       | Model            |
| `query`           | `String`               | —       | Query            |
| `documents`       | `List<RerankDocument>` | `[]`    | Documents        |
| `topN`            | `Int?`                 | `null`  | Top n            |
| `returnDocuments` | `Boolean?`             | `null`  | Return documents |

---

#### RerankResponse

Response from the rerank endpoint.

| Field     | Type                 | Default | Description       |
| --------- | -------------------- | ------- | ----------------- |
| `id`      | `String?`            | `null`  | Unique identifier |
| `results` | `List<RerankResult>` | —       | Results           |
| `meta`    | `Any?`               | `null`  | Meta              |

---

#### RerankResult

A single reranked document with its relevance score.

| Field            | Type                    | Default | Description                       |
| ---------------- | ----------------------- | ------- | --------------------------------- |
| `index`          | `Int`                   | —       | Index                             |
| `relevanceScore` | `Double`                | —       | Relevance score                   |
| `document`       | `RerankResultDocument?` | `null`  | Document (rerank result document) |

---

#### RerankResultDocument

The text content of a reranked document, returned when `return_documents` is true.

| Field  | Type     | Default | Description |
| ------ | -------- | ------- | ----------- |
| `text` | `String` | —       | Text        |

---

#### ResponseObject

| Field       | Type                       | Default | Description            |
| ----------- | -------------------------- | ------- | ---------------------- |
| `id`        | `String`                   | —       | Unique identifier      |
| `object`    | `String`                   | —       | Object                 |
| `createdAt` | `Long`                     | —       | Created at             |
| `model`     | `String`                   | —       | Model                  |
| `status`    | `String`                   | —       | Status                 |
| `output`    | `List<ResponseOutputItem>` | `[]`    | Output                 |
| `usage`     | `ResponseUsage?`           | `null`  | Usage (response usage) |
| `error`     | `Any?`                     | `null`  | Error                  |

---

#### ResponseOutputItem

| Field      | Type     | Default | Description                |
| ---------- | -------- | ------- | -------------------------- |
| `itemType` | `String` | —       | Item type                  |
| `content`  | `Any`    | —       | The extracted text content |

---

#### ResponseTool

| Field      | Type     | Default | Description |
| ---------- | -------- | ------- | ----------- |
| `toolType` | `String` | —       | Tool type   |
| `config`   | `Any`    | —       | Config      |

---

#### ResponseUsage

| Field          | Type   | Default | Description   |
| -------------- | ------ | ------- | ------------- |
| `inputTokens`  | `Long` | —       | Input tokens  |
| `outputTokens` | `Long` | —       | Output tokens |
| `totalTokens`  | `Long` | —       | Total tokens  |

---

#### SearchRequest

A search request.

| Field                | Type            | Default | Description                                                               |
| -------------------- | --------------- | ------- | ------------------------------------------------------------------------- |
| `model`              | `String`        | —       | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query`              | `String`        | —       | The search query.                                                         |
| `maxResults`         | `Int?`          | `null`  | Maximum number of results to return.                                      |
| `searchDomainFilter` | `List<String>?` | `[]`    | Domain filter — restrict results to specific domains.                     |
| `country`            | `String?`       | `null`  | Country code for localized results (ISO 3166-1 alpha-2).                  |

---

#### SearchResponse

A search response.

| Field     | Type                 | Default | Description         |
| --------- | -------------------- | ------- | ------------------- |
| `results` | `List<SearchResult>` | —       | The search results. |
| `model`   | `String`             | —       | The model used.     |

---

#### SearchResult

An individual search result.

| Field     | Type      | Default | Description                                     |
| --------- | --------- | ------- | ----------------------------------------------- |
| `title`   | `String`  | —       | Title of the result.                            |
| `url`     | `String`  | —       | URL of the result.                              |
| `snippet` | `String`  | —       | Text snippet / excerpt.                         |
| `date`    | `String?` | `null`  | Publication or last-updated date, if available. |

---

#### SpecificFunction

| Field  | Type     | Default | Description |
| ------ | -------- | ------- | ----------- |
| `name` | `String` | —       | The name    |

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
| `index`        | `Int`           | —       | Index                         |
| `delta`        | `StreamDelta`   | —       | Delta (stream delta)          |
| `finishReason` | `FinishReason?` | `null`  | Finish reason (finish reason) |

---

#### StreamDelta

| Field          | Type                    | Default | Description                                                            |
| -------------- | ----------------------- | ------- | ---------------------------------------------------------------------- |
| `role`         | `String?`               | `null`  | Role                                                                   |
| `content`      | `String?`               | `null`  | The extracted text content                                             |
| `toolCalls`    | `List<StreamToolCall>?` | `[]`    | Tool calls                                                             |
| `functionCall` | `StreamFunctionCall?`   | `null`  | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal`      | `String?`               | `null`  | Refusal                                                                |

---

#### StreamFunctionCall

| Field       | Type      | Default | Description |
| ----------- | --------- | ------- | ----------- |
| `name`      | `String?` | `null`  | The name    |
| `arguments` | `String?` | `null`  | Arguments   |

---

#### StreamOptions

| Field          | Type       | Default | Description   |
| -------------- | ---------- | ------- | ------------- |
| `includeUsage` | `Boolean?` | `null`  | Include usage |

---

#### StreamToolCall

| Field      | Type                  | Default | Description                     |
| ---------- | --------------------- | ------- | ------------------------------- |
| `index`    | `Int`                 | —       | Index                           |
| `id`       | `String?`             | `null`  | Unique identifier               |
| `callType` | `ToolType?`           | `null`  | Call type (tool type)           |
| `function` | `StreamFunctionCall?` | `null`  | Function (stream function call) |

---

#### SystemMessage

| Field     | Type      | Default | Description                |
| --------- | --------- | ------- | -------------------------- |
| `content` | `String`  | —       | The extracted text content |
| `name`    | `String?` | `null`  | The name                   |

---

#### ToolCall

| Field      | Type           | Default | Description              |
| ---------- | -------------- | ------- | ------------------------ |
| `id`       | `String`       | —       | Unique identifier        |
| `callType` | `ToolType`     | —       | Call type (tool type)    |
| `function` | `FunctionCall` | —       | Function (function call) |

---

#### ToolMessage

| Field        | Type      | Default | Description                |
| ------------ | --------- | ------- | -------------------------- |
| `content`    | `String`  | —       | The extracted text content |
| `toolCallId` | `String`  | —       | Tool call id               |
| `name`       | `String?` | `null`  | The name                   |

---

#### TranscriptionResponse

Response from a transcription request.

| Field      | Type                          | Default | Description |
| ---------- | ----------------------------- | ------- | ----------- |
| `text`     | `String`                      | —       | Text        |
| `language` | `String?`                     | `null`  | Language    |
| `duration` | `Double?`                     | `null`  | Duration    |
| `segments` | `List<TranscriptionSegment>?` | `[]`    | Segments    |

---

#### TranscriptionSegment

A segment of transcribed audio with timing information.

| Field   | Type     | Default | Description       |
| ------- | -------- | ------- | ----------------- |
| `id`    | `Int`    | —       | Unique identifier |
| `start` | `Double` | —       | Start             |
| `end`   | `Double` | —       | End               |
| `text`  | `String` | —       | Text              |

---

#### Usage

| Field                 | Type                   | Default | Description                                                                                                                                                                         |
| --------------------- | ---------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `promptTokens`        | `Long`                 | —       | Prompt tokens used. Defaults to 0 when absent (some providers omit this).                                                                                                           |
| `completionTokens`    | `Long`                 | —       | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses).                                                                                                       |
| `totalTokens`         | `Long`                 | —       | Total tokens used. Defaults to 0 when absent (some providers omit this).                                                                                                            |
| `promptTokensDetails` | `PromptTokensDetails?` | `null`  | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

#### UserMessage

| Field     | Type          | Default            | Description                |
| --------- | ------------- | ------------------ | -------------------------- |
| `content` | `UserContent` | `UserContent.Text` | The extracted text content |
| `name`    | `String?`     | `null`             | The name                   |

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

| Value   | Description                              |
| ------- | ---------------------------------------- |
| `Text`  | Text format — Fields: `0`: `String`      |
| `Parts` | Parts — Fields: `0`: `List<ContentPart>` |

---

#### ContentPart

| Value        | Description                                        |
| ------------ | -------------------------------------------------- |
| `Text`       | Text format — Fields: `text`: `String`             |
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

| Value      | Description                            |
| ---------- | -------------------------------------- |
| `Single`   | Single — Fields: `0`: `String`         |
| `Multiple` | Multiple — Fields: `0`: `List<String>` |

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

| Value      | Description                            |
| ---------- | -------------------------------------- |
| `Single`   | Single — Fields: `0`: `String`         |
| `Multiple` | Multiple — Fields: `0`: `List<String>` |

---

#### ModerationInput

Input to the moderation endpoint — a single string or multiple strings.

| Value      | Description                            |
| ---------- | -------------------------------------- |
| `Single`   | Single — Fields: `0`: `String`         |
| `Multiple` | Multiple — Fields: `0`: `List<String>` |

---

#### RerankDocument

A document to be reranked — either a plain string or an object with a text field.

| Value    | Description                         |
| -------- | ----------------------------------- |
| `Text`   | Text format — Fields: `0`: `String` |
| `Object` | Object — Fields: `text`: `String`   |

---

#### OcrDocument

Document input for OCR — either a URL or inline base64 data.

| Value    | Description                                                                            |
| -------- | -------------------------------------------------------------------------------------- |
| `Url`    | A publicly accessible document URL. — Fields: `url`: `String`                          |
| `Base64` | Inline base64-encoded document data. — Fields: `data`: `String`, `mediaType`: `String` |

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

| Value    | Description                                                     |
| -------- | --------------------------------------------------------------- |
| `Bearer` | Bearer token: `Authorization: Bearer <key>`                     |
| `ApiKey` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `0`: `String` |
| `None`   | No authentication required.                                     |

---

### Errors

#### LiterLlmError

All errors that can occur when using `liter-llm`.

| Variant                 | Description                                                                                                                                                                                                                                                                                                                                                      |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Authentication`        | `status` preserves the exact HTTP status code received (401 or 403).                                                                                                                                                                                                                                                                                             |
| `RateLimited`           | rate limited: {message}                                                                                                                                                                                                                                                                                                                                          |
| `BadRequest`            | `status` preserves the exact HTTP status code received (400, 405, 413, 422, …).                                                                                                                                                                                                                                                                                  |
| `ContextWindowExceeded` | context window exceeded: {message}                                                                                                                                                                                                                                                                                                                               |
| `ContentPolicy`         | content policy violation: {message}                                                                                                                                                                                                                                                                                                                              |
| `NotFound`              | not found: {message}                                                                                                                                                                                                                                                                                                                                             |
| `ServerError`           | `status` preserves the exact HTTP status code received (500, or other 5xx not covered by `ServiceUnavailable`).                                                                                                                                                                                                                                                  |
| `ServiceUnavailable`    | `status` preserves the exact HTTP status code received (502, 503, or 504).                                                                                                                                                                                                                                                                                       |
| `Timeout`               | request timeout                                                                                                                                                                                                                                                                                                                                                  |
| `Streaming`             | A catch-all for errors that occur during streaming response processing. This variant covers multiple sub-conditions including UTF-8 decoding failures, CRC/checksum mismatches (AWS EventStream), JSON parse errors in individual SSE chunks, and buffer overflow conditions. The `message` field contains a human-readable description of the specific failure. |
| `EndpointNotSupported`  | provider {provider} does not support {endpoint}                                                                                                                                                                                                                                                                                                                  |
| `InvalidHeader`         | invalid header {name:?}: {reason}                                                                                                                                                                                                                                                                                                                                |
| `Serialization`         | serialization error: {0}                                                                                                                                                                                                                                                                                                                                         |
| `BudgetExceeded`        | budget exceeded: {message}                                                                                                                                                                                                                                                                                                                                       |
| `HookRejected`          | hook rejected: {message}                                                                                                                                                                                                                                                                                                                                         |
| `InternalError`         | An internal logic error (e.g. unexpected Tower response variant). This should never surface in normal operation — if it does, it indicates a bug in the library.                                                                                                                                                                                                 |

---
