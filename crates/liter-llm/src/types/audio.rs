use serde::{Deserialize, Serialize};

/// Request to generate speech audio from text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateSpeechRequest {
    /// Model ID (e.g., `"tts-1"`, `"tts-1-hd"`).
    pub model: String,
    /// Text to synthesize into speech.
    pub input: String,
    /// Voice name (e.g., `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"`).
    pub voice: String,
    /// Audio format (e.g., `"mp3"`, `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    /// Playback speed in `[0.25, 4.0]`. Defaults to 1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

/// Request to transcribe audio into text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTranscriptionRequest {
    /// Model ID (e.g., `"whisper-1"`).
    pub model: String,
    /// Base64-encoded audio file data.
    pub file: String,
    /// Language ISO-639-1 code (e.g., `"en"`, `"fr"`, `"de"`). Optional; model auto-detects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Optional text to guide the model (improves accuracy for domain-specific terms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Output format (e.g., `"json"`, `"text"`, `"vtt"`, `"srt"`, `"verbose_json"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    /// Sampling temperature in `[0.0, 1.0]`. Higher increases variability. Defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

/// Response from a transcription request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionResponse {
    /// The transcribed text.
    pub text: String,
    /// Detected language (ISO-639-1 code).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Total audio duration in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Detailed segment-level transcription (if response_format is "verbose_json").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<TranscriptionSegment>>,
}

/// A segment of transcribed audio with timing information.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// Segment index (0-based).
    pub id: u32,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
    /// Transcribed text for this segment.
    pub text: String,
}
