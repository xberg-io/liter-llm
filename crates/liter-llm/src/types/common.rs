use serde::{Deserialize, Serialize};

// ─── Messages ────────────────────────────────────────────────────────────────

/// A chat message in a conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "system")]
    System(SystemMessage),
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "tool")]
    Tool(ToolMessage),
    #[serde(rename = "developer")]
    Developer(DeveloperMessage),
    /// Deprecated legacy function-role message; retained for API compatibility.
    #[serde(rename = "function")]
    Function(FunctionMessage),
}

#[cfg_attr(alef, alef(skip))]
impl Default for Message {
    fn default() -> Self {
        Self::Assistant(AssistantMessage::default())
    }
}

/// System message guiding model behavior for the entire conversation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    /// Instructions or context that apply throughout the conversation.
    pub content: String,
    /// Optional name for the system message source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// User message in the conversation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    /// Message content as plain text or array of content parts (text, images, documents, audio).
    pub content: UserContent,
    /// Optional name for the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// User message content as either plain text or a list of multimodal parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserContent {
    /// Plain text content.
    Text(String),
    /// Array of content parts (text, images, documents, audio).
    Parts(Vec<ContentPart>),
}

#[cfg_attr(alef, alef(skip))]
impl Default for UserContent {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

/// A single content part in a user message — text, image, document, or audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    /// Plain text.
    #[serde(rename = "text")]
    Text { text: String },
    /// Image identified by URL (with optional detail level).
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
    /// Document file (PDF, CSV, etc.) as base64 or URL.
    #[serde(rename = "document")]
    Document { document: DocumentContent },
    /// Audio input as base64.
    #[serde(rename = "input_audio")]
    InputAudio { input_audio: AudioContent },
}

#[cfg_attr(alef, alef(skip))]
impl Default for ContentPart {
    fn default() -> Self {
        Self::Text { text: String::new() }
    }
}

/// An image URL reference with optional detail level for processing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageUrl {
    /// URL of the image (data URI or HTTP/HTTPS URL).
    pub url: String,
    /// Detail level: low (512x512), high (2x2 tiles), or auto (model-selected).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetail>,
}

/// Image detail level controlling token cost and processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    /// Low detail: scales image to 512x512, uses fewer tokens.
    Low,
    /// High detail: processes up to 2x2 grid of tiles, higher token cost.
    High,
    /// Auto: model chooses low or high based on image dimensions.
    Auto,
}

/// PDF/document content part for vision-capable models.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentContent {
    /// Base64-encoded document data or URL.
    pub data: String,
    /// MIME type (e.g., "application/pdf", "text/csv").
    pub media_type: String,
}

/// Audio content part for speech-capable models.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioContent {
    /// Base64-encoded audio data.
    pub data: String,
    /// Audio format (e.g., "wav", "mp3", "ogg").
    pub format: String,
}

/// Assistant's response to a user message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// The assistant's text response. Absent if tool calls are returned instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Optional name for the assistant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool calls the model wants to execute, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Refusal reason, if the model declined to respond per safety policies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// Deprecated legacy function_call field; retained for API compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
}

/// Tool execution result returned to the model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolMessage {
    /// Result of the tool execution.
    pub content: String,
    /// ID of the tool call this result responds to.
    pub tool_call_id: String,
    /// Optional tool/function name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Developer message (system-like message for Claude models).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeveloperMessage {
    /// Developer-specific instructions or context.
    pub content: String,
    /// Optional name for the developer message source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Deprecated legacy function-role message body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FunctionMessage {
    pub content: String,
    pub name: String,
}

// ─── Ergonomic constructors ───────────────────────────────────────────────────

impl Message {
    /// Build a user message with multimodal content parts.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::{Message, ContentPart};
    ///
    /// let msg = Message::user_with_parts(vec![
    ///     ContentPart::text("Describe this image:"),
    ///     ContentPart::image_png(b"\x89PNG"),
    /// ]);
    /// ```
    pub fn user_with_parts(parts: Vec<ContentPart>) -> Self {
        Self::User(UserMessage {
            content: UserContent::Parts(parts),
            name: None,
        })
    }
}

impl ContentPart {
    /// Create a text content part.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::ContentPart;
    ///
    /// let part = ContentPart::text("Hello, world!");
    /// ```
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text { text: s.into() }
    }

    /// Create an image content part from a data URL or HTTP/HTTPS URL.
    ///
    /// Both `image_data_url` and `image_url` produce identical output —
    /// `ImageUrl { url, detail: None }`. The two names exist for caller
    /// clarity: use `image_data_url` when passing a `data:` URI and
    /// `image_url` when passing an HTTPS URL.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::ContentPart;
    /// use liter_llm::image::{encode_data_url, IMAGE_PNG};
    ///
    /// let url = encode_data_url(b"\x89PNG", Some(IMAGE_PNG));
    /// let part = ContentPart::image_data_url(url);
    /// ```
    pub fn image_data_url(url: impl Into<String>) -> Self {
        Self::ImageUrl {
            image_url: ImageUrl {
                url: url.into(),
                detail: None,
            },
        }
    }

    /// Create an image content part from an HTTP/HTTPS URL.
    ///
    /// Both `image_url` and `image_data_url` produce identical output —
    /// `ImageUrl { url, detail: None }`. The two names exist for caller
    /// clarity: use `image_url` when passing an HTTPS URL and
    /// `image_data_url` when passing a `data:` URI.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::ContentPart;
    ///
    /// let part = ContentPart::image_url("https://example.com/photo.jpg");
    /// ```
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::ImageUrl {
            image_url: ImageUrl {
                url: url.into(),
                detail: None,
            },
        }
    }

    /// Create an image content part with an explicit [`ImageDetail`] level.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::{ContentPart, ImageDetail};
    ///
    /// let part = ContentPart::image_with_detail(
    ///     "https://example.com/photo.jpg",
    ///     ImageDetail::High,
    /// );
    /// ```
    pub fn image_with_detail(url: impl Into<String>, detail: ImageDetail) -> Self {
        Self::ImageUrl {
            image_url: ImageUrl {
                url: url.into(),
                detail: Some(detail),
            },
        }
    }

    /// Create an image content part from raw PNG bytes, encoding as a data URL.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::ContentPart;
    ///
    /// let part = ContentPart::image_png(b"\x89PNG\r\n\x1a\n");
    /// ```
    pub fn image_png(bytes: &[u8]) -> Self {
        Self::image_data_url(crate::image::encode_data_url(bytes, Some(crate::image::IMAGE_PNG)))
    }

    /// Create an image content part from raw JPEG bytes, encoding as a data URL.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::ContentPart;
    ///
    /// let part = ContentPart::image_jpeg(b"\xff\xd8\xff");
    /// ```
    pub fn image_jpeg(bytes: &[u8]) -> Self {
        Self::image_data_url(crate::image::encode_data_url(bytes, Some(crate::image::IMAGE_JPEG)))
    }

    /// Create an image content part from raw WebP bytes, encoding as a data URL.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::ContentPart;
    ///
    /// let part = ContentPart::image_webp(b"RIFF");
    /// ```
    pub fn image_webp(bytes: &[u8]) -> Self {
        Self::image_data_url(crate::image::encode_data_url(bytes, Some(crate::image::IMAGE_WEBP)))
    }

    /// Create an image content part from raw TIFF bytes, encoding as a data URL.
    ///
    /// # Example
    ///
    /// ```
    /// use liter_llm::types::ContentPart;
    ///
    /// let part = ContentPart::image_tiff(b"II*\0");
    /// ```
    pub fn image_tiff(bytes: &[u8]) -> Self {
        Self::image_data_url(crate::image::encode_data_url(bytes, Some(crate::image::IMAGE_TIFF)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_part_text_constructor() {
        let part = ContentPart::text("hi");
        let json = serde_json::to_string(&part).expect("serialization should not fail");
        assert_eq!(json, r#"{"type":"text","text":"hi"}"#);
    }

    #[test]
    fn content_part_image_data_url_constructor() {
        let part = ContentPart::image_data_url("data:image/png;base64,aGk=");
        let json = serde_json::to_string(&part).expect("serialization should not fail");
        assert_eq!(
            json,
            r#"{"type":"image_url","image_url":{"url":"data:image/png;base64,aGk="}}"#
        );
    }

    #[test]
    fn content_part_image_with_detail() {
        let part = ContentPart::image_with_detail("https://example.com/img.png", ImageDetail::High);
        let json = serde_json::to_string(&part).expect("serialization should not fail");
        assert_eq!(
            json,
            r#"{"type":"image_url","image_url":{"url":"https://example.com/img.png","detail":"high"}}"#
        );
    }

    #[test]
    fn content_part_image_png_round_trip() {
        let part = ContentPart::image_png(b"hi");
        match &part {
            ContentPart::ImageUrl { image_url } => {
                assert!(
                    image_url.url.starts_with("data:image/png;base64,"),
                    "expected png data URL, got: {}",
                    image_url.url
                );
            }
            other => panic!("expected ImageUrl variant, got: {other:?}"),
        }
    }

    #[test]
    fn message_user_with_parts() {
        let msg = Message::user_with_parts(vec![
            ContentPart::text("hello"),
            ContentPart::image_data_url("data:image/png;base64,aGk="),
        ]);
        let json = serde_json::to_string(&msg).expect("serialization should not fail");
        assert_eq!(
            json,
            r#"{"role":"user","content":[{"type":"text","text":"hello"},{"type":"image_url","image_url":{"url":"data:image/png;base64,aGk="}}]}"#
        );
    }

    // ── ResponseFormat / JsonSchemaFormat constructors ──────────────────────

    #[test]
    fn json_schema_new_defaults_strict_true() {
        let fmt = JsonSchemaFormat::new("S", serde_json::json!({}));
        assert_eq!(fmt.strict, Some(true));
        assert_eq!(fmt.description, None);
        assert_eq!(fmt.name, "S");
    }

    #[test]
    fn json_schema_strict_toggle() {
        let fmt = JsonSchemaFormat::new("S", serde_json::json!({})).strict(false);
        assert_eq!(fmt.strict, Some(false));
    }

    #[test]
    fn json_schema_description_attaches() {
        let fmt = JsonSchemaFormat::new("S", serde_json::json!({})).description("d");
        assert_eq!(fmt.description.as_deref(), Some("d"));
    }

    #[test]
    fn response_format_json_schema_serializes() {
        let fmt = ResponseFormat::json_schema("S", serde_json::json!({"type": "object"}));
        let value = serde_json::to_value(&fmt).expect("serialization must succeed");
        assert_eq!(
            value,
            serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "S",
                    "schema": {"type": "object"},
                    "strict": true
                }
            })
        );
        // description must be absent (skip_serializing_if = "Option::is_none")
        assert!(value["json_schema"].get("description").is_none());
    }

    #[test]
    fn response_format_json_object_serializes() {
        let value = serde_json::to_value(ResponseFormat::json_object()).expect("serialization must succeed");
        assert_eq!(value, serde_json::json!({"type": "json_object"}));
    }

    #[test]
    fn response_format_text_serializes() {
        let value = serde_json::to_value(ResponseFormat::text()).expect("serialization must succeed");
        assert_eq!(value, serde_json::json!({"type": "text"}));
    }

    #[test]
    fn chat_request_serializes_response_format() {
        use crate::types::chat::ChatCompletionRequest;
        let request = ChatCompletionRequest {
            model: "gpt-4o".into(),
            messages: vec![],
            response_format: Some(ResponseFormat::json_schema(
                "PersonSchema",
                serde_json::json!({"type": "object", "properties": {"name": {"type": "string"}}}),
            )),
            ..Default::default()
        };
        let value = serde_json::to_value(&request).expect("serialization must succeed");
        let rf = &value["response_format"];
        assert_eq!(rf["type"], "json_schema");
        assert_eq!(rf["json_schema"]["name"], "PersonSchema");
        assert_eq!(rf["json_schema"]["strict"], true);
    }
}

// ─── Tools ───────────────────────────────────────────────────────────────────

/// The type discriminator for tool/tool-call objects.
///
/// Per the OpenAI spec this is always `"function"`. Using an enum enforces
/// that constraint at the type level and rejects any other value on
/// deserialization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolType {
    #[default]
    #[serde(rename = "function")]
    Function,
}

/// A tool the model can invoke (currently, all tools are functions).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionTool {
    /// Tool type (always "function" in OpenAI spec).
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    /// Function definition with name, description, and JSON schema parameters.
    pub function: FunctionDefinition,
}

/// Function definition exposed to the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionDefinition {
    /// Name of the function. Required and must be alphanumeric + underscores.
    pub name: String,
    /// Human-readable description explaining what the function does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema defining the function's parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// If true, enforce strict JSON schema validation for arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// A tool call the model wants to execute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this call, used to reference in tool result messages.
    pub id: String,
    /// Tool type (always "function").
    #[serde(rename = "type")]
    pub call_type: ToolType,
    /// Function name and arguments.
    pub function: FunctionCall,
}

/// Function call details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name.
    pub name: String,
    /// Arguments as a JSON string (parse with serde_json::from_str).
    pub arguments: String,
}

// ─── Tool Choice ─────────────────────────────────────────────────────────────

/// Tool usage mode or a specific tool to call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Predefined mode: auto, required, or none.
    Mode(ToolChoiceMode),
    /// Force a specific tool to be called.
    Specific(SpecificToolChoice),
}

#[cfg_attr(alef, alef(skip))]
impl Default for ToolChoice {
    fn default() -> Self {
        Self::Mode(ToolChoiceMode::default())
    }
}

/// Tool choice mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceMode {
    /// Model may or may not call tools; default behavior.
    #[default]
    Auto,
    /// Model must call at least one tool.
    Required,
    /// Model must not call any tools.
    None,
}

/// Directive to call a specific tool.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpecificToolChoice {
    /// Tool type (always "function").
    #[serde(rename = "type")]
    pub choice_type: ToolType,
    /// The specific function to invoke.
    pub function: SpecificFunction,
}

/// Name of the specific function to invoke.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpecificFunction {
    /// Function name.
    pub name: String,
}

// ─── Response Format ─────────────────────────────────────────────────────────

/// Wire format for the chat completions `response_format` field.
///
/// # Provider mapping
///
/// - **OpenAI** (and OpenAI-compatible providers): emitted verbatim as
///   `{"type": "json_schema", "json_schema": {...}}` per the
///   chat-completions spec.
/// - **Gemini / Vertex AI**: translated to
///   `generationConfig.responseMimeType = "application/json"` and
///   `generationConfig.responseSchema = <schema>`. The `name`,
///   `description`, and `strict` fields are dropped — Gemini's
///   structured-output API does not consume them.
/// - **Anthropic**: no native JSON mode. A system instruction is
///   prepended asking the model to respond with valid JSON.
///   `strict` is advisory only; callers should still validate the
///   returned JSON if the schema is load-bearing.
///
/// # Example
///
/// ```no_run
/// # use liter_llm::types::{ResponseFormat, ChatCompletionRequest};
/// # use serde_json::json;
/// let request = ChatCompletionRequest {
///     model: "gpt-4o".into(),
///     messages: vec![],
///     response_format: Some(ResponseFormat::json_schema(
///         "PersonSchema",
///         json!({ "type": "object", "properties": { "name": { "type": "string" } } }),
///     )),
///     ..Default::default()
/// };
/// # let _ = request;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    /// Plain text output (default).
    #[default]
    #[serde(rename = "text")]
    Text,
    /// Output must be valid JSON object (no schema validation).
    #[serde(rename = "json_object")]
    JsonObject,
    /// Output must conform to the specified JSON schema.
    #[serde(rename = "json_schema")]
    JsonSchema { json_schema: JsonSchemaFormat },
}

impl ResponseFormat {
    /// Construct a `json_schema` response format with `strict = true`.
    ///
    /// # Example
    ///
    /// ```
    /// # use liter_llm::types::ResponseFormat;
    /// # use serde_json::json;
    /// let fmt = ResponseFormat::json_schema("MySchema", json!({"type": "object"}));
    /// ```
    pub fn json_schema(name: impl Into<String>, schema: serde_json::Value) -> Self {
        Self::JsonSchema {
            json_schema: JsonSchemaFormat::new(name, schema),
        }
    }

    /// Construct a `json_object` response format (unvalidated JSON).
    ///
    /// # Example
    ///
    /// ```
    /// # use liter_llm::types::ResponseFormat;
    /// let fmt = ResponseFormat::json_object();
    /// ```
    pub fn json_object() -> Self {
        Self::JsonObject
    }

    /// Construct a plain-text response format (the default).
    ///
    /// # Example
    ///
    /// ```
    /// # use liter_llm::types::ResponseFormat;
    /// let fmt = ResponseFormat::text();
    /// ```
    pub fn text() -> Self {
        Self::Text
    }
}

/// JSON Schema specification for constrained output.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonSchemaFormat {
    /// Name of the schema (must be unique in the request).
    pub name: String,
    /// Description of what the schema represents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema object defining the output structure.
    pub schema: serde_json::Value,
    /// If true, enforce strict schema validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl JsonSchemaFormat {
    /// Create a strict `json_schema` response format with the given name and schema.
    ///
    /// Defaults: `strict = Some(true)`, `description = None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use liter_llm::types::JsonSchemaFormat;
    /// # use serde_json::json;
    /// let fmt = JsonSchemaFormat::new("PersonSchema", json!({"type": "object"}));
    /// assert_eq!(fmt.strict, Some(true));
    /// ```
    pub fn new(name: impl Into<String>, schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: None,
            schema,
            strict: Some(true),
        }
    }

    /// Override the strict-mode flag.
    ///
    /// # Example
    ///
    /// ```
    /// # use liter_llm::types::JsonSchemaFormat;
    /// # use serde_json::json;
    /// let fmt = JsonSchemaFormat::new("S", json!({})).strict(false);
    /// assert_eq!(fmt.strict, Some(false));
    /// ```
    #[must_use]
    pub fn strict(mut self, on: bool) -> Self {
        self.strict = Some(on);
        self
    }

    /// Attach a description shown to the model.
    ///
    /// # Example
    ///
    /// ```
    /// # use liter_llm::types::JsonSchemaFormat;
    /// # use serde_json::json;
    /// let fmt = JsonSchemaFormat::new("S", json!({})).description("A person object");
    /// assert_eq!(fmt.description.as_deref(), Some("A person object"));
    /// ```
    #[must_use]
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
}

// ─── Usage ───────────────────────────────────────────────────────────────────

/// Token-usage accounting returned by the provider on each completion / embedding call.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Prompt tokens used. Defaults to 0 when absent (some providers omit this).
    #[serde(default)]
    pub prompt_tokens: u64,
    /// Completion tokens used. Defaults to 0 when absent (e.g. embedding responses).
    #[serde(default)]
    pub completion_tokens: u64,
    /// Total tokens used. Defaults to 0 when absent (some providers omit this).
    #[serde(default)]
    pub total_tokens: u64,
    /// Breakdown of tokens used in the prompt, including cached tokens served
    /// at the provider's discounted cache-read rate. Absent when the provider
    /// does not return prompt-token details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

/// Breakdown of tokens used in the prompt portion of a request.
///
/// `cached_tokens` is included in `Usage::prompt_tokens` — it is *not* an
/// additional charge on top of the prompt token count. When pricing supports
/// a `cache_read_input_token_cost`, the cached portion is billed at the
/// discounted rate and the remainder at the regular input rate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    /// Cached tokens present in the prompt. Defaults to 0 when absent.
    #[serde(default)]
    pub cached_tokens: u64,
    /// Audio input tokens present in the prompt. Defaults to 0 when absent.
    #[serde(default)]
    pub audio_tokens: u64,
}

// ─── Stop Sequence ───────────────────────────────────────────────────────────

/// Stop sequence(s) that cause the model to stop generating.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopSequence {
    /// Single stop sequence.
    Single(String),
    /// Multiple stop sequences.
    Multiple(Vec<String>),
}

#[cfg_attr(alef, alef(skip))]
impl Default for StopSequence {
    fn default() -> Self {
        Self::Single(String::new())
    }
}
