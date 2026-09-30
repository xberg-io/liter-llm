//! Anthropic Messages API response -> OpenAI chat completion translation.

use serde_json::{Value, json};

use super::map_stop_reason;
use crate::error::Result;

/// Normalize an Anthropic Messages API response into OpenAI chat completion
/// format. A body without `stop_reason` is assumed to already be OpenAI-shaped
/// and is left untouched.
pub(super) fn transform_response(body: &mut Value) -> Result<()> {
    if body.get("stop_reason").is_none() {
        return Ok(());
    }

    let id = body.get("id").cloned().unwrap_or(json!(""));
    let model = body.get("model").cloned().unwrap_or(json!(""));

    let content_blocks = body.get("content").and_then(|v| v.as_array()).cloned();

    // ~keep Exclude Anthropic thinking blocks from user-facing content; they are surfaced
    // ~keep separately via `reasoning_content` below.
    // ~keep Citation text is already present in adjacent text blocks.
    let text_content: Option<String> = content_blocks
        .as_ref()
        .map(|blocks| joined_block_text(blocks, "text", "text"));

    // ~keep Fold `thinking` blocks' text into `reasoning_content`, mirroring the
    // ~keep OpenAI-compatible `reasoning_content` extension (DeepSeek R1, Qwen).
    // ~keep `redacted_thinking` blocks carry no visible text and are skipped.
    let reasoning_content: Option<String> = content_blocks.as_ref().and_then(|blocks| {
        let joined = joined_block_text(blocks, "thinking", "thinking");
        if joined.is_empty() { None } else { Some(joined) }
    });

    let tool_calls: Option<Vec<Value>> = content_blocks.as_ref().map(|blocks| tool_calls_from_blocks(blocks));

    let stop_reason = body.get("stop_reason").and_then(|v| v.as_str()).unwrap_or("end_turn");
    let finish_reason = map_stop_reason(stop_reason);

    let (prompt_tokens, output_tokens) = usage_token_counts(body);

    let message = assistant_message(text_content.as_deref(), tool_calls, reasoning_content);

    *body = json!({
        "id": id,
        "object": "chat.completion",
        "created": crate::provider::unix_timestamp_secs(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": prompt_tokens + output_tokens
        }
    });

    Ok(())
}

/// Concatenate, in order, the `field` text of every block whose `type` is
/// `block_type`.
fn joined_block_text(blocks: &[Value], block_type: &str, field: &str) -> String {
    blocks
        .iter()
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some(block_type))
        .filter_map(|b| b.get(field).and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("")
}

/// OpenAI `tool_calls` entries for every `tool_use` / `server_tool_use` block.
fn tool_calls_from_blocks(blocks: &[Value]) -> Vec<Value> {
    blocks
        .iter()
        .filter(|b| {
            matches!(
                b.get("type").and_then(|t| t.as_str()),
                Some("tool_use") | Some("server_tool_use")
            )
        })
        .map(|b| {
            let arguments = serde_json::to_string(b.get("input").unwrap_or(&json!({}))).unwrap_or_default();
            json!({
                "id": b.get("id").cloned().unwrap_or(json!("")),
                "type": "function",
                "function": {
                    "name": b.get("name").cloned().unwrap_or(json!("")),
                    "arguments": arguments
                }
            })
        })
        .collect()
}

/// `(prompt_tokens, completion_tokens)` from Anthropic `usage`. Prompt tokens
/// include cache-creation and cache-read input tokens.
fn usage_token_counts(body: &Value) -> (u64, u64) {
    let input_tokens = body
        .pointer("/usage/input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_creation_tokens = body
        .pointer("/usage/cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_read_tokens = body
        .pointer("/usage/cache_read_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = body
        .pointer("/usage/output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let prompt_tokens = input_tokens + cache_creation_tokens + cache_read_tokens;
    (prompt_tokens, output_tokens)
}

/// Build the OpenAI `assistant` message from the extracted parts.
fn assistant_message(
    text_content: Option<&str>,
    tool_calls: Option<Vec<Value>>,
    reasoning_content: Option<String>,
) -> Value {
    let has_tool_calls = tool_calls.as_ref().is_some_and(|tc| !tc.is_empty());
    // ~keep Absent visible text maps to null (OpenAI convention), whether the turn
    // ~keep carried only tool calls, only reasoning/thinking, or nothing at all.
    let message_content = match text_content {
        Some(text) if !text.is_empty() => json!(text),
        _ => Value::Null,
    };

    let mut message = json!({
        "role": "assistant",
        "content": message_content
    });

    if let (Some(tc), true) = (tool_calls, has_tool_calls) {
        message["tool_calls"] = json!(tc);
    }

    if let Some(reasoning) = reasoning_content {
        message["reasoning_content"] = json!(reasoning);
    }

    message
}
