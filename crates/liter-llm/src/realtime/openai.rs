//! OpenAI Realtime API translator.
//!
//! OpenAI's Realtime API wire format is the origin of liter-llm's unified
//! [`RealtimeEvent`] schema, so the mapping here is intentionally 1-to-1.
//! The translator handles serialisation and deserialisation via `serde_json`
//! and maps unknown event types to [`RealtimeEvent::Raw`].

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::{ContentPart, RealtimeEvent, RealtimeTranslator, ResponseStatus};
use crate::error::{LiterLlmError, Result};

/// OpenAI Realtime API wire-format translator.
///
/// Maps OpenAI's server-sent events to the unified [`RealtimeEvent`] enum and
/// back.  Because liter-llm's schema was designed to match OpenAI's Realtime
/// API closely, most translations are straightforward field extractions.
///
/// # Thread-safety
///
/// `OpenAiRealtimeTranslator` is `Send + Sync + 'static`; a single shared
/// instance can service the entire proxy's WebSocket session pool.
#[derive(Debug, Clone, Default)]
pub struct OpenAiRealtimeTranslator;

impl OpenAiRealtimeTranslator {
    /// Create a new translator.
    pub fn new() -> Self {
        Self
    }
}

// ── Helper: extract a required string field ───────────────────────────────────

fn get_str<'a>(obj: &'a Value, key: &str) -> Result<&'a str> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LiterLlmError::BadRequest {
            message: format!("realtime event missing required field '{key}'"),
            status: 400,
        })
}

fn get_str_opt<'a>(obj: &'a Value, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(|v| v.as_str())
}

fn get_u32(obj: &Value, key: &str) -> Option<u32> {
    obj.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

// ── Helper: parse content parts from OpenAI content array ────────────────────

fn parse_content_parts(raw: &Value) -> Vec<ContentPart> {
    let Some(arr) = raw.as_array() else {
        return vec![];
    };
    arr.iter()
        .filter_map(|item| {
            let kind = item.get("type").and_then(|v| v.as_str())?;
            match kind {
                "text" | "input_text" => {
                    let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    Some(ContentPart::text(text))
                }
                "audio" | "input_audio" => {
                    let base64 = item.get("audio").and_then(|v| v.as_str()).unwrap_or("");
                    Some(ContentPart::audio(base64))
                }
                "image_url" => {
                    let url = item
                        .get("image_url")
                        .and_then(|u| u.get("url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Some(ContentPart::image_ref(url))
                }
                _ => None,
            }
        })
        .collect()
}

// ── Helper: parse a reset_at timestamp ───────────────────────────────────────

fn parse_reset_at(obj: &Value) -> SystemTime {
    // OpenAI may supply `reset_at` as a Unix timestamp (f64 or integer).
    if let Some(ts) = obj.get("reset_at").and_then(|v| v.as_f64()) {
        let secs = ts as u64;
        let nanos = ((ts - secs as f64) * 1_000_000_000.0) as u32;
        return UNIX_EPOCH + Duration::new(secs, nanos);
    }
    // Fallback: reset 60 seconds from now.
    SystemTime::now() + Duration::from_secs(60)
}

// ── RealtimeTranslator impl ───────────────────────────────────────────────────

impl RealtimeTranslator for OpenAiRealtimeTranslator {
    fn provider(&self) -> &'static str {
        "openai"
    }

    fn translate_inbound(&self, raw: Value) -> Result<RealtimeEvent> {
        let event_type = get_str(&raw, "type")?;

        let event = match event_type {
            // ── Session ───────────────────────────────────────────────────────
            "session.created" => {
                let session = raw.get("session").unwrap_or(&Value::Null);
                RealtimeEvent::SessionCreated {
                    session_id: get_str_opt(session, "id").unwrap_or("").into(),
                    model: get_str_opt(session, "model").unwrap_or("").into(),
                }
            }
            "session.updated" => {
                let session = raw.get("session").unwrap_or(&Value::Null);
                RealtimeEvent::SessionUpdated {
                    session_id: get_str_opt(session, "id").unwrap_or("").into(),
                    instructions: get_str_opt(session, "instructions").map(str::to_owned),
                }
            }

            // ── Conversation items ────────────────────────────────────────────
            "conversation.item.created" | "conversation.item.added" => {
                let item = raw.get("item").unwrap_or(&Value::Null);
                let content = item.get("content").map(parse_content_parts).unwrap_or_default();
                RealtimeEvent::ConversationItemCreated {
                    item_id: get_str_opt(item, "id").unwrap_or("").into(),
                    role: get_str_opt(item, "role").unwrap_or("").into(),
                    content,
                }
            }
            "conversation.item.deleted" => RealtimeEvent::ConversationItemDeleted {
                item_id: raw.get("item_id").and_then(|v| v.as_str()).unwrap_or("").into(),
            },

            // ── Response lifecycle ────────────────────────────────────────────
            "response.created" => {
                let response = raw.get("response").unwrap_or(&Value::Null);
                RealtimeEvent::ResponseCreated {
                    response_id: get_str_opt(response, "id").unwrap_or("").into(),
                }
            }
            "response.done" => {
                let response = raw.get("response").unwrap_or(&Value::Null);
                let status_str = get_str_opt(response, "status").unwrap_or("completed");
                let status = match status_str {
                    "cancelled" => ResponseStatus::Cancelled,
                    "failed" => ResponseStatus::Failed,
                    "incomplete" => ResponseStatus::Incomplete,
                    _ => ResponseStatus::Completed,
                };
                RealtimeEvent::ResponseDone {
                    response_id: get_str_opt(response, "id").unwrap_or("").into(),
                    status,
                }
            }

            // ── Text streaming ────────────────────────────────────────────────
            "response.text.delta" => RealtimeEvent::ResponseTextDelta {
                response_id: raw.get("response_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                delta: raw.get("delta").and_then(|v| v.as_str()).unwrap_or("").into(),
            },
            "response.text.done" => RealtimeEvent::ResponseTextDone {
                response_id: raw.get("response_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                text: raw.get("text").and_then(|v| v.as_str()).unwrap_or("").into(),
            },

            // ── Audio streaming ───────────────────────────────────────────────
            "response.audio.delta" => RealtimeEvent::ResponseAudioDelta {
                response_id: raw.get("response_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                delta_base64: raw.get("delta").and_then(|v| v.as_str()).unwrap_or("").into(),
            },
            "response.audio.done" => RealtimeEvent::ResponseAudioDone {
                response_id: raw.get("response_id").and_then(|v| v.as_str()).unwrap_or("").into(),
            },

            // ── Audio transcript streaming ─────────────────────────────────────
            "response.audio_transcript.delta" => RealtimeEvent::ResponseAudioTranscriptDelta {
                response_id: raw.get("response_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                delta: raw.get("delta").and_then(|v| v.as_str()).unwrap_or("").into(),
            },
            "response.audio_transcript.done" => RealtimeEvent::ResponseAudioTranscriptDone {
                response_id: raw.get("response_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                transcript: raw.get("transcript").and_then(|v| v.as_str()).unwrap_or("").into(),
            },

            // ── Function call streaming ───────────────────────────────────────
            "response.function_call_arguments.delta" => RealtimeEvent::ResponseFunctionCallArgumentsDelta {
                response_id: raw.get("response_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                call_id: raw.get("call_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                delta: raw.get("delta").and_then(|v| v.as_str()).unwrap_or("").into(),
            },
            "response.function_call_arguments.done" => RealtimeEvent::ResponseFunctionCallArgumentsDone {
                response_id: raw.get("response_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                call_id: raw.get("call_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                name: raw.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
                arguments: raw.get("arguments").and_then(|v| v.as_str()).unwrap_or("").into(),
            },

            // ── Input audio buffer ────────────────────────────────────────────
            "input_audio_buffer.append" => RealtimeEvent::InputAudioBufferAppend {
                audio_base64: raw.get("audio").and_then(|v| v.as_str()).unwrap_or("").into(),
            },
            "input_audio_buffer.commit" => RealtimeEvent::InputAudioBufferCommit,
            "input_audio_buffer.clear" => RealtimeEvent::InputAudioBufferClear,
            "input_audio_buffer.speech_started" => RealtimeEvent::InputAudioBufferSpeechStarted {
                item_id: raw.get("item_id").and_then(|v| v.as_str()).unwrap_or("").into(),
            },
            "input_audio_buffer.speech_stopped" => RealtimeEvent::InputAudioBufferSpeechStopped {
                item_id: raw.get("item_id").and_then(|v| v.as_str()).unwrap_or("").into(),
                audio_end_ms: get_u32(&raw, "audio_end_ms").unwrap_or(0),
            },

            // ── Rate limits ───────────────────────────────────────────────────
            "rate_limits.updated" => {
                // OpenAI sends an array of rate-limit objects.
                let limits = raw.get("rate_limits").and_then(|v| v.as_array());
                let mut remaining_requests = None;
                let mut remaining_tokens = None;
                let mut reset_at = SystemTime::now() + Duration::from_secs(60);

                if let Some(limits) = limits {
                    for limit in limits {
                        let name = limit.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        match name {
                            "requests" => {
                                remaining_requests = get_u32(limit, "remaining");
                                reset_at = parse_reset_at(limit);
                            }
                            "tokens" => {
                                remaining_tokens = get_u32(limit, "remaining");
                            }
                            _ => {}
                        }
                    }
                }

                RealtimeEvent::RateLimitsUpdated {
                    remaining_requests,
                    remaining_tokens,
                    reset_at,
                }
            }

            // ── Error ─────────────────────────────────────────────────────────
            "error" => {
                let err = raw.get("error").unwrap_or(&raw);
                RealtimeEvent::Error {
                    code: get_str_opt(err, "code").unwrap_or("unknown").into(),
                    message: get_str_opt(err, "message").unwrap_or("").into(),
                    event_id: get_str_opt(&raw, "event_id").map(str::to_owned),
                }
            }

            // ── Catch-all ─────────────────────────────────────────────────────
            other => RealtimeEvent::Raw {
                event_type: other.into(),
                payload: raw,
            },
        };

        Ok(event)
    }

    fn translate_outbound(&self, event: &RealtimeEvent) -> Result<serde_json::Value> {
        use serde_json::json;

        let value = match event {
            RealtimeEvent::SessionCreated { session_id, model } => json!({
                "type": "session.created",
                "session": { "id": session_id, "model": model }
            }),
            RealtimeEvent::SessionUpdated {
                session_id,
                instructions,
            } => {
                let mut session = serde_json::Map::new();
                session.insert("id".into(), json!(session_id));
                if let Some(instr) = instructions {
                    session.insert("instructions".into(), json!(instr));
                }
                json!({ "type": "session.updated", "session": session })
            }
            RealtimeEvent::ConversationItemCreated { item_id, role, content } => {
                let content_json: Vec<_> = content
                    .iter()
                    .map(|part| match part {
                        ContentPart::Text { text } => json!({"type": "text", "text": text}),
                        ContentPart::Audio { base64 } => {
                            json!({"type": "audio", "audio": base64})
                        }
                        ContentPart::ImageRef { url } => {
                            json!({"type": "image_url", "image_url": {"url": url}})
                        }
                    })
                    .collect();
                json!({
                    "type": "conversation.item.created",
                    "item": { "id": item_id, "role": role, "content": content_json }
                })
            }
            RealtimeEvent::ConversationItemDeleted { item_id } => {
                json!({ "type": "conversation.item.deleted", "item_id": item_id })
            }
            RealtimeEvent::ResponseCreated { response_id } => {
                json!({ "type": "response.created", "response": { "id": response_id } })
            }
            RealtimeEvent::ResponseDone { response_id, status } => {
                let status_str = match status {
                    ResponseStatus::Completed => "completed",
                    ResponseStatus::Cancelled => "cancelled",
                    ResponseStatus::Failed => "failed",
                    ResponseStatus::Incomplete => "incomplete",
                };
                json!({
                    "type": "response.done",
                    "response": { "id": response_id, "status": status_str }
                })
            }
            RealtimeEvent::ResponseTextDelta { response_id, delta } => {
                json!({ "type": "response.text.delta", "response_id": response_id, "delta": delta })
            }
            RealtimeEvent::ResponseTextDone { response_id, text } => {
                json!({ "type": "response.text.done", "response_id": response_id, "text": text })
            }
            RealtimeEvent::ResponseAudioDelta {
                response_id,
                delta_base64,
            } => {
                json!({
                    "type": "response.audio.delta",
                    "response_id": response_id,
                    "delta": delta_base64
                })
            }
            RealtimeEvent::ResponseAudioDone { response_id } => {
                json!({ "type": "response.audio.done", "response_id": response_id })
            }
            RealtimeEvent::ResponseAudioTranscriptDelta { response_id, delta } => {
                json!({
                    "type": "response.audio_transcript.delta",
                    "response_id": response_id,
                    "delta": delta
                })
            }
            RealtimeEvent::ResponseAudioTranscriptDone {
                response_id,
                transcript,
            } => {
                json!({
                    "type": "response.audio_transcript.done",
                    "response_id": response_id,
                    "transcript": transcript
                })
            }
            RealtimeEvent::ResponseFunctionCallArgumentsDelta {
                response_id,
                call_id,
                delta,
            } => {
                json!({
                    "type": "response.function_call_arguments.delta",
                    "response_id": response_id,
                    "call_id": call_id,
                    "delta": delta
                })
            }
            RealtimeEvent::ResponseFunctionCallArgumentsDone {
                response_id,
                call_id,
                name,
                arguments,
            } => {
                json!({
                    "type": "response.function_call_arguments.done",
                    "response_id": response_id,
                    "call_id": call_id,
                    "name": name,
                    "arguments": arguments
                })
            }
            RealtimeEvent::InputAudioBufferAppend { audio_base64 } => {
                json!({ "type": "input_audio_buffer.append", "audio": audio_base64 })
            }
            RealtimeEvent::InputAudioBufferCommit => {
                json!({ "type": "input_audio_buffer.commit" })
            }
            RealtimeEvent::InputAudioBufferClear => {
                json!({ "type": "input_audio_buffer.clear" })
            }
            RealtimeEvent::InputAudioBufferSpeechStarted { item_id } => {
                json!({ "type": "input_audio_buffer.speech_started", "item_id": item_id })
            }
            RealtimeEvent::InputAudioBufferSpeechStopped { item_id, audio_end_ms } => {
                json!({
                    "type": "input_audio_buffer.speech_stopped",
                    "item_id": item_id,
                    "audio_end_ms": audio_end_ms
                })
            }
            RealtimeEvent::RateLimitsUpdated {
                remaining_requests,
                remaining_tokens,
                reset_at,
            } => {
                let reset_ts = reset_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64();
                let mut limits = vec![];
                if let Some(r) = remaining_requests {
                    limits.push(json!({"name": "requests", "remaining": r, "reset_at": reset_ts}));
                }
                if let Some(t) = remaining_tokens {
                    limits.push(json!({"name": "tokens", "remaining": t, "reset_at": reset_ts}));
                }
                json!({ "type": "rate_limits.updated", "rate_limits": limits })
            }
            RealtimeEvent::Error {
                code,
                message,
                event_id,
            } => {
                let mut obj = json!({
                    "type": "error",
                    "error": { "code": code, "message": message }
                });
                if let Some(eid) = event_id {
                    obj["event_id"] = json!(eid);
                }
                obj
            }
            RealtimeEvent::Raw { event_type, payload } => {
                // Forward raw events as-is, but normalise the type field.
                let mut out = payload.clone();
                if let Some(obj) = out.as_object_mut() {
                    obj.insert("type".into(), json!(event_type));
                }
                out
            }
        };

        Ok(value)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn translator() -> OpenAiRealtimeTranslator {
        OpenAiRealtimeTranslator::new()
    }

    // ─── provider() ─────────────────────────────────────────────────────────

    #[test]
    fn provider_returns_openai() {
        assert_eq!(translator().provider(), "openai");
    }

    // ─── round-trip through openai translator ─────────────────────────────────

    #[test]
    fn realtime_event_round_trips_through_openai_translator() {
        let tr = translator();

        // Canonical OpenAI session.created event.
        let original = json!({
            "type": "session.created",
            "event_id": "evt_abc123",
            "session": {
                "id": "sess_xyz",
                "model": "gpt-4o-realtime-preview"
            }
        });

        // Inbound: OpenAI JSON → unified RealtimeEvent
        let event = tr.translate_inbound(original.clone()).unwrap();
        assert!(
            matches!(event, RealtimeEvent::SessionCreated { ref session_id, ref model }
                if session_id == "sess_xyz" && model == "gpt-4o-realtime-preview")
        );

        // Outbound: unified RealtimeEvent → OpenAI JSON
        let outbound = tr.translate_outbound(&event).unwrap();

        // The round-tripped output must carry the same session id and model.
        let session = outbound.get("session").expect("session field present");
        assert_eq!(session["id"], json!("sess_xyz"));
        assert_eq!(session["model"], json!("gpt-4o-realtime-preview"));
        assert_eq!(outbound["type"], json!("session.created"));
    }

    #[test]
    fn realtime_event_response_done_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "response.done",
            "response": { "id": "resp_001", "status": "completed" }
        });
        let event = tr.translate_inbound(raw).unwrap();
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "response.done");
        assert_eq!(out["response"]["id"], "resp_001");
        assert_eq!(out["response"]["status"], "completed");
    }

    #[test]
    fn realtime_event_text_delta_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "response.text.delta",
            "response_id": "resp_002",
            "delta": "Hello, "
        });
        let event = tr.translate_inbound(raw).unwrap();
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "response.text.delta");
        assert_eq!(out["delta"], "Hello, ");
    }

    #[test]
    fn realtime_event_audio_delta_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "response.audio.delta",
            "response_id": "resp_003",
            "delta": "YWJj"
        });
        let event = tr.translate_inbound(raw).unwrap();
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "response.audio.delta");
        assert_eq!(out["delta"], "YWJj");
    }

    #[test]
    fn realtime_event_function_call_done_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "response.function_call_arguments.done",
            "response_id": "resp_004",
            "call_id": "call_001",
            "name": "get_weather",
            "arguments": "{\"location\":\"London\"}"
        });
        let event = tr.translate_inbound(raw).unwrap();
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["name"], "get_weather");
        assert_eq!(out["arguments"], "{\"location\":\"London\"}");
    }

    // ─── unknown event type falls to Raw ─────────────────────────────────────

    #[test]
    fn realtime_event_unknown_event_type_falls_to_raw() {
        let tr = translator();
        let raw = json!({
            "type": "some.future.event",
            "custom_field": 42,
            "nested": { "x": true }
        });
        let event = tr.translate_inbound(raw.clone()).unwrap();
        match event {
            RealtimeEvent::Raw {
                ref event_type,
                ref payload,
            } => {
                assert_eq!(event_type, "some.future.event");
                assert_eq!(payload["custom_field"], 42);
                assert_eq!(payload["nested"]["x"], true);
            }
            other => panic!("expected Raw, got {other:?}"),
        }
    }

    // ─── error event ─────────────────────────────────────────────────────────

    #[test]
    fn realtime_event_error_preserves_fields() {
        let tr = translator();
        let raw = json!({
            "type": "error",
            "event_id": "evt_err_1",
            "error": { "code": "invalid_session", "message": "session expired" }
        });
        let event = tr.translate_inbound(raw).unwrap();
        match event {
            RealtimeEvent::Error {
                ref code,
                ref message,
                ref event_id,
            } => {
                assert_eq!(code, "invalid_session");
                assert_eq!(message, "session expired");
                assert_eq!(event_id.as_deref(), Some("evt_err_1"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    // ─── input audio buffer events ────────────────────────────────────────────

    #[test]
    fn input_audio_buffer_commit_round_trips() {
        let tr = translator();
        let raw = json!({"type": "input_audio_buffer.commit"});
        let event = tr.translate_inbound(raw).unwrap();
        assert_eq!(event, RealtimeEvent::InputAudioBufferCommit);
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "input_audio_buffer.commit");
    }

    #[test]
    fn input_audio_buffer_clear_round_trips() {
        let tr = translator();
        let raw = json!({"type": "input_audio_buffer.clear"});
        let event = tr.translate_inbound(raw).unwrap();
        assert_eq!(event, RealtimeEvent::InputAudioBufferClear);
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "input_audio_buffer.clear");
    }

    // ─── missing required field returns error ─────────────────────────────────

    #[test]
    fn translate_inbound_missing_type_returns_error() {
        let tr = translator();
        let raw = json!({"event_id": "evt_1", "session": {}});
        let result = tr.translate_inbound(raw);
        assert!(result.is_err());
    }

    // ─── Pass-2 round-trips for previously-uncovered variants ────────────────

    #[test]
    fn realtime_session_updated_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "session.updated",
            "session": {
                "id": "sess_upd_001",
                "instructions": "Be terse and accurate."
            }
        });
        let event = tr.translate_inbound(raw).unwrap();
        match &event {
            RealtimeEvent::SessionUpdated { session_id, instructions } => {
                assert_eq!(session_id, "sess_upd_001");
                assert_eq!(instructions.as_deref(), Some("Be terse and accurate."));
            }
            other => panic!("expected SessionUpdated, got {other:?}"),
        }
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "session.updated");
        assert_eq!(out["session"]["id"], "sess_upd_001");
        assert_eq!(out["session"]["instructions"], "Be terse and accurate.");
    }

    #[test]
    fn realtime_conversation_item_deleted_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "conversation.item.deleted",
            "item_id": "item_xyz_42"
        });
        let event = tr.translate_inbound(raw).unwrap();
        match &event {
            RealtimeEvent::ConversationItemDeleted { item_id } => {
                assert_eq!(item_id, "item_xyz_42");
            }
            other => panic!("expected ConversationItemDeleted, got {other:?}"),
        }
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "conversation.item.deleted");
        assert_eq!(out["item_id"], "item_xyz_42");
    }

    #[test]
    fn realtime_response_audio_transcript_delta_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "response.audio_transcript.delta",
            "response_id": "resp_at_001",
            "delta": "Hello, world"
        });
        let event = tr.translate_inbound(raw).unwrap();
        match &event {
            RealtimeEvent::ResponseAudioTranscriptDelta { response_id, delta } => {
                assert_eq!(response_id, "resp_at_001");
                assert_eq!(delta, "Hello, world");
            }
            other => panic!("expected ResponseAudioTranscriptDelta, got {other:?}"),
        }
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "response.audio_transcript.delta");
        assert_eq!(out["response_id"], "resp_at_001");
        assert_eq!(out["delta"], "Hello, world");
    }

    #[test]
    fn realtime_response_function_call_arguments_delta_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "response.function_call_arguments.delta",
            "response_id": "resp_fn_001",
            "call_id": "call_abc",
            "delta": "{\"x\":"
        });
        let event = tr.translate_inbound(raw).unwrap();
        match &event {
            RealtimeEvent::ResponseFunctionCallArgumentsDelta {
                response_id,
                call_id,
                delta,
            } => {
                assert_eq!(response_id, "resp_fn_001");
                assert_eq!(call_id, "call_abc");
                assert_eq!(delta, "{\"x\":");
            }
            other => panic!("expected ResponseFunctionCallArgumentsDelta, got {other:?}"),
        }
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "response.function_call_arguments.delta");
        assert_eq!(out["call_id"], "call_abc");
        assert_eq!(out["delta"], "{\"x\":");
    }

    #[test]
    fn realtime_input_audio_buffer_speech_started_round_trips() {
        let tr = translator();
        let raw = json!({
            "type": "input_audio_buffer.speech_started",
            "item_id": "item_speech_001"
        });
        let event = tr.translate_inbound(raw).unwrap();
        match &event {
            RealtimeEvent::InputAudioBufferSpeechStarted { item_id } => {
                assert_eq!(item_id, "item_speech_001");
            }
            other => panic!("expected InputAudioBufferSpeechStarted, got {other:?}"),
        }
        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "input_audio_buffer.speech_started");
        assert_eq!(out["item_id"], "item_speech_001");
    }

    #[test]
    fn realtime_rate_limits_updated_round_trips() {
        let tr = translator();
        // OpenAI reports per-axis limits in an array.  Use a reset_at one hour
        // into the future so the field is non-trivial.
        let reset_ts: f64 = 1_700_000_000.0;
        let raw = json!({
            "type": "rate_limits.updated",
            "rate_limits": [
                { "name": "requests", "remaining": 42u32, "reset_at": reset_ts },
                { "name": "tokens",   "remaining": 9_000u32, "reset_at": reset_ts },
            ]
        });
        let event = tr.translate_inbound(raw).unwrap();
        match &event {
            RealtimeEvent::RateLimitsUpdated {
                remaining_requests,
                remaining_tokens,
                ..
            } => {
                assert_eq!(*remaining_requests, Some(42));
                assert_eq!(*remaining_tokens, Some(9_000));
            }
            other => panic!("expected RateLimitsUpdated, got {other:?}"),
        }

        let out = tr.translate_outbound(&event).unwrap();
        assert_eq!(out["type"], "rate_limits.updated");
        let arr = out["rate_limits"].as_array().expect("rate_limits array");
        assert_eq!(arr.len(), 2, "both axes must be serialised back");
        // requests entry first, tokens second.
        let req_entry = arr
            .iter()
            .find(|e| e["name"] == "requests")
            .expect("requests entry present");
        let tok_entry = arr.iter().find(|e| e["name"] == "tokens").expect("tokens entry present");
        assert_eq!(req_entry["remaining"], 42);
        assert_eq!(tok_entry["remaining"], 9_000);
    }
}
