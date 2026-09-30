//! Gemini `generateContent` response -> OpenAI chat completion translation.

use std::sync::atomic::Ordering;

use serde_json::{Value, json};

use super::TOOL_CALL_COUNTER;
use crate::error::Result;

/// Normalize a Gemini `generateContent` response to OpenAI chat completion format.
///
/// Gemini wraps the response in `candidates[0].content.parts[]`.
/// Finish reasons use Gemini terminology (`STOP`, `MAX_TOKENS`, `SAFETY`, ...)
/// and are mapped to the OpenAI `finish_reason` set.
///
/// If `groundingMetadata` is present on the candidate, it is included in the
/// response as `_grounding_metadata` for supplementary use by callers.
///
/// **Known limitation:** The `model` field in the normalized response is
/// always `""`.  Gemini/Vertex AI does not include the model name in its
/// response body -- the model is only present in the request URL path.
pub(crate) fn transform_gemini_response(body: &mut Value) -> Result<()> {
    if let Some(normalized) = non_chat_response(body) {
        *body = normalized;
        return Ok(());
    }

    let candidates = body.get("candidates").and_then(|c| c.as_array());
    if candidates.is_none_or(|c| c.is_empty()) {
        *body = blocked_prompt_response(body);
        return Ok(());
    }

    let candidate = body.pointer("/candidates/0").cloned();
    let finish_reason_raw = candidate
        .as_ref()
        .and_then(|c| c.get("finishReason"))
        .and_then(|f| f.as_str())
        .unwrap_or("STOP");
    let parts = candidate
        .as_ref()
        .and_then(|c| c.pointer("/content/parts"))
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    let collected = collect_parts(&parts);
    let tool_calls = tool_calls_from_parts(&parts);
    let finish_reason = gemini_finish_reason(finish_reason_raw);

    let (prompt_tokens, completion_tokens) = usage_token_counts(body);

    let response_id = body.get("responseId").cloned().unwrap_or_else(|| json!("gemini-resp"));

    let content_value = collected.content_value();

    let mut message = json!({"role": "assistant", "content": content_value});
    if !tool_calls.is_empty() {
        message["tool_calls"] = json!(tool_calls);
    }
    if let Some(reasoning) = collected.reasoning_text {
        message["reasoning_content"] = json!(reasoning);
    }

    let grounding_metadata = candidate.as_ref().and_then(|c| c.get("groundingMetadata")).cloned();

    let mut result = json!({
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
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens
        }
    });

    if let Some(gm) = grounding_metadata {
        result["_grounding_metadata"] = gm;
    }

    *body = result;

    Ok(())
}

/// Normalize the non-chat response shapes this endpoint also carries: Vertex
/// `:predict` embeddings, a model listing, or a Gemini `embedContent` result.
fn non_chat_response(body: &Value) -> Option<Value> {
    if let Some(predictions) = body.get("predictions").and_then(|p| p.as_array()) {
        let data: Vec<Value> = predictions
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let values = p.pointer("/embeddings/values").cloned().unwrap_or(json!([]));
                json!({"object": "embedding", "embedding": values, "index": i})
            })
            .collect();
        return Some(json!({
            "object": "list",
            "data": data,
            "model": "",
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        }));
    }

    if let Some(models) = body.get("models").and_then(|m| m.as_array()) {
        let data: Vec<Value> = models
            .iter()
            .map(|m| {
                let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let id = name.strip_prefix("models/").unwrap_or(name);
                json!({
                    "id": id,
                    "object": "model",
                    "created": 0,
                    "owned_by": "google"
                })
            })
            .collect();
        return Some(json!({
            "object": "list",
            "data": data
        }));
    }

    if body.get("embedding").is_some() {
        let values = body.pointer("/embedding/values").cloned().unwrap_or(json!([]));
        return Some(json!({
            "object": "list",
            "data": [{"object": "embedding", "embedding": values, "index": 0}],
            "model": "",
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        }));
    }

    None
}

/// Visible text, reasoning text and typed output parts gathered from a
/// candidate's `parts`.
struct CollectedParts {
    text: String,
    reasoning_text: Option<String>,
    output_parts: Vec<Value>,
    has_non_text: bool,
}

/// Sort a candidate's parts into visible text, thought (reasoning) text, and
/// inline image/audio output parts.
fn collect_parts(parts: &[Value]) -> CollectedParts {
    let mut text_parts: Vec<String> = vec![];
    let mut output_parts: Vec<Value> = vec![];
    // ~keep Gemini extended thinking marks reasoning parts with `"thought": true`
    // alongside their `text` field. These must never reach visible `content` —
    // route them to `reasoning_content` instead, mirroring how Anthropic's
    // `thinking` blocks are handled (#52).
    let mut reasoning_parts: Vec<String> = vec![];
    for p in parts {
        let is_thought = p.get("thought").and_then(Value::as_bool).unwrap_or(false);
        if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
            if !t.is_empty() {
                if is_thought {
                    reasoning_parts.push(t.to_owned());
                } else {
                    text_parts.push(t.to_owned());
                    output_parts.push(json!({"type": "text", "text": t}));
                }
            }
        } else if let Some(inline) = p.get("inlineData").or_else(|| p.get("inline_data")) {
            let mime_type = inline
                .get("mimeType")
                .or_else(|| inline.get("mime_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("application/octet-stream");
            let data = inline.get("data").and_then(|v| v.as_str()).unwrap_or("");
            let data_url = format!("data:{mime_type};base64,{data}");
            if mime_type.starts_with("image/") {
                output_parts.push(json!({
                    "type": "output_image",
                    "image_url": {"url": data_url}
                }));
            } else if mime_type.starts_with("audio/") {
                let fmt = mime_type.split('/').nth(1).unwrap_or("wav");
                output_parts.push(json!({
                    "type": "output_audio",
                    "audio": {"data": data, "format": fmt}
                }));
            } else {
                output_parts.push(json!({
                    "type": "output_image",
                    "image_url": {"url": data_url}
                }));
            }
        }
    }
    let has_non_text = output_parts
        .iter()
        .any(|p| p.get("type").and_then(|t| t.as_str()) != Some("text"));
    let text: String = text_parts.join("");
    let reasoning_text: Option<String> = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join(""))
    };

    CollectedParts {
        text,
        reasoning_text,
        output_parts,
        has_non_text,
    }
}

/// OpenAI `tool_calls` for every `functionCall` part, each with a fresh
/// process-unique id.
fn tool_calls_from_parts(parts: &[Value]) -> Vec<Value> {
    parts
        .iter()
        .filter_map(|p| {
            p.get("functionCall").map(|fc| {
                let name = fc.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                let call_id = TOOL_CALL_COUNTER.fetch_add(1, Ordering::Relaxed);
                let arguments = serde_json::to_string(fc.get("args").unwrap_or(&json!({}))).unwrap_or_default();
                json!({
                    "id": format!("call_{name}_{call_id}"),
                    "type": "function",
                    "function": {
                        "name": fc.get("name"),
                        "arguments": arguments
                    }
                })
            })
        })
        .collect()
}

/// Map a Gemini `finishReason` to an OpenAI `finish_reason`.
fn gemini_finish_reason(finish_reason_raw: &str) -> &'static str {
    match finish_reason_raw {
        "STOP" => "stop",
        "MAX_TOKENS" => "length",
        "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" | "IMAGE_SAFETY" => "content_filter",
        "LANGUAGE" | "OTHER" => "stop",
        "TOOL_CODE" | "FUNCTION_CALL" => "tool_calls",
        _ => "stop",
    }
}

/// The chat response for a prompt Gemini blocked outright: no candidates, a
/// `content_filter` finish, and the block reason surfaced as `_block_reason`.
fn blocked_prompt_response(body: &Value) -> Value {
    let block_reason = body
        .pointer("/promptFeedback/blockReason")
        .and_then(|r| r.as_str())
        .unwrap_or("UNKNOWN");
    let prompt_tokens = body
        .pointer("/usageMetadata/promptTokenCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    json!({
        "id": "gemini-resp",
        "object": "chat.completion",
        "created": crate::provider::unix_timestamp_secs(),
        "model": "",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": null},
            "finish_reason": "content_filter"
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": 0,
            "total_tokens": prompt_tokens
        },
        "system_fingerprint": null,
        "_block_reason": block_reason
    })
}

/// `(prompt_tokens, completion_tokens)` from Gemini `usageMetadata`.
fn usage_token_counts(body: &Value) -> (u64, u64) {
    let prompt_tokens = body
        .pointer("/usageMetadata/promptTokenCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let completion_tokens = body
        .pointer("/usageMetadata/candidatesTokenCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    (prompt_tokens, completion_tokens)
}

impl CollectedParts {
    /// The OpenAI message `content`: typed output parts when any non-text part
    /// exists, otherwise the joined text, or null when there is none.
    fn content_value(&self) -> Value {
        if self.has_non_text && !self.output_parts.is_empty() {
            json!(self.output_parts)
        } else if self.text.is_empty() {
            json!(null)
        } else {
            json!(self.text)
        }
    }
}
