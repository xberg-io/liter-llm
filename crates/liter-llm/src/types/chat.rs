use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::common::{
    AssistantMessage, ChatCompletionTool, Message, ResponseFormat, StopSequence, ToolChoice, ToolType, Usage,
};
use crate::cost;

// ─── Finish Reason ────────────────────────────────────────────────────────────

/// Why a choice stopped generating tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    #[default]
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    /// Deprecated legacy finish reason; retained for API compatibility.
    #[serde(rename = "function_call")]
    FunctionCall,
    /// Catch-all for unknown finish reasons returned by non-OpenAI providers.
    ///
    /// Note: this intentionally does **not** carry the original string (e.g.
    /// `Other(String)`).  Using `#[serde(other)]` requires a unit variant, and
    /// switching to `#[serde(untagged)]` would change deserialization semantics
    /// for all variants.  The original value can be recovered by inspecting the
    /// raw JSON if needed.
    #[serde(other)]
    Other,
}

#[cfg_attr(alef, alef(skip))]
impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        f.write_str(&s)
    }
}

// ─── Reasoning Effort ────────────────────────────────────────────────────────

/// Controls how much reasoning effort the model should use.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    #[default]
    Medium,
    High,
}

// ─── Request ─────────────────────────────────────────────────────────────────

/// Chat completion request (compatible with OpenAI and similar APIs).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionRequest {
    /// Model ID (e.g., `"gpt-4o-mini"`, `"claude-3-5-sonnet"`).
    pub model: String,
    /// Conversation history from oldest to newest.
    pub messages: Vec<Message>,
    /// Sampling temperature in `[0.0, 2.0]`. Higher increases randomness. Defaults to 1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Nucleus sampling parameter in `[0.0, 1.0]`. Lower is more focused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Number of chat completions to generate. Defaults to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    /// Whether to stream the response.
    ///
    /// Managed by the client layer — do not set directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Stop sequence(s) that halt token generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopSequence>,
    /// Max output tokens. Different from max_completion_tokens in some providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Presence penalty in `[-2.0, 2.0]`. Positive discourages repeated topics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Frequency penalty in `[-2.0, 2.0]`. Positive discourages repeated tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Token bias map.  Uses `BTreeMap` (sorted keys) for deterministic
    /// serialization order — important when hashing or signing requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<BTreeMap<String, f64>>,
    /// User identifier for request tracking and abuse detection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Tools the model can invoke.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionTool>>,
    /// Tool usage mode (auto, required, none, or specific tool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Whether the model can call multiple tools in parallel. Defaults to true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Output format constraint (text, JSON, JSON schema).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// Streaming options (e.g., include_usage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    /// Random seed for reproducible outputs. Provider support varies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Reasoning effort level (low, medium, high) for extended-thinking models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
    /// Provider-specific extra parameters merged into the request body.
    /// Use for guardrails, safety settings, grounding config, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<serde_json::Value>,
}

/// Options for streaming responses.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamOptions {
    /// If true, include token usage in the final stream chunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

// ─── Response ────────────────────────────────────────────────────────────────

/// Chat completion response from the API.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Unique identifier for this response.
    pub id: String,
    /// Always `"chat.completion"` from OpenAI-compatible APIs.  Stored as a
    /// plain `String` so non-standard provider values do not break deserialization.
    pub object: String,
    /// Unix timestamp of response creation.
    pub created: u64,
    /// Model used to generate the response.
    pub model: String,
    /// List of completion choices.
    pub choices: Vec<Choice>,
    /// Token usage statistics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Fingerprint of the system configuration (OpenAI-specific).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Service tier used (OpenAI-specific).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
}

impl ChatCompletionResponse {
    /// Estimate the cost of this response based on embedded pricing data.
    ///
    /// Returns `None` if:
    /// - the `model` field is not present in the embedded pricing registry, or
    /// - the `usage` field is absent from the response.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cost = response.estimated_cost();
    /// if let Some(usd) = cost {
    ///     println!("Request cost: ${usd:.6}");
    /// }
    /// ```
    #[cfg_attr(alef, alef(skip))]
    #[must_use]
    pub fn estimated_cost(&self) -> Option<f64> {
        let usage = self.usage.as_ref()?;
        let cached = usage.prompt_tokens_details.as_ref().map_or(0, |d| d.cached_tokens);
        cost::completion_cost_with_cache(&self.model, usage.prompt_tokens, cached, usage.completion_tokens)
    }
}

/// A single completion choice.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    /// Index of this choice in the choices array.
    pub index: u32,
    /// The assistant's message response.
    pub message: AssistantMessage,
    /// Why the model stopped generating (stop, length, tool_calls, content_filter, etc.).
    pub finish_reason: Option<FinishReason>,
}

// ─── Stream Chunk ────────────────────────────────────────────────────────────

/// A streamed chunk of a chat completion response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for this stream.
    pub id: String,
    /// Always `"chat.completion.chunk"` from OpenAI-compatible APIs.  Stored
    /// as a plain `String` so non-standard provider values do not fail parsing.
    pub object: String,
    /// Unix timestamp of chunk creation.
    pub created: u64,
    /// Model used to generate the chunk.
    pub model: String,
    /// Streaming choices (delta updates).
    pub choices: Vec<StreamChoice>,
    /// Token usage (typically only in the final chunk).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Fingerprint of the system configuration (OpenAI-specific).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Service tier used (OpenAI-specific).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
}

/// A streaming choice with incremental delta.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamChoice {
    /// Index of this choice in the choices array.
    pub index: u32,
    /// Incremental update to the message (content, tool calls, etc.).
    pub delta: StreamDelta,
    /// Why the stream ended (present only in final chunk).
    pub finish_reason: Option<FinishReason>,
}

/// Incremental delta in a stream chunk.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamDelta {
    /// Role (typically present only in the first chunk).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Partial content chunk (e.g., a few words of the response).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Partial tool calls being streamed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<StreamToolCall>>,
    /// Deprecated legacy function_call delta; retained for API compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_call: Option<StreamFunctionCall>,
    /// Partial refusal message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

/// A streaming tool call being built incrementally.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamToolCall {
    /// Index of this tool call in the tool_calls array.
    pub index: u32,
    /// Tool call ID (typically in the first chunk for this call).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool type (typically "function").
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub call_type: Option<ToolType>,
    /// Partial function name and arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<StreamFunctionCall>,
}

/// Partial function call details in a stream.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamFunctionCall {
    /// Function name (typically in the first chunk).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Partial JSON arguments chunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::common::PromptTokensDetails;

    fn make_response(model: &str, usage: Usage) -> ChatCompletionResponse {
        ChatCompletionResponse {
            id: "test".into(),
            object: "chat.completion".into(),
            created: 0,
            model: model.into(),
            choices: vec![],
            usage: Some(usage),
            system_fingerprint: None,
            service_tier: None,
        }
    }

    #[test]
    fn estimated_cost_applies_cache_discount_when_prompt_tokens_details_present() {
        // claude-sonnet-4-5 has cache_read pricing in the registry.
        let resp = make_response(
            "claude-sonnet-4-5",
            Usage {
                prompt_tokens: 1_000,
                completion_tokens: 50,
                total_tokens: 1_050,
                prompt_tokens_details: Some(PromptTokensDetails {
                    cached_tokens: 200,
                    audio_tokens: 0,
                }),
            },
        );
        let with_cache = resp.estimated_cost().expect("should price");
        let no_cache = make_response(
            "claude-sonnet-4-5",
            Usage {
                prompt_tokens: 1_000,
                completion_tokens: 50,
                total_tokens: 1_050,
                prompt_tokens_details: None,
            },
        )
        .estimated_cost()
        .expect("should price");
        assert!(
            with_cache < no_cache,
            "cached cost ({with_cache}) must be cheaper than uncached ({no_cache})"
        );
    }

    #[test]
    fn estimated_cost_ignores_cached_tokens_when_no_pricing_difference() {
        // gpt-4 has no cache pricing — cached tokens should not change cost.
        let usage_with_cached = Usage {
            prompt_tokens: 1_000,
            completion_tokens: 50,
            total_tokens: 1_050,
            prompt_tokens_details: Some(PromptTokensDetails {
                cached_tokens: 500,
                audio_tokens: 0,
            }),
        };
        let usage_no_details = Usage {
            prompt_tokens: 1_000,
            completion_tokens: 50,
            total_tokens: 1_050,
            prompt_tokens_details: None,
        };
        let a = make_response("gpt-4", usage_with_cached).estimated_cost().expect("cost estimation should succeed for known model");
        let b = make_response("gpt-4", usage_no_details).estimated_cost().expect("cost estimation should succeed for known model");
        assert!((a - b).abs() < 1e-12);
    }

    #[test]
    fn usage_round_trips_prompt_tokens_details_via_serde() {
        let json = r#"{
            "prompt_tokens": 100,
            "completion_tokens": 20,
            "total_tokens": 120,
            "prompt_tokens_details": {"cached_tokens": 30, "audio_tokens": 0}
        }"#;
        let usage: Usage = serde_json::from_str(json).expect("valid OpenAI usage shape");
        assert_eq!(usage.prompt_tokens_details.as_ref().map(|d| d.cached_tokens), Some(30));
        let reser = serde_json::to_string(&usage).expect("serialization should not fail");
        assert!(reser.contains("\"cached_tokens\":30"));
    }
}
