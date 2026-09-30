//! Bedrock Converse response -> OpenAI chat completion translation.

use serde_json::{Value, json};

use super::converse_finish_reason;
use crate::error::Result;

/// Normalize a Bedrock Converse API response to OpenAI chat completion format.
///
/// See [`super::BedrockProvider`]'s `transform_response` for the known
/// limitation on the `model` field.
pub(super) fn transform_converse_response(body: &mut Value) -> Result<()> {
    let stop_reason = body.get("stopReason").and_then(|s| s.as_str()).unwrap_or("end_turn");
    let usage = body.get("usage").cloned();

    let content_blocks = body
        .pointer("/output/message/content")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let text: String = content_blocks
        .iter()
        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("");

    let tool_calls = tool_calls_from_blocks(&content_blocks);

    let finish_reason = converse_finish_reason(stop_reason);

    let input_tokens = usage
        .as_ref()
        .and_then(|u| u.get("inputTokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .as_ref()
        .and_then(|u| u.get("outputTokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let response_id = body
        .get("requestId")
        .or_else(|| body.get("conversationId"))
        .cloned()
        .unwrap_or_else(|| json!("bedrock-resp"));

    let content_value: serde_json::Value = if text.is_empty() { json!(null) } else { json!(text) };

    let mut message = json!({"role": "assistant", "content": content_value});
    if !tool_calls.is_empty() {
        message["tool_calls"] = json!(tool_calls);
    }

    *body = json!({
        "id": response_id,
        "object": "chat.completion",
        "created": crate::provider::unix_timestamp_secs(),
        "model": "",
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason
        }],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens
        }
    });

    Ok(())
}

/// OpenAI `tool_calls` entries for every `toolUse` content block.
fn tool_calls_from_blocks(content_blocks: &[Value]) -> Vec<Value> {
    content_blocks
        .iter()
        .filter_map(|b| {
            b.get("toolUse").map(|tu| {
                let arguments = serde_json::to_string(tu.get("input").unwrap_or(&json!({}))).unwrap_or_default();
                json!({
                    "id": tu.get("toolUseId"),
                    "type": "function",
                    "function": {
                        "name": tu.get("name"),
                        "arguments": arguments
                    }
                })
            })
        })
        .collect()
}
