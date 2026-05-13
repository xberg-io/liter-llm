---
title: "Configuration Reference"
---

## Configuration Reference

This page documents all configuration types and their defaults across all languages.

### SystemMessage
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | The extracted text content |
| `name` | `str | None` | `None` | The name |

---

### UserMessage
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `UserContent` | `UserContent.TEXT` | The extracted text content |
| `name` | `str | None` | `None` | The name |

---

### ImageUrl
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `str` | — | Url |
| `detail` | `ImageDetail | None` | `None` | Detail (image detail) |

---

### DocumentContent
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `str` | — | Base64-encoded document data or URL. |
| `media_type` | `str` | — | MIME type (e.g., "application/pdf", "text/csv"). |

---

### AudioContent
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `str` | — | Base64-encoded audio data. |
| `format` | `str` | — | Audio format (e.g., "wav", "mp3", "ogg"). |

---

### AssistantMessage
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str | None` | `None` | The extracted text content |
| `name` | `str | None` | `None` | The name |
| `tool_calls` | `list[ToolCall] | None` | `[]` | Tool calls |
| `refusal` | `str | None` | `None` | Refusal |
| `function_call` | `FunctionCall | None` | `None` | Deprecated legacy function_call field; retained for API compatibility. |

---

### ToolMessage
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | The extracted text content |
| `tool_call_id` | `str` | — | Tool call id |
| `name` | `str | None` | `None` | The name |

---

### DeveloperMessage
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | The extracted text content |
| `name` | `str | None` | `None` | The name |

---

### FunctionMessage
Deprecated legacy function-role message body.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | The extracted text content |
| `name` | `str` | — | The name |

---

### SpecificToolChoice
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `choice_type` | `ToolType` | `ToolType.FUNCTION` | Choice type (tool type) |
| `function` | `SpecificFunction` | — | Function (specific function) |

---

### SpecificFunction
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | The name |

---

### JsonSchemaFormat
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | The name |
| `description` | `str | None` | `None` | Human-readable description |
| `schema` | `dict[str, Any]` | — | Schema |
| `strict` | `bool | None` | `None` | Strict |

---

### Usage
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_tokens` | `int` | — | Prompt tokens used. Defaults to 0 when absent (some providers omit this). |
| `completion_tokens` | `int` | — | Completion tokens used. Defaults to 0 when absent (e.g. embedding responses). |
| `total_tokens` | `int` | — | Total tokens used. Defaults to 0 when absent (some providers omit this). |
| `prompt_tokens_details` | `PromptTokensDetails | None` | `None` | Breakdown of tokens used in the prompt, including cached tokens served at the provider's discounted cache-read rate. Absent when the provider does not return prompt-token details. |

---

### PromptTokensDetails
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

### ChatCompletionRequest
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model |
| `messages` | `list[Message]` | `[]` | Messages |
| `temperature` | `float | None` | `None` | Temperature |
| `top_p` | `float | None` | `None` | Top p |
| `n` | `int | None` | `None` | N |
| `stream` | `bool | None` | `None` | Whether to stream the response. Managed by the client layer — do not set directly. |
| `stop` | `StopSequence | None` | `None` | Stop (stop sequence) |
| `max_tokens` | `int | None` | `None` | Maximum tokens |
| `presence_penalty` | `float | None` | `None` | Presence penalty |
| `frequency_penalty` | `float | None` | `None` | Frequency penalty |
| `logit_bias` | `dict[str, float] | None` | `{}` | Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic serialization order — important when hashing or signing requests. |
| `user` | `str | None` | `None` | User |
| `tools` | `list[ChatCompletionTool] | None` | `[]` | Tools |
| `tool_choice` | `ToolChoice | None` | `None` | Tool choice (tool choice) |
| `parallel_tool_calls` | `bool | None` | `None` | Parallel tool calls |
| `response_format` | `ResponseFormat | None` | `None` | Response format (response format) |
| `stream_options` | `StreamOptions | None` | `None` | Stream options (stream options) |
| `seed` | `int | None` | `None` | Seed |
| `reasoning_effort` | `ReasoningEffort | None` | `None` | Reasoning effort (reasoning effort) |
| `extra_body` | `dict[str, Any] | None` | `None` | Provider-specific extra parameters merged into the request body. Use for guardrails, safety settings, grounding config, etc. |

---

### StreamOptions
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_usage` | `bool | None` | `None` | Include usage |

---

### ChatCompletionResponse
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier |
| `object` | `str` | — | Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `int` | — | Created |
| `model` | `str` | — | Model |
| `choices` | `list[Choice]` | `[]` | Choices |
| `usage` | `Usage | None` | `None` | Usage (usage) |
| `system_fingerprint` | `str | None` | `None` | System fingerprint |
| `service_tier` | `str | None` | `None` | Service tier |

---

### Choice
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index |
| `message` | `AssistantMessage` | — | Message (assistant message) |
| `finish_reason` | `FinishReason | None` | `None` | Finish reason (finish reason) |

---

### ChatCompletionChunk
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier |
| `object` | `str` | — | Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not fail parsing. |
| `created` | `int` | — | Created |
| `model` | `str` | — | Model |
| `choices` | `list[StreamChoice]` | `[]` | Choices |
| `usage` | `Usage | None` | `None` | Usage (usage) |
| `system_fingerprint` | `str | None` | `None` | System fingerprint |
| `service_tier` | `str | None` | `None` | Service tier |

---

### StreamChoice
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index |
| `delta` | `StreamDelta` | — | Delta (stream delta) |
| `finish_reason` | `FinishReason | None` | `None` | Finish reason (finish reason) |

---

### StreamDelta
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `role` | `str | None` | `None` | Role |
| `content` | `str | None` | `None` | The extracted text content |
| `tool_calls` | `list[StreamToolCall] | None` | `[]` | Tool calls |
| `function_call` | `StreamFunctionCall | None` | `None` | Deprecated legacy function_call delta; retained for API compatibility. |
| `refusal` | `str | None` | `None` | Refusal |

---

### StreamToolCall
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `int` | — | Index |
| `id` | `str | None` | `None` | Unique identifier |
| `call_type` | `ToolType | None` | `None` | Call type (tool type) |
| `function` | `StreamFunctionCall | None` | `None` | Function (stream function call) |

---

### StreamFunctionCall
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str | None` | `None` | The name |
| `arguments` | `str | None` | `None` | Arguments |

---

### EmbeddingRequest
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model |
| `input` | `EmbeddingInput` | `EmbeddingInput.SINGLE` | Input (embedding input) |
| `encoding_format` | `EmbeddingFormat | None` | `None` | Encoding format (embedding format) |
| `dimensions` | `int | None` | `None` | Dimensions |
| `user` | `str | None` | `None` | User |

---

### CreateImageRequest
Request to create images from a text prompt.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt` | `str` | — | Prompt |
| `model` | `str | None` | `None` | Model |
| `n` | `int | None` | `None` | N |
| `size` | `str | None` | `None` | Size in bytes |
| `quality` | `str | None` | `None` | Quality |
| `style` | `str | None` | `None` | Style |
| `response_format` | `str | None` | `None` | Response format |
| `user` | `str | None` | `None` | User |

---

### ImagesResponse
Response containing generated images.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `created` | `int` | — | Created |
| `data` | `list[Image]` | `[]` | Data |

---

### Image
A single generated image, returned as either a URL or base64 data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `str | None` | `None` | Url |
| `b64_json` | `str | None` | `None` | B64 json |
| `revised_prompt` | `str | None` | `None` | Revised prompt |

---

### CreateSpeechRequest
Request to generate speech audio from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model |
| `input` | `str` | — | Input |
| `voice` | `str` | — | Voice |
| `response_format` | `str | None` | `None` | Response format |
| `speed` | `float | None` | `None` | Speed |

---

### CreateTranscriptionRequest
Request to transcribe audio into text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model |
| `file` | `str` | — | Base64-encoded audio file data. |
| `language` | `str | None` | `None` | Language |
| `prompt` | `str | None` | `None` | Prompt |
| `response_format` | `str | None` | `None` | Response format |
| `temperature` | `float | None` | `None` | Temperature |

---

### TranscriptionResponse
Response from a transcription request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `str` | — | Text |
| `language` | `str | None` | `None` | Language |
| `duration` | `float | None` | `None` | Duration |
| `segments` | `list[TranscriptionSegment] | None` | `[]` | Segments |

---

### TranscriptionSegment
A segment of transcribed audio with timing information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `int` | — | Unique identifier |
| `start` | `float` | — | Start |
| `end` | `float` | — | End |
| `text` | `str` | — | Text |

---

### ModerationRequest
Request to classify content for policy violations.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input` | `ModerationInput` | `ModerationInput.SINGLE` | Input (moderation input) |
| `model` | `str | None` | `None` | Model |

---

### ModerationCategories
Boolean flags for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `bool` | — | Sexual |
| `hate` | `bool` | — | Hate |
| `harassment` | `bool` | — | Harassment |
| `self_harm` | `bool` | — | Self harm |
| `sexual_minors` | `bool` | — | Sexual minors |
| `hate_threatening` | `bool` | — | Hate threatening |
| `violence_graphic` | `bool` | — | Violence graphic |
| `self_harm_intent` | `bool` | — | Self harm intent |
| `self_harm_instructions` | `bool` | — | Self harm instructions |
| `harassment_threatening` | `bool` | — | Harassment threatening |
| `violence` | `bool` | — | Violence |

---

### ModerationCategoryScores
Confidence scores for each moderation category.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sexual` | `float` | — | Sexual |
| `hate` | `float` | — | Hate |
| `harassment` | `float` | — | Harassment |
| `self_harm` | `float` | — | Self harm |
| `sexual_minors` | `float` | — | Sexual minors |
| `hate_threatening` | `float` | — | Hate threatening |
| `violence_graphic` | `float` | — | Violence graphic |
| `self_harm_intent` | `float` | — | Self harm intent |
| `self_harm_instructions` | `float` | — | Self harm instructions |
| `harassment_threatening` | `float` | — | Harassment threatening |
| `violence` | `float` | — | Violence |

---

### RerankRequest
Request to rerank documents by relevance to a query.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model |
| `query` | `str` | — | Query |
| `documents` | `list[RerankDocument]` | `[]` | Documents |
| `top_n` | `int | None` | `None` | Top n |
| `return_documents` | `bool | None` | `None` | Return documents |

---

### SearchRequest
A search request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | The model/provider to use (e.g. `"brave/web-search"`, `"tavily/search"`). |
| `query` | `str` | — | The search query. |
| `max_results` | `int | None` | `None` | Maximum number of results to return. |
| `search_domain_filter` | `list[str] | None` | `[]` | Domain filter — restrict results to specific domains. |
| `country` | `str | None` | `None` | Country code for localized results (ISO 3166-1 alpha-2). |

---

### OcrRequest
An OCR request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | The model/provider to use (e.g. `"mistral/mistral-ocr-latest"`). |
| `document` | `OcrDocument` | `OcrDocument.URL` | The document to process. |
| `pages` | `list[int] | None` | `[]` | Specific pages to process (1-indexed). `None` means all pages. |
| `include_image_base64` | `bool | None` | `None` | Whether to include base64-encoded images of each page. |

---

### ModelsListResponse
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `str` | — | Always `"list"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `data` | `list[ModelObject]` | `[]` | Data |

---

### ModelObject
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier |
| `object` | `str` | — | Always `"model"` from OpenAI-compatible APIs.  Stored as a plain `String` so non-standard provider values do not break deserialization. |
| `created` | `int` | — | Created |
| `owned_by` | `str` | — | Owned by |

---

### CreateFileRequest
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | `str` | — | Base64-encoded file data. |
| `purpose` | `FilePurpose` | `FilePurpose.ASSISTANTS` | Purpose (file purpose) |
| `filename` | `str | None` | `None` | Filename |

---

### FileObject
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier |
| `object` | `str` | — | Object |
| `bytes` | `int` | — | Bytes |
| `created_at` | `int` | — | Created at |
| `filename` | `str` | — | Filename |
| `purpose` | `str` | — | Purpose |
| `status` | `str | None` | `None` | Status |

---

### FileListResponse
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `str` | — | Object |
| `data` | `list[FileObject]` | `[]` | Data |
| `has_more` | `bool | None` | `None` | Whether more |

---

### FileListQuery
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `purpose` | `str | None` | `None` | Purpose |
| `limit` | `int | None` | `None` | Limit |
| `after` | `str | None` | `None` | After |

---

### DeleteResponse
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier |
| `object` | `str` | — | Object |
| `deleted` | `bool` | — | Deleted |

---

### CreateBatchRequest
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_file_id` | `str` | — | Input file id |
| `endpoint` | `str` | — | Endpoint |
| `completion_window` | `str` | — | Completion window |
| `metadata` | `dict[str, Any] | None` | `None` | Document metadata |

---

### BatchObject
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier |
| `object` | `str` | — | Object |
| `endpoint` | `str` | — | Endpoint |
| `input_file_id` | `str` | — | Input file id |
| `completion_window` | `str` | — | Completion window |
| `status` | `BatchStatus` | `BatchStatus.VALIDATING` | Status (batch status) |
| `output_file_id` | `str | None` | `None` | Output file id |
| `error_file_id` | `str | None` | `None` | Error file id |
| `created_at` | `int` | — | Created at |
| `completed_at` | `int | None` | `None` | Completed at |
| `failed_at` | `int | None` | `None` | Failed at |
| `expired_at` | `int | None` | `None` | Expired at |
| `request_counts` | `BatchRequestCounts | None` | `None` | Request counts (batch request counts) |
| `metadata` | `dict[str, Any] | None` | `None` | Document metadata |

---

### BatchRequestCounts
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total` | `int` | — | Total |
| `completed` | `int` | — | Completed |
| `failed` | `int` | — | Failed |

---

### BatchListResponse
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `object` | `str` | — | Object |
| `data` | `list[BatchObject]` | `[]` | Data |
| `has_more` | `bool | None` | `None` | Whether more |
| `first_id` | `str | None` | `None` | First id |
| `last_id` | `str | None` | `None` | Last id |

---

### BatchListQuery
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `limit` | `int | None` | `None` | Limit |
| `after` | `str | None` | `None` | After |

---

### CreateResponseRequest
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Model |
| `input` | `dict[str, Any]` | — | Input |
| `instructions` | `str | None` | `None` | Instructions |
| `tools` | `list[ResponseTool] | None` | `[]` | Tools |
| `temperature` | `float | None` | `None` | Temperature |
| `max_output_tokens` | `int | None` | `None` | Maximum output tokens |
| `metadata` | `dict[str, Any] | None` | `None` | Document metadata |

---

### ResponseTool
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tool_type` | `str` | — | Tool type |
| `config` | `dict[str, Any]` | — | Config |

---

### ResponseObject
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `str` | — | Unique identifier |
| `object` | `str` | — | Object |
| `created_at` | `int` | — | Created at |
| `model` | `str` | — | Model |
| `status` | `str` | — | Status |
| `output` | `list[ResponseOutputItem]` | `[]` | Output |
| `usage` | `ResponseUsage | None` | `None` | Usage (response usage) |
| `error` | `dict[str, Any] | None` | `None` | Error |

---

### ResponseOutputItem
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `item_type` | `str` | — | Item type |
| `content` | `dict[str, Any]` | — | The extracted text content |

---

### ResponseUsage
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_tokens` | `int` | — | Input tokens |
| `output_tokens` | `int` | — | Output tokens |
| `total_tokens` | `int` | — | Total tokens |

---

### CustomProviderConfig
Configuration for registering a custom LLM provider at runtime.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | Unique name for this provider (e.g., "my-provider"). |
| `base_url` | `str` | — | Base URL for the provider's API (e.g., "<https://api.my-provider.com/v1">). |
| `auth_header` | `AuthHeaderFormat` | — | Authentication header format. |
| `model_prefixes` | `list[str]` | — | Model name prefixes that route to this provider (e.g., ["my-"]). |

---

### Enums

#### AuthHeaderFormat
How the API key is sent in the HTTP request.

| Variant | Description |
|---------|-------------|
| `Bearer` | Bearer token: `Authorization: Bearer <key>` |
| `ApiKey` | Custom header: e.g., `X-Api-Key: <key>` — Fields: `_0`: `String` |
| `None` | No authentication required. |

---

#### BatchStatus
| Variant | Wire value | Description |
|---------|------------|-------------|
| `Validating` | `validating` | Validating |
| `Failed` | `failed` | Failed |
| `InProgress` | `in_progress` | In progress |
| `Finalizing` | `finalizing` | Finalizing |
| `Completed` | `completed` | Completed |
| `Expired` | `expired` | Expired |
| `Cancelling` | `cancelling` | Cancelling |
| `Cancelled` | `cancelled` | Cancelled |

---

#### EmbeddingFormat
The format in which the embedding vectors are returned.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Float` | `float` | 32-bit floating-point numbers (default). |
| `Base64` | `base64` | Base64-encoded string representation of the floats. |

---

#### EmbeddingInput
| Variant | Description |
|---------|-------------|
| `Single` | Single — Fields: `_0`: `String` |
| `Multiple` | Multiple — Fields: `_0`: `Vec<String>` |

---

#### FilePurpose
| Variant | Wire value | Description |
|---------|------------|-------------|
| `Assistants` | `assistants` | Assistants |
| `Batch` | `batch` | Batch |
| `FineTune` | `fine-tune` | Fine tune |
| `Vision` | `vision` | Vision |

---

#### FinishReason
Why a choice stopped generating tokens.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Stop` | `stop` | Stop |
| `Length` | `length` | Length |
| `ToolCalls` | `tool_calls` | Tool calls |
| `ContentFilter` | `content_filter` | Content filter |
| `FunctionCall` | `function_call` | Deprecated legacy finish reason; retained for API compatibility. |
| `Other` | `other` | Catch-all for unknown finish reasons returned by non-OpenAI providers. Note: this intentionally does **not** carry the original string (e.g. `Other(String)`).  Using `#[serde(other)]` requires a unit variant, and switching to `#[serde(untagged)]` would change deserialization semantics for all variants.  The original value can be recovered by inspecting the raw JSON if needed. |

---

#### ImageDetail
| Variant | Wire value | Description |
|---------|------------|-------------|
| `Low` | `low` | Low |
| `High` | `high` | High |
| `Auto` | `auto` | Auto |

---

#### Message
A chat message in a conversation.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `System` | `system` | System — Fields: `_0`: `SystemMessage` |
| `User` | `user` | User — Fields: `_0`: `UserMessage` |
| `Assistant` | `assistant` | Assistant — Fields: `_0`: `AssistantMessage` |
| `Tool` | `tool` | Tool — Fields: `_0`: `ToolMessage` |
| `Developer` | `developer` | Developer — Fields: `_0`: `DeveloperMessage` |
| `Function` | `function` | Deprecated legacy function-role message; retained for API compatibility. — Fields: `_0`: `FunctionMessage` |

---

#### ModerationInput
Input to the moderation endpoint — a single string or multiple strings.

| Variant | Description |
|---------|-------------|
| `Single` | Single — Fields: `_0`: `String` |
| `Multiple` | Multiple — Fields: `_0`: `Vec<String>` |

---

#### OcrDocument
Document input for OCR — either a URL or inline base64 data.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Url` | `document_url` | A publicly accessible document URL. — Fields: `url`: `String` |
| `Base64` | `base64` | Inline base64-encoded document data. — Fields: `data`: `String`, `media_type`: `String` |

---

#### ReasoningEffort
Controls how much reasoning effort the model should use.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Low` | `low` | Low |
| `Medium` | `medium` | Medium |
| `High` | `high` | High |

---

#### RerankDocument
A document to be reranked — either a plain string or an object with a text field.

| Variant | Description |
|---------|-------------|
| `Text` | Text format — Fields: `_0`: `String` |
| `Object` | Object — Fields: `text`: `String` |

---

#### ResponseFormat
| Variant | Wire value | Description |
|---------|------------|-------------|
| `Text` | `text` | Text format |
| `JsonObject` | `json_object` | Json object |
| `JsonSchema` | `json_schema` | Json schema — Fields: `json_schema`: `JsonSchemaFormat` |

---

#### StopSequence
| Variant | Description |
|---------|-------------|
| `Single` | Single — Fields: `_0`: `String` |
| `Multiple` | Multiple — Fields: `_0`: `Vec<String>` |

---

#### ToolChoice
| Variant | Description |
|---------|-------------|
| `Mode` | Mode — Fields: `_0`: `ToolChoiceMode` |
| `Specific` | Specific — Fields: `_0`: `SpecificToolChoice` |

---

#### ToolType
The type discriminator for tool/tool-call objects.

Per the OpenAI spec this is always `"function"`. Using an enum enforces
that constraint at the type level and rejects any other value on
deserialization.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Function` | `function` | Function |

---

#### UserContent
| Variant | Description |
|---------|-------------|
| `Text` | Text format — Fields: `_0`: `String` |
| `Parts` | Parts — Fields: `_0`: `Vec<ContentPart>` |

---
