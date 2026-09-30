//! Gemini SSE event -> OpenAI `ChatCompletionChunk` translation.

use serde_json::Value;

use super::transform_gemini_response;
use crate::error::{LiterLlmError, Result};
use crate::types::{
    ChatCompletionChunk, FinishReason, StreamChoice, StreamDelta, StreamFunctionCall, StreamToolCall, ToolType,
};

/// Parse a single SSE event from Gemini's streaming endpoint.
///
/// Gemini streaming uses SSE with `?alt=sse`.  Each event data is a complete
/// `generateContent` JSON response.  We reuse `transform_gemini_response` to
/// normalize it into OpenAI format, then build a `ChatCompletionChunk` from
/// the first choice's message content.
///
/// **Note:** The `id` and `model` fields are empty strings on every chunk
/// because Gemini's streaming payloads do not include them, and this parser
/// is stateless.
pub(crate) fn parse_gemini_stream_event(event_data: &str) -> Result<Option<ChatCompletionChunk>> {
    // ~keep `[DONE]` is consumed by the SSE parser before provider parsing.
    if event_data.trim().is_empty() {
        return Ok(None);
    }

    let mut body: Value = serde_json::from_str(event_data).map_err(|e| LiterLlmError::Streaming {
        message: format!("failed to parse Gemini SSE data: {e}"),
    })?;

    transform_gemini_response(&mut body)?;

    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("gemini-resp")
        .to_owned();
    let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("").to_owned();

    let choice = body.pointer("/choices/0");
    let content = choice
        .and_then(|c| c.pointer("/message/content"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    // ~keep Thread `reasoning_content` through the streaming path too (#52); without
    // this it would only ever be populated on the non-streaming response.
    let reasoning_content = choice
        .and_then(|c| c.pointer("/message/reasoning_content"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let finish_reason_str = choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let stream_tool_calls = stream_tool_calls(choice);
    let finish_reason = stream_finish_reason(finish_reason_str);

    let chunk = ChatCompletionChunk {
        id,
        object: "chat.completion.chunk".to_owned(),
        created: crate::provider::unix_timestamp_secs(),
        model,
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: Some("assistant".to_owned()),
                content,
                tool_calls: stream_tool_calls,
                function_call: None,
                refusal: None,
                reasoning_content,
            },
            finish_reason,
        }],
        usage: None,
        system_fingerprint: None,
        service_tier: None,
    };

    Ok(Some(chunk))
}

/// Streaming tool-call deltas from the normalized choice's `tool_calls`, or
/// `None` when there are none.
fn stream_tool_calls(choice: Option<&Value>) -> Option<Vec<StreamToolCall>> {
    choice
        .and_then(|c| c.pointer("/message/tool_calls"))
        .and_then(|v| v.as_array())
        .filter(|arr| !arr.is_empty())
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(idx, tc)| StreamToolCall {
                    index: idx as u32,
                    id: tc.get("id").and_then(|v| v.as_str()).map(ToOwned::to_owned),
                    call_type: Some(ToolType::Function),
                    function: tc.get("function").map(|f| StreamFunctionCall {
                        name: f.get("name").and_then(|v| v.as_str()).map(ToOwned::to_owned),
                        arguments: f.get("arguments").and_then(|v| v.as_str()).map(ToOwned::to_owned),
                    }),
                })
                .collect::<Vec<_>>()
        })
}

/// Map the normalized OpenAI `finish_reason` string to [`FinishReason`].
fn stream_finish_reason(finish_reason_str: &str) -> Option<FinishReason> {
    match finish_reason_str {
        "stop" => Some(FinishReason::Stop),
        "length" => Some(FinishReason::Length),
        "tool_calls" => Some(FinishReason::ToolCalls),
        "content_filter" => Some(FinishReason::ContentFilter),
        _ => None,
    }
}
