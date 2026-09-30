//! Bedrock ConverseStream event -> OpenAI `ChatCompletionChunk` translation.

use serde_json::{Value, json};

use super::converse_finish_reason;
use crate::error::{LiterLlmError, Result};
use crate::types::ChatCompletionChunk;

/// Parse a Bedrock ConverseStream EventStream event into a `ChatCompletionChunk`.
///
/// Bedrock ConverseStream events:
/// - `messageStart` → role delta
/// - `contentBlockStart` → tool_use start (with toolUseId and name)
/// - `contentBlockDelta` → text delta or tool_use input delta
/// - `contentBlockStop` → (ignored)
/// - `messageStop` → finish_reason
/// - `metadata` → usage (emitted as a final chunk with empty delta)
///
/// Returns `Ok(None)` for events that don't map to a chunk (e.g. `contentBlockStop`).
///
/// **Known limitation:** The `id` field is hardcoded to `"bedrock-stream"` and
/// `model` is always `""` on every chunk.  Bedrock's ConverseStream protocol does
/// not include a request/response ID or model name in its event payloads, and
/// this parser is stateless so it cannot carry forward values from the original
/// request.  This differs from the OpenAI format where every chunk includes the
/// real `id` and `model`.
pub(crate) fn parse_bedrock_stream_event(event_type: &str, payload: &str) -> Result<Option<ChatCompletionChunk>> {
    let v: Value = serde_json::from_str(payload).map_err(|e| LiterLlmError::Streaming {
        message: format!("Bedrock stream event parse error: {e}"),
    })?;

    match event_type {
        "messageStart" => message_start_chunk(&v),
        "contentBlockStart" => content_block_start_chunk(&v),
        "contentBlockDelta" => content_block_delta_chunk(&v),
        "contentBlockStop" => Ok(None),
        "messageStop" => message_stop_chunk(&v),
        "metadata" => metadata_chunk(&v),
        _ => Ok(None),
    }
}

/// Deserialize a chunk built as JSON, mapping failures to a streaming error.
fn chunk_from_json(chunk_json: Value) -> Result<ChatCompletionChunk> {
    serde_json::from_value(chunk_json).map_err(|e| LiterLlmError::Streaming {
        message: format!("Bedrock chunk deserialization error: {e}"),
    })
}

/// `messageStart`: a role-only delta.
fn message_start_chunk(v: &Value) -> Result<Option<ChatCompletionChunk>> {
    let role = v.get("role").and_then(|r| r.as_str()).unwrap_or("assistant");
    chunk_from_json(json!({
        "id": "bedrock-stream",
        "object": "chat.completion.chunk",
        "created": 0,
        "model": "",
        "choices": [{
            "index": 0,
            "delta": {"role": role},
            "finish_reason": null
        }]
    }))
    .map(Some)
}

/// `contentBlockStart`: a tool-call header for `toolUse` blocks; nothing otherwise.
fn content_block_start_chunk(v: &Value) -> Result<Option<ChatCompletionChunk>> {
    let index = v.get("contentBlockIndex").and_then(|i| i.as_u64()).unwrap_or(0);
    if let Some(tool_use) = v.pointer("/start/toolUse") {
        let tool_use_id = tool_use.get("toolUseId").and_then(|t| t.as_str()).unwrap_or("");
        let name = tool_use.get("name").and_then(|n| n.as_str()).unwrap_or("");
        chunk_from_json(json!({
            "id": "bedrock-stream",
            "object": "chat.completion.chunk",
            "created": 0,
            "model": "",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": index,
                        "id": tool_use_id,
                        "type": "function",
                        "function": {"name": name, "arguments": ""}
                    }]
                },
                "finish_reason": null
            }]
        }))
        .map(Some)
    } else {
        Ok(None)
    }
}

/// `contentBlockDelta`: a text delta or a tool-argument delta; unknown shapes
/// are logged and skipped.
fn content_block_delta_chunk(v: &Value) -> Result<Option<ChatCompletionChunk>> {
    let index = v.get("contentBlockIndex").and_then(|i| i.as_u64()).unwrap_or(0);

    if let Some(text) = v.pointer("/delta/text").and_then(|t| t.as_str()) {
        return chunk_from_json(json!({
            "id": "bedrock-stream",
            "object": "chat.completion.chunk",
            "created": 0,
            "model": "",
            "choices": [{
                "index": 0,
                "delta": {"content": text},
                "finish_reason": null
            }]
        }))
        .map(Some);
    }

    if let Some(input_json) = v.pointer("/delta/toolUse/input").and_then(|i| i.as_str()) {
        return chunk_from_json(json!({
            "id": "bedrock-stream",
            "object": "chat.completion.chunk",
            "created": 0,
            "model": "",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": index,
                        "function": {"arguments": input_json}
                    }]
                },
                "finish_reason": null
            }]
        }))
        .map(Some);
    }

    tracing::warn!(
        content_block_index = index,
        "Bedrock contentBlockDelta with unrecognized delta shape; skipping"
    );

    Ok(None)
}

/// `messageStop`: an empty delta carrying the mapped `finish_reason`.
fn message_stop_chunk(v: &Value) -> Result<Option<ChatCompletionChunk>> {
    let stop_reason = v.get("stopReason").and_then(|s| s.as_str()).unwrap_or("end_turn");
    let finish_reason = converse_finish_reason(stop_reason);
    chunk_from_json(json!({
        "id": "bedrock-stream",
        "object": "chat.completion.chunk",
        "created": 0,
        "model": "",
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": finish_reason
        }]
    }))
    .map(Some)
}

/// `metadata`: a choice-less chunk carrying usage.
fn metadata_chunk(v: &Value) -> Result<Option<ChatCompletionChunk>> {
    let input_tokens = v.pointer("/usage/inputTokens").and_then(|t| t.as_u64()).unwrap_or(0);
    let output_tokens = v.pointer("/usage/outputTokens").and_then(|t| t.as_u64()).unwrap_or(0);
    chunk_from_json(json!({
        "id": "bedrock-stream",
        "object": "chat.completion.chunk",
        "created": 0,
        "model": "",
        "choices": [],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens
        }
    }))
    .map(Some)
}
