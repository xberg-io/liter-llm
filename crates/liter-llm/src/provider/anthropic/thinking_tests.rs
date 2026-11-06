//! Thinking blocks survive a round trip: a response keeps them, with their signatures and
//! positions, in `thinking_blocks`, and the next request sends them back unchanged (#474).

use serde_json::{Value, json};

use super::*;
use crate::types::{AssistantMessage, ThinkingBlock};

use super::tests::provider;

fn thinking_turn() -> Value {
    json!([
        {"type": "thinking", "thinking": "Need the weather.", "signature": "sig-1"},
        {"type": "redacted_thinking", "data": "opaque-1"},
        {"type": "text", "text": "Checking."},
        {"type": "tool_use", "id": "toolu_01", "name": "get_weather", "input": {"city": "Paris"}},
        {"type": "thinking", "thinking": "", "signature": "sig-2"},
        {"type": "tool_use", "id": "toolu_02", "name": "get_weather", "input": {"city": "Rome"}}
    ])
}

fn transformed_response(content: Value) -> Value {
    let mut body = json!({
        "id": "msg_1",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-5-5",
        "content": content,
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 10, "output_tokens": 20}
    });
    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");
    body["choices"][0]["message"].clone()
}

fn transformed_request(assistant: Value) -> Value {
    let mut body = json!({
        "model": "claude-haiku-5-5",
        "messages": [
            {"role": "user", "content": "Weather in Paris and Rome?"},
            assistant,
            {"role": "tool", "tool_call_id": "toolu_01", "content": "18C"},
            {"role": "tool", "tool_call_id": "toolu_02", "content": "21C"}
        ]
    });
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    body["messages"][1].clone()
}

#[test]
fn response_keeps_thinking_blocks_with_signature_and_position() {
    let message = transformed_response(thinking_turn());
    assert_eq!(
        message["thinking_blocks"],
        json!([
            {"type": "thinking", "thinking": "Need the weather.", "signature": "sig-1", "index": 0},
            {"type": "redacted_thinking", "data": "opaque-1", "index": 1},
            {"type": "thinking", "thinking": "", "signature": "sig-2", "index": 4}
        ])
    );
    assert_eq!(message["reasoning_content"], "Need the weather.");
    assert_eq!(message["content"], "Checking.");
}

#[test]
fn response_without_thinking_has_no_thinking_blocks() {
    let message = transformed_response(json!([{"type": "text", "text": "Hi"}]));
    assert!(message.get("thinking_blocks").is_none(), "got: {message}");
}

#[test]
fn a_replayed_thinking_turn_is_the_turn_anthropic_sent() {
    let message = transformed_response(thinking_turn());
    let typed: AssistantMessage = serde_json::from_value(message).expect("assistant message");
    let mut replayed = serde_json::to_value(typed).expect("serialize");
    replayed["role"] = json!("assistant");

    let sent = transformed_request(replayed);
    assert_eq!(sent, json!({"role": "assistant", "content": thinking_turn()}));
}

#[test]
fn thinking_blocks_without_index_go_before_the_text_and_tool_calls() {
    let sent = transformed_request(json!({
        "role": "assistant",
        "content": "Checking.",
        "thinking_blocks": [
            {"type": "thinking", "thinking": "Need the weather.", "signature": "sig-1"},
            {"type": "redacted_thinking", "data": "opaque-1"}
        ],
        "tool_calls": [{
            "id": "toolu_01",
            "type": "function",
            "function": {"name": "get_weather", "arguments": "{\"city\":\"Paris\"}"}
        }]
    }));
    assert_eq!(
        sent["content"],
        json!([
            {"type": "thinking", "thinking": "Need the weather.", "signature": "sig-1"},
            {"type": "redacted_thinking", "data": "opaque-1"},
            {"type": "text", "text": "Checking."},
            {"type": "tool_use", "id": "toolu_01", "name": "get_weather", "input": {"city": "Paris"}}
        ])
    );
}

fn stream_blocks(event: &str) -> Option<Vec<ThinkingBlock>> {
    provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .and_then(|chunk| chunk.choices[0].delta.thinking_blocks.clone())
}

fn fragment(value: Value) -> Option<Vec<ThinkingBlock>> {
    Some(vec![serde_json::from_value(value).expect("thinking block")])
}

#[test]
fn stream_emits_thinking_block_fragments_keyed_by_block_index() {
    assert_eq!(
        stream_blocks(
            r#"{"type":"content_block_start","index":2,"content_block":{"type":"thinking","thinking":"","signature":""}}"#
        ),
        fragment(json!({"index": 2, "type": "thinking", "thinking": "", "signature": ""}))
    );
    assert_eq!(
        stream_blocks(r#"{"type":"content_block_delta","index":2,"delta":{"type":"thinking_delta","thinking":"Hmm"}}"#),
        fragment(json!({"index": 2, "thinking": "Hmm"}))
    );
    assert_eq!(
        stream_blocks(
            r#"{"type":"content_block_delta","index":2,"delta":{"type":"signature_delta","signature":"sig"}}"#
        ),
        fragment(json!({"index": 2, "signature": "sig"}))
    );
    assert_eq!(
        stream_blocks(
            r#"{"type":"content_block_start","index":3,"content_block":{"type":"redacted_thinking","data":"opaque"}}"#
        ),
        fragment(json!({"index": 3, "type": "redacted_thinking", "data": "opaque"}))
    );
}

#[test]
fn stream_thinking_delta_still_fills_reasoning_content() {
    let chunk = provider()
        .parse_stream_event(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Hmm"}}"#,
        )
        .expect("parse_stream_event should not fail")
        .expect("chunk");
    assert_eq!(chunk.choices[0].delta.reasoning_content.as_deref(), Some("Hmm"));
}

#[test]
fn stream_text_has_no_thinking_blocks() {
    assert_eq!(
        stream_blocks(r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#),
        None
    );
}
