use std::borrow::Cow;

use serde_json::Value;

use crate::error::Result;
use crate::provider::Provider;
use crate::types::ChatCompletionChunk;

mod request;
mod response;
mod stream;

/// Anthropic's stable API version. This is the only GA version as of 2025;
/// Anthropic gates new features via beta headers (e.g. `anthropic-beta`),
/// not by bumping the version date.
static ANTHROPIC_EXTRA_HEADERS: &[(&str, &str)] = &[("anthropic-version", "2023-06-01")];

/// Default max_tokens for Anthropic requests when none is specified.
/// Anthropic requires this field; OpenAI makes it optional.
const DEFAULT_MAX_TOKENS: u64 = 4096;

/// Known Anthropic hosted tool type names that require beta headers.
const HOSTED_TOOL_TYPES: &[&str] = &[
    "computer_20241022",
    "computer_use_20250124",
    "web_search_20250305",
    "code_execution_20250522",
];

/// Anthropic beta header values for hosted tool features.
const BETA_COMPUTER_USE: &str = "computer-use-2025-01-24";
const BETA_WEB_SEARCH: &str = "web-search-2025-03-05";
const BETA_CODE_EXECUTION: &str = "code-execution-2025-05-22";
const BETA_THINKING: &str = "thinking-2025-04-14";
const BETA_PROMPT_CACHING: &str = "prompt-caching-2024-07-31";
const BETA_PDFS: &str = "pdfs-2024-09-25";

/// Anthropic provider (Claude model family).
///
/// Differences from the OpenAI-compatible baseline:
/// - Auth uses `x-api-key` instead of `Authorization: Bearer`.
/// - Requires a mandatory `anthropic-version` header on every request.
/// - Model names start with `claude-` or are routed via the `anthropic/` prefix.
/// - Chat endpoint is `/messages`, not `/chat/completions`.
/// - Request and response JSON formats differ from OpenAI.
pub struct AnthropicProvider {
    /// Anthropic API base URL, e.g. `https://api.anthropic.com/v1`.
    ///
    /// Defaults to the official Anthropic endpoint; overridable via
    /// [`AnthropicProvider::with_base_url`] for proxies and self-hosted
    /// gateways that speak the Anthropic Messages API (issue #159).
    base_url: String,
}

/// Official Anthropic API base URL.
const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";

impl AnthropicProvider {
    /// Construct an [`AnthropicProvider`] pointed at the official Anthropic API.
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_ANTHROPIC_BASE_URL.to_owned(),
        }
    }

    /// Construct an [`AnthropicProvider`] with an explicit `base_url`.
    ///
    /// Used when a `[[models]]` config entry or client `base_url` override
    /// pins a per-model Anthropic-compatible endpoint — e.g. a proxy or
    /// self-hosted gateway (see issue #159). Trailing slashes are stripped.
    /// Falls back to the official Anthropic URL if the trimmed string is empty.
    #[must_use]
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let trimmed = base_url.into().trim_end_matches('/').to_owned();
        let base_url = if trimmed.is_empty() {
            DEFAULT_ANTHROPIC_BASE_URL.to_owned()
        } else {
            trimmed
        };
        Self { base_url }
    }
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn env_var(&self) -> Option<&str> {
        Some("ANTHROPIC_API_KEY")
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        // ~keep Anthropic uses x-api-key, not Authorization: Bearer.
        Some((Cow::Borrowed("x-api-key"), Cow::Borrowed(api_key)))
    }

    fn extra_headers(&self) -> &'static [(&'static str, &'static str)] {
        ANTHROPIC_EXTRA_HEADERS
    }

    /// Compute request-dependent beta headers based on the request body.
    ///
    /// Inspects the body for features that require Anthropic beta headers:
    /// - `thinking` field present -> `anthropic-beta: thinking-2025-04-14`
    /// - Hosted tools (computer_use, web_search, code_execution) -> appropriate betas
    ///
    /// Multiple betas are combined with comma separator.
    fn dynamic_headers(&self, body: &serde_json::Value) -> Vec<(String, String)> {
        let mut betas: Vec<&str> = Vec::new();

        if body.get("thinking").is_some() {
            betas.push(BETA_THINKING);
        }

        if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
            for tool in tools {
                let tool_type = tool.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match tool_type {
                    "computer_20241022" | "computer_use_20250124" if !betas.contains(&BETA_COMPUTER_USE) => {
                        betas.push(BETA_COMPUTER_USE);
                    }
                    "web_search_20250305" if !betas.contains(&BETA_WEB_SEARCH) => {
                        betas.push(BETA_WEB_SEARCH);
                    }
                    "code_execution_20250522" if !betas.contains(&BETA_CODE_EXECUTION) => {
                        betas.push(BETA_CODE_EXECUTION);
                    }
                    _ => {}
                }
            }
        }

        if body_contains_cache_control(body) && !betas.contains(&BETA_PROMPT_CACHING) {
            betas.push(BETA_PROMPT_CACHING);
        }

        if body_contains_document_block(body) && !betas.contains(&BETA_PDFS) {
            betas.push(BETA_PDFS);
        }

        if betas.is_empty() {
            vec![]
        } else {
            vec![("anthropic-beta".to_owned(), betas.join(","))]
        }
    }

    fn matches_model(&self, model: &str) -> bool {
        model.starts_with("claude-") || model.starts_with("anthropic/")
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        model.strip_prefix("anthropic/").unwrap_or(model)
    }

    /// Anthropic uses `/messages` instead of `/chat/completions`.
    fn chat_completions_path(&self) -> &str {
        "/messages"
    }

    /// Transform an OpenAI-format request body into Anthropic Messages API format.
    ///
    /// Key differences handled here:
    /// - System messages extracted to top-level `system` field as content blocks.
    /// - User/assistant messages converted to Anthropic content block arrays.
    /// - Tool messages (role=tool) become user messages with `tool_result` blocks.
    /// - Consecutive same-role messages are merged (Anthropic requires alternating roles).
    /// - `max_tokens` defaults to 4096 if not set (Anthropic requires it).
    /// - `stop` renamed to `stop_sequences` and normalised to an array.
    /// - `tool_choice` mapped from OpenAI semantics to Anthropic semantics.
    /// - Tools converted from OpenAI `function` wrappers to Anthropic `input_schema` format.
    /// - Unsupported parameters removed: `n`, `presence_penalty`, `frequency_penalty`,
    ///   `logit_bias`, `stream` (the client handles stream separately), `logprobs`,
    ///   `top_logprobs`, `store`, `metadata`, `prediction`, `audio`, `web_search_options`,
    ///   `modalities`, `seed` (these have no Anthropic equivalent; `logprobs`/`top_logprobs`/
    ///   `metadata`/`audio`/`web_search_options`/`seed` are logged at WARN when actually
    ///   requested, and `modalities` is logged at WARN only when it asks for a non-text
    ///   modality Claude cannot produce).
    /// - `temperature` above `1.0` (Anthropic's documented maximum, narrower than the
    ///   `[0.0, 2.0]` this crate documents generically) is rejected with a `BadRequest`
    ///   error rather than forwarded and left for Anthropic to reject.
    fn transform_request(&self, body: &mut Value) -> Result<()> {
        request::transform_request(body)
    }

    /// Normalize an Anthropic Messages API response into OpenAI chat completion format.
    ///
    /// Anthropic response shape:
    /// ```json
    /// { "id": "msg_...", "type": "message", "role": "assistant",
    ///   "content": [{"type": "text", "text": "..."}],
    ///   "stop_reason": "end_turn",
    ///   "usage": {"input_tokens": N, "output_tokens": M} }
    /// ```
    fn transform_response(&self, body: &mut Value) -> Result<()> {
        response::transform_response(body)
    }

    /// Parse an Anthropic SSE event into an OpenAI-compatible `ChatCompletionChunk`.
    ///
    /// Anthropic event types handled:
    /// - `message_start`: emits a role-only delta chunk.
    /// - `content_block_start`: emits empty delta (tool_use: emits tool_call header chunk).
    /// - `content_block_delta`: emits text, thinking, or tool input JSON delta.
    /// - `message_delta`: emits final chunk with finish_reason and usage.
    /// - `message_stop`: signals end of stream, returns `Ok(None)`.
    /// - `content_block_stop`, `ping`: skipped (returns `Ok(None)` — no content to emit).
    /// - `error`: returns `Err(LiterLlmError::Streaming)`.
    ///
    /// **Note:** The `id` and `model` fields are only populated on the first
    /// chunk (`message_start`).  Subsequent chunks emit empty strings for both
    /// fields because this parser is stateless — it cannot carry forward values
    /// from earlier events.  This differs from the OpenAI format where every
    /// chunk includes `id` and `model`.
    fn parse_stream_event(&self, event_data: &str) -> Result<Option<ChatCompletionChunk>> {
        stream::parse_stream_event(event_data)
    }
}

/// Sanitize a tool_call_id so it only contains characters allowed by Anthropic: `[a-zA-Z0-9_-]`.
/// Any other character is replaced with `_`.
///
/// Returns a borrowed `Cow` when the ID is already valid, avoiding allocation
/// on the common path (e.g. IDs starting with `toolu_`).
fn sanitize_tool_call_id(id: &str) -> Cow<'_, str> {
    if id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
        Cow::Borrowed(id)
    } else {
        Cow::Owned(
            id.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect(),
        )
    }
}

/// Check whether a tool type string represents an Anthropic hosted tool.
fn is_hosted_tool_type(tool_type: &str) -> bool {
    HOSTED_TOOL_TYPES.contains(&tool_type)
}

/// Return `true` if any `cache_control` key appears anywhere in the JSON body.
///
/// Searches messages, system blocks, and tool definitions recursively.
fn body_contains_cache_control(body: &Value) -> bool {
    match body {
        Value::Object(map) => {
            if map.contains_key("cache_control") {
                return true;
            }
            map.values().any(body_contains_cache_control)
        }
        Value::Array(arr) => arr.iter().any(body_contains_cache_control),
        _ => false,
    }
}

/// Return `true` if the body contains any content block with `"type": "document"`.
///
/// Scans the messages array for document content parts (PDF uploads, etc.).
fn body_contains_document_block(body: &Value) -> bool {
    body.get("messages").and_then(|m| m.as_array()).is_some_and(|messages| {
        messages.iter().any(|msg| {
            msg.get("content").and_then(|c| c.as_array()).is_some_and(|content| {
                content
                    .iter()
                    .any(|part| part.get("type").and_then(|t| t.as_str()) == Some("document"))
            })
        })
    })
}

/// Map an Anthropic `stop_reason` string to an OpenAI `finish_reason` string.
fn map_stop_reason(stop_reason: &str) -> &'static str {
    match stop_reason {
        "end_turn" | "stop_sequence" => "stop",
        "tool_use" => "tool_calls",
        "max_tokens" => "length",
        "content_filtered" | "refusal" => "content_filter",
        _ => "stop",
    }
}

#[cfg(test)]
mod feature_tests;
#[cfg(test)]
mod tests;
