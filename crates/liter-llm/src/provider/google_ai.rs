use std::borrow::Cow;

use crate::error::Result;
use crate::provider::Provider;
use crate::types::ChatCompletionChunk;

use super::vertex::{parse_gemini_stream_event, transform_gemini_request, transform_gemini_response};

/// Google AI Studio base URL (Gemini API via API key).
const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Google AI Studio (Gemini) provider.
///
/// Uses the same Gemini `generateContent` format as Vertex AI but with:
/// - Different base URL: `https://generativelanguage.googleapis.com/v1beta`
/// - API key authentication via `x-goog-api-key` header (not OAuth2).
/// - Model routing via `gemini/` or `google_ai/` prefix.
///
/// Request/response translation is shared with [`super::vertex::VertexAiProvider`]
/// via the `pub(crate)` Gemini transform functions.
///
/// # Configuration
///
/// ```rust,ignore
/// let config = ClientConfigBuilder::new("AIza...your-api-key").build();
/// let client = DefaultClient::new(config, Some("gemini/gemini-2.0-flash"))?;
///
/// // Or using the google_ai/ prefix:
/// let client = DefaultClient::new(config, Some("google_ai/gemini-2.0-flash"))?;
/// ```
pub struct GoogleAiProvider {
    base_url: String,
}

impl Default for GoogleAiProvider {
    fn default() -> Self {
        Self {
            base_url: BASE_URL.to_owned(),
        }
    }
}

impl GoogleAiProvider {
    #[must_use]
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
        }
    }
}

impl Provider for GoogleAiProvider {
    fn name(&self) -> &str {
        "google_ai"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn env_var(&self) -> Option<&str> {
        Some("GEMINI_API_KEY")
    }

    /// Google AI Studio uses `x-goog-api-key` header for authentication.
    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        Some((Cow::Borrowed("x-goog-api-key"), Cow::Borrowed(api_key)))
    }

    fn matches_model(&self, model: &str) -> bool {
        model.starts_with("gemini/") || model.starts_with("google_ai/")
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        model
            .strip_prefix("gemini/")
            .or_else(|| model.strip_prefix("google_ai/"))
            .unwrap_or(model)
    }

    /// Build the full URL for a Google AI Studio request.
    ///
    /// Chat completions → `{base}/models/{model}:generateContent`
    /// Embeddings       → `{base}/models/{model}:embedContent`
    /// Other paths      → `{base}{endpoint_path}`
    fn build_url(&self, endpoint_path: &str, model: &str) -> String {
        let base = self.base_url().trim_end_matches('/');
        if endpoint_path.contains("chat/completions") {
            format!("{base}/models/{model}:generateContent")
        } else if endpoint_path.contains("embeddings") {
            format!("{base}/models/{model}:batchEmbedContents")
        } else {
            format!("{base}{endpoint_path}")
        }
    }

    fn transform_request(&self, body: &mut serde_json::Value) -> Result<()> {
        transform_gemini_request(body)
    }

    fn transform_response(&self, body: &mut serde_json::Value) -> Result<()> {
        transform_gemini_response(body)
    }

    /// Build the streaming URL: `:streamGenerateContent` plus `?alt=sse`.
    ///
    /// ~keep Streaming is a DISTINCT method, not `generateContent` with a flag.
    /// ~keep `:generateContent` ignores `alt=sse` and returns one ordinary JSON
    /// ~keep body, which the SSE parser cannot frame — the caller sees a single
    /// ~keep `Streaming { "SSE stream truncated" }` error whose message gives no
    /// ~keep hint that the endpoint was wrong.
    fn build_stream_url(&self, endpoint_path: &str, model: &str) -> String {
        let url = self.build_url(endpoint_path, model);
        match url.strip_suffix(":generateContent") {
            Some(prefix) => format!("{prefix}:streamGenerateContent?alt=sse"),
            None => url,
        }
    }

    fn parse_stream_event(&self, event_data: &str) -> Result<Option<ChatCompletionChunk>> {
        parse_gemini_stream_event(event_data)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::provider::Provider;

    fn provider() -> GoogleAiProvider {
        GoogleAiProvider::default()
    }

    #[test]
    fn base_url_is_generative_language_api() {
        let p = provider();
        assert_eq!(p.base_url(), "https://generativelanguage.googleapis.com/v1beta");
    }

    #[test]
    fn auth_header_uses_x_goog_api_key() {
        let p = provider();
        let (name, value) = p.auth_header("test-key").expect("auth_header should be present");
        assert_eq!(name.as_ref(), "x-goog-api-key");
        assert_eq!(value.as_ref(), "test-key");
    }

    #[test]
    fn matches_gemini_prefix() {
        let p = provider();
        assert!(p.matches_model("gemini/gemini-2.0-flash"));
        assert!(p.matches_model("google_ai/gemini-2.0-flash"));
        assert!(!p.matches_model("vertex_ai/gemini-2.0-flash"));
        assert!(!p.matches_model("gpt-4"));
    }

    #[test]
    fn strip_model_prefix_gemini() {
        let p = provider();
        assert_eq!(p.strip_model_prefix("gemini/gemini-2.0-flash"), "gemini-2.0-flash");
        assert_eq!(p.strip_model_prefix("google_ai/gemini-pro"), "gemini-pro");
        assert_eq!(p.strip_model_prefix("gemini-2.0-flash"), "gemini-2.0-flash");
    }

    #[test]
    fn build_url_chat_completions() {
        let p = provider();
        let url = p.build_url("/chat/completions", "gemini-2.0-flash");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        );
    }

    #[test]
    fn build_url_embeddings() {
        let p = provider();
        let url = p.build_url("/embeddings", "text-embedding-004");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:batchEmbedContents"
        );
    }

    /// Streaming must target `:streamGenerateContent`.  This test previously
    /// asserted `:generateContent`, pinning the bug: that method ignores
    /// `alt=sse` and returns one ordinary JSON body, so every Gemini stream
    /// failed with a misleading "SSE stream truncated" error.
    #[test]
    fn build_stream_url_targets_stream_generate_content() {
        let p = provider();
        let url = p.build_stream_url("/chat/completions", "gemini-2.0-flash");

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:streamGenerateContent?alt=sse",
            "streaming must use the streamGenerateContent method"
        );
    }

    /// The non-streaming path must be untouched by the streaming rewrite.
    #[test]
    fn build_url_still_targets_generate_content_for_non_streaming() {
        let p = provider();
        let url = p.build_url("/chat/completions", "gemini-2.0-flash");

        assert!(url.ends_with(":generateContent"), "got {url}");
    }

    /// The rewrite keys off `:generateContent`, so the batch embeddings URL
    /// must pass through unchanged.
    #[test]
    fn build_stream_url_leaves_embeddings_endpoint_alone() {
        let p = provider();
        let url = p.build_stream_url("/embeddings", "text-embedding-004");

        assert!(url.contains(":batchEmbedContents"), "got {url}");
        assert!(!url.contains("streamGenerateContent"), "got {url}");
        assert!(!url.contains("alt=sse"), "got {url}");
    }

    #[test]
    fn transform_request_basic_chat() {
        let p = provider();
        let mut body = json!({
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello!"}
            ],
            "max_tokens": 100
        });

        p.transform_request(&mut body)
            .expect("transform_request should not fail");

        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "You are helpful.");
        assert_eq!(body["contents"][0]["role"], "user");
        assert_eq!(body["contents"][0]["parts"][0]["text"], "Hello!");
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 100);
    }

    #[test]
    fn transform_request_safety_settings() {
        let p = provider();
        let mut body = json!({
            "messages": [{"role": "user", "content": "hi"}],
            "extra_body": {
                "safety_settings": [
                    {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_MEDIUM_AND_ABOVE"}
                ]
            }
        });

        p.transform_request(&mut body)
            .expect("transform_request should not fail");

        let settings = body["safetySettings"]
            .as_array()
            .expect("safetySettings should be an array");
        assert_eq!(settings.len(), 1);
        assert_eq!(settings[0]["category"], "HARM_CATEGORY_HATE_SPEECH");
    }

    /// Shared with vertex.rs via `transform_gemini_request`; exercised here too since
    /// GoogleAiProvider is a distinct entry point a caller can hit directly.
    #[test]
    fn transform_request_logprobs_maps_to_response_logprobs() {
        let p = provider();
        let mut body = json!({
            "messages": [{"role": "user", "content": "hi"}],
            "logprobs": true,
            "top_logprobs": 3
        });

        p.transform_request(&mut body)
            .expect("transform_request should not fail");

        assert_eq!(body["generationConfig"]["responseLogprobs"], true);
        assert_eq!(body["generationConfig"]["logprobs"], 3);
    }

    #[test]
    fn transform_request_audio_and_web_search_options_dropped_not_forwarded() {
        let p = provider();
        let mut body = json!({
            "messages": [{"role": "user", "content": "hi"}],
            "audio": {"voice": "alloy", "format": "wav"},
            "web_search_options": {"search_context_size": "medium"}
        });

        p.transform_request(&mut body)
            .expect("transform_request should not fail");

        assert!(body.get("audio").is_none());
        assert!(body.get("web_search_options").is_none());
    }

    #[test]
    fn transform_request_cached_content() {
        let p = provider();
        let mut body = json!({
            "messages": [{"role": "user", "content": "hi"}],
            "extra_body": {
                "cached_content": "cachedContents/abc123"
            }
        });

        p.transform_request(&mut body)
            .expect("transform_request should not fail");

        assert_eq!(body["cachedContent"], "cachedContents/abc123");
    }

    #[test]
    fn transform_response_basic() {
        let p = provider();
        let mut body = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello from Google AI!"}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 4
            }
        });

        p.transform_response(&mut body)
            .expect("transform_response should not fail");

        assert_eq!(body["object"], "chat.completion");
        assert_eq!(body["choices"][0]["message"]["content"], "Hello from Google AI!");
        assert_eq!(body["choices"][0]["finish_reason"], "stop");
        assert_eq!(body["usage"]["prompt_tokens"], 5);
        assert_eq!(body["usage"]["completion_tokens"], 4);
    }

    #[test]
    fn parse_stream_event_empty_returns_none() {
        let p = provider();
        let result = p.parse_stream_event("").expect("parse should not fail");
        assert!(result.is_none());
    }

    #[test]
    fn parse_stream_event_basic_chunk() {
        let p = provider();
        let event_data = r#"{
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hi"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {"promptTokenCount": 3, "candidatesTokenCount": 1}
        }"#;

        let chunk = p
            .parse_stream_event(event_data)
            .expect("parse should not fail")
            .expect("should yield a chunk");

        assert_eq!(chunk.object, "chat.completion.chunk");
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hi"));
    }
}
