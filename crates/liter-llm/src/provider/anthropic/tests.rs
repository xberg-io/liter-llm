use serde_json::json;

use super::*;
use crate::types::FinishReason;

pub(super) fn provider() -> AnthropicProvider {
    AnthropicProvider::default()
}

#[test]
fn new_and_default_use_official_anthropic_base_url() {
    assert_eq!(AnthropicProvider::new().base_url(), "https://api.anthropic.com/v1");
    assert_eq!(AnthropicProvider::default().base_url(), "https://api.anthropic.com/v1");
}

#[test]
fn with_base_url_trims_trailing_slash() {
    let p = AnthropicProvider::with_base_url("https://proxy.internal/anthropic/");
    assert_eq!(p.base_url(), "https://proxy.internal/anthropic");
}

#[test]
fn with_base_url_falls_back_to_official_url_when_empty() {
    let p = AnthropicProvider::with_base_url("");
    assert_eq!(p.base_url(), "https://api.anthropic.com/v1");
}

#[test]
fn transform_request_extracts_system_message() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello!"}
        ]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(
        body["system"],
        json!([{"type": "text", "text": "You are a helpful assistant."}])
    );

    let messages = body["messages"].as_array().expect("messages should be an array");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
}

#[test]
fn transform_request_multiple_system_messages_merged() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "system", "content": "First instruction."},
            {"role": "system", "content": "Second instruction."},
            {"role": "user", "content": "Question"}
        ]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    let system = body["system"].as_array().expect("system should be an array");
    assert_eq!(system.len(), 2);
    assert_eq!(system[0]["text"], "First instruction.");
    assert_eq!(system[1]["text"], "Second instruction.");
}

#[test]
fn transform_request_defaults_max_tokens() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    let expected = crate::cost::model_info("claude-3-5-sonnet-20241022")
        .and_then(|info| info.max_output_tokens)
        .unwrap_or(DEFAULT_MAX_TOKENS);
    assert_eq!(body["max_tokens"], json!(expected));
}

#[test]
fn transform_request_uses_catalog_output_limit_by_default() {
    let mut body = json!({
        "model": "claude-opus-5",
        "messages": [{"role": "user", "content": "Hi"}]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["max_tokens"], json!(128_000));
}

#[test]
fn transform_request_preserves_explicit_max_tokens() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 1024
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["max_tokens"], json!(1024u64));
}

#[test]
fn transform_request_converts_stop_string_to_array() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "stop": "\n"
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["stop_sequences"], json!(["\n"]));
    assert!(body.get("stop").is_none(), "old `stop` key should be removed");
}

#[test]
fn transform_request_stop_array_passes_through() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "stop": ["STOP", "END"]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["stop_sequences"], json!(["STOP", "END"]));
    assert!(body.get("stop").is_none());
}

#[test]
fn transform_request_tool_choice_required_maps_to_any() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tool_choice": "required",
        "tools": [{"type": "function", "function": {"name": "f", "parameters": {}}}]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["tool_choice"], json!({"type": "any"}));
}

#[test]
fn transform_request_tool_choice_none_keeps_tools_and_sends_none() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tool_choice": "none",
        "tools": [{"type": "function", "function": {"name": "f", "parameters": {}}}]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["tool_choice"], json!({"type": "none"}));
    assert_eq!(body["tools"].as_array().map(Vec::len), Some(1), "tools must stay");
    assert_eq!(body["tools"][0]["name"], "f");
}

/// Collect every `text` block with an empty `text` value, at any depth of `messages`.
fn empty_text_blocks(value: &serde_json::Value, found: &mut usize) {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(|t| t.as_str()) == Some("text")
                && map.get("text").and_then(|t| t.as_str()) == Some("")
            {
                *found += 1;
            }
            map.values().for_each(|v| empty_text_blocks(v, found));
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| empty_text_blocks(v, found)),
        _ => {}
    }
}

fn transformed_messages(messages: serde_json::Value) -> Vec<serde_json::Value> {
    let mut body = json!({"model": "claude-3-5-sonnet-20241022", "messages": messages});
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let out = body["messages"].clone();
    let mut found = 0;
    empty_text_blocks(&out, &mut found);
    assert_eq!(found, 0, "empty text block on the wire: {out}");
    out.as_array().expect("messages should be an array").clone()
}

#[test]
fn transform_request_empty_user_string_is_dropped_not_sent_as_empty_text() {
    let out = transformed_messages(json!([
        {"role": "user", "content": ""},
        {"role": "user", "content": "real question"}
    ]));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["content"], json!([{"type": "text", "text": "real question"}]));
}

#[test]
fn transform_request_empty_user_text_part_is_skipped() {
    let out = transformed_messages(json!([
        {"role": "user", "content": [
            {"type": "text", "text": ""},
            {"type": "text", "text": "kept"}
        ]}
    ]));
    assert_eq!(out[0]["content"], json!([{"type": "text", "text": "kept"}]));
}

#[test]
fn transform_request_empty_assistant_turn_is_dropped() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "one"},
        {"role": "assistant", "content": ""},
        {"role": "user", "content": "two"}
    ]));
    assert_eq!(out.len(), 1, "user turns around the empty assistant turn merge");
    assert_eq!(out[0]["role"], "user");
    assert_eq!(out[0]["content"].as_array().map(Vec::len), Some(2));
}

#[test]
fn transform_request_empty_tool_and_function_results_carry_no_empty_text() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "go"},
        {"role": "assistant", "tool_calls": [{
            "id": "call_1", "type": "function",
            "function": {"name": "f", "arguments": "{}"}
        }]},
        {"role": "tool", "tool_call_id": "call_1", "content": ""},
        {"role": "function", "name": "g", "content": ""}
    ]));
    let results: Vec<_> = out
        .iter()
        .flat_map(|m| m["content"].as_array().cloned().unwrap_or_default())
        .filter(|b| b["type"] == "tool_result")
        .collect();
    assert_eq!(results.len(), 2, "both results stay, with their ids");
    assert!(
        results.iter().all(|r| r.get("content").is_none()),
        "an empty result omits content, which Anthropic documents as optional"
    );
}

#[test]
fn transform_request_tool_choice_none_without_tools_is_omitted() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tool_choice": "none"
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert!(body.get("tool_choice").is_none(), "no tools, so no tool_choice");
    assert!(body.get("tools").is_none());
}

#[test]
fn transform_request_tool_choice_none_with_empty_tools_is_omitted() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tools": [],
        "tool_choice": "none"
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert!(body.get("tool_choice").is_none(), "empty tools, so no tool_choice");
    assert!(body.get("tools").is_none());
}

#[test]
fn transform_request_tool_choice_auto_without_tools_is_unchanged() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tool_choice": "auto"
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["tool_choice"], json!({"type": "auto"}));
}

#[test]
fn transform_request_assistant_parts_become_text_blocks() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "a"},
        {"role": "assistant", "content": [
            {"type": "text", "text": "first"},
            {"type": "text", "text": ""},
            {"type": "text", "text": "  \n"},
            {"type": "text", "text": "second", "cache_control": {"type": "ephemeral"}}
        ]},
        {"role": "user", "content": "b"}
    ]));
    assert_eq!(out.len(), 3, "the assistant turn is kept");
    assert_eq!(out[1]["role"], "assistant");
    assert_eq!(
        out[1]["content"],
        json!([
            {"type": "text", "text": "first"},
            {"type": "text", "text": "second", "cache_control": {"type": "ephemeral"}}
        ])
    );
}

#[test]
fn transform_request_assistant_parts_with_only_blank_text_are_dropped() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "a"},
        {"role": "assistant", "content": [{"type": "text", "text": " "}]},
        {"role": "user", "content": "b"}
    ]));
    assert_eq!(out.len(), 1);
}

#[test]
fn transform_request_assistant_parts_keep_tool_calls() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "a"},
        {"role": "assistant",
         "content": [{"type": "text", "text": "calling"}],
         "tool_calls": [{
            "id": "call_1", "type": "function",
            "function": {"name": "f", "arguments": "{}"}
         }]}
    ]));
    let blocks = out[1]["content"].as_array().expect("blocks");
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["type"], "text");
    assert_eq!(blocks[1]["type"], "tool_use");
}

#[test]
fn transform_request_whitespace_only_text_counts_as_empty() {
    let out = transformed_messages(json!([
        {"role": "system", "content": "  "},
        {"role": "user", "content": " \t\n"},
        {"role": "user", "content": [{"type": "text", "text": "  "}, {"type": "text", "text": "kept"}]},
        {"role": "assistant", "content": "   "},
        {"role": "user", "content": "end"}
    ]));
    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0]["content"],
        json!([{"type": "text", "text": "kept"}, {"type": "text", "text": "end"}])
    );
}

#[test]
fn transform_request_whitespace_system_string_is_not_sent() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "system", "content": "  "}, {"role": "user", "content": "hi"}]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert!(body.get("system").is_none());
}

#[test]
fn transform_request_blank_system_parts_are_skipped() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "system", "content": [
                {"type": "text", "text": ""},
                {"type": "text", "text": "  "},
                {"type": "text", "text": "kept"}
            ]},
            {"role": "user", "content": "hi"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["system"], json!([{"type": "text", "text": "kept"}]));
}

#[test]
fn transform_request_only_blank_system_parts_omit_system() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "system", "content": [{"type": "text", "text": ""}]},
            {"role": "developer", "content": [{"type": "text", "text": "  "}]},
            {"role": "user", "content": "hi"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert!(body.get("system").is_none());
}

#[test]
fn transform_request_whitespace_tool_result_omits_content() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "go"},
        {"role": "assistant", "tool_calls": [{
            "id": "call_1", "type": "function",
            "function": {"name": "f", "arguments": "{}"}
        }]},
        {"role": "tool", "tool_call_id": "call_1", "content": "  "}
    ]));
    let result = &out[2]["content"][0];
    assert_eq!(result["type"], "tool_result");
    assert!(result.get("content").is_none());
}

#[test]
fn transform_request_unknown_role_empty_string_adds_no_text_block() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "go"},
        {"role": "mystery", "content": "a"},
        {"role": "mystery", "content": ""},
        {"role": "mystery", "content": "  "}
    ]));
    assert_eq!(out[1]["content"], json!([{"type": "text", "text": "a"}]));
}

#[test]
fn transform_request_unknown_role_empty_first_then_text_keeps_only_text() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "go"},
        {"role": "mystery", "content": ""},
        {"role": "mystery", "content": "b"}
    ]));
    assert_eq!(out[1]["content"], json!([{"type": "text", "text": "b"}]));
}

#[test]
fn transform_request_tool_result_document_part_degrades_to_text() {
    let out = transformed_messages(json!([
        {"role": "user", "content": "go"},
        {"role": "assistant", "tool_calls": [{
            "id": "call_1", "type": "function",
            "function": {"name": "f", "arguments": "{}"}
        }]},
        {"role": "tool", "tool_call_id": "call_1", "content": [
            {"type": "document", "document": {"data": "AAAA"}}
        ]}
    ]));
    assert_eq!(
        out[2]["content"][0]["content"],
        json!([{"type": "text", "text": "[unsupported content in tool result]"}])
    );
}

#[test]
fn transform_request_only_empty_messages_is_a_local_error() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": ""}]
    });
    let result = provider().transform_request(&mut body);
    assert!(result.is_err(), "nothing is left to send, so the call fails locally");
}

#[test]
fn transform_request_tool_choice_specific_function() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tool_choice": {"type": "function", "function": {"name": "my_tool"}},
        "tools": [{"type": "function", "function": {"name": "my_tool", "parameters": {}}}]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["tool_choice"], json!({"type": "tool", "name": "my_tool"}));
}

#[test]
fn transform_request_converts_tools_to_anthropic_format() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather",
                "parameters": {"type": "object", "properties": {}}
            }
        }]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    let tools = body["tools"].as_array().expect("tools should be an array");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "get_weather");
    assert_eq!(tools[0]["description"], "Get current weather");
    assert!(tools[0].get("input_schema").is_some());
    assert!(tools[0].get("function").is_none());
}

/// Revert line: delete the
/// `super::validate_sampling_param_range(body, "temperature", "Anthropic", 0.0, 1.0)?;`
/// call at the top of `transform_request` to make this test fail (the request
/// would then be transformed successfully instead of rejected).
#[test]
fn transform_request_rejects_temperature_above_anthropic_maximum() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "temperature": 1.8
    });

    let err = provider()
        .transform_request(&mut body)
        .expect_err("temperature above Anthropic's 1.0 maximum should be rejected");

    assert_eq!(err.status_code(), 400);
    let message = err.to_string();
    assert!(
        message.contains("temperature=1.8"),
        "error message should name the offending value: {message}"
    );
    assert!(
        message.contains("Anthropic"),
        "error message should name the provider: {message}"
    );
}

#[test]
fn transform_request_accepts_temperature_at_anthropic_maximum() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "temperature": 1.0
    });

    provider()
        .transform_request(&mut body)
        .expect("temperature exactly at Anthropic's 1.0 maximum should be accepted");
    assert_eq!(body["temperature"], 1.0);
}

#[test]
fn transform_request_removes_unsupported_fields() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "n": 2,
        "presence_penalty": 0.5,
        "frequency_penalty": 0.3,
        "logit_bias": {"1234": 5},
        "stream": true
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    for key in &["n", "presence_penalty", "frequency_penalty", "logit_bias"] {
        assert!(body.get(key).is_none(), "`{key}` should be removed");
    }
    assert_eq!(body["stream"], true);
}

/// Regression test: `transform_request` mutates `body` in place rather than rebuilding it
/// wholesale (unlike vertex.rs/bedrock.rs), so `logprobs`, `top_logprobs`, `store`,
/// `metadata`, `prediction`, `audio`, `web_search_options`, `modalities` and `seed` used to
/// leak onto the Anthropic wire verbatim instead of being dropped or mapped. None of them
/// have an Anthropic equivalent, so they must be stripped like the other unsupported fields
/// above. Anthropic's Messages API validates the body strictly and rejects *any*
/// unrecognized top-level key with a 400, so an unstripped field breaks the whole request,
/// not just that one parameter.
#[test]
fn transform_request_strips_unmappable_openai_only_fields() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "logprobs": true,
        "top_logprobs": 5,
        "store": true,
        "metadata": {"run": "nightly"},
        "prediction": {"type": "content", "content": "draft"},
        "audio": {"voice": "alloy", "format": "wav"},
        "web_search_options": {"search_context_size": "medium"},
        "modalities": ["text", "audio"],
        "seed": 42
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    for key in &[
        "logprobs",
        "top_logprobs",
        "store",
        "metadata",
        "prediction",
        "audio",
        "web_search_options",
        "modalities",
        "seed",
    ] {
        assert!(body.get(key).is_none(), "`{key}` must not be forwarded to Anthropic");
    }
}

/// `modalities: ["text"]` is a no-op for Anthropic, but the field still must not reach the
/// wire: Anthropic's Messages API rejects any unrecognized top-level key with a 400, so
/// even a no-op value would fail the entire request if left in the body.
#[test]
fn transform_request_strips_text_only_modalities() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "modalities": ["text"]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert!(
        body.get("modalities").is_none(),
        "`modalities` must not be forwarded to Anthropic"
    );
}

/// `modalities: ["audio"]` asks for output Claude cannot produce; the request must still
/// succeed (text is returned instead) rather than leaking the field and getting a 400.
#[test]
fn transform_request_strips_audio_only_modalities() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "modalities": ["audio"]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    assert!(
        body.get("modalities").is_none(),
        "`modalities` must not be forwarded to Anthropic"
    );
}

#[test]
fn transform_request_converts_tool_message_to_tool_result() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "What is the weather?"},
            {"role": "assistant", "content": null, "tool_calls": [{
                "id": "call_abc",
                "type": "function",
                "function": {"name": "get_weather", "arguments": "{\"location\": \"London\"}"}
            }]},
            {"role": "tool", "tool_call_id": "call_abc", "content": "15°C, sunny"}
        ]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    let messages = body["messages"].as_array().expect("messages should be an array");
    let tool_result_msg = &messages[2];
    assert_eq!(tool_result_msg["role"], "user");
    let content = tool_result_msg["content"]
        .as_array()
        .expect("content should be an array");
    assert_eq!(content[0]["type"], "tool_result");
    assert_eq!(content[0]["tool_use_id"], "call_abc");
}

#[test]
fn transform_request_converts_user_content_parts() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "What is in this image?"},
                {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/abc=="}}
            ]
        }]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    let messages = body["messages"].as_array().expect("messages should be an array");
    let content = messages[0]["content"].as_array().expect("content should be an array");
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[1]["type"], "image");
    assert_eq!(content[1]["source"]["type"], "base64");
    assert_eq!(content[1]["source"]["media_type"], "image/jpeg");
}

#[test]
fn transform_response_basic_text() {
    let mut body = json!({
        "id": "msg_01Xfn7",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "Hello, world!"}],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 10, "output_tokens": 5}
    });

    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["id"], "msg_01Xfn7");
    let choice = &body["choices"][0];
    assert_eq!(choice["message"]["content"], "Hello, world!");
    assert_eq!(choice["finish_reason"], "stop");
    assert_eq!(body["usage"]["prompt_tokens"], 10);
    assert_eq!(body["usage"]["completion_tokens"], 5);
    assert_eq!(body["usage"]["total_tokens"], 15);
}

#[test]
fn transform_response_stop_reason_max_tokens_maps_to_length() {
    let mut body = json!({
        "id": "msg_abc",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "truncated"}],
        "model": "claude-3-haiku-20240307",
        "stop_reason": "max_tokens",
        "usage": {"input_tokens": 5, "output_tokens": 50}
    });

    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["choices"][0]["finish_reason"], "length");
}

#[test]
fn transform_response_tool_use_block() {
    let mut body = json!({
        "id": "msg_tool",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "tool_use",
            "id": "toolu_01abc",
            "name": "get_weather",
            "input": {"location": "London"}
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 20, "output_tokens": 10}
    });

    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");

    let choice = &body["choices"][0];
    assert_eq!(choice["finish_reason"], "tool_calls");
    assert_eq!(choice["message"]["content"], Value::Null);

    let tool_calls = choice["message"]["tool_calls"]
        .as_array()
        .expect("tool_calls should be an array");
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["id"], "toolu_01abc");
    assert_eq!(tool_calls[0]["function"]["name"], "get_weather");

    let args_str = tool_calls[0]["function"]["arguments"]
        .as_str()
        .expect("arguments should be a string");
    let args: Value = serde_json::from_str(args_str).expect("arguments should be valid JSON");
    assert_eq!(args["location"], "London");
}

#[test]
fn transform_response_is_noop_for_openai_format() {
    let original = json!({
        "id": "chatcmpl-xxx",
        "object": "chat.completion",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"}, "finish_reason": "stop"}]
    });
    let mut body = original.clone();

    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body, original);
}

#[test]
fn parse_stream_event_done_is_handled_at_sse_level() {
    let result = provider().parse_stream_event("[DONE]");
    assert!(
        result.is_err(),
        "[DONE] is not valid JSON and should error if it reaches the provider"
    );
}

#[test]
fn parse_stream_event_message_stop_returns_none() {
    let event = r#"{"type":"message_stop"}"#;
    let result = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail");
    assert!(result.is_none());
}

#[test]
fn parse_stream_event_text_delta() {
    let event = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
    let chunk = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .expect("expected chunk");
    assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
}

#[test]
fn parse_stream_event_message_delta_with_finish_reason() {
    let event = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":12}}"#;
    let chunk = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .expect("expected chunk");
    assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Stop));
    let usage = chunk.usage.expect("usage should be present");
    assert_eq!(usage.completion_tokens, 12);
}

#[test]
fn parse_stream_event_message_delta_tool_use_stop_reason() {
    let event = r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":5}}"#;
    let chunk = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .expect("expected chunk");
    assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::ToolCalls));
}

#[test]
fn parse_stream_event_message_start() {
    let event = r#"{"type":"message_start","message":{"id":"msg_abc","type":"message","role":"assistant","content":[],"model":"claude-3-5-sonnet-20241022","stop_reason":null,"usage":{"input_tokens":25,"output_tokens":1}}}"#;
    let chunk = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .expect("expected chunk");
    assert_eq!(chunk.id, "msg_abc");
    assert_eq!(chunk.model, "claude-3-5-sonnet-20241022");
    assert_eq!(chunk.choices[0].delta.role.as_deref(), Some("assistant"));
    let usage = chunk.usage.expect("usage should be present");
    assert_eq!(usage.prompt_tokens, 25);
}

#[test]
fn parse_stream_event_input_json_delta() {
    let event =
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"loc"}}"#;
    let chunk = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .expect("expected chunk");
    let tc = &chunk.choices[0]
        .delta
        .tool_calls
        .as_ref()
        .expect("tool_calls should be present")[0];
    assert_eq!(
        tc.function
            .as_ref()
            .expect("function should be present")
            .arguments
            .as_deref(),
        Some("{\"loc")
    );
}

#[test]
fn parse_stream_event_error_returns_err() {
    let event = r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
    let result = provider().parse_stream_event(event);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Overloaded"));
}

/// Revert line: make the `error` arm of `parse_stream_event` in `anthropic/stream.rs`
/// build `LiterLlmError::Streaming` again to make this test fail.
#[test]
fn parse_stream_event_error_event_has_the_http_error_kind() {
    use std::time::Duration;
    let cases = [
        (r#""type":"overloaded_error""#, 529),
        (r#""type":"rate_limit_error""#, 429),
        (r#""type":"authentication_error""#, 401),
        (r#""type":"permission_error""#, 403),
        (r#""type":"not_found_error""#, 404),
        (r#""type":"api_error""#, 500),
    ];
    for (kind, status) in cases {
        let event = format!(r#"{{"type":"error","error":{{{kind},"message":"boom"}}}}"#);
        let err = provider().parse_stream_event(&event).unwrap_err();
        let http = crate::error::LiterLlmError::from_status(status, r#"{"error":{"message":"boom"}}"#, None);
        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(&http),
            "{kind}: stream gave {err:?}, HTTP {status} gives {http:?}"
        );
        assert_eq!(err.status_code(), status, "{kind}");
        assert!(err.to_string().contains("boom"), "{kind}: {err}");
    }
    let event = r#"{"type":"error","error":{"type":"rate_limit_error","message":"x","retry_after":9}}"#;
    let err = provider().parse_stream_event(event).unwrap_err();
    assert_eq!(err.retry_after(), Some(Duration::from_secs(9)));
}

/// An `error` event with no error object has no type to classify; it stays a
/// generic streaming error with a fixed message.
#[test]
fn parse_stream_event_error_event_without_error_object_is_a_streaming_error() {
    let err = provider().parse_stream_event(r#"{"type":"error"}"#).unwrap_err();
    assert!(
        matches!(err, crate::error::LiterLlmError::Streaming { .. }),
        "got: {err:?}"
    );
    assert!(
        err.to_string().contains("unknown Anthropic streaming error"),
        "got: {err}"
    );
}

#[test]
fn parse_stream_event_ping_returns_none() {
    let event = r#"{"type":"ping"}"#;
    let result = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail");
    assert!(result.is_none(), "ping should return Ok(None), not a chunk");
}

#[test]
fn parse_stream_event_content_block_stop_returns_none() {
    let event = r#"{"type":"content_block_stop","index":0}"#;
    let result = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail");
    assert!(result.is_none(), "content_block_stop should return Ok(None)");
}
