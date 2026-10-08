use std::borrow::Cow;

use serde_json::Value;

use crate::error::{LiterLlmError, Result};
use crate::provider::{Provider, unix_timestamp_secs};
use crate::types::{
    ChatCompletionChunk, FinishReason, StreamChoice, StreamDelta, StreamFunctionCall, StreamToolCall, Usage,
};

/// Cohere provider (Command model family).
///
/// Differences from the OpenAI-compatible baseline:
/// - Chat endpoint is `/chat` instead of `/chat/completions`.
/// - Rerank endpoint is `/rerank` instead of the default path.
/// - `stream_options` is an OpenAI-specific field and must be stripped; `stream` is kept (Cohere v2 requires it).
/// - The non-streaming `/chat` response has **no `choices` wrapper**: `id`, `finish_reason`,
///   `message`, and `usage` are top-level fields (verified against Cohere's v2 API reference).
///   `finish_reason` uses Cohere-specific names (`COMPLETE`, `MAX_TOKENS`, `TOOL_CALL`).
///   `message.content` is an array of typed blocks (`text`, `thinking`), not a flat string.
///   `usage` is reported as `{billed_units: {...}, tokens: {...}}`, not OpenAI's flat
///   `prompt_tokens` / `completion_tokens`. `transform_response` rebuilds the whole body
///   into the OpenAI `choices` shape from these fields (#53).
pub struct CohereProvider;

impl Provider for CohereProvider {
    fn name(&self) -> &str {
        "cohere"
    }

    fn base_url(&self) -> &str {
        // ~keep `api.cohere.ai` is a working alias for the same API (both hosts route to the
        // ~keep same backend and return 401 rather than a DNS/redirect failure when probed
        // ~keep unauthenticated), but `api.cohere.com` is what Cohere's own API reference uses
        // ~keep for the v2 `/chat` endpoint, so it is the canonical choice here. schemas/
        // ~keep providers.json's `cohere`/`cohere_chat` entries must agree with this — do not
        // ~keep "fix" this back to `.ai`.
        "https://api.cohere.com/v2"
    }

    fn env_var(&self) -> Option<&str> {
        Some("COHERE_API_KEY")
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}"))))
    }

    fn matches_model(&self, model: &str) -> bool {
        model.starts_with("command-r") || model.starts_with("command-") || model.starts_with("cohere/")
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        model.strip_prefix("cohere/").unwrap_or(model)
    }

    /// Cohere uses `/chat` instead of `/chat/completions`.
    fn chat_completions_path(&self) -> &str {
        "/chat"
    }

    /// Cohere uses `/rerank` at the v2 base.
    fn rerank_path(&self) -> &str {
        "/rerank"
    }

    /// Strip transport-level parameters that Cohere does not accept in the body, and
    /// rename `top_p` to Cohere's own nucleus-sampling field name.
    ///
    /// Note: Cohere v2 requires `stream` in the body, so only `stream_options`
    /// (an OpenAI-specific field) is removed.
    ///
    /// `temperature` is NOT range-checked here, unlike Anthropic and Bedrock. Cohere's
    /// API reference documents `temperature` as "a non-negative float" with no stated
    /// maximum, so there is no upper bound to enforce — the "0-1" figure that appears
    /// in Cohere's conceptual guides is guidance, not a schema constraint, and
    /// rejecting on it would fail requests Cohere accepts.
    ///
    /// `top_p` IS range-checked and renamed. Cohere's v2 `/chat` endpoint has no
    /// `top_p` field at all; its nucleus-sampling parameter is named `p` (default
    /// `0.75`, min `0.01`, max `0.99`, verified against Cohere's API reference) — so
    /// forwarding `top_p` verbatim meant the value was silently dropped, never
    /// reaching Cohere as a recognised field at any value the caller chose. The range
    /// check runs against the caller-facing `top_p` name *before* the rename, so a
    /// rejected request's error names the field the caller actually set (`top_p`)
    /// rather than Cohere's internal name (`p`), which the caller may never have
    /// written. Note the minimum is `0.01`, not `0.0`: `top_p: 0.0` is legal for
    /// OpenAI (and documented as legal by this crate's own `ChatCompletionRequest`)
    /// but must be rejected here rather than silently coerced to `0.01`.
    fn transform_request(&self, body: &mut Value) -> Result<()> {
        super::validate_sampling_param_range(body, "top_p", "Cohere", 0.01, 0.99)?;

        if let Some(obj) = body.as_object_mut() {
            obj.remove("stream_options");
            if let Some(top_p) = obj.remove("top_p") {
                obj.insert("p".to_owned(), top_p);
            }
        }
        Ok(())
    }

    /// Parse a Cohere v2 streaming SSE event into a `ChatCompletionChunk`.
    ///
    /// Cohere v2 streaming events use a `type` field to distinguish event kinds
    /// (verified against Cohere's v2 API reference, `docs.cohere.com/reference/chat-stream`
    /// and `docs.cohere.com/v2/docs/streaming`):
    /// - `message-start`: beginning of stream; id is top-level, role at `delta.message.role`
    /// - `content-delta`: text content token, extract from `delta.message.content.text`
    /// - `tool-call-start`: start of a tool call; id/name at `delta.message.tool_calls.{id,function.name}`
    /// - `tool-call-delta`: partial tool call arguments at `delta.message.tool_calls.function.arguments`
    /// - `tool-call-end`: end of a tool call (skipped)
    /// - `message-end`: end of stream; finish reason at `delta.finish_reason`, usage at
    ///   `delta.usage.billed_units.{input,output}_tokens`
    ///
    /// ~keep The previous implementation matched Cohere's legacy v1 NDJSON event names
    /// (`stream-start`/`stream-end`) and flat field paths (`delta.text`, `delta.id`,
    /// `delta.function.name`) against this v2 SSE endpoint. Every real v2 event silently
    /// missed: `content-delta` always yielded empty text, tool-call id/name/arguments were
    /// always empty, and `finish_reason`/`usage` never surfaced because the event-type
    /// strings never matched — streaming opened without error but delivered nothing.
    fn parse_stream_event(&self, event_data: &str) -> Result<Option<ChatCompletionChunk>> {
        let v: Value = serde_json::from_str(event_data).map_err(|e| LiterLlmError::Streaming {
            message: format!("failed to parse Cohere SSE event: {e}"),
        })?;

        let event_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        Ok(match event_type {
            "message-start" => Some(cohere_message_start_chunk(&v)),
            "content-delta" => Some(cohere_content_delta_chunk(&v)),
            "tool-call-start" => Some(cohere_tool_call_start_chunk(&v)),
            "tool-call-delta" => Some(cohere_tool_call_delta_chunk(&v)),
            "message-end" => Some(cohere_message_end_chunk(&v)),
            _ => None,
        })
    }

    /// Normalize a Cohere v2 `/chat` response to OpenAI chat completion format.
    ///
    /// Cohere's response has no `choices` wrapper: `finish_reason` and `message`
    /// are top-level fields, `message.content` is an array of typed blocks
    /// (`text`, `thinking`, ...) rather than a flat string, `message.tool_calls`
    /// already matches the OpenAI `{id, type, function: {name, arguments}}` shape
    /// verbatim, and usage is reported under `usage.billed_units.{input,output}_tokens`
    /// (not OpenAI's flat `prompt_tokens` / `completion_tokens`). This rebuilds the
    /// whole body into the shape `ChatCompletionResponse` expects (#53): the
    /// previous implementation assumed the response was already `choices`-wrapped,
    /// which meant every real non-streaming Cohere response failed
    /// deserialization with a missing `choices` field.
    fn transform_response(&self, body: &mut Value) -> Result<()> {
        use serde_json::json;

        let id = body.get("id").cloned().unwrap_or_else(|| Value::String(String::new()));

        let finish_reason = normalize_cohere_response_finish_reason(body);

        let content_blocks = body.pointer("/message/content").and_then(Value::as_array).cloned();

        let text: String = content_blocks
            .as_ref()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        // ~keep Cohere "thinking" blocks are internal reasoning, not visible content;
        // route them to `reasoning_content` the same way Anthropic/Gemini thinking is handled.
        let reasoning_text: Option<String> = content_blocks.as_ref().and_then(|blocks| {
            let joined = blocks
                .iter()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("thinking"))
                .filter_map(|b| b.get("thinking").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("");
            if joined.is_empty() { None } else { Some(joined) }
        });

        let tool_calls = body.pointer("/message/tool_calls").and_then(Value::as_array).cloned();
        let has_tool_calls = tool_calls.as_ref().is_some_and(|tc| !tc.is_empty());

        let message_content = if text.is_empty() {
            Value::Null
        } else {
            Value::String(text)
        };

        let mut message = json!({"role": "assistant", "content": message_content});
        if let (Some(tc), true) = (tool_calls, has_tool_calls) {
            message["tool_calls"] = Value::Array(tc);
        }
        if let Some(reasoning) = reasoning_text {
            message["reasoning_content"] = Value::String(reasoning);
        }

        let input_tokens = body
            .pointer("/usage/billed_units/input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let output_tokens = body
            .pointer("/usage/billed_units/output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);

        *body = json!({
            "id": id,
            "object": "chat.completion",
            "created": unix_timestamp_secs(),
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
}

fn cohere_message_start_chunk(event: &Value) -> ChatCompletionChunk {
    let id = event.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
    let role = event
        .pointer("/delta/message/role")
        .and_then(Value::as_str)
        .unwrap_or("assistant")
        .to_owned();
    let delta = StreamDelta {
        role: Some(role),
        ..Default::default()
    };
    cohere_stream_chunk(id, delta, None, None)
}

fn cohere_content_delta_chunk(event: &Value) -> ChatCompletionChunk {
    let text = event
        .pointer("/delta/message/content/text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let delta = StreamDelta {
        content: Some(text),
        ..Default::default()
    };
    cohere_stream_chunk(String::new(), delta, None, None)
}

fn cohere_tool_call_start_chunk(event: &Value) -> ChatCompletionChunk {
    let index = event.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
    let tool_id = event
        .pointer("/delta/message/tool_calls/id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let tool_name = event
        .pointer("/delta/message/tool_calls/function/name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let delta = StreamDelta {
        tool_calls: Some(vec![StreamToolCall {
            index,
            id: Some(tool_id),
            call_type: Some(crate::types::ToolType::Function),
            function: Some(StreamFunctionCall {
                name: Some(tool_name),
                arguments: None,
            }),
        }]),
        ..Default::default()
    };
    cohere_stream_chunk(String::new(), delta, None, None)
}

fn cohere_tool_call_delta_chunk(event: &Value) -> ChatCompletionChunk {
    let index = event.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
    let arguments = event
        .pointer("/delta/message/tool_calls/function/arguments")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let delta = StreamDelta {
        tool_calls: Some(vec![StreamToolCall {
            index,
            id: None,
            call_type: None,
            function: Some(StreamFunctionCall {
                name: None,
                arguments: Some(arguments),
            }),
        }]),
        ..Default::default()
    };
    cohere_stream_chunk(String::new(), delta, None, None)
}

fn cohere_message_end_chunk(event: &Value) -> ChatCompletionChunk {
    let finish_reason = event
        .pointer("/delta/finish_reason")
        .and_then(Value::as_str)
        .map(map_cohere_finish_reason);
    cohere_stream_chunk(
        String::new(),
        StreamDelta::default(),
        finish_reason,
        extract_cohere_stream_usage(event),
    )
}

fn normalize_cohere_response_finish_reason(body: &Value) -> &str {
    match body.get("finish_reason").and_then(Value::as_str).unwrap_or("COMPLETE") {
        "COMPLETE" => "stop",
        "MAX_TOKENS" => "length",
        "TOOL_CALL" => "tool_calls",
        other => other,
    }
}

fn cohere_stream_chunk(
    id: String,
    delta: StreamDelta,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id,
        object: "chat.completion.chunk".to_owned(),
        created: unix_timestamp_secs(),
        model: String::new(),
        choices: vec![StreamChoice {
            index: 0,
            delta,
            finish_reason,
        }],
        usage,
        system_fingerprint: None,
        service_tier: None,
    }
}

/// Map Cohere finish reason strings to OpenAI-compatible `FinishReason`.
fn map_cohere_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "COMPLETE" => FinishReason::Stop,
        "MAX_TOKENS" => FinishReason::Length,
        "TOOL_CALL" => FinishReason::ToolCalls,
        _ => FinishReason::Other,
    }
}

/// Extract usage from a Cohere `message-end` event.
///
/// Cohere v2 reports usage under `delta.usage.billed_units.{input_tokens, output_tokens}`.
fn extract_cohere_stream_usage(v: &Value) -> Option<crate::types::Usage> {
    let billed = v.pointer("/delta/usage/billed_units")?;
    let input = billed.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0);
    let output = billed.get("output_tokens").and_then(|t| t.as_u64()).unwrap_or(0);

    Some(crate::types::Usage {
        prompt_tokens: input,
        completion_tokens: output,
        total_tokens: input + output,
        prompt_tokens_details: None,
        completion_tokens_details: None,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_cohere_name_and_base_url() {
        let provider = CohereProvider;
        assert_eq!(provider.name(), "cohere");
        assert_eq!(provider.base_url(), "https://api.cohere.com/v2");
    }

    /// Regression test: schemas/providers.json's `cohere_chat` entry hardcoded
    /// `https://api.cohere.ai/v2` while this file hardcoded `https://api.cohere.com/v2` — both
    /// hosts work (Cohere serves the same v2 API from either), but a silent disagreement
    /// between the registry and the code means the registry can no longer be trusted as
    /// documentation. Both now agree on `.com`; this test would have failed before that fix.
    #[test]
    fn base_url_agrees_with_the_providers_json_registry() {
        let configs = crate::provider::all_providers().expect("embedded providers.json must parse");
        let entry = configs
            .iter()
            .find(|c| c.name == "cohere_chat")
            .expect("providers.json must have a `cohere_chat` entry");

        assert_eq!(
            entry.base_url.as_deref(),
            Some(CohereProvider.base_url()),
            "schemas/providers.json's `cohere_chat` base_url must match CohereProvider::base_url()"
        );
    }

    #[test]
    fn test_cohere_auth_header() {
        let provider = CohereProvider;
        let (name, value) = provider.auth_header("test-key").expect("should return auth header");
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer test-key");
    }

    #[test]
    fn test_cohere_matches_model() {
        let provider = CohereProvider;
        assert!(provider.matches_model("command-r-plus"));
        assert!(provider.matches_model("command-r"));
        assert!(provider.matches_model("command-light"));
        assert!(provider.matches_model("cohere/command-r-plus"));
        assert!(!provider.matches_model("gpt-4"));
        assert!(!provider.matches_model("claude-3"));
    }

    #[test]
    fn test_cohere_strip_prefix() {
        let provider = CohereProvider;
        assert_eq!(provider.strip_model_prefix("cohere/command-r"), "command-r");
        assert_eq!(provider.strip_model_prefix("command-r"), "command-r");
    }

    #[test]
    fn test_cohere_endpoints() {
        let provider = CohereProvider;
        assert_eq!(provider.chat_completions_path(), "/chat");
        assert_eq!(provider.rerank_path(), "/rerank");
    }

    #[test]
    fn test_cohere_transform_request_preserves_stream_strips_options() {
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true,
            "stream_options": {"include_usage": true}
        });
        provider.transform_request(&mut body).expect("transform should succeed");
        assert_eq!(body["stream"], true);
        assert!(body.get("stream_options").is_none());
        assert_eq!(body["model"], "command-r-plus");
    }

    // ~keep Cohere's v2 `/chat` endpoint has no `top_p` field; its nucleus-sampling
    // parameter is named `p` (min 0.01, max 0.99), verified against
    // docs.cohere.com/reference/chat. Forwarding `top_p` verbatim meant the value was
    // silently dropped, never reaching Cohere as a recognised field at any value.

    #[test]
    fn transform_request_renames_top_p_to_cohere_p() {
        // Delete the `obj.insert("p".to_owned(), top_p);` line (or the whole `if let
        // Some(top_p) = ...` rename block) in `CohereProvider::transform_request` to
        // make this fail: `p` would then be absent from the transformed body.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "top_p": 0.5
        });
        provider.transform_request(&mut body).expect("transform should succeed");
        assert_eq!(body["p"], 0.5);
    }

    #[test]
    fn transform_request_removes_top_p_field_name() {
        // Delete `obj.remove("top_p")` (replacing it with a non-removing read) in
        // `CohereProvider::transform_request` to make this fail: `top_p` would still
        // be present in the transformed body alongside `p`.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "top_p": 0.5
        });
        provider.transform_request(&mut body).expect("transform should succeed");
        assert!(body.get("top_p").is_none());
    }

    #[test]
    fn transform_request_rejects_top_p_zero_point_zero() {
        // Delete the `super::validate_sampling_param_range(body, "top_p", "Cohere",
        // 0.01, 0.99)?;` line in `CohereProvider::transform_request` to make this
        // fail: 0.0 is legal for OpenAI (and this crate's own `ChatCompletionRequest`
        // docs) but is below Cohere's documented minimum of 0.01 and must be rejected,
        // not silently coerced.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "top_p": 0.0
        });
        let err = provider
            .transform_request(&mut body)
            .expect_err("0.0 is below Cohere's minimum p of 0.01");
        assert_eq!(err.status_code(), 400);
        assert_eq!(
            err.to_string(),
            "bad request: top_p=0 is outside Cohere's supported range [0.01, 0.99]; lower \
             the requested value or omit `top_p` to use the provider default"
        );
    }

    #[test]
    fn transform_request_accepts_top_p_zero_point_nine_nine() {
        // Change the `max` argument passed to `validate_sampling_param_range` from
        // `0.99` to something smaller (e.g. `0.9`) in `CohereProvider::transform_request`
        // to make this fail: 0.99 is Cohere's documented maximum and must be accepted.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "top_p": 0.99
        });
        provider
            .transform_request(&mut body)
            .expect("0.99 is within Cohere's range");
        assert_eq!(body["p"], 0.99);
        assert!(body.get("top_p").is_none());
    }

    #[test]
    fn transform_request_without_top_p_is_untouched() {
        // Change the rename block to unconditionally insert a `p` field (dropping the
        // `if let Some(top_p) = obj.remove("top_p")` guard) in
        // `CohereProvider::transform_request` to make this fail: a body with no
        // `top_p` would gain a spurious `p` field.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}]
        });
        provider.transform_request(&mut body).expect("transform should succeed");
        assert!(body.get("p").is_none());
        assert!(body.get("top_p").is_none());
        assert_eq!(body["model"], "command-r-plus");
    }

    /// Build a Cohere v2 `/chat` response body in the real shape (no `choices`
    /// wrapper), per Cohere's own API reference — see #53.
    fn real_cohere_response(finish_reason: &str, content: serde_json::Value) -> serde_json::Value {
        json!({
            "id": "resp-abc123",
            "finish_reason": finish_reason,
            "message": {
                "role": "assistant",
                "content": content
            },
            "usage": {
                "billed_units": {"input_tokens": 10, "output_tokens": 20},
                "tokens": {"input_tokens": 12, "output_tokens": 22}
            }
        })
    }

    #[test]
    fn test_cohere_transform_response_wraps_top_level_fields_into_choices() {
        // ~keep Regression test for #53: Cohere's real response has no `choices` array —
        // `finish_reason` and `message` are top-level. The old implementation assumed a
        // `choices`-wrapped input and was therefore a no-op on real responses, leaving
        // the body without the `choices` field `ChatCompletionResponse` requires.
        let provider = CohereProvider;
        let mut body = real_cohere_response("COMPLETE", json!([{"type": "text", "text": "hi"}]));

        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        assert_eq!(body["id"], "resp-abc123");
        assert_eq!(body["object"], "chat.completion");
        assert!(body["created"].as_u64().is_some());
        let choices = body["choices"].as_array().expect("choices array must be present");
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0]["index"], 0);
        assert_eq!(choices[0]["finish_reason"], "stop");
        assert_eq!(choices[0]["message"]["role"], "assistant");
        assert_eq!(choices[0]["message"]["content"], "hi");
    }

    #[test]
    fn test_cohere_transform_response_finish_reasons() {
        let provider = CohereProvider;
        for (raw, expected) in [
            ("COMPLETE", "stop"),
            ("MAX_TOKENS", "length"),
            ("TOOL_CALL", "tool_calls"),
        ] {
            let mut body = real_cohere_response(raw, json!([{"type": "text", "text": "hi"}]));
            provider
                .transform_response(&mut body)
                .expect("transform should succeed");
            assert_eq!(
                body["choices"][0]["finish_reason"], expected,
                "mapping {raw} -> {expected}"
            );
        }
    }

    #[test]
    fn test_cohere_transform_response_concatenates_text_blocks() {
        let provider = CohereProvider;
        let mut body = real_cohere_response(
            "COMPLETE",
            json!([{"type": "text", "text": "Hello, "}, {"type": "text", "text": "world!"}]),
        );
        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        assert_eq!(body["choices"][0]["message"]["content"], "Hello, world!");
    }

    #[test]
    fn test_cohere_transform_response_thinking_block_routes_to_reasoning_content() {
        let provider = CohereProvider;
        let mut body = real_cohere_response(
            "COMPLETE",
            json!([{"type": "thinking", "thinking": "step 1..."}, {"type": "text", "text": "answer"}]),
        );
        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        assert_eq!(body["choices"][0]["message"]["content"], "answer");
        assert_eq!(body["choices"][0]["message"]["reasoning_content"], "step 1...");
    }

    #[test]
    fn test_cohere_transform_response_no_text_block_is_null_content() {
        let provider = CohereProvider;
        let mut body = real_cohere_response("TOOL_CALL", json!([]));
        body["message"]["tool_calls"] = json!([{
            "id": "call_1",
            "type": "function",
            "function": {"name": "get_weather", "arguments": "{\"city\":\"Berlin\"}"}
        }]);

        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        assert!(body["choices"][0]["message"]["content"].is_null());
        let tool_calls = body["choices"][0]["message"]["tool_calls"]
            .as_array()
            .expect("tool_calls array");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["id"], "call_1");
        assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
        assert_eq!(tool_calls[0]["function"]["arguments"], "{\"city\":\"Berlin\"}");
    }

    #[test]
    fn test_cohere_transform_response_output_deserializes_as_chat_completion_response() {
        // ~keep Regression test for #53: the whole point of `transform_response` is that
        // its output must deserialize as `ChatCompletionResponse`. Before the fix this
        // failed with "missing field `choices`" on every real (non-`choices`-wrapped)
        // Cohere response.
        let provider = CohereProvider;
        let mut body = real_cohere_response("COMPLETE", json!([{"type": "text", "text": "hi"}]));

        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        let parsed: crate::types::ChatCompletionResponse =
            serde_json::from_value(body).expect("transform_response output must deserialize as ChatCompletionResponse");
        assert_eq!(
            parsed.choices[0]
                .message
                .content
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("hi")
        );
    }

    #[test]
    fn test_cohere_transform_response_usage_uses_billed_units() {
        let provider = CohereProvider;
        let mut body = real_cohere_response("COMPLETE", json!([{"type": "text", "text": "hi"}]));

        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        let usage = &body["usage"];
        assert_eq!(usage["prompt_tokens"], 10);
        assert_eq!(usage["completion_tokens"], 20);
        assert_eq!(usage["total_tokens"], 30);
    }

    // ~keep Regression coverage for the streaming fix: the previous event shapes below
    // (`stream-start`/`stream-end` with flat `delta.text` / `delta.id` / `delta.function.*`
    // paths) encoded Cohere's legacy v1 NDJSON format, not the v2 SSE format this provider
    // actually targets (base_url is `.../v2`). Every one of those tests passed against a
    // shape the real v2 endpoint never sends, so they asserted the *broken* production
    // behaviour (content-delta always yielding empty text, tool-call fields always empty,
    // finish_reason/usage never surfacing) as if it were correct. Shapes below are quoted
    // verbatim from Cohere's v2 API reference (`docs.cohere.com/reference/chat-stream`).

    #[test]
    fn test_parse_stream_event_message_start() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"role":"assistant"}},"id":"gen-123","type":"message-start"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.id, "gen-123");
        assert_eq!(chunk.object, "chat.completion.chunk");
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.role.as_deref(), Some("assistant"));
        assert!(chunk.choices[0].delta.content.is_none());
        assert!(chunk.choices[0].finish_reason.is_none());
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn test_parse_stream_event_content_delta() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"content":{"text":"Hello"}}},"index":0,"type":"content-delta"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
        assert!(chunk.choices[0].delta.role.is_none());
        assert!(chunk.choices[0].delta.tool_calls.is_none());
    }

    #[test]
    fn test_parse_stream_event_content_delta_whitespace() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"content":{"text":" world"}}},"index":0,"type":"content-delta"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some(" world"));
    }

    #[test]
    fn test_parse_stream_event_tool_call_start() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"tool_calls":{"function":{"arguments":"","name":"get_weather"},"id":"tc-001","type":"function"}}},"index":0,"type":"tool-call-start"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        let tool_calls = chunk.choices[0]
            .delta
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].index, 0);
        assert_eq!(tool_calls[0].id.as_deref(), Some("tc-001"));
        let func = tool_calls[0].function.as_ref().expect("should have function");
        assert_eq!(func.name.as_deref(), Some("get_weather"));
        assert!(func.arguments.is_none());
    }

    #[test]
    fn test_parse_stream_event_tool_call_delta() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"tool_calls":{"function":{"arguments":"{\"ci"}}}},"index":0,"type":"tool-call-delta"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        let tool_calls = chunk.choices[0]
            .delta
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].index, 0);
        assert!(tool_calls[0].id.is_none());
        let func = tool_calls[0].function.as_ref().expect("should have function");
        assert!(func.name.is_none());
        assert_eq!(func.arguments.as_deref(), Some("{\"ci"));
    }

    #[test]
    fn test_parse_stream_event_tool_call_end_returns_none() {
        let provider = CohereProvider;
        let event = r#"{"index":0,"type":"tool-call-end"}"#;
        let result = provider.parse_stream_event(event).expect("should parse");

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_stream_event_message_end_complete() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"COMPLETE","usage":{"billed_units":{"input_tokens":10,"output_tokens":5}}},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Stop));
        let usage = chunk.usage.as_ref().expect("should have usage");
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn test_parse_stream_event_message_end_max_tokens() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"MAX_TOKENS","usage":{"billed_units":{"input_tokens":20,"output_tokens":100}}},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Length));
        let usage = chunk.usage.as_ref().expect("should have usage");
        assert_eq!(usage.prompt_tokens, 20);
        assert_eq!(usage.completion_tokens, 100);
        assert_eq!(usage.total_tokens, 120);
    }

    #[test]
    fn test_parse_stream_event_message_end_tool_call() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"TOOL_CALL","usage":{"billed_units":{"input_tokens":15,"output_tokens":8}}},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::ToolCalls));
    }

    #[test]
    fn test_parse_stream_event_message_end_no_usage() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"COMPLETE"},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Stop));
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn test_parse_stream_event_unknown_type_returns_none() {
        let provider = CohereProvider;
        let event = r#"{"type":"debug","message":"some debug info"}"#;
        let result = provider.parse_stream_event(event).expect("should parse");

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_stream_event_invalid_json_returns_err() {
        let provider = CohereProvider;
        let result = provider.parse_stream_event("not valid json");

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_stream_event_tool_call_start_index_1() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"tool_calls":{"function":{"arguments":"","name":"search"},"id":"tc-002","type":"function"}}},"index":1,"type":"tool-call-start"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        let tool_calls = chunk.choices[0]
            .delta
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls[0].index, 1);
        assert_eq!(tool_calls[0].id.as_deref(), Some("tc-002"));
    }

    #[test]
    fn test_parse_stream_event_message_end_unknown_finish_reason() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"ERROR"},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Other));
    }

    #[test]
    fn test_parse_stream_event_message_start_missing_role_defaults_to_assistant() {
        // ~keep Cohere's docs always show role="assistant" on message-start, but the parser
        // should not panic/error if a future event omits it — default rather than fail.
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{}},"id":"gen-456","type":"message-start"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].delta.role.as_deref(), Some("assistant"));
    }
}
