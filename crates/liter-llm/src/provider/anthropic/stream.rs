//! Anthropic SSE event -> OpenAI `ChatCompletionChunk` translation.

use serde_json::Value;

use super::map_stop_reason;
use crate::error::{LiterLlmError, Result};
use crate::types::{ChatCompletionChunk, FinishReason, StreamChoice, StreamDelta, StreamFunctionCall, StreamToolCall};

/// Parse an Anthropic SSE event into an OpenAI-compatible `ChatCompletionChunk`.
///
/// See [`super::AnthropicProvider`]'s `parse_stream_event` for the event types
/// handled and the stateless `id` / `model` caveat.
pub(super) fn parse_stream_event(event_data: &str) -> Result<Option<ChatCompletionChunk>> {
    // ~keep `[DONE]` is consumed by the SSE parser before provider parsing.

    let event: Value = serde_json::from_str(event_data).map_err(|e| LiterLlmError::Streaming {
        message: format!("failed to parse Anthropic SSE event: {e}"),
    })?;

    let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match event_type {
        "message_start" => Ok(Some(message_start_chunk(&event))),

        "content_block_start" => Ok(content_block_start_chunk(&event)),

        "content_block_delta" => Ok(content_block_delta_chunk(&event)),

        "message_delta" => Ok(Some(message_delta_chunk(&event))),

        "message_stop" => Ok(None),

        "content_block_stop" | "ping" => Ok(None),

        "error" => {
            let message = event
                .pointer("/error/message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown Anthropic streaming error");
            Err(LiterLlmError::Streaming {
                message: message.to_owned(),
            })
        }

        _ => Ok(None),
    }
}

/// `message_start`: a role-only delta carrying `id`, `model` and prompt usage
/// (including cache tokens) when non-zero.
fn message_start_chunk(event: &Value) -> ChatCompletionChunk {
    let msg = &event["message"];
    let id = msg.get("id").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    let model = msg.get("model").and_then(|v| v.as_str()).unwrap_or("").to_owned();

    let input_tokens = msg.pointer("/usage/input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let cache_creation = msg
        .pointer("/usage/cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_read = msg
        .pointer("/usage/cache_read_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let prompt_tokens = input_tokens + cache_creation + cache_read;

    let usage = if prompt_tokens > 0 {
        Some(crate::types::Usage {
            prompt_tokens,
            completion_tokens: 0,
            total_tokens: prompt_tokens,
            prompt_tokens_details: None,
        })
    } else {
        None
    };

    ChatCompletionChunk {
        id,
        object: "chat.completion.chunk".to_owned(),
        created: crate::provider::unix_timestamp_secs(),
        model,
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: Some("assistant".to_owned()),
                content: None,
                tool_calls: None,
                function_call: None,
                refusal: None,
                reasoning_content: None,
            },
            finish_reason: None,
        }],
        usage,
        system_fingerprint: None,
        service_tier: None,
    }
}

/// `content_block_start`: a tool-call header chunk for `tool_use` /
/// `server_tool_use` blocks; nothing for other block types.
fn content_block_start_chunk(event: &Value) -> Option<ChatCompletionChunk> {
    let block = &event["content_block"];
    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
    // ~keep Anthropic block indices include text/thinking/tool blocks, so tool indices may have gaps.
    // ~keep The same index appears in start and delta events, and clients correlate by id.
    let anthropic_index = event.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    if block_type == "tool_use" || block_type == "server_tool_use" {
        let tool_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_owned();
        let tool_name = block.get("name").and_then(|v| v.as_str()).unwrap_or("").to_owned();

        return Some(make_empty_chunk_with_tool_start(anthropic_index, tool_id, tool_name));
    }

    None
}

/// `content_block_delta`: text, thinking or tool-argument deltas.
fn content_block_delta_chunk(event: &Value) -> Option<ChatCompletionChunk> {
    let delta = &event["delta"];
    let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let index = event.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    match delta_type {
        "text_delta" => {
            let text = delta.get("text").and_then(|t| t.as_str()).unwrap_or("");
            Some(make_text_chunk("", "", text))
        }
        "thinking_delta" => {
            // ~keep Route extended-thinking text into `reasoning_content`, mirroring
            // ~keep the OpenAI-compatible `reasoning_content` extension (DeepSeek R1, Qwen).
            let thinking = delta.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
            Some(make_reasoning_chunk("", "", thinking))
        }
        "signature_delta" => {
            // ~keep The thinking-block signature is an opaque verification token, not
            // ~keep visible text; it is never surfaced in `content` or `reasoning_content`.
            None
        }
        "input_json_delta" => {
            let partial_json = delta.get("partial_json").and_then(|v| v.as_str()).unwrap_or("");
            Some(make_tool_arguments_delta(index, partial_json))
        }
        _ => None,
    }
}

/// `message_delta`: the final chunk carrying `finish_reason` and completion usage.
fn message_delta_chunk(event: &Value) -> ChatCompletionChunk {
    let stop_reason = event.pointer("/delta/stop_reason").and_then(|v| v.as_str());
    let finish_reason = stop_reason.map(map_stop_reason);
    let output_tokens = event.pointer("/usage/output_tokens").and_then(|v| v.as_u64());

    let finish = finish_reason.map(|fr| match fr {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "tool_calls" => FinishReason::ToolCalls,
        _ => FinishReason::Other,
    });

    let usage = output_tokens.map(|ct| crate::types::Usage {
        prompt_tokens: 0,
        completion_tokens: ct,
        total_tokens: ct,
        prompt_tokens_details: None,
    });

    ChatCompletionChunk {
        id: String::new(),
        object: "chat.completion.chunk".to_owned(),
        created: crate::provider::unix_timestamp_secs(),
        model: String::new(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: None,
                content: None,
                tool_calls: None,
                function_call: None,
                refusal: None,
                reasoning_content: None,
            },
            finish_reason: finish,
        }],
        usage,
        system_fingerprint: None,
        service_tier: None,
    }
}

/// Build a `ChatCompletionChunk` with a text content delta.
fn make_text_chunk(id: &str, model: &str, text: &str) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: id.to_owned(),
        object: "chat.completion.chunk".to_owned(),
        created: crate::provider::unix_timestamp_secs(),
        model: model.to_owned(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: None,
                content: Some(text.to_owned()),
                tool_calls: None,
                function_call: None,
                refusal: None,
                reasoning_content: None,
            },
            finish_reason: None,
        }],
        usage: None,
        system_fingerprint: None,
        service_tier: None,
    }
}

/// Build a `ChatCompletionChunk` with a reasoning/thinking content delta.
fn make_reasoning_chunk(id: &str, model: &str, text: &str) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: id.to_owned(),
        object: "chat.completion.chunk".to_owned(),
        created: crate::provider::unix_timestamp_secs(),
        model: model.to_owned(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: None,
                content: None,
                tool_calls: None,
                function_call: None,
                refusal: None,
                reasoning_content: Some(text.to_owned()),
            },
            finish_reason: None,
        }],
        usage: None,
        system_fingerprint: None,
        service_tier: None,
    }
}

/// Build a `ChatCompletionChunk` that starts a tool call (id + name, no arguments yet).
fn make_empty_chunk_with_tool_start(tool_index: u32, tool_id: String, tool_name: String) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: String::new(),
        object: "chat.completion.chunk".to_owned(),
        created: crate::provider::unix_timestamp_secs(),
        model: String::new(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: None,
                content: None,
                tool_calls: Some(vec![StreamToolCall {
                    index: tool_index,
                    id: Some(tool_id),
                    call_type: Some(crate::types::ToolType::Function),
                    function: Some(StreamFunctionCall {
                        name: Some(tool_name),
                        arguments: None,
                    }),
                }]),
                function_call: None,
                refusal: None,
                reasoning_content: None,
            },
            finish_reason: None,
        }],
        usage: None,
        system_fingerprint: None,
        service_tier: None,
    }
}

/// Build a `ChatCompletionChunk` that carries a partial tool arguments JSON delta.
fn make_tool_arguments_delta(tool_index: u32, partial_json: &str) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: String::new(),
        object: "chat.completion.chunk".to_owned(),
        created: crate::provider::unix_timestamp_secs(),
        model: String::new(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: None,
                content: None,
                tool_calls: Some(vec![StreamToolCall {
                    index: tool_index,
                    id: None,
                    call_type: None,
                    function: Some(StreamFunctionCall {
                        name: None,
                        arguments: Some(partial_json.to_owned()),
                    }),
                }]),
                function_call: None,
                refusal: None,
                reasoning_content: None,
            },
            finish_reason: None,
        }],
        usage: None,
        system_fingerprint: None,
        service_tier: None,
    }
}
