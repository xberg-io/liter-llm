//! Unified Realtime API types and translator trait for liter-llm.
//!
//! # Architecture
//!
//! - [`RealtimeEvent`] — the unified envelope variant enum; every provider's
//!   wire events are normalised into this shape by a [`RealtimeTranslator`].
//! - [`RealtimeEnvelope`] — wraps a [`RealtimeEvent`] with a per-frame `event_id`.
//! - [`RealtimeTranslator`] — pluggable per-provider translation trait.  Implement
//!   this to add a new provider without touching the proxy routing logic.
//! - [`ContentPart`] — content variant used inside
//!   [`RealtimeEvent::ConversationItemCreated`].
//! - [`ResponseStatus`] — terminal status for a completed realtime response.
//!
//! # Provider support
//!
//! The crate ships a single built-in translator:
//! - [`openai::OpenAiRealtimeTranslator`] — maps OpenAI Realtime API wire
//!   events 1-to-1 to the unified schema (the schemas are intentionally aligned).
//!
//! # Example
//!
//! ```rust,ignore
//! use liter_llm::realtime::{RealtimeTranslator, openai::OpenAiRealtimeTranslator};
//!
//! let translator = OpenAiRealtimeTranslator::new();
//! let raw = serde_json::json!({"type": "session.created", "event_id": "evt_1",
//!     "session": {"id": "sess_abc", "model": "gpt-4o-realtime-preview"}});
//! let event = translator.translate_inbound(raw).unwrap();
//! ```

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

pub mod openai;
pub use openai::OpenAiRealtimeTranslator;

use crate::error::Result;

// ── Supporting types ──────────────────────────────────────────────────────────

/// A single content part within a conversation item.
///
/// Conversation items may carry text, audio, or an image (by reference).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// A plain-text segment.
    Text {
        /// The text content.
        text: String,
    },
    /// A raw audio segment encoded as base64.
    Audio {
        /// Base64-encoded audio bytes.
        base64: String,
    },
    /// An image referenced by a URL or ID rather than inline bytes.
    ImageRef {
        /// The image URL or reference ID.
        url: String,
    },
}

impl ContentPart {
    /// Construct a text content part.
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    /// Construct an audio content part from a base64 string.
    pub fn audio(base64: impl Into<String>) -> Self {
        Self::Audio {
            base64: base64.into(),
        }
    }

    /// Construct an image-ref content part from a URL.
    pub fn image_ref(url: impl Into<String>) -> Self {
        Self::ImageRef { url: url.into() }
    }
}

/// Terminal status for a completed [`RealtimeEvent::ResponseDone`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    /// The response was produced in full.
    Completed,
    /// The response was cancelled before completion.
    Cancelled,
    /// The response failed due to an upstream error.
    Failed,
    /// The response hit a token/time limit before completing.
    Incomplete,
}

// ── Core event enum ───────────────────────────────────────────────────────────

/// Unified Realtime event — the normalised in-memory representation of every
/// server-to-client or client-to-server message in the liter-llm Realtime API.
///
/// All provider-specific wire formats are translated *into* this enum by a
/// [`RealtimeTranslator`], and translated *back out of* it when forwarding to
/// the upstream provider.  Unknown event types are preserved as [`Raw`] so the
/// proxy never silently drops events it does not yet understand.
///
/// [`Raw`]: RealtimeEvent::Raw
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeEvent {
    // ── Session ───────────────────────────────────────────────────────────────
    /// The provider confirmed that a new session was created.
    SessionCreated {
        /// Provider-assigned session identifier.
        session_id: String,
        /// The model that the session is running against.
        model: String,
    },
    /// The session configuration was updated (e.g. instructions changed).
    SessionUpdated {
        /// The session that was updated.
        session_id: String,
        /// New system instructions, when present.
        instructions: Option<String>,
    },

    // ── Conversation items ────────────────────────────────────────────────────
    /// A new conversation item was appended to the conversation.
    ConversationItemCreated {
        /// Provider-assigned item identifier.
        item_id: String,
        /// The role of the message author (`"user"`, `"assistant"`, `"system"`).
        role: String,
        /// Content parts that make up the item.
        content: Vec<ContentPart>,
    },
    /// A conversation item was deleted.
    ConversationItemDeleted {
        /// Identifier of the deleted item.
        item_id: String,
    },

    // ── Response lifecycle ────────────────────────────────────────────────────
    /// The provider started generating a new response.
    ResponseCreated {
        /// Provider-assigned response identifier.
        response_id: String,
    },
    /// The provider finished generating a response.
    ResponseDone {
        /// Identifier of the completed response.
        response_id: String,
        /// Terminal status of the response.
        status: ResponseStatus,
    },

    // ── Text streaming ────────────────────────────────────────────────────────
    /// An incremental text delta from the model's response.
    ResponseTextDelta {
        /// Response this delta belongs to.
        response_id: String,
        /// The new text fragment.
        delta: String,
    },
    /// The model's text output for this response is complete.
    ResponseTextDone {
        /// Response this completion belongs to.
        response_id: String,
        /// The full concatenated text output.
        text: String,
    },

    // ── Audio streaming ───────────────────────────────────────────────────────
    /// An incremental audio delta from the model's response.
    ResponseAudioDelta {
        /// Response this delta belongs to.
        response_id: String,
        /// Base64-encoded audio chunk.
        delta_base64: String,
    },
    /// The model's audio output for this response is complete.
    ResponseAudioDone {
        /// Response this completion belongs to.
        response_id: String,
    },

    // ── Audio transcript streaming ────────────────────────────────────────────
    /// An incremental transcript delta for the model's audio output.
    ResponseAudioTranscriptDelta {
        /// Response this delta belongs to.
        response_id: String,
        /// The new transcript fragment.
        delta: String,
    },
    /// The model's audio transcript for this response is complete.
    ResponseAudioTranscriptDone {
        /// Response this completion belongs to.
        response_id: String,
        /// The full concatenated transcript.
        transcript: String,
    },

    // ── Function call streaming ───────────────────────────────────────────────
    /// An incremental JSON delta for a function-call's arguments.
    ResponseFunctionCallArgumentsDelta {
        /// Response this delta belongs to.
        response_id: String,
        /// The specific tool call this delta belongs to.
        call_id: String,
        /// The new JSON fragment.
        delta: String,
    },
    /// The function-call arguments for a tool call are complete.
    ResponseFunctionCallArgumentsDone {
        /// Response this completion belongs to.
        response_id: String,
        /// The specific tool call that completed.
        call_id: String,
        /// The name of the function that was called.
        name: String,
        /// The full concatenated JSON arguments string.
        arguments: String,
    },

    // ── Input audio buffer ────────────────────────────────────────────────────
    /// Client is appending a chunk of audio to the input buffer.
    InputAudioBufferAppend {
        /// Base64-encoded audio bytes to append.
        audio_base64: String,
    },
    /// Client is committing the current input audio buffer for processing.
    InputAudioBufferCommit,
    /// Client is clearing the input audio buffer.
    InputAudioBufferClear,
    /// The provider detected the start of speech in the audio buffer.
    InputAudioBufferSpeechStarted {
        /// The conversation item that will contain the speech, when known.
        item_id: String,
    },
    /// The provider detected the end of speech in the audio buffer.
    InputAudioBufferSpeechStopped {
        /// The conversation item that will contain the speech.
        item_id: String,
        /// Millisecond offset within the audio buffer where speech ended.
        audio_end_ms: u32,
    },

    // ── Rate limits ───────────────────────────────────────────────────────────
    /// The provider sent updated rate-limit information.
    RateLimitsUpdated {
        /// Remaining request quota, when reported.
        remaining_requests: Option<u32>,
        /// Remaining token quota, when reported.
        remaining_tokens: Option<u32>,
        /// Timestamp when the quota resets.
        reset_at: SystemTime,
    },

    // ── Error ─────────────────────────────────────────────────────────────────
    /// The provider or proxy encountered an error.
    Error {
        /// Provider-specific or proxy error code string.
        code: String,
        /// Human-readable error message.
        message: String,
        /// The event ID that triggered the error, when applicable.
        event_id: Option<String>,
    },

    // ── Catch-all ────────────────────────────────────────────────────────────
    /// An event type that this library does not yet model explicitly.
    ///
    /// The proxy forwards `Raw` events transparently to avoid data loss when
    /// a provider introduces new event types that the translator has not yet
    /// mapped.
    Raw {
        /// The `type` field from the provider's wire message.
        event_type: String,
        /// The full, unparsed JSON payload.
        payload: serde_json::Value,
    },
}

// ── Envelope ──────────────────────────────────────────────────────────────────

/// Wire-level envelope that associates a per-frame `event_id` with a
/// [`RealtimeEvent`].
///
/// The `event_id` is optional: provider-to-client messages typically carry one,
/// but client-to-server messages (e.g. audio buffer appends) may omit it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealtimeEnvelope {
    /// Provider-assigned or proxy-generated identifier for this event frame.
    ///
    /// Used for correlation in logs and error responses (see
    /// [`RealtimeEvent::Error::event_id`]).
    pub event_id: Option<String>,
    /// The parsed event payload.
    pub event: RealtimeEvent,
}

impl RealtimeEnvelope {
    /// Construct an envelope without an event ID.
    pub fn new(event: RealtimeEvent) -> Self {
        Self {
            event_id: None,
            event,
        }
    }

    /// Construct an envelope with an explicit event ID.
    pub fn with_id(event_id: impl Into<String>, event: RealtimeEvent) -> Self {
        Self {
            event_id: Some(event_id.into()),
            event,
        }
    }
}

// ── Translator trait ──────────────────────────────────────────────────────────

/// Per-provider translation between the provider's native wire format and the
/// unified [`RealtimeEvent`] schema.
///
/// # Implementing a new provider
///
/// 1. Create a struct that holds any provider-specific config (e.g. base URL,
///    API version).
/// 2. Implement `translate_inbound` to parse the provider's JSON into a
///    [`RealtimeEvent`].  Unknown event types MUST be returned as
///    [`RealtimeEvent::Raw`] to avoid silent data loss.
/// 3. Implement `translate_outbound` to serialise a [`RealtimeEvent`] back into
///    the provider's expected wire format.
/// 4. Register the translator in the proxy's router.
///
/// # Thread-safety
///
/// Implementations MUST be `Send + Sync + 'static` so that the same translator
/// instance can be shared across the proxy's async tasks without cloning.
pub trait RealtimeTranslator: Send + Sync + 'static {
    /// Translate an incoming provider-native JSON event into the unified
    /// [`RealtimeEvent`].
    ///
    /// Unknown event types MUST be returned as [`RealtimeEvent::Raw`].
    /// Errors are reserved for malformed JSON or missing required fields.
    fn translate_inbound(&self, raw: serde_json::Value) -> Result<RealtimeEvent>;

    /// Translate an outgoing unified [`RealtimeEvent`] into the provider's
    /// native wire format.
    ///
    /// The returned [`serde_json::Value`] will be serialised and sent over the
    /// upstream WebSocket.
    fn translate_outbound(&self, event: &RealtimeEvent) -> Result<serde_json::Value>;

    /// Stable provider identifier used for routing, logging, and metric labels.
    ///
    /// Must be a static string (e.g. `"openai"`, `"anthropic-realtime"`).
    fn provider(&self) -> &'static str;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_part_text_serialises_and_deserialises() {
        let part = ContentPart::text("hello");
        let json = serde_json::to_string(&part).unwrap();
        let back: ContentPart = serde_json::from_str(&json).unwrap();
        assert_eq!(back, part);
    }

    #[test]
    fn response_status_all_variants_round_trip() {
        for status in [
            ResponseStatus::Completed,
            ResponseStatus::Cancelled,
            ResponseStatus::Failed,
            ResponseStatus::Incomplete,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: ResponseStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn realtime_envelope_with_id_sets_event_id() {
        let env = RealtimeEnvelope::with_id("evt_1", RealtimeEvent::InputAudioBufferCommit);
        assert_eq!(env.event_id.as_deref(), Some("evt_1"));
    }

    #[test]
    fn realtime_envelope_new_has_no_event_id() {
        let env = RealtimeEnvelope::new(RealtimeEvent::InputAudioBufferCommit);
        assert!(env.event_id.is_none());
    }

    #[test]
    fn realtime_event_raw_round_trips() {
        let payload = serde_json::json!({"foo": "bar"});
        let event = RealtimeEvent::Raw {
            event_type: "some.new.event".into(),
            payload: payload.clone(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: RealtimeEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }
}
