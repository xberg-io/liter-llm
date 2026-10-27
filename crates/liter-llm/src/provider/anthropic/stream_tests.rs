//! Coverage for the finish reason of a streamed reply: it equals the finish reason of the full reply.

use serde_json::json;

use super::*;
use crate::types::FinishReason;

use super::tests::provider;

/// The finish reason of a reply without streaming, read the way the client reads it.
fn finish_reason_of_full_reply(stop_reason: &str) -> FinishReason {
    let mut body = json!({
        "id": "msg_1",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5",
        "content": [{"type": "text", "text": "I cannot help with that."}],
        "stop_reason": stop_reason,
        "usage": {"input_tokens": 10, "output_tokens": 6}
    });
    provider()
        .transform_response(&mut body)
        .expect("transform_response should not fail");
    serde_json::from_value(body["choices"][0]["finish_reason"].clone()).expect("finish reason")
}

fn finish_reason_of_stream(stop_reason: &str) -> Option<FinishReason> {
    let event = format!(
        r#"{{"type":"message_delta","delta":{{"stop_reason":"{stop_reason}"}},"usage":{{"output_tokens":6}}}}"#
    );
    provider()
        .parse_stream_event(&event)
        .expect("parse_stream_event should not fail")
        .expect("expected chunk")
        .choices[0]
        .finish_reason
        .clone()
}

/// Revert line: restore the hand-written `match` on the finish reason string in
/// `message_delta_chunk` in `anthropic/stream.rs`, and a refusal streams as `Other`.
#[test]
fn parse_stream_event_refusal_finishes_with_content_filter() {
    assert_eq!(
        finish_reason_of_stream("refusal"),
        Some(FinishReason::ContentFilter),
        "a streamed refusal"
    );
    assert_eq!(finish_reason_of_full_reply("refusal"), FinishReason::ContentFilter);
}

#[test]
fn parse_stream_event_finish_reason_matches_the_full_reply_for_every_stop_reason() {
    for stop_reason in [
        "end_turn",
        "stop_sequence",
        "tool_use",
        "max_tokens",
        "refusal",
        "content_filtered",
        "pause_turn",
        "a_reason_from_a_newer_api",
    ] {
        assert_eq!(
            finish_reason_of_stream(stop_reason),
            Some(finish_reason_of_full_reply(stop_reason)),
            "stop reason {stop_reason}"
        );
    }
}

#[test]
fn parse_stream_event_message_delta_without_stop_reason_has_no_finish_reason() {
    let event = r#"{"type":"message_delta","delta":{},"usage":{"output_tokens":6}}"#;
    let chunk = provider()
        .parse_stream_event(event)
        .expect("parse_stream_event should not fail")
        .expect("expected chunk");
    assert_eq!(chunk.choices[0].finish_reason, None);
}
