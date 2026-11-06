//! Thinking blocks survive a round trip: a response keeps them, with their signatures and
//! positions, in `thinking_blocks`, and the next request sends them back unchanged (#474).

use serde_json::{Value, json};

use super::*;
use crate::error::LiterLlmError;
use crate::types::{AssistantMessage, ThinkingBlock, ThinkingBlockAccumulator};

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

fn request_body(assistant: Value) -> Value {
    json!({
        "model": "claude-haiku-5-5",
        "messages": [
            {"role": "user", "content": "Weather in Paris and Rome?"},
            assistant,
            {"role": "tool", "tool_call_id": "toolu_01", "content": "18C"},
            {"role": "tool", "tool_call_id": "toolu_02", "content": "21C"}
        ]
    })
}

fn transformed_request(assistant: Value) -> Value {
    let mut body = request_body(assistant);
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    body["messages"][1].clone()
}

/// The assistant turn the next request sends after a reply without streaming.
fn replayed(content: Value) -> Value {
    let message = transformed_response(content);
    let typed: AssistantMessage = serde_json::from_value(message).expect("assistant message");
    let mut assistant = serde_json::to_value(typed).expect("serialize");
    assistant["role"] = json!("assistant");
    transformed_request(assistant)
}

#[test]
fn response_keeps_thinking_blocks_with_signature_and_position() {
    let message = transformed_response(thinking_turn());
    assert_eq!(
        message["thinking_blocks"],
        json!([
            {"type": "thinking", "thinking": "Need the weather.", "signature": "sig-1",
             "index": 0, "text_offset": 0, "tool_call_offset": 0},
            {"type": "redacted_thinking", "data": "opaque-1", "index": 1, "text_offset": 0, "tool_call_offset": 0},
            {"type": "thinking", "thinking": "", "signature": "sig-2",
             "index": 4, "text_offset": 9, "tool_call_offset": 1}
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
    assert_eq!(
        replayed(thinking_turn()),
        json!({"role": "assistant", "content": thinking_turn()})
    );
}

#[test]
fn text_between_thinking_blocks_is_replayed_in_place() {
    let turn = json!([
        {"type": "thinking", "thinking": "First.", "signature": "sig-1"},
        {"type": "text", "text": "One."},
        {"type": "thinking", "thinking": "Second.", "signature": "sig-2"},
        {"type": "text", "text": "Two."},
        {"type": "tool_use", "id": "toolu_01", "name": "get_weather", "input": {"city": "Paris"}}
    ]);
    assert_eq!(replayed(turn.clone()), json!({"role": "assistant", "content": turn}));
}

#[test]
fn a_blank_text_block_does_not_move_the_thinking_blocks() {
    let turn = json!([
        {"type": "thinking", "thinking": "First.", "signature": "sig-1"},
        {"type": "text", "text": ""},
        {"type": "thinking", "thinking": "Second.", "signature": "sig-2"},
        {"type": "text", "text": "Done."},
        {"type": "tool_use", "id": "toolu_01", "name": "get_weather", "input": {"city": "Paris"}}
    ]);
    let mut expected = turn.clone();
    expected.as_array_mut().expect("array").remove(1);
    assert_eq!(replayed(turn), json!({"role": "assistant", "content": expected}));
}

#[test]
fn thinking_blocks_without_offsets_go_in_index_order() {
    let sent = transformed_request(json!({
        "role": "assistant",
        "content": "Checking.",
        "thinking_blocks": [
            {"type": "thinking", "thinking": "Later.", "signature": "sig-2", "index": 3},
            {"type": "thinking", "thinking": "First.", "signature": "sig-1", "index": 0}
        ],
        "tool_calls": [
            {"id": "toolu_01", "type": "function", "function": {"name": "get_weather", "arguments": "{}"}},
            {"id": "toolu_02", "type": "function", "function": {"name": "get_weather", "arguments": "{}"}}
        ]
    }));
    let types: Vec<&str> = sent["content"]
        .as_array()
        .expect("array")
        .iter()
        .map(|block| block["type"].as_str().expect("type"))
        .collect();
    assert_eq!(types, ["thinking", "text", "tool_use", "thinking", "tool_use"]);
    assert_eq!(sent["content"][0]["thinking"], "First.");
    assert_eq!(sent["content"][3]["thinking"], "Later.");
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

#[test]
fn an_invalid_thinking_block_is_a_bad_request_naming_the_field() {
    for (block, field) in [
        (json!({"thinking": "x", "signature": "s"}), "thinking_blocks[0].type"),
        (json!({"type": "text", "text": "x"}), "thinking_blocks[0].type"),
        (
            json!({"type": "thinking", "thinking": "x"}),
            "thinking_blocks[0].signature",
        ),
        (
            json!({"type": "thinking", "thinking": "x", "signature": ""}),
            "thinking_blocks[0].signature",
        ),
        (json!({"type": "redacted_thinking"}), "thinking_blocks[0].data"),
    ] {
        let mut body = request_body(json!({"role": "assistant", "content": "Hi", "thinking_blocks": [block]}));
        match provider().transform_request(&mut body) {
            Err(LiterLlmError::BadRequest { message, .. }) => assert!(message.contains(field), "{field}: {message}"),
            other => panic!("{field}: expected a bad request, got {other:?}"),
        }
    }
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

/// The SSE events Anthropic streams for [`thinking_turn`].
const THINKING_TURN_EVENTS: &[&str] = &[
    r#"{"type":"message_start","message":{"id":"msg_1","model":"claude-haiku-5-5","usage":{"input_tokens":10}}}"#,
    r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}"#,
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Need the "}}"#,
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"weather."}}"#,
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig-1"}}"#,
    r#"{"type":"content_block_stop","index":0}"#,
    r#"{"type":"content_block_start","index":1,"content_block":{"type":"redacted_thinking","data":"opaque-1"}}"#,
    r#"{"type":"content_block_stop","index":1}"#,
    r#"{"type":"content_block_start","index":2,"content_block":{"type":"text","text":""}}"#,
    r#"{"type":"content_block_delta","index":2,"delta":{"type":"text_delta","text":"Checking."}}"#,
    r#"{"type":"content_block_stop","index":2}"#,
    r#"{"type":"content_block_start","index":3,"content_block":{"type":"tool_use","id":"toolu_01","name":"get_weather","input":{}}}"#,
    r#"{"type":"content_block_delta","index":3,"delta":{"type":"input_json_delta","partial_json":"{\"city\":\"Paris\"}"}}"#,
    r#"{"type":"content_block_stop","index":3}"#,
    r#"{"type":"content_block_start","index":4,"content_block":{"type":"thinking","thinking":"","signature":""}}"#,
    r#"{"type":"content_block_delta","index":4,"delta":{"type":"signature_delta","signature":"sig-2"}}"#,
    r#"{"type":"content_block_stop","index":4}"#,
    r#"{"type":"content_block_start","index":5,"content_block":{"type":"tool_use","id":"toolu_02","name":"get_weather","input":{}}}"#,
    r#"{"type":"content_block_delta","index":5,"delta":{"type":"input_json_delta","partial_json":"{\"city\":\"Rome\"}"}}"#,
    r#"{"type":"content_block_stop","index":5}"#,
    r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":20}}"#,
];

#[test]
fn thinking_blocks_accumulated_from_a_stream_equal_the_reply_without_streaming() {
    let mut accumulator = ThinkingBlockAccumulator::new();
    for event in THINKING_TURN_EVENTS {
        if let Some(chunk) = provider().parse_stream_event(event).expect("parse_stream_event") {
            accumulator.push(&chunk);
        }
    }
    let streamed = accumulator.finish();

    let reply = transformed_response(thinking_turn());
    let expected: Vec<ThinkingBlock> = serde_json::from_value(reply["thinking_blocks"].clone()).expect("blocks");
    assert_eq!(streamed, expected);

    let mut assistant = reply.clone();
    assistant["thinking_blocks"] = serde_json::to_value(&streamed).expect("serialize");
    assert_eq!(
        transformed_request(assistant),
        json!({"role": "assistant", "content": thinking_turn()})
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
