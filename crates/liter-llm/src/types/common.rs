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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentContent {
    /// Base64-encoded document data or URL.
    pub data: String,
    /// MIME type (e.g., "application/pdf", "text/csv").
    pub media_type: String,
}

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

/// Response format constraint.
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

// ─── Usage ───────────────────────────────────────────────────────────────────

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
