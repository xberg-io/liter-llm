//! Coverage for cache control, extended thinking, reasoning effort, beta headers,
//! document blocks, response formats and hosted tools.

use serde_json::json;

use super::*;

use super::tests::provider;

#[test]
fn chat_completions_path_is_messages() {
    assert_eq!(provider().chat_completions_path(), "/messages");
}

#[test]
fn transform_request_empty_messages_returns_error() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": []
    });
    let result = provider().transform_request(&mut body);
    assert!(result.is_err(), "empty messages should return an error");
}

#[test]
fn transform_request_sanitizes_tool_call_id() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "What is the weather?"},
            {"role": "assistant", "content": null, "tool_calls": [{
                "id": "call_abc.123",
                "type": "function",
                "function": {"name": "get_weather", "arguments": "{}"}
            }]},
            {"role": "tool", "tool_call_id": "call_abc.123", "content": "Sunny"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    let tool_result_msg = messages
        .iter()
        .find(|m| m["role"] == "user" && m["content"][0]["type"] == "tool_result")
        .expect("tool_result message should be present");
    assert_eq!(tool_result_msg["content"][0]["tool_use_id"], "call_abc_123");
}

#[test]
fn transform_request_merges_consecutive_user_messages() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "First"},
            {"role": "user", "content": "Second"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
    let content = messages[0]["content"].as_array().expect("content should be an array");
    assert_eq!(content.len(), 2);
}

#[test]
fn transform_request_system_content_array_passed_through() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "system", "content": [
                {"type": "text", "text": "Block one"},
                {"type": "text", "text": "Block two"}
            ]},
            {"role": "user", "content": "Hello"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let system = body["system"].as_array().expect("system should be an array");
    assert_eq!(system.len(), 2);
    assert_eq!(system[0]["text"], "Block one");
}

#[test]
fn transform_request_system_cache_control_propagated() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "system", "content": "Cached instructions", "cache_control": {"type": "ephemeral"}},
            {"role": "user", "content": "Hi"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let system = body["system"].as_array().expect("system should be an array");
    assert_eq!(system[0]["cache_control"]["type"], "ephemeral");
}

#[test]
fn transform_request_user_content_cache_control_propagated() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Cached text", "cache_control": {"type": "ephemeral"}}
            ]
        }]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    let content = messages[0]["content"].as_array().expect("content should be an array");
    assert_eq!(content[0]["cache_control"]["type"], "ephemeral");
}

#[test]
fn transform_request_tool_input_schema_type_normalized() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tools": [{
            "type": "function",
            "function": {
                "name": "my_tool",
                "parameters": {"properties": {}}
            }
        }]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let tools = body["tools"].as_array().expect("tools should be an array");
    assert_eq!(tools[0]["input_schema"]["type"], "object");
}

#[test]
fn transform_request_max_completion_tokens_mapped() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "max_completion_tokens": 512
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["max_tokens"], json!(512u64));
    assert!(body.get("max_completion_tokens").is_none());
}

#[test]
fn transform_request_tool_result_content_array_preserved() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "Look"},
            {"role": "assistant", "content": null, "tool_calls": [{
                "id": "call_img",
                "type": "function",
                "function": {"name": "get_image", "arguments": "{}"}
            }]},
            {"role": "tool", "tool_call_id": "call_img", "content": [
                {"type": "text", "text": "Here is the image"},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc123"}}
            ]}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    let tool_result_msg = messages
        .iter()
        .find(|m| {
            m["role"] == "user"
                && m["content"]
                    .as_array()
                    .is_some_and(|c| c.first().is_some_and(|b| b["type"] == "tool_result"))
        })
        .expect("tool_result message with image should be present");
    let result_content = tool_result_msg["content"][0]["content"]
        .as_array()
        .expect("content should be an array");
    assert_eq!(result_content.len(), 2);
    assert_eq!(result_content[0]["type"], "text");
    assert_eq!(result_content[1]["type"], "image");
}

/// Asserts the exact serialized JSON shape of an Anthropic `tool_result` image
/// block, using the typed `Message`/`ToolMessage` API end to end (not a raw
/// JSON fixture) to prove the public Rust surface reaches the provider intact.
#[test]
fn transform_request_tool_result_image_part_maps_to_exact_anthropic_block() {
    use crate::types::{ContentPart, Message, ToolMessage, UserContent};

    let messages = vec![Message::Tool(ToolMessage {
        content: UserContent::Parts(vec![ContentPart::image_data_url("data:image/png;base64,abc123")]),
        tool_call_id: "call_img".into(),
        name: None,
    })];
    let mut body = serde_json::to_value(&messages).expect("messages must serialise");
    body = json!({"model": "claude-3-5-sonnet-20241022", "messages": body});

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    let messages = body["messages"].as_array().expect("messages should be an array");
    assert_eq!(
        messages[0],
        json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "call_img",
                "content": [{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": "abc123"
                    }
                }]
            }]
        })
    );
}

#[test]
fn transform_request_tool_result_document_part_degrades_to_text() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{
            "role": "tool",
            "tool_call_id": "call_doc",
            "content": [{"type": "document", "document": {
                "data": "JVBERi0xLjQ=",
                "media_type": "application/pdf"
            }}]
        }]
    });

    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");

    let messages = body["messages"].as_array().expect("messages should be an array");
    let result_content = messages[0]["content"][0]["content"]
        .as_array()
        .expect("content should be an array");
    assert_eq!(result_content.len(), 1);
    assert_eq!(result_content[0]["type"], "text");
    assert_eq!(result_content[0]["text"], "[unsupported content in tool result]");
}

#[test]
fn transform_response_thinking_block_excluded_from_content() {
    let mut body = json!({
        "id": "msg_think",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "thinking", "thinking": "Let me reason..."},
            {"type": "text", "text": "The answer is 42."}
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 10, "output_tokens": 20}
    });
    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .expect("content should be a string");
    assert!(
        !content.contains("Let me reason..."),
        "thinking blocks should be filtered out"
    );
    assert_eq!(content, "The answer is 42.");
    assert_eq!(body["choices"][0]["message"]["reasoning_content"], "Let me reason...");
}

#[test]
fn transform_response_multiple_thinking_blocks_are_concatenated_in_order() {
    let mut body = json!({
        "id": "msg_think_multi",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "thinking", "thinking": "First, "},
            {"type": "text", "text": "Answer."},
            {"type": "thinking", "thinking": "then more."}
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 10, "output_tokens": 20}
    });
    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");
    let message = &body["choices"][0]["message"];
    assert_eq!(message["content"], "Answer.", "text blocks stay in content, in order");
    assert_eq!(
        message["reasoning_content"], "First, then more.",
        "thinking blocks are concatenated in document order"
    );
}

#[test]
fn transform_response_thinking_only_yields_null_content() {
    let mut body = json!({
        "id": "msg_think_only",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "thinking", "thinking": "Still working on it..."}
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "max_tokens",
        "usage": {"input_tokens": 10, "output_tokens": 20}
    });
    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");
    let message = &body["choices"][0]["message"];
    assert!(
        message["content"].is_null(),
        "a thinking-only response must expose null content, not an empty string, got: {}",
        message["content"]
    );
    assert_eq!(message["reasoning_content"], "Still working on it...");
}

#[test]
fn transform_response_server_tool_use_treated_as_tool_call() {
    let mut body = json!({
        "id": "msg_srv",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "server_tool_use",
            "id": "srvtool_01",
            "name": "web_search",
            "input": {"query": "Rust programming"}
        }],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 5, "output_tokens": 5}
    });
    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");
    let tool_calls = body["choices"][0]["message"]["tool_calls"]
        .as_array()
        .expect("tool_calls should be an array");
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["id"], "srvtool_01");
    assert_eq!(tool_calls[0]["function"]["name"], "web_search");
}

#[test]
fn transform_response_cache_tokens_counted_in_prompt() {
    let mut body = json!({
        "id": "msg_cache",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "ok"}],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 100,
            "cache_creation_input_tokens": 50,
            "cache_read_input_tokens": 25,
            "output_tokens": 10
        }
    });
    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");
    assert_eq!(body["usage"]["prompt_tokens"], 175u64);
    assert_eq!(body["usage"]["completion_tokens"], 10u64);
    assert_eq!(body["usage"]["total_tokens"], 185u64);
}

#[test]
fn transform_response_tool_only_no_empty_text_block_in_request() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "Call a tool"},
            {"role": "assistant", "tool_calls": [{
                "id": "call_xyz",
                "type": "function",
                "function": {"name": "my_fn", "arguments": "{}"}
            }]},
            {"role": "tool", "tool_call_id": "call_xyz", "content": "result"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    let assistant_msg = messages
        .iter()
        .find(|m| m["role"] == "assistant")
        .expect("assistant message should be present");
    let blocks = assistant_msg["content"].as_array().expect("content should be an array");
    assert!(blocks.iter().all(|b| b["type"] != "text" || b["text"] != ""));
    assert!(blocks.iter().any(|b| b["type"] == "tool_use"));
}

#[test]
fn parse_stream_event_thinking_delta_routes_to_reasoning_content() {
    let event =
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"I am thinking..."}}"#;
    let result = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .expect("thinking_delta should produce a chunk");
    let delta = &result.choices[0].delta;
    assert_eq!(delta.reasoning_content.as_deref(), Some("I am thinking..."));
    assert_eq!(delta.content, None, "thinking text must not leak into `content`");
}

#[test]
fn parse_stream_event_signature_delta_returns_none() {
    let event = r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"abc123"}}"#;
    let result = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail");
    assert!(
        result.is_none(),
        "signature_delta carries no visible text and should be ignored"
    );
}

#[test]
fn parse_stream_event_full_thinking_block_sequence_routes_text_to_reasoning_content() {
    let provider = provider();

    let start = r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#;
    let start_result = provider
        .parse_stream_event(start)
        .expect("parse_stream_event should not fail");
    assert!(
        start_result.is_none(),
        "thinking content_block_start should emit no chunk"
    );

    let delta_one =
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me "}}"#;
    let chunk_one = provider
        .parse_stream_event(delta_one)
        .expect("parse_stream_event should not fail")
        .expect("thinking_delta should produce a chunk");
    assert_eq!(chunk_one.choices[0].delta.reasoning_content.as_deref(), Some("Let me "));
    assert_eq!(chunk_one.choices[0].delta.content, None);

    let delta_two =
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"reason."}}"#;
    let chunk_two = provider
        .parse_stream_event(delta_two)
        .expect("parse_stream_event should not fail")
        .expect("thinking_delta should produce a chunk");
    assert_eq!(chunk_two.choices[0].delta.reasoning_content.as_deref(), Some("reason."));
    assert_eq!(chunk_two.choices[0].delta.content, None);

    let signature = r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig"}}"#;
    assert!(
        provider
            .parse_stream_event(signature)
            .expect("parse_stream_event should not fail")
            .is_none()
    );

    let stop = r#"{"type":"content_block_stop","index":0}"#;
    assert!(
        provider
            .parse_stream_event(stop)
            .expect("parse_stream_event should not fail")
            .is_none()
    );

    let concatenated = format!(
        "{}{}",
        chunk_one.choices[0].delta.reasoning_content.as_deref().unwrap_or(""),
        chunk_two.choices[0].delta.reasoning_content.as_deref().unwrap_or("")
    );
    assert_eq!(concatenated, "Let me reason.");
}

#[test]
fn parse_stream_event_message_start_cache_tokens_in_usage() {
    let event = r#"{"type":"message_start","message":{"id":"msg_x","model":"claude-opus","content":[],"usage":{"input_tokens":100,"cache_creation_input_tokens":50,"cache_read_input_tokens":25,"output_tokens":0}}}"#;
    let chunk = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .expect("expected chunk");
    let usage = chunk.usage.expect("usage should be present");
    assert_eq!(usage.prompt_tokens, 175);
}

#[test]
fn sanitize_tool_call_id_replaces_invalid_chars() {
    assert_eq!(sanitize_tool_call_id("call.abc!123").as_ref(), "call_abc_123");
    assert_eq!(sanitize_tool_call_id("call-abc_123").as_ref(), "call-abc_123");
    assert_eq!(sanitize_tool_call_id("call abc").as_ref(), "call_abc");
    assert!(matches!(sanitize_tool_call_id("toolu_01abc"), Cow::Borrowed(_)));
    assert!(matches!(sanitize_tool_call_id("call.123"), Cow::Owned(_)));
}

#[test]
fn map_stop_reason_content_filter() {
    assert_eq!(map_stop_reason("content_filtered"), "content_filter");
    assert_eq!(map_stop_reason("refusal"), "content_filter");
}

#[test]
fn transform_request_reasoning_effort_low() {
    let mut body = json!({
        "model": "claude-sonnet-4-20250514",
        "messages": [{"role": "user", "content": "Think about this"}],
        "reasoning_effort": "low"
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["thinking"]["type"], "enabled");
    assert_eq!(body["thinking"]["budget_tokens"], 1024);
    assert!(
        body.get("reasoning_effort").is_none(),
        "reasoning_effort should be removed"
    );
}

#[test]
fn transform_request_reasoning_effort_medium() {
    let mut body = json!({
        "model": "claude-sonnet-4-20250514",
        "messages": [{"role": "user", "content": "Think about this"}],
        "reasoning_effort": "medium"
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["thinking"]["type"], "enabled");
    assert_eq!(body["thinking"]["budget_tokens"], 4096);
}

#[test]
fn transform_request_reasoning_effort_high() {
    let mut body = json!({
        "model": "claude-sonnet-4-20250514",
        "messages": [{"role": "user", "content": "Think deeply"}],
        "reasoning_effort": "high"
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["thinking"]["type"], "enabled");
    assert_eq!(body["thinking"]["budget_tokens"], 16384);
}

#[test]
fn transform_request_reasoning_effort_minimal_maps_to_1024_budget_tokens() {
    let mut body = json!({
        "model": "claude-sonnet-4-20250514",
        "messages": [{"role": "user", "content": "Quick answer"}],
        "reasoning_effort": "minimal"
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["thinking"]["type"], "enabled");
    assert_eq!(body["thinking"]["budget_tokens"], 1024);
}

#[test]
fn transform_request_reasoning_effort_max_maps_to_32768_budget_tokens() {
    let mut body = json!({
        "model": "claude-sonnet-4-20250514",
        "messages": [{"role": "user", "content": "Think as hard as possible"}],
        "reasoning_effort": "max"
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["thinking"]["type"], "enabled");
    assert_eq!(body["thinking"]["budget_tokens"], 32768);
}

#[test]
fn transform_request_reasoning_effort_from_extra_body() {
    let mut body = json!({
        "model": "claude-sonnet-4-20250514",
        "messages": [{"role": "user", "content": "Think"}],
        "extra_body": {"reasoning_effort": "high"}
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert_eq!(body["thinking"]["type"], "enabled");
    assert_eq!(body["thinking"]["budget_tokens"], 16384);
}

#[test]
fn dynamic_headers_thinking_beta() {
    let body = json!({
        "thinking": {"type": "enabled", "budget_tokens": 4096},
        "messages": [{"role": "user", "content": "Hi"}]
    });
    let headers = provider().dynamic_headers(&body);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].0, "anthropic-beta");
    assert!(headers[0].1.contains("thinking-2025-04-14"));
}

#[test]
fn dynamic_headers_web_search_beta() {
    let body = json!({
        "tools": [{"type": "web_search_20250305", "name": "web_search"}],
        "messages": [{"role": "user", "content": "Search for Rust"}]
    });
    let headers = provider().dynamic_headers(&body);
    assert_eq!(headers.len(), 1);
    assert!(headers[0].1.contains("web-search-2025-03-05"));
}

#[test]
fn dynamic_headers_multiple_betas_combined() {
    let body = json!({
        "thinking": {"type": "enabled", "budget_tokens": 4096},
        "tools": [
            {"type": "computer_use_20250124", "display_width_px": 1024, "display_height_px": 768},
            {"type": "web_search_20250305", "name": "web_search"}
        ]
    });
    let headers = provider().dynamic_headers(&body);
    assert_eq!(headers.len(), 1);
    let beta_value = &headers[0].1;
    assert!(beta_value.contains("thinking-2025-04-14"));
    assert!(beta_value.contains("computer-use-2025-01-24"));
    assert!(beta_value.contains("web-search-2025-03-05"));
}

#[test]
fn dynamic_headers_no_betas_returns_empty() {
    let body = json!({
        "messages": [{"role": "user", "content": "Hi"}]
    });
    let headers = provider().dynamic_headers(&body);
    assert!(headers.is_empty());
}

#[test]
fn dynamic_headers_code_execution_beta() {
    let body = json!({
        "tools": [{"type": "code_execution_20250522"}]
    });
    let headers = provider().dynamic_headers(&body);
    assert_eq!(headers.len(), 1);
    assert!(headers[0].1.contains("code-execution-2025-05-22"));
}

#[test]
fn transform_request_tool_cache_control_propagated() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Hi"}],
        "tools": [{
            "type": "function",
            "cache_control": {"type": "ephemeral"},
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {"type": "object", "properties": {}}
            }
        }]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let tools = body["tools"].as_array().expect("tools should be an array");
    assert_eq!(tools[0]["cache_control"]["type"], "ephemeral");
}

#[test]
fn transform_request_assistant_message_cache_control() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "user", "content": "Hi"},
            {"role": "assistant", "content": "Hello!", "cache_control": {"type": "ephemeral"}},
            {"role": "user", "content": "How are you?"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    let assistant_msg = messages
        .iter()
        .find(|m| m["role"] == "assistant")
        .expect("assistant message should be present");
    let content = assistant_msg["content"].as_array().expect("content should be an array");
    assert_eq!(content[0]["cache_control"]["type"], "ephemeral");
}

#[test]
fn transform_request_user_message_level_cache_control() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{
            "role": "user",
            "content": "Hello",
            "cache_control": {"type": "ephemeral"}
        }]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    let content = messages[0]["content"].as_array().expect("content should be an array");
    assert_eq!(content[0]["cache_control"]["type"], "ephemeral");
}

#[test]
fn transform_request_document_content_part() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Analyze this document"},
                {"type": "document", "document": {
                    "data": "JVBERi0xLjQ=",
                    "media_type": "application/pdf"
                }}
            ]
        }]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    let content = messages[0]["content"].as_array().expect("content should be an array");
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[1]["type"], "document");
    assert_eq!(content[1]["source"]["type"], "base64");
    assert_eq!(content[1]["source"]["media_type"], "application/pdf");
    assert_eq!(content[1]["source"]["data"], "JVBERi0xLjQ=");
}

#[test]
fn transform_request_document_with_cache_control() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{
            "role": "user",
            "content": [
                {"type": "document", "document": {
                    "data": "JVBERi0xLjQ=",
                    "media_type": "application/pdf"
                }, "cache_control": {"type": "ephemeral"}}
            ]
        }]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let messages = body["messages"].as_array().expect("messages should be an array");
    let content = messages[0]["content"].as_array().expect("content should be an array");
    assert_eq!(content[0]["cache_control"]["type"], "ephemeral");
}

#[test]
fn transform_request_json_object_response_format() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Give me JSON"}],
        "response_format": {"type": "json_object"}
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert!(body.get("response_format").is_none());
    let system = body["system"].as_array().expect("system should be an array");
    assert!(
        system[0]["text"]
            .as_str()
            .expect("text should be a string")
            .contains("valid JSON")
    );
}

#[test]
fn transform_request_json_schema_response_format() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Give me structured output"}],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "person",
                "schema": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer"}
                    }
                }
            }
        }
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    assert!(body.get("response_format").is_none());
    let system = body["system"].as_array().expect("system should be an array");
    let instruction = system[0]["text"].as_str().expect("text should be a string");
    assert!(instruction.contains("person"));
    assert!(instruction.contains("schema"));
}

#[test]
fn transform_request_json_object_with_existing_system() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "Give me JSON"}
        ],
        "response_format": {"type": "json_object"}
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let system = body["system"].as_array().expect("system should be an array");
    assert_eq!(system.len(), 2);
    assert!(
        system[0]["text"]
            .as_str()
            .expect("text should be a string")
            .contains("valid JSON")
    );
    assert_eq!(system[1]["text"], "You are helpful.");
}

#[test]
fn transform_request_hosted_tool_passed_through() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Search the web"}],
        "tools": [
            {"type": "web_search_20250305", "name": "web_search", "max_uses": 3},
            {"type": "function", "function": {
                "name": "get_weather",
                "parameters": {"type": "object", "properties": {}}
            }}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let tools = body["tools"].as_array().expect("tools should be an array");
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["type"], "web_search_20250305");
    assert_eq!(tools[0]["max_uses"], 3);
    assert_eq!(tools[1]["name"], "get_weather");
    assert!(tools[1].get("input_schema").is_some());
}

#[test]
fn transform_request_computer_use_tool_passed_through() {
    let mut body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [{"role": "user", "content": "Use the computer"}],
        "tools": [{
            "type": "computer_20241022",
            "display_width_px": 1024,
            "display_height_px": 768
        }]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    let tools = body["tools"].as_array().expect("tools should be an array");
    assert_eq!(tools[0]["type"], "computer_20241022");
    assert_eq!(tools[0]["display_width_px"], 1024);
}

#[test]
fn transform_response_citation_blocks_skipped() {
    let mut body = json!({
        "id": "msg_cite",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "text", "text": "According to the document, "},
            {"type": "citation", "cited_text": "Rust is fast", "document_index": 0},
            {"type": "text", "text": "Rust is a fast language."}
        ],
        "model": "claude-3-5-sonnet-20241022",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 50, "output_tokens": 20}
    });
    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .expect("content should be a string");
    assert_eq!(content, "According to the document, Rust is a fast language.");
    assert!(!content.contains("citation"));
}

#[test]
fn is_hosted_tool_type_recognizes_all_types() {
    assert!(is_hosted_tool_type("computer_20241022"));
    assert!(is_hosted_tool_type("computer_use_20250124"));
    assert!(is_hosted_tool_type("web_search_20250305"));
    assert!(is_hosted_tool_type("code_execution_20250522"));
    assert!(!is_hosted_tool_type("function"));
    assert!(!is_hosted_tool_type("custom_tool"));
}
