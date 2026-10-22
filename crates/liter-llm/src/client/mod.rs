/// Type-state builder for [`DefaultClient`] ([`ClientBuilder`]).
pub mod builder;
/// Client builder configuration ([`ClientConfig`] and related helpers).
pub mod config;
/// On-disk client configuration schema (TOML / JSON / YAML).
#[allow(missing_docs)]
pub mod config_file;
/// Canonical, binding-friendly [`LlmConfig`] and its conversion into [`ClientConfigBuilder`].
pub mod llm_config;
/// Tower-backed managed client wired with rate limit, cache, routing, etc.
#[cfg(all(feature = "native-http", feature = "tower"))]
pub mod managed;

use std::future::Future;
use std::pin::Pin;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use std::sync::Arc;

use futures_core::Stream;

use crate::error::{LiterLlmError, Result};
use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
use crate::types::batch::{BatchListQuery, BatchListResponse, BatchObject, CreateBatchRequest};
use crate::types::files::{CreateFileRequest, DeleteResponse, FileListQuery, FileListResponse, FileObject};
use crate::types::image::{CreateImageRequest, ImagesResponse};
use crate::types::moderation::{ModerationRequest, ModerationResponse};
use crate::types::ocr::{OcrRequest, OcrResponse};
use crate::types::raw::{RawExchange, RawStreamExchange};
use crate::types::rerank::{RerankRequest, RerankResponse};
use crate::types::responses::{CreateResponseRequest, ResponseObject, ResponseStreamEvent};
use crate::types::search::{SearchRequest, SearchResponse};
use crate::types::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ModelsListResponse, Usage,
};

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::auth::Credential;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::http;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::provider::{self, OpenAiCompatibleProvider, OpenAiProvider, Provider};
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use secrecy::ExposeSecret;

pub use builder::{ClientBuilder, NoApiKey, NoProvider, WithApiKey, WithProvider};
pub use config::{ClientConfig, ClientConfigBuilder};
pub use config_file::FileConfig;
pub use llm_config::{
    BedrockConfig, LlmBudgetConfig, LlmCacheConfig, LlmConfig, LlmInFlightLimitConfig, LlmProviderConfig,
    LlmRateLimitConfig,
};

use crate::types::batch::BatchStatus;
use std::time::Duration;

/// Configuration for polling a batch until terminal status.
///
/// All time values are in seconds as `f64` so the struct bridges across FFI
/// boundaries without requiring a `Duration` shim.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WaitForBatchConfig {
    /// Initial interval between polls, in seconds.
    pub initial_interval_secs: f64,
    /// Maximum interval between polls (backoff plateau), in seconds.
    pub max_interval_secs: f64,
    /// Exponential backoff multiplier (e.g., 1.5 increases delay by 50% each poll).
    pub backoff_multiplier: f32,
    /// Optional timeout in seconds — polling fails if this duration is exceeded.
    pub timeout_secs: Option<f64>,
}

impl Default for WaitForBatchConfig {
    fn default() -> Self {
        Self {
            initial_interval_secs: 5.0,
            max_interval_secs: 60.0,
            backoff_multiplier: 1.5,
            timeout_secs: None,
        }
    }
}

/// Error type for batch polling operations.
///
/// All fields use FFI-friendly types so the error can be represented across
/// every language binding without a shim. `Duration` fields are expressed
/// as `f64` seconds; `LiterLlmError` is flattened to `message` + `code`.
#[derive(Debug, thiserror::Error)]
pub enum BatchWaitError {
    /// Batch reached a terminal failure state.
    #[error("batch reached terminal failure state: {status:?}")]
    Failed {
        /// Terminal batch status (Failed, Expired, or Cancelled).
        status: BatchStatus,
    },

    /// Polling timed out before reaching terminal status.
    #[error("polling timed out after {timeout_secs:.1}s")]
    Timeout {
        /// Configured timeout in seconds.
        timeout_secs: f64,
    },

    /// Underlying client error, flattened to `message` + numeric `code`.
    #[error("client error (code {code}): {message}")]
    Client {
        /// Human-readable error description.
        message: String,
        /// Numeric error code (HTTP status, or 0 for non-HTTP errors).
        code: u32,
    },
}

impl From<LiterLlmError> for BatchWaitError {
    fn from(err: LiterLlmError) -> Self {
        Self::Client {
            code: u32::from(err.status_code()),
            message: err.to_string(),
        }
    }
}

/// A boxed future returning `T`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A boxed future returning `T` (WASM variant — not `Send` because JS is single-threaded).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A boxed stream of `T`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

/// A boxed stream of `T` (WASM variant — not `Send` because JS is single-threaded).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + 'a>>;

/// Result of [`DefaultClient::prepare_request`].
///
/// The body is pre-serialized into `bytes::Bytes` so it is serialized exactly
/// once — the same bytes are used for signing headers and for the HTTP request
/// body.  On retry, cloning `Bytes` is a zero-copy ref-count bump.
///
/// `body_json` is the pre-serialization JSON value, retained so that
/// [`Provider::dynamic_headers`] can inspect request fields without
/// re-parsing.
///
/// The `provider` is the resolved provider for this specific request — it may
/// differ from `self.provider` when the model prefix identifies a different
/// provider.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
struct PreparedRequest {
    url: String,
    provider: Arc<dyn Provider>,
    body_json: serde_json::Value,
    body_bytes: bytes::Bytes,
}

/// Convert an owned `(String, String)` auth header pair to `(&str, &str)` borrows.
///
/// Centralises the four identical `map(|(n, v)| (n.as_str(), v.as_str()))` expressions
/// that appear wherever we hand headers to the HTTP layer.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn str_pair(pair: &(String, String)) -> (&str, &str) {
    (pair.0.as_str(), pair.1.as_str())
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn provider_auth_header(provider: &dyn Provider, api_key: &str) -> Option<(String, String)> {
    (!api_key.is_empty())
        .then(|| provider.auth_header(api_key))
        .flatten()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[derive(Default)]
struct ChatStreamState {
    usage: Option<Usage>,
    model: String,
    /// Tool calls already streamed, so later chunks continue the index numbering.
    tool_calls_seen: u32,
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn attribute_chat_model(provider: &dyn Provider, model: &mut String, fallback_model: &str) {
    if model.is_empty() && matches!(provider.name(), "google_ai" | "vertex_ai") {
        tracing::debug!(
            provider = provider.name(),
            fallback_model,
            "Gemini omitted modelVersion; using request or prior stream model context"
        );
        *model = fallback_model.to_owned();
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn parse_chat_stream_event(
    provider: &dyn Provider,
    state: &std::sync::Mutex<ChatStreamState>,
    data: &str,
) -> Result<Option<ChatCompletionChunk>> {
    let mut chunk = match provider.parse_stream_event(data)? {
        Some(chunk) => chunk,
        None => return Ok(None),
    };
    if provider.stream_tool_call_indexes_restart() {
        let mut state = state.lock().map_err(|_| LiterLlmError::InternalError {
            message: "chat stream state lock poisoned".into(),
        })?;
        provider::ollama::number_stream_tool_calls(&mut chunk, &mut state.tool_calls_seen);
        return Ok(Some(chunk));
    }
    if !matches!(provider.name(), "anthropic" | "google_ai" | "vertex_ai") {
        return Ok(Some(chunk));
    }
    let mut state = state.lock().map_err(|_| LiterLlmError::InternalError {
        message: "chat stream state lock poisoned".into(),
    })?;
    attribute_chat_model(provider, &mut chunk.model, &state.model);
    if matches!(provider.name(), "google_ai" | "vertex_ai") {
        state.model.clone_from(&chunk.model);
        return Ok(Some(chunk));
    }
    if let Some(usage) = chunk.usage.as_mut() {
        let aggregate = state.usage.get_or_insert_with(Usage::default);
        aggregate.prompt_tokens = aggregate.prompt_tokens.max(usage.prompt_tokens);
        aggregate.completion_tokens = aggregate.completion_tokens.max(usage.completion_tokens);
        aggregate.total_tokens = aggregate.prompt_tokens + aggregate.completion_tokens;
        if usage.prompt_tokens_details.is_some() {
            aggregate.prompt_tokens_details.clone_from(&usage.prompt_tokens_details);
        }
        if usage.completion_tokens_details.is_some() {
            aggregate
                .completion_tokens_details
                .clone_from(&usage.completion_tokens_details);
        }
        usage.clone_from(aggregate);
    }
    Ok(Some(chunk))
}

/// Whether a chat chunk ends the reply: some choice carries a finish reason.
///
/// An SSE chat stream ends with `[DONE]` or with such a chunk. A stream that
/// closes with neither was cut.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) fn chunk_has_finish_reason(chunk: &ChatCompletionChunk) -> bool {
    chunk.choices.iter().any(|choice| choice.finish_reason.is_some())
}

/// Whether a Responses event ends the reply: `response.completed`,
/// `response.incomplete` or `response.failed`.
///
/// A Responses stream ends with `[DONE]` or with such an event. A stream that
/// closes with neither was cut.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
pub(crate) fn response_event_is_terminal(event: &ResponseStreamEvent) -> bool {
    matches!(
        event,
        ResponseStreamEvent::Completed(_) | ResponseStreamEvent::Incomplete(_) | ResponseStreamEvent::Failed(_)
    )
}

/// Whether a provider's native chat stream protocol always ends with an event
/// that carries a finish reason.
///
/// Anthropic (`message_delta` with `stop_reason`), Gemini/Vertex
/// (`finishReason`), Cohere (`message-end`), Ollama's native API (`done: true`) and
/// Bedrock (`messageStop`) all do,
/// so a stream from them that ends without one was cut off. OpenAI-compatible
/// servers may end with `[DONE]` alone, so this wrapper does not hold them to a
/// finish reason; the SSE reader reports one that closes with neither (#356).
/// The reader's check is off for the providers this returns `true` for, so the
/// cut stream of a native protocol keeps the error that names the provider.
///
/// Ollama is found by the capability its native provider declares and Bedrock by its event
/// stream framing, so a custom provider that borrows either name keeps the lenient rule.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn requires_terminal_chat_chunk(provider: &dyn Provider) -> bool {
    matches!(provider.name(), "anthropic" | "google_ai" | "vertex_ai" | "cohere")
        || provider.stream_has_final_event()
        || matches!(provider.stream_format(), provider::StreamFormat::AwsEventStream)
}

/// Turns a clean end of a chat stream into [`LiterLlmError::Streaming`] when no
/// chunk carried a finish reason, so a response truncated by a dropped
/// connection, proxy timeout or upstream crash is not reported as complete.
///
/// A stream that already yielded an error is passed through unchanged.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
struct RequireTerminalChunk {
    inner: BoxStream<'static, Result<ChatCompletionChunk>>,
    provider: String,
    finished: bool,
    errored: bool,
    ended: bool,
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl RequireTerminalChunk {
    fn wrap(
        inner: BoxStream<'static, Result<ChatCompletionChunk>>,
        provider: String,
    ) -> BoxStream<'static, Result<ChatCompletionChunk>> {
        Box::pin(Self {
            inner,
            provider,
            finished: false,
            errored: false,
            ended: false,
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl Stream for RequireTerminalChunk {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        if self.ended {
            return Poll::Ready(None);
        }
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                if chunk_has_finish_reason(&chunk) {
                    self.finished = true;
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(error))) => {
                self.errored = true;
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                self.ended = true;
                if self.finished || self.errored {
                    return Poll::Ready(None);
                }
                tracing::warn!(
                    provider = %self.provider,
                    "chat stream ended before the provider sent its final event"
                );
                Poll::Ready(Some(Err(LiterLlmError::Streaming {
                    message: format!(
                        "{} stream ended before its final event; the response is incomplete",
                        self.provider
                    ),
                })))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Shallow-merge a top-level `"extra_body"` key into `body`, OpenAI-Python-SDK
/// style: `{**body, **extra_body}` — extra_body keys override identically
/// named top-level keys.
///
/// On the chat path, providers that consume `extra_body` themselves (Anthropic,
/// Vertex, Bedrock) already strip it inside their own `transform_request`, so by the
/// time this runs there is nothing left for them to merge; there it only has an
/// effect for OpenAI-compatible providers, which pass `extra_body` through
/// unchanged. The Bedrock and Vertex embed transforms differ: they rebuild the body
/// and keep `extra_body` on it, so this merge applies the caller's keys. The Ollama
/// embed transform consumes them itself: top-level keys stay, the rest go into `options`.
/// The Gemini API embed transform merges them into each embed request itself. The Responses path
/// ([`response_request_body`]) runs no provider transform at all, so the merged keys
/// always reach the wire as literal OpenAI Responses fields.
///
/// A non-object `extra_body` cannot be merged into the body root, so it is dropped
/// with a warning rather than sent to the wire.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn merge_extra_body(body: &mut serde_json::Value) {
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    let Some(extra_body) = obj.remove("extra_body") else {
        return;
    };
    match extra_body {
        serde_json::Value::Object(extra_fields) => {
            obj.extend(extra_fields);
        }
        other => {
            tracing::warn!(
                extra_body_type = json_value_type_name(&other),
                "ignoring non-object extra_body; it cannot be merged into the request body"
            );
        }
    }
}

/// Serialize a Responses API request body with `extra_body` merged in,
/// via [`merge_extra_body`] — the same merge semantics as the chat path.
///
/// The merge is the *only* thing shared with the chat path. There is deliberately no
/// [`Provider::transform_request`] call here: the Responses body is a different wire
/// shape from the chat body, and the provider transforms are written against the chat
/// shape, so applying them would mangle the request rather than adapt it. The Responses
/// path is OpenAI-only — see [`ResponseClient`](trait@ResponseClient).
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn response_request_body(req: &CreateResponseRequest) -> Result<serde_json::Value> {
    let mut body = serde_json::to_value(req)?;
    merge_extra_body(&mut body);
    Ok(body)
}

/// Like [`response_request_body`], but for `create_response_stream`: forces
/// `stream: true` in the request, both before and after the `extra_body`
/// merge.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn response_stream_request_body(mut req: CreateResponseRequest) -> Result<serde_json::Value> {
    req.stream = Some(true);
    let mut body = response_request_body(&req)?;
    // ~keep extra_body must never override `stream`: the transport path (post_json_raw vs
    // post_stream) is chosen by the calling method, not the body, so a mismatched wire flag
    // would desync request from response parsing.
    if let Some(obj) = body.as_object_mut() {
        obj.insert("stream".into(), serde_json::Value::Bool(true));
    }
    Ok(body)
}

/// Human-readable JSON type name, used only for diagnostic tracing fields.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn json_value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Core LLM client trait.
///
/// Provides unified access to LLM and multimodal APIs across 174 providers.
/// Requests are routed to the correct provider based on the model name prefix
/// (e.g. `anthropic/claude-3-5-sonnet` routes to Anthropic) or via explicit
/// `base_url` override.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait LlmClient: Send + Sync {
    /// Send a chat completion request.
    ///
    /// Routes the request to the detected provider based on the model prefix
    /// in the `ChatCompletionRequest`. Provider-specific transformations
    /// (request normalization, header signing) are applied automatically.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the model is empty.
    /// Returns `LiterLlmError::Authentication` if credentials are missing or invalid.
    /// Returns `LiterLlmError::Http` for network or HTTP-level errors.
    /// Returns `LiterLlmError::ProviderError` if the provider rejects the request.
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>>;

    /// Send a streaming chat completion request.
    ///
    /// Returns a stream of `ChatCompletionChunk` items, each representing a
    /// single token delta from the provider. The stream terminates when the
    /// model reaches `stop_reason = "stop"` or `"length"`.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`chat`](Self::chat).
    /// Stream errors are returned as `Err` items in the stream itself.
    ///
    /// # Notes
    ///
    /// Chunks are yielded as soon as they arrive; the stream is not buffered.
    fn chat_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>>;

    /// Send an embedding request.
    ///
    /// Computes dense vector representations for semantic search, clustering, or similarity.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if input is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>>;

    /// List available models.
    ///
    /// Queries the provider's model list endpoint.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::Http` for network errors.
    /// Returns `LiterLlmError::Authentication` if the API key lacks list permissions.
    fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>>;

    /// Generate an image.
    ///
    /// Creates one or more images based on a text prompt.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the prompt is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    /// Returns `LiterLlmError::ProviderError` if the prompt violates content policy.
    fn image_generate(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>>;

    /// Generate speech audio from text.
    ///
    /// Converts text to speech (TTS) using the specified voice model.
    /// Returns raw audio bytes in the requested format.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if text is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn speech(&self, req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>>;

    /// Transcribe audio to text.
    ///
    /// Converts audio files to text using automatic speech recognition (ASR).
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the audio file is missing.
    /// Returns `LiterLlmError::Http` for network errors.
    fn transcribe(&self, req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>>;

    /// Check content against moderation policies.
    ///
    /// Evaluates text or images for potentially harmful content.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if input is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn moderate(&self, req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>>;

    /// Rerank documents by relevance to a query.
    ///
    /// Orders a list of documents by their relevance to a search query.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if query or documents are empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn rerank(&self, req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>>;

    /// Perform a web/document search.
    ///
    /// Searches the web or a provider's document index for results matching the query.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the query is empty.
    /// Returns `LiterLlmError::Http` for network errors.
    fn search(&self, req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>>;

    /// Extract text from a document via OCR.
    ///
    /// Performs optical character recognition (OCR) on images or scanned PDFs.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::BadRequest` if the document is missing.
    /// Returns `LiterLlmError::Http` for network errors.
    fn ocr(&self, req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>>;
}

/// Core LLM client trait (WASM variant — no `Send + Sync` because JS is single-threaded).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait LlmClient {
    /// Send a chat completion request.
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>>;

    /// Send a streaming chat completion request.
    fn chat_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>>;

    /// Send an embedding request.
    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>>;

    /// List available models.
    fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>>;

    /// Generate an image.
    fn image_generate(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>>;

    /// Generate speech audio from text.
    fn speech(&self, req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>>;

    /// Transcribe audio to text.
    fn transcribe(&self, req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>>;

    /// Check content against moderation policies.
    fn moderate(&self, req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>>;

    /// Rerank documents by relevance to a query.
    fn rerank(&self, req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>>;

    /// Perform a web/document search.
    fn search(&self, req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>>;

    /// Extract text from a document via OCR.
    fn ocr(&self, req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>>;
}

/// Extension of [`LlmClient`] that returns raw request/response data
/// alongside the typed response.
///
/// Every `_raw` method mirrors its counterpart on [`LlmClient`] but wraps the
/// result in a [`RawExchange`] that exposes the final request body (after
/// `transform_request`) and the raw provider response (before
/// `transform_response`). This is useful for debugging provider-specific
/// transformations, capturing wire-level data, or implementing custom parsing.
#[cfg_attr(alef, alef(skip))]
pub trait LlmClientRaw: LlmClient {
    /// Send a chat completion request and return the raw exchange.
    ///
    /// The `raw_request` field contains the final JSON body sent to the
    /// provider; `raw_response` contains the provider JSON before
    /// normalization.
    fn chat_raw(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<RawExchange<ChatCompletionResponse>>>;

    /// Send a streaming chat completion request and return the raw exchange.
    ///
    /// Only `raw_request` is available upfront — the stream itself is
    /// returned in `stream` and consumed incrementally.
    fn chat_stream_raw(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<RawStreamExchange<BoxStream<'static, Result<ChatCompletionChunk>>>>>;

    /// Send an embedding request and return the raw exchange.
    fn embed_raw(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<RawExchange<EmbeddingResponse>>>;

    /// Generate an image and return the raw exchange.
    fn image_generate_raw(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<RawExchange<ImagesResponse>>>;

    /// Transcribe audio to text and return the raw exchange.
    fn transcribe_raw(
        &self,
        req: CreateTranscriptionRequest,
    ) -> BoxFuture<'_, Result<RawExchange<TranscriptionResponse>>>;

    /// Check content against moderation policies and return the raw exchange.
    fn moderate_raw(&self, req: ModerationRequest) -> BoxFuture<'_, Result<RawExchange<ModerationResponse>>>;

    /// Rerank documents by relevance to a query and return the raw exchange.
    fn rerank_raw(&self, req: RerankRequest) -> BoxFuture<'_, Result<RawExchange<RerankResponse>>>;

    /// Perform a web/document search and return the raw exchange.
    fn search_raw(&self, req: SearchRequest) -> BoxFuture<'_, Result<RawExchange<SearchResponse>>>;

    /// Extract text from a document via OCR and return the raw exchange.
    fn ocr_raw(&self, req: OcrRequest) -> BoxFuture<'_, Result<RawExchange<OcrResponse>>>;
}

/// File management operations (upload, list, retrieve, delete).
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait FileClient: Send + Sync {
    /// Upload a file.
    fn create_file(&self, req: CreateFileRequest) -> BoxFuture<'_, Result<FileObject>>;

    /// Retrieve metadata for a file.
    fn retrieve_file(&self, file_id: &str) -> BoxFuture<'_, Result<FileObject>>;

    /// Delete a file.
    fn delete_file(&self, file_id: &str) -> BoxFuture<'_, Result<DeleteResponse>>;

    /// List files, optionally filtered by query parameters.
    fn list_files(&self, query: Option<FileListQuery>) -> BoxFuture<'_, Result<FileListResponse>>;

    /// Retrieve the raw content of a file.
    fn file_content(&self, file_id: &str) -> BoxFuture<'_, Result<bytes::Bytes>>;
}

/// File management operations (upload, list, retrieve, delete) (WASM variant).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait FileClient {
    /// Upload a file.
    fn create_file(&self, req: CreateFileRequest) -> BoxFuture<'_, Result<FileObject>>;

    /// Retrieve metadata for a file.
    fn retrieve_file(&self, file_id: &str) -> BoxFuture<'_, Result<FileObject>>;

    /// Delete a file.
    fn delete_file(&self, file_id: &str) -> BoxFuture<'_, Result<DeleteResponse>>;

    /// List files, optionally filtered by query parameters.
    fn list_files(&self, query: Option<FileListQuery>) -> BoxFuture<'_, Result<FileListResponse>>;

    /// Retrieve the raw content of a file.
    fn file_content(&self, file_id: &str) -> BoxFuture<'_, Result<bytes::Bytes>>;
}

/// Batch processing operations (create, list, retrieve, cancel).
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait BatchClient: Send + Sync {
    /// Create a new batch job.
    fn create_batch(&self, req: CreateBatchRequest) -> BoxFuture<'_, Result<BatchObject>>;

    /// Retrieve a batch by ID.
    fn retrieve_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>>;

    /// List batches, optionally filtered by query parameters.
    fn list_batches(&self, query: Option<BatchListQuery>) -> BoxFuture<'_, Result<BatchListResponse>>;

    /// Cancel an in-progress batch.
    fn cancel_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>>;
}

/// Batch processing operations (create, list, retrieve, cancel) (WASM variant).
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait BatchClient {
    /// Create a new batch job.
    fn create_batch(&self, req: CreateBatchRequest) -> BoxFuture<'_, Result<BatchObject>>;

    /// Retrieve a batch by ID.
    fn retrieve_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>>;

    /// List batches, optionally filtered by query parameters.
    fn list_batches(&self, query: Option<BatchListQuery>) -> BoxFuture<'_, Result<BatchListResponse>>;

    /// Cancel an in-progress batch.
    fn cancel_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>>;
}

/// OpenAI Responses API operations (create, retrieve, cancel).
///
/// # Provider support: OpenAI only
///
/// Every other client trait in this module is provider-agnostic — requests go through
/// a shared `prepare_request` step that resolves the provider from the model prefix,
/// strips that prefix, and applies [`Provider::transform_request`] /
/// [`Provider::transform_response`]. **This trait does none of that.** The
/// implementation sends [`CreateResponseRequest`] to
/// `{base_url}` + [`Provider::responses_path`] verbatim and deserializes the reply
/// straight into [`ResponseObject`], so both directions assume the OpenAI Responses
/// wire format.
///
/// Consequently:
///
/// - No provider overrides [`Provider::responses_path`], and no entry in
///   `schemas/providers.json` declares a `responses` endpoint — provider Responses
///   support is not modeled anywhere in the registry.
/// - Requests stay pinned to the provider the client was constructed with. A
///   `provider/model` prefix in [`CreateResponseRequest::model`] is neither stripped
///   nor used for routing; it travels to the wire intact.
/// - Providers that rewrite the request or normalize the response for chat (Anthropic,
///   Vertex, Bedrock, Cohere, Google AI) get no such chance here. Their transforms are
///   written against the Chat Completions body shape and would corrupt a Responses body,
///   which is why they are not applied rather than merely forgotten.
/// - Bedrock, Vertex, and Google AI embed the model into `build_url`, but only for the
///   `chat/completions` and `embeddings` paths they special-case; `responses_path()` falls
///   through to their plain `{base}{endpoint_path}` branch, so the empty model passed here is
///   harmless for them. Azure embeds the model unconditionally, so the empty model yields
///   `.../openai/deployments//responses` — an empty path segment. The four call sites below
///   detect that shape via `reject_malformed_responses_url` and fail fast with
///   [`LiterLlmError::EndpointNotSupported`] instead of sending it.
///
/// Use these methods with OpenAI, or with a gateway that serves OpenAI's `/responses`
/// contract natively. For cross-provider work use [`LlmClient::chat`].
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub trait ResponseClient: Send + Sync {
    /// Create a new response.
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Create a new response and stream its SSE events as they arrive.
    ///
    /// Returns a stream of `ResponseStreamEvent` items. The stream terminates
    /// when a terminal event (`response.completed`, `response.incomplete`, or
    /// `response.failed`) is yielded and the underlying connection closes.
    /// If the connection closes before a terminal event, the stream yields one
    /// `Streaming` error after the last event.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`create_response`](Self::create_response).
    /// Stream errors are returned as `Err` items in the stream itself.
    fn create_response_stream(
        &self,
        req: CreateResponseRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ResponseStreamEvent>>>>;

    /// Retrieve a response by ID.
    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Cancel an in-progress response.
    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;
}

/// OpenAI Responses API operations (create, retrieve, cancel) (WASM variant).
///
/// # Provider support: OpenAI only
///
/// See the non-WASM `ResponseClient` for the full rationale. In short: this path sends
/// `CreateResponseRequest` verbatim and parses the reply as the OpenAI Responses wire
/// format. It applies no provider request/response transform, performs no model-prefix
/// routing, and no provider in `schemas/providers.json` declares a `responses`
/// endpoint. Use the chat path for cross-provider work.
#[cfg(target_arch = "wasm32")]
#[cfg_attr(alef, alef(skip))]
pub trait ResponseClient {
    /// Create a new response.
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Create a new response and stream its SSE events as they arrive.
    ///
    /// Returns a stream of `ResponseStreamEvent` items. The stream terminates
    /// when a terminal event (`response.completed`, `response.incomplete`, or
    /// `response.failed`) is yielded and the underlying connection closes.
    /// If the connection closes before a terminal event, the stream yields one
    /// `Streaming` error after the last event.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`create_response`](Self::create_response).
    /// Stream errors are returned as `Err` items in the stream itself.
    fn create_response_stream(
        &self,
        req: CreateResponseRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ResponseStreamEvent>>>>;

    /// Retrieve a response by ID.
    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;

    /// Cancel an in-progress response.
    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>>;
}

/// Default client implementation backed by `reqwest`.
///
/// Sends requests to 174 LLM providers with automatic provider detection
/// and per-request routing. The provider is resolved at construction time
/// from `model_hint` (or defaults to OpenAI), but individual requests can
/// override the provider via model name prefix (e.g. `"anthropic/claude-3-5-sonnet"`
/// routes to Anthropic regardless of construction-time setting).
///
/// When the model prefix does not match any known provider, the construction-time
/// provider is used as the fallback. This enables seamless migration between
/// providers by changing only the model name.
///
/// The provider is stored behind an [`Arc`] so it can be shared cheaply into
/// async closures and streaming tasks. Pre-computed auth headers and extra
/// headers are cached at construction to avoid redundant encoding on every request.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[derive(Clone)]
pub struct DefaultClient {
    config: ClientConfig,
    http: reqwest::Client,
    /// Client for streaming calls: no whole-request deadline, `timeout` limits the gap between reads.
    http_stream: reqwest::Client,
    /// Provider resolved at construction; shared via Arc so streaming closures
    /// can capture an owned reference without requiring `unsafe`.
    provider: Arc<dyn Provider>,
    /// Pre-computed auth header `(name, value)` — avoids `format!("Bearer {key}")`
    /// on every request.  `None` when the provider requires no authentication.
    cached_auth_header: Option<(String, String)>,
    /// Pre-computed static extra headers — avoids converting `&'static str` pairs
    /// to `(String, String)` on every request.
    cached_extra_headers: Vec<(String, String)>,
    /// The OpenAI-compatible endpoint at `base_url` for a model with another provider's
    /// prefix, on a client whose hint named Ollama. `None` for every other client.
    prefixed_model_provider: Option<Arc<dyn Provider>>,
    /// Tower middleware stack installed by [`create_client_from_json`](crate::create_client_from_json).
    ///
    /// ~keep The managed client wraps a separate `DefaultClient` whose own `managed` is `None`,
    /// so delegation never recurses.
    #[cfg_attr(not(all(feature = "native-http", feature = "tower")), allow(dead_code))]
    managed: Option<ManagedHandle>,
}

/// Handle to the managed middleware stack; uninhabited when the stack is not compiled in.
#[cfg(all(feature = "native-http", feature = "tower"))]
type ManagedHandle = Arc<managed::ManagedClient>;
#[cfg(all(
    any(feature = "native-http", feature = "wasm-http"),
    not(all(feature = "native-http", feature = "tower"))
))]
type ManagedHandle = std::convert::Infallible;

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl DefaultClient {
    /// Route the [`LlmClient`] methods through `managed`'s middleware stack.
    #[cfg(all(feature = "native-http", feature = "tower"))]
    pub(crate) fn with_managed(mut self, managed: Arc<managed::ManagedClient>) -> Self {
        self.managed = Some(managed);
        self
    }

    fn response_read_options(&self) -> http::request::ResponseReadOptions {
        http::request::ResponseReadOptions {
            max_retries: self.config.max_retries,
            max_response_bytes: self.config.max_response_bytes,
            timeout: Some(self.config.timeout),
        }
    }

    /// Build a client.
    ///
    /// Constructs an HTTP client with the given configuration and provider hint.
    /// If `model_hint` is provided, its prefix determines the default provider
    /// (e.g. `"groq/llama3-70b"` selects Groq; `"claude-3-5-sonnet"` defaults to OpenAI).
    /// Pass `None` to use OpenAI as the default. The hint does not constrain
    /// per-request routing — individual requests can override the provider via
    /// their own model prefix.
    ///
    /// When `config.load_env` is true (the default), and no API key was provided,
    /// the client reads the provider's designated environment variable
    /// (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`) at construction time.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::Authentication` if `load_env` is enabled, no explicit
    /// API key was provided, and the provider's environment variable is unset or empty.
    /// Returns `LiterLlmError::BadRequest` if no `base_url` is set and `model_hint` names a
    /// registry provider that has no route of its own.
    /// Returns `LiterLlmError::BadRequest` if `max_response_bytes` or `connect_timeout` is zero.
    /// Returns `LiterLlmError::InvalidHeader` if pre-validated headers somehow
    /// become invalid during client construction (extremely rare; indicates a bug).
    /// Returns `LiterLlmError::Http` if the underlying HTTP client cannot be constructed.
    pub fn new(config: ClientConfig, model_hint: Option<&str>) -> Result<Self> {
        config::validate_response_limit(config.max_response_bytes)?;
        config::validate_connect_timeout(config.connect_timeout)?;
        let provider = build_provider(&config, model_hint)?;
        provider.validate()?;
        let prefixed_model_provider = prefixed_model_provider(&config, model_hint);

        #[cfg(not(target_arch = "wasm32"))]
        let mut config = config;
        #[cfg(not(target_arch = "wasm32"))]
        if config.load_env
            && config.api_key.expose_secret().is_empty()
            && let Some(env_var_name) = provider.env_var()
        {
            match std::env::var(env_var_name) {
                Ok(val) if !val.is_empty() => {
                    config.api_key = secrecy::SecretString::from(val);
                }
                _ => {
                    return Err(LiterLlmError::Authentication {
                        message: format!("no API key provided and environment variable {env_var_name} is not set"),
                        status: 401,
                    });
                }
            }
        }

        // ~keep Auto-install Vertex ADC only when Vertex AI has no explicit api_key or credential_provider.
        // ~keep Explicit tokens and credential providers stay authoritative over Workload Identity defaults.
        #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
        if config.credential_provider.is_none()
            && config.api_key.expose_secret().is_empty()
            && provider.name() == "vertex_ai"
        {
            config.credential_provider = Some(Arc::new(crate::auth::vertex_adc::VertexAdcCredentialProvider::new()));
        }

        let mut header_map = reqwest::header::HeaderMap::new();
        for (k, v) in config.headers() {
            let name =
                reqwest::header::HeaderName::from_bytes(k.as_bytes()).map_err(|_| LiterLlmError::InvalidHeader {
                    name: k.clone(),
                    reason: "pre-validated header name became invalid".into(),
                })?;
            let val = reqwest::header::HeaderValue::from_str(v).map_err(|_| LiterLlmError::InvalidHeader {
                name: k.clone(),
                reason: "pre-validated header value became invalid".into(),
            })?;
            header_map.insert(name, val);
        }

        #[cfg(feature = "native-http")]
        crate::ensure_crypto_provider();
        // ~keep Two clients share one config: `timeout` is a whole-request deadline for calls that
        // ~keep return one reply, and a limit on the gap between reads for streams. A whole-request
        // ~keep deadline would end a healthy stream whose total time passes it.
        let build_http = |stream: bool| -> Result<reqwest::Client> {
            let builder = reqwest::Client::builder().default_headers(header_map.clone());
            #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
            let builder = crate::provider::configure_outbound_client_builder(builder, config.transport.dns_cache_ttl);
            #[cfg(not(target_arch = "wasm32"))]
            let builder = builder.connect_timeout(config.connect_timeout);
            #[cfg(not(target_arch = "wasm32"))]
            let builder = if stream {
                builder.read_timeout(config.timeout)
            } else {
                builder.timeout(config.timeout)
            };
            // ~keep The wasm32 client has no builder timeout: `http::deadline` applies `timeout` per call.
            #[cfg(target_arch = "wasm32")]
            let _ = stream;
            #[cfg(not(target_arch = "wasm32"))]
            let builder = config.transport.apply_to_builder(builder);
            builder.build().map_err(LiterLlmError::from)
        };
        let http = build_http(false)?;
        let http_stream = build_http(true)?;

        let cached_auth_header = provider_auth_header(provider.as_ref(), config.api_key.expose_secret());
        let cached_extra_headers = provider
            .extra_headers()
            .iter()
            .map(|&(name, value)| (name.to_owned(), value.to_owned()))
            .collect();

        Ok(Self {
            config,
            http,
            http_stream,
            provider,
            cached_auth_header,
            cached_extra_headers,
            prefixed_model_provider,
            managed: None,
        })
    }

    /// Resolve the provider for a specific request based on the model string.
    ///
    /// If the model prefix clearly identifies a provider that differs from the
    /// construction-time default, the detected provider is returned.  Otherwise
    /// the construction-time provider is reused (zero allocation).
    fn resolve_provider_for_model(&self, model: &str) -> Result<Arc<dyn Provider>> {
        if let Some(base_url) = self.config.effective_base_url() {
            // ~keep The `ollama/` prefix names a wire format, not a host, so it wins over a base_url
            // override: the override then becomes the Ollama server address. Without an override the
            // prefix goes through `detect_provider`, where a registered custom provider wins.
            if provider::ollama::OllamaProvider::owns(model) {
                return Ok(Arc::new(provider::ollama::OllamaProvider::with_base_url(
                    base_url.to_owned(),
                )));
            }
            // ~keep Only a bare model goes to the native API of a client hinted at Ollama. A model
            // with another provider's prefix goes to the OpenAI-compatible endpoint at the base_url,
            // as it did before the native provider existed.
            if let Some(compatible) = &self.prefixed_model_provider
                && provider::has_provider_prefix(model)
            {
                return Ok(Arc::clone(compatible));
            }
        }
        // ~keep A base_url override pins the construction-time provider for mock/proxy endpoints. A
        // client hinted at Ollama is pinned to the native API like any other provider: a bare model
        // goes to `/api/chat` whatever the base_url is.
        if self.config.effective_base_url().is_some() {
            return Ok(Arc::clone(&self.provider));
        }
        // ~keep Keep the construction-time provider when it already matches the model.
        if self.provider.matches_model(model) {
            return Ok(Arc::clone(&self.provider));
        }
        if model.starts_with("bedrock/") {
            return Ok(build_bedrock_provider(&self.config));
        }
        if let Some(detected) = provider::route_model(model)? {
            return Ok(Arc::from(detected));
        }
        Ok(Arc::clone(&self.provider))
    }

    /// Refuse a model that names a registry provider with no route.
    ///
    /// For endpoints that always use the construction-time provider (the Responses API),
    /// so a model for another provider is not sent to that provider's endpoint.
    /// Does nothing when a `base_url` is set.
    fn refuse_unrouted_model(&self, model: &str) -> Result<()> {
        if self.config.effective_base_url().is_none() {
            provider::route_model(model)?;
        }
        Ok(())
    }

    /// Compute the auth header for a given provider (potentially different from
    /// the construction-time cached one).
    async fn resolve_auth_header_for_provider(
        &self,
        prov: &dyn Provider,
    ) -> Result<(Option<(String, String)>, Option<Credential>)> {
        if let Some(ref cp) = self.config.credential_provider {
            let credential = cp.resolve().await?;
            let header = match &credential {
                Credential::BearerToken(token) if token.expose_secret().is_empty() => None,
                Credential::BearerToken(token) => {
                    Some(("Authorization".to_owned(), format!("Bearer {}", token.expose_secret())))
                }
                Credential::AwsCredentials { .. } => None,
            };
            Ok((header, Some(credential)))
        } else {
            // ~keep Auth headers must be recomputed after per-request provider resolution.
            Ok((provider_auth_header(prov, self.config.api_key.expose_secret()), None))
        }
    }

    /// Build the combined header list for a request using a specific provider.
    ///
    /// # Errors
    ///
    /// Propagates a signing failure from [`Provider::signing_headers`] (e.g. an
    /// internal Bedrock SigV4 error) instead of sending an unsigned request.
    fn all_headers_for_provider(
        &self,
        prov: &dyn Provider,
        credential: Option<&Credential>,
        method: &str,
        url: &str,
        body_json: &serde_json::Value,
        body_bytes: &[u8],
    ) -> Result<Vec<(String, String)>> {
        let mut headers = prov.signing_headers_with_credential(method, url, body_bytes, credential)?;
        headers.extend(
            prov.extra_headers()
                .iter()
                .map(|&(name, value)| (name.to_owned(), value.to_owned())),
        );
        headers.extend(prov.dynamic_headers(body_json));
        Ok(headers)
    }

    /// Shared helper: resolve the per-request provider, build the URL, strip
    /// model prefix from the request body, set the `stream` flag, apply provider
    /// transform, and return everything needed to fire a request.
    ///
    /// `endpoint_fn` receives the resolved provider and returns the endpoint
    /// path (e.g. `|p| p.chat_completions_path()`), ensuring the path comes
    /// from the correct provider when per-request routing overrides the default.
    ///
    /// `stream` is inserted into the body **before** `transform_request` runs,
    /// so providers can inspect the final body state in one pass.
    fn prepare_request(
        &self,
        serializable: &impl serde::Serialize,
        endpoint_fn: impl FnOnce(&dyn Provider) -> &str,
        model: &str,
        stream: Option<bool>,
    ) -> Result<PreparedRequest> {
        self.prepare_request_with(
            serializable,
            endpoint_fn,
            |p, body| p.transform_request(body),
            model,
            stream,
            |_, _| Ok(()),
        )
    }

    /// Prepare an embedding request. The provider rebuilds the body in
    /// [`Provider::transform_embedding_request`], chosen by the endpoint being called.
    fn prepare_embedding_request(&self, req: &EmbeddingRequest) -> Result<PreparedRequest> {
        self.prepare_request_with(
            req,
            |p| p.embeddings_path(),
            |p, body| p.transform_embedding_request(body),
            &req.model,
            None,
            |_, _| Ok(()),
        )
    }

    /// [`Self::prepare_request`] with the provider's body transform for the endpoint chosen by
    /// the caller, and a request step for one endpoint.
    ///
    /// `transform_fn` is the provider's own body transform: `transform_request`, or the
    /// embedding hook for the embeddings endpoint. `endpoint_step` runs right after it, so a
    /// rerank request can be reworded without any other endpoint seeing the change.
    fn prepare_request_with(
        &self,
        serializable: &impl serde::Serialize,
        endpoint_fn: impl FnOnce(&dyn Provider) -> &str,
        transform_fn: impl FnOnce(&dyn Provider, &mut serde_json::Value) -> Result<()>,
        model: &str,
        stream: Option<bool>,
        endpoint_step: impl FnOnce(&dyn Provider, &mut serde_json::Value) -> Result<()>,
    ) -> Result<PreparedRequest> {
        if model.is_empty() {
            return Err(LiterLlmError::BadRequest {
                message: "model must not be empty".into(),
                status: 400,
            });
        }

        let prov = self.resolve_provider_for_model(model)?;
        let bare_model = prov.strip_model_prefix(model).to_owned();
        // ~keep Providers such as Azure and Bedrock embed model/deployment identifiers in URLs.
        let endpoint_path = endpoint_fn(prov.as_ref());
        let url = prov.build_url(endpoint_path, &bare_model);

        let mut body = serde_json::to_value(serializable)?;
        if let Some(obj) = body.as_object_mut() {
            obj.insert("model".into(), serde_json::Value::String(bare_model.clone()));
            if let Some(s) = stream {
                obj.insert("stream".into(), serde_json::Value::Bool(s));
                if s && matches!(prov.name(), "openai" | "azure" | "custom") && !obj.contains_key("stream_options") {
                    obj.insert("stream_options".into(), serde_json::json!({"include_usage": true}));
                }
            }
            if matches!(prov.name(), "openai" | "azure")
                && obj.get("max_completion_tokens").is_none()
                && obj.get("max_tokens").is_some()
                && model_uses_max_completion_tokens(&bare_model)
                && let Some(max_tokens) = obj.remove("max_tokens")
            {
                obj.insert("max_completion_tokens".into(), max_tokens);
            }
        }
        transform_fn(prov.as_ref(), &mut body)?;
        endpoint_step(prov.as_ref(), &mut body)?;
        // ~keep Whether the provider's wire format carries `stream` at all is the provider's
        // decision, recorded here by whether the key survived `transform_request`. Providers that
        // rebuild the body into a native format (Vertex/Gemini `generateContent`, Bedrock
        // `converse`) drop it deliberately -- those APIs select streaming by endpoint and reject
        // the field outright ("Unknown name \"stream\": Cannot find field", HTTP 400). Providers
        // whose format does carry it (OpenAI-compatible, Anthropic, Cohere) keep it.
        let provider_wire_carries_stream = body.get("stream").is_some();
        merge_extra_body(&mut body);
        // ~keep extra_body must never override `stream`: the transport path
        // (post_json_raw vs post_stream) is chosen by the calling method, not the
        // body, so a mismatched wire flag would desync request from response parsing.
        // Restoring it is therefore scoped to providers that had it after the transform.
        if provider_wire_carries_stream
            && let Some(s) = stream
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert("stream".into(), serde_json::Value::Bool(s));
        }

        // ~keep Serialize once so signing bytes and request body bytes are identical.
        let body_bytes = bytes::Bytes::from(serde_json::to_vec(&body)?);

        Ok(PreparedRequest {
            url,
            provider: prov,
            body_json: body,
            body_bytes,
        })
    }

    /// Resolve the auth header for a request using the construction-time provider.
    ///
    /// Uses the pre-computed cached auth header for efficiency.  When a
    /// [`CredentialProvider`] is configured, it is called to obtain a fresh
    /// credential which overrides the cached header.
    async fn resolve_auth_header(&self) -> Result<(Option<(String, String)>, Option<Credential>)> {
        if let Some(ref cp) = self.config.credential_provider {
            let credential = cp.resolve().await?;
            let header = match &credential {
                Credential::BearerToken(token) => {
                    Some(("Authorization".to_owned(), format!("Bearer {}", token.expose_secret())))
                }
                Credential::AwsCredentials { .. } => None,
            };
            Ok((header, Some(credential)))
        } else {
            Ok((self.cached_auth_header.clone(), None))
        }
    }

    /// Build the combined header list using the construction-time provider.
    ///
    /// Uses pre-computed cached extra headers for efficiency.
    ///
    /// # Errors
    ///
    /// Propagates a signing failure from [`Provider::signing_headers`] (e.g. an
    /// internal Bedrock SigV4 error) instead of sending an unsigned request.
    fn all_headers(
        &self,
        credential: Option<&Credential>,
        method: &str,
        url: &str,
        body_json: &serde_json::Value,
        body_bytes: &[u8],
    ) -> Result<Vec<(String, String)>> {
        let mut headers = self
            .provider
            .signing_headers_with_credential(method, url, body_bytes, credential)?;
        headers.extend(self.cached_extra_headers.iter().cloned());
        headers.extend(self.provider.dynamic_headers(body_json));
        Ok(headers)
    }

    /// Send a prepared chat request and read the reply as a stream of chunks. `chat_stream` and
    /// `chat_stream_raw` both use it, so a stream cut before the provider's final event is an
    /// error on both.
    async fn send_chat_stream(
        &self,
        model: &str,
        prepared: PreparedRequest,
    ) -> Result<BoxStream<'static, Result<ChatCompletionChunk>>> {
        // ~keep Streaming endpoints can differ from non-streaming provider URLs.
        let bare_model = prepared.provider.strip_model_prefix(model);
        let url = prepared
            .provider
            .build_stream_url(prepared.provider.chat_completions_path(), bare_model);

        let (auth_header, credential) = self
            .resolve_auth_header_for_provider(prepared.provider.as_ref())
            .await?;
        let all_headers = self.all_headers_for_provider(
            prepared.provider.as_ref(),
            credential.as_ref(),
            "POST",
            &url,
            &prepared.body_json,
            &prepared.body_bytes,
        )?;
        let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
        let auth = auth_header.as_ref().map(str_pair);
        let require_terminal = requires_terminal_chat_chunk(prepared.provider.as_ref());
        let provider_name = prepared.provider.name().to_owned();
        // ~keep The provider wrapper below reports a cut stream of a native protocol
        // with the provider's own message, so the SSE reader's check stays off for it.
        let end_check: Option<fn(&ChatCompletionChunk) -> bool> = if require_terminal {
            None
        } else {
            Some(chunk_has_finish_reason)
        };

        let stream = match prepared.provider.stream_format() {
            format @ (provider::StreamFormat::Sse | provider::StreamFormat::Ndjson) => {
                let framing = match format {
                    provider::StreamFormat::Ndjson => http::streaming::Framing::Ndjson,
                    _ => http::streaming::Framing::Sse,
                };
                let provider = Arc::clone(&prepared.provider);
                let state = std::sync::Mutex::new(ChatStreamState {
                    model: model.to_owned(),
                    ..Default::default()
                });
                let parse_event = move |data: &str| parse_chat_stream_event(provider.as_ref(), &state, data);
                http::streaming::post_stream_framed(
                    &self.http_stream,
                    http::request::StreamingPost {
                        url: &url,
                        auth_header: auth,
                        extra_headers: &extra,
                        body: prepared.body_bytes,
                    },
                    parse_event,
                    end_check,
                    self.response_read_options(),
                    framing,
                )
                .await
                .map_err(|e| prepared.provider.transform_error(e))?
            }
            provider::StreamFormat::AwsEventStream => {
                http::eventstream::post_eventstream_bounded(
                    &self.http_stream,
                    http::request::StreamingPost {
                        url: &url,
                        auth_header: auth,
                        extra_headers: &extra,
                        body: prepared.body_bytes,
                    },
                    provider::bedrock::parse_bedrock_stream_event,
                    self.response_read_options(),
                )
                .await?
            }
        };
        Ok(if require_terminal {
            RequireTerminalChunk::wrap(stream, provider_name)
        } else {
            stream
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn model_uses_max_completion_tokens(model: &str) -> bool {
    crate::cost::model_info(model).is_some_and(|info| info.supports_reasoning == Some(true))
        || model.starts_with("gpt-5")
        || model
            .strip_prefix('o')
            .and_then(|suffix| suffix.chars().next())
            .is_some_and(|character| character.is_ascii_digit())
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
/// Map a rerank reply onto the canonical `results` shape.
///
/// The wire format of the reply belongs to the provider that the request model names with
/// its `provider/` prefix. That provider is a built-in one, so a custom provider that claims
/// the model for routing does not hide it. A model that names no provider uses the provider
/// the request resolved to, which for a client with a `base_url` and a `model_hint` follows
/// the hint. A `base_url` or a custom provider only says where to send the request, not how
/// that endpoint words its reply. A provider whose reply already has the canonical shape
/// leaves it unchanged, so a gateway that normalizes replies is not affected. Rerank replies
/// never pass through the chat transform of the provider.
fn transform_rerank_reply(resolved: &dyn Provider, model: &str, raw: &mut serde_json::Value) -> Result<()> {
    match rerank_wire_provider(model) {
        Some(named) => named.transform_rerank_response(raw),
        None => resolved.transform_rerank_response(raw),
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
/// Word a rerank request for the provider that reads it.
///
/// The provider is chosen as for the reply: the `provider/` prefix of the model, else the
/// provider the request resolved to, which for a client with a `base_url` and a
/// `model_hint` follows the hint.
fn transform_rerank_request(resolved: &dyn Provider, model: &str, body: &mut serde_json::Value) -> Result<()> {
    match rerank_wire_provider(model) {
        Some(named) => named.transform_rerank_request(body),
        None => resolved.transform_rerank_request(body),
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
/// The rerank path for `model`, chosen as for the request and the reply: the `provider/`
/// prefix of the model, else the provider the request resolved to.
fn rerank_endpoint<'a>(resolved: &'a dyn Provider, model: &str) -> &'a str {
    match rerank_wire_provider(model) {
        Some(named) => provider::rerank_shape::rerank_path_of(named.name()),
        None => resolved.rerank_path(),
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
/// The built-in provider that the `provider/` prefix of a rerank model names.
fn rerank_wire_provider(model: &str) -> Option<Box<dyn Provider>> {
    // ~keep Only the `provider/` prefix names a provider here. A bare model such as `rerank-2` can
    // ~keep match a registry model prefix of a provider that has no rerank step, and that match
    // ~keep would hide the provider the client names through its `model_hint`.
    model
        .split_once('/')
        .and_then(|(prefix, _)| provider::detect_builtin_provider(&format!("{prefix}/")))
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
/// Resolve the provider to use for all requests on this client.
///
/// Priority:
/// 1. Explicit `base_url` in config:
///    - If `model_hint` identifies a provider with a non-standard URL format
///      (Azure embeds the deployment name and `?api-version=…`), construct
///      that provider with the override (issue #83).
///    - Otherwise, treat the override as a generic OpenAI-compatible endpoint
///      (LM Studio, Ollama, vLLM, gateways, etc.).  Models are sent verbatim
///      except for a leading `custom/` and a leading `X/` where `X` is the
///      provider named by `model_hint` (issue #249).
/// 2. `model_hint` -> auto-detect by model name prefix.
/// 3. Default -> OpenAI.
fn build_provider(config: &ClientConfig, model_hint: Option<&str>) -> Result<Arc<dyn Provider>> {
    if let Some(base_url) = config.effective_base_url().map(str::to_owned) {
        if let Some(model) = model_hint
            && model.starts_with("azure/")
        {
            return Ok(Arc::new(provider::azure::AzureProvider::with_base_url(
                base_url.clone(),
            )));
        }
        if let Some(model) = model_hint
            && provider::ollama::OllamaProvider::names_hint(model)
        {
            return Ok(Arc::new(provider::ollama::OllamaProvider::with_base_url(
                base_url.clone(),
            )));
        }
        if let Some(model) = model_hint
            && model.starts_with("anthropic/")
        {
            return Ok(Arc::new(provider::anthropic::AnthropicProvider::with_base_url(
                base_url.clone(),
            )));
        }
        if let Some(model) = model_hint
            && (model.starts_with("gemini/") || model.starts_with("google_ai/"))
        {
            return Ok(Arc::new(provider::google_ai::GoogleAiProvider::with_base_url(
                base_url.clone(),
            )));
        }
        if let Some(model) = model_hint
            && model.starts_with("vertex_ai/")
        {
            return Ok(Arc::new(provider::vertex::VertexAiProvider::with_base_url(
                base_url.clone(),
            )));
        }
        if let Some(model) = model_hint
            && model.starts_with("bedrock/")
        {
            return Ok(Arc::new(
                build_bedrock_provider_config(config).with_base_url(base_url.clone()),
            ));
        }
        return Ok(Arc::new(OpenAiCompatibleProvider {
            name: "custom".into(),
            base_url: base_url.clone(),
            env_var: None,
            model_prefixes: vec![],
            hint_provider: model_hint.and_then(provider::hint_provider_name),
        }));
    }

    if let Some(model) = model_hint {
        if model.starts_with("bedrock/") {
            return Ok(build_bedrock_provider(config));
        }
        if provider::ollama::OllamaProvider::names_hint(model) {
            return Ok(Arc::new(provider::ollama::OllamaProvider::default()));
        }
        if let Some(p) = provider::route_model(model)? {
            return Ok(Arc::from(p));
        }
        provider::refuse_unrouted_hint(model)?;
    }

    Ok(Arc::new(OpenAiProvider))
}

/// The provider for a model with another provider's prefix on a client whose hint names Ollama
/// and whose `base_url` is set.
///
/// Only a bare model goes to the native API on such a client. A model such as `openai/m` or
/// `groq/m` goes to the OpenAI-compatible endpoint at the `base_url`, where a client without the
/// hint sends it, with the model name as given. `None` for every other client.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn prefixed_model_provider(config: &ClientConfig, model_hint: Option<&str>) -> Option<Arc<dyn Provider>> {
    let base_url = config.effective_base_url()?;
    model_hint.filter(|hint| provider::ollama::OllamaProvider::names_hint(hint))?;
    Some(Arc::new(OpenAiCompatibleProvider {
        name: "custom".into(),
        base_url: base_url.to_owned(),
        env_var: None,
        model_prefixes: vec![],
        // ~keep No hint provider: the native provider owns every `ollama/` and `ollama_chat/`
        // model before this provider sees one, so there is no hint prefix left to strip.
        hint_provider: None,
    }))
}

/// Build a [`provider::bedrock::BedrockProvider`] from `config`, threading
/// through any explicit region, cross-region prefix, and credentials set on
/// [`ClientConfig`]. Falls back to the environment for anything left unset,
/// matching [`provider::bedrock::BedrockProvider::from_env`].
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn build_bedrock_provider(config: &ClientConfig) -> Arc<dyn Provider> {
    Arc::new(build_bedrock_provider_config(config))
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn build_bedrock_provider_config(config: &ClientConfig) -> provider::bedrock::BedrockProvider {
    provider::bedrock::BedrockProvider::from_config(
        config.bedrock_region.clone(),
        config.bedrock_cross_region_prefix.clone(),
        config.bedrock_access_key_id.clone(),
        config.bedrock_secret_access_key.clone(),
        config.bedrock_session_token.clone(),
    )
    .with_dynamic_credentials(config.credential_provider.is_some())
}

/// The most pages that list models follows. A provider that never ends its paging is an error.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
const MAX_MODEL_LIST_PAGES: usize = 100;

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl LlmClient for DefaultClient {
    fn chat(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<ChatCompletionResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.chat(req);
        }
        Box::pin(async move {
            // ~keep Pass stream=false so providers can transform non-streaming chat correctly.
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await
            .map_err(|e| prepared.provider.transform_error(e))?;
            prepared.provider.transform_response(&mut raw)?;
            let mut response = serde_json::from_value::<ChatCompletionResponse>(raw).map_err(LiterLlmError::from)?;
            attribute_chat_model(prepared.provider.as_ref(), &mut response.model, &req.model);
            Ok(response)
        })
    }

    fn chat_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ChatCompletionChunk>>>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.chat_stream(req);
        }
        Box::pin(async move {
            // ~keep Prepare first for validation/transforms, then override with provider stream URL.
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))?;

            self.send_chat_stream(&req.model, prepared).await
        })
    }

    fn embed(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<EmbeddingResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.embed(req);
        }
        Box::pin(async move {
            // ~keep Embeddings have no stream flag; passing None prevents inserting one.
            let prepared = self.prepare_embedding_request(&req)?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await
            .map_err(|e| prepared.provider.transform_error(e))?;
            prepared.provider.transform_embedding_response(&mut raw, &req.model)?;
            serde_json::from_value::<EmbeddingResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn list_models(&self) -> BoxFuture<'_, Result<ModelsListResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.list_models();
        }
        Box::pin(async move {
            // ~keep list_models has no model string, so use the construction-time provider.
            let mut url = self.provider.build_url(self.provider.models_path(), "");
            let mut listing: Option<ModelsListResponse> = None;
            for _ in 0..MAX_MODEL_LIST_PAGES {
                let (auth_header, credential) = self.resolve_auth_header().await?;
                let auth = auth_header.as_ref().map(str_pair);
                let all_headers = self.all_headers(credential.as_ref(), "GET", &url, &serde_json::Value::Null, &[])?;
                let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

                let mut raw =
                    http::request::get_json_raw_bounded(&self.http, &url, auth, &extra, self.response_read_options())
                        .await
                        .map_err(|e| self.provider.transform_error(e))?;
                let next_url = self.provider.next_models_page_url(&url, &raw);
                self.provider.transform_response(&mut raw)?;
                let page = serde_json::from_value::<ModelsListResponse>(raw).map_err(LiterLlmError::from)?;
                match listing.as_mut() {
                    Some(listing) => listing.data.extend(page.data),
                    None => listing = Some(page),
                }
                match next_url {
                    Some(next_url) => url = next_url,
                    None => return Ok(listing.unwrap_or_default()),
                }
            }
            Err(LiterLlmError::InternalError {
                message: format!("the model list did not end after {MAX_MODEL_LIST_PAGES} pages"),
            })
        })
    }

    fn image_generate(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<ImagesResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.image_generate(req);
        }
        Box::pin(async move {
            let model = req.model.as_deref().unwrap_or_default();
            let prepared = self.prepare_request(&req, |p| p.image_generations_path(), model, None)?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<ImagesResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn speech(&self, req: CreateSpeechRequest) -> BoxFuture<'_, Result<bytes::Bytes>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.speech(req);
        }
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.audio_speech_path(), &req.model, None)?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            http::request::post_binary_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await
        })
    }

    fn transcribe(&self, req: CreateTranscriptionRequest) -> BoxFuture<'_, Result<TranscriptionResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.transcribe(req);
        }
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.audio_transcriptions_path(), &req.model, None)?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<TranscriptionResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn moderate(&self, req: ModerationRequest) -> BoxFuture<'_, Result<ModerationResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.moderate(req);
        }
        Box::pin(async move {
            let model = req.model.as_deref().unwrap_or_default();
            let prepared = self.prepare_request(&req, |p| p.moderations_path(), model, None)?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<ModerationResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn rerank(&self, req: RerankRequest) -> BoxFuture<'_, Result<RerankResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.rerank(req);
        }
        Box::pin(async move {
            let prepared = self.prepare_request_with(
                &req,
                |p| rerank_endpoint(p, &req.model),
                |p, body| p.transform_request(body),
                &req.model,
                None,
                |resolved, body| transform_rerank_request(resolved, &req.model, body),
            )?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            transform_rerank_reply(prepared.provider.as_ref(), &req.model, &mut raw)?;
            serde_json::from_value::<RerankResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn search(&self, req: SearchRequest) -> BoxFuture<'_, Result<SearchResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.search(req);
        }
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.search_path(), &req.model, None)?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<SearchResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn ocr(&self, req: OcrRequest) -> BoxFuture<'_, Result<OcrResponse>> {
        #[cfg(all(feature = "native-http", feature = "tower"))]
        if let Some(managed) = &self.managed {
            return managed.ocr(req);
        }
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.ocr_path(), &req.model, None)?;

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;
            prepared.provider.transform_response(&mut raw)?;
            serde_json::from_value::<OcrResponse>(raw).map_err(LiterLlmError::from)
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl LlmClientRaw for DefaultClient {
    fn chat_raw(&self, req: ChatCompletionRequest) -> BoxFuture<'_, Result<RawExchange<ChatCompletionResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))?;
            let raw_request = prepared.body_json.clone();

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await
            .map_err(|e| prepared.provider.transform_error(e))?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let mut data = serde_json::from_value::<ChatCompletionResponse>(raw).map_err(LiterLlmError::from)?;
            attribute_chat_model(prepared.provider.as_ref(), &mut data.model, &req.model);

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn chat_stream_raw(
        &self,
        req: ChatCompletionRequest,
    ) -> BoxFuture<'_, Result<RawStreamExchange<BoxStream<'static, Result<ChatCompletionChunk>>>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))?;
            let raw_request = prepared.body_json.clone();

            let stream = self.send_chat_stream(&req.model, prepared).await?;
            Ok(RawStreamExchange { stream, raw_request })
        })
    }

    fn embed_raw(&self, req: EmbeddingRequest) -> BoxFuture<'_, Result<RawExchange<EmbeddingResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_embedding_request(&req)?;
            let raw_request = prepared.body_json.clone();

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await
            .map_err(|e| prepared.provider.transform_error(e))?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_embedding_response(&mut raw, &req.model)?;
            let data = serde_json::from_value::<EmbeddingResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn image_generate_raw(&self, req: CreateImageRequest) -> BoxFuture<'_, Result<RawExchange<ImagesResponse>>> {
        Box::pin(async move {
            let model = req.model.as_deref().unwrap_or_default();
            let prepared = self.prepare_request(&req, |p| p.image_generations_path(), model, None)?;
            let raw_request = prepared.body_json.clone();

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<ImagesResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn transcribe_raw(
        &self,
        req: CreateTranscriptionRequest,
    ) -> BoxFuture<'_, Result<RawExchange<TranscriptionResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.audio_transcriptions_path(), &req.model, None)?;
            let raw_request = prepared.body_json.clone();

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<TranscriptionResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn moderate_raw(&self, req: ModerationRequest) -> BoxFuture<'_, Result<RawExchange<ModerationResponse>>> {
        Box::pin(async move {
            let model = req.model.as_deref().unwrap_or_default();
            let prepared = self.prepare_request(&req, |p| p.moderations_path(), model, None)?;
            let raw_request = prepared.body_json.clone();

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<ModerationResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn rerank_raw(&self, req: RerankRequest) -> BoxFuture<'_, Result<RawExchange<RerankResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request_with(
                &req,
                |p| rerank_endpoint(p, &req.model),
                |p, body| p.transform_request(body),
                &req.model,
                None,
                |resolved, body| transform_rerank_request(resolved, &req.model, body),
            )?;
            let raw_request = prepared.body_json.clone();

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            transform_rerank_reply(prepared.provider.as_ref(), &req.model, &mut raw)?;
            let data = serde_json::from_value::<RerankResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn search_raw(&self, req: SearchRequest) -> BoxFuture<'_, Result<RawExchange<SearchResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.search_path(), &req.model, None)?;
            let raw_request = prepared.body_json.clone();

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<SearchResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }

    fn ocr_raw(&self, req: OcrRequest) -> BoxFuture<'_, Result<RawExchange<OcrResponse>>> {
        Box::pin(async move {
            let prepared = self.prepare_request(&req, |p| p.ocr_path(), &req.model, None)?;
            let raw_request = prepared.body_json.clone();

            let (auth_header, credential) = self
                .resolve_auth_header_for_provider(prepared.provider.as_ref())
                .await?;
            let all_headers = self.all_headers_for_provider(
                prepared.provider.as_ref(),
                credential.as_ref(),
                "POST",
                &prepared.url,
                &prepared.body_json,
                &prepared.body_bytes,
            )?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let auth = auth_header.as_ref().map(str_pair);
            let mut raw = http::request::post_json_raw_bounded(
                &self.http,
                &prepared.url,
                auth,
                &extra,
                prepared.body_bytes,
                self.response_read_options(),
            )
            .await?;

            let raw_response = Some(raw.clone());
            prepared.provider.transform_response(&mut raw)?;
            let data = serde_json::from_value::<OcrResponse>(raw).map_err(LiterLlmError::from)?;

            Ok(RawExchange {
                data,
                raw_request,
                raw_response,
            })
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl FileClient for DefaultClient {
    fn create_file(&self, req: CreateFileRequest) -> BoxFuture<'_, Result<FileObject>> {
        Box::pin(async move {
            let url = self.provider.build_url(self.provider.files_path(), "");
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers(credential.as_ref(), "POST", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            use base64::Engine;
            let file_bytes = base64::engine::general_purpose::STANDARD
                .decode(&req.file)
                .map_err(|e| LiterLlmError::BadRequest {
                    message: format!("invalid base64 file data: {e}"),
                    status: 400,
                })?;

            let filename = req.filename.unwrap_or_else(|| "upload".to_owned());
            let file_part = reqwest::multipart::Part::bytes(file_bytes).file_name(filename);
            let purpose_str = serde_json::to_value(&req.purpose)?
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let form = reqwest::multipart::Form::new()
                .part("file", file_part)
                .text("purpose", purpose_str);

            let raw = http::request::post_multipart_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                form,
                self.config.max_response_bytes,
                Some(self.config.timeout),
            )
            .await?;
            serde_json::from_value::<FileObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn retrieve_file(&self, file_id: &str) -> BoxFuture<'_, Result<FileObject>> {
        let file_id = file_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}",
                self.provider.build_url(self.provider.files_path(), ""),
                file_id
            );
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers(credential.as_ref(), "GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(&self.http, &url, auth, &extra, self.response_read_options())
                .await?;
            serde_json::from_value::<FileObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn delete_file(&self, file_id: &str) -> BoxFuture<'_, Result<DeleteResponse>> {
        let file_id = file_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}",
                self.provider.build_url(self.provider.files_path(), ""),
                file_id
            );
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers(credential.as_ref(), "DELETE", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::delete_json_bounded(&self.http, &url, auth, &extra, self.response_read_options())
                .await?;
            serde_json::from_value::<DeleteResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn list_files(&self, query: Option<FileListQuery>) -> BoxFuture<'_, Result<FileListResponse>> {
        Box::pin(async move {
            let base_url = self.provider.build_url(self.provider.files_path(), "");
            let url = if let Some(ref q) = query {
                let mut params = Vec::new();
                if let Some(ref purpose) = q.purpose {
                    params.push(format!("purpose={purpose}"));
                }
                if let Some(limit) = q.limit {
                    params.push(format!("limit={limit}"));
                }
                if let Some(ref after) = q.after {
                    params.push(format!("after={after}"));
                }
                if params.is_empty() {
                    base_url
                } else {
                    format!("{base_url}?{}", params.join("&"))
                }
            } else {
                base_url
            };
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers(credential.as_ref(), "GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(&self.http, &url, auth, &extra, self.response_read_options())
                .await?;
            serde_json::from_value::<FileListResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn file_content(&self, file_id: &str) -> BoxFuture<'_, Result<bytes::Bytes>> {
        let file_id = file_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}/content",
                self.provider.build_url(self.provider.files_path(), ""),
                file_id
            );
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers(credential.as_ref(), "GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            http::request::get_binary_bounded(&self.http, &url, auth, &extra, self.response_read_options()).await
        })
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl BatchClient for DefaultClient {
    fn create_batch(&self, req: CreateBatchRequest) -> BoxFuture<'_, Result<BatchObject>> {
        Box::pin(async move {
            let url = self.provider.build_url(self.provider.batches_path(), "");
            let body_bytes = bytes::Bytes::from(serde_json::to_vec(&req)?);
            let body_json = serde_json::to_value(&req)?;

            let (auth_header, credential) = self.resolve_auth_header().await?;
            let all_headers = self.all_headers(credential.as_ref(), "POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                body_bytes,
                self.response_read_options(),
            )
            .await?;
            serde_json::from_value::<BatchObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn retrieve_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>> {
        let batch_id = batch_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}",
                self.provider.build_url(self.provider.batches_path(), ""),
                batch_id
            );
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers(credential.as_ref(), "GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(&self.http, &url, auth, &extra, self.response_read_options())
                .await?;
            serde_json::from_value::<BatchObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn list_batches(&self, query: Option<BatchListQuery>) -> BoxFuture<'_, Result<BatchListResponse>> {
        Box::pin(async move {
            let base_url = self.provider.build_url(self.provider.batches_path(), "");
            let url = if let Some(ref q) = query {
                let mut params = Vec::new();
                if let Some(limit) = q.limit {
                    params.push(format!("limit={limit}"));
                }
                if let Some(ref after) = q.after {
                    params.push(format!("after={after}"));
                }
                if params.is_empty() {
                    base_url
                } else {
                    format!("{base_url}?{}", params.join("&"))
                }
            } else {
                base_url
            };
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers(credential.as_ref(), "GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(&self.http, &url, auth, &extra, self.response_read_options())
                .await?;
            serde_json::from_value::<BatchListResponse>(raw).map_err(LiterLlmError::from)
        })
    }

    fn cancel_batch(&self, batch_id: &str) -> BoxFuture<'_, Result<BatchObject>> {
        let batch_id = batch_id.to_owned();
        Box::pin(async move {
            let url = format!(
                "{}/{}/cancel",
                self.provider.build_url(self.provider.batches_path(), ""),
                batch_id
            );
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let body_json = serde_json::Value::Null;
            let body_bytes = bytes::Bytes::new();
            let all_headers = self.all_headers(credential.as_ref(), "POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                body_bytes,
                self.response_read_options(),
            )
            .await?;
            serde_json::from_value::<BatchObject>(raw).map_err(LiterLlmError::from)
        })
    }
}

/// Internal trait for batch retrieval, used to abstract polling logic for testability.
///
/// Method name avoids collision with the inherent `DefaultClient::retrieve_batch`
/// so alef-generated bindings don't need to import this trait into scope.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[doc(hidden)]
#[cfg_attr(alef, alef(skip))]
pub trait BatchRetriever {
    /// Retrieve a batch by ID for polling purposes.
    async fn fetch_batch_for_polling(&self, batch_id: &str) -> Result<BatchObject>;
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl BatchRetriever for DefaultClient {
    async fn fetch_batch_for_polling(&self, batch_id: &str) -> Result<BatchObject> {
        self.retrieve_batch(batch_id).await
    }
}

/// Poll a batch until it reaches a terminal status.
///
/// This is the internal implementation shared by tests and `DefaultClient::wait_for_batch`.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
#[doc(hidden)]
#[cfg_attr(alef, alef(skip))]
pub async fn wait_for_batch_impl<R: BatchRetriever>(
    retriever: &R,
    batch_id: &str,
    config: WaitForBatchConfig,
) -> std::result::Result<BatchObject, BatchWaitError> {
    #[cfg(not(target_arch = "wasm32"))]
    let started = tokio::time::Instant::now();
    #[cfg(target_arch = "wasm32")]
    let started = web_time::Instant::now();
    let mut interval_secs = config.initial_interval_secs;

    loop {
        let batch = retriever.fetch_batch_for_polling(batch_id).await?;

        match batch.status {
            BatchStatus::Completed => return Ok(batch),
            BatchStatus::Failed | BatchStatus::Expired | BatchStatus::Cancelled => {
                return Err(BatchWaitError::Failed { status: batch.status });
            }
            BatchStatus::Validating | BatchStatus::InProgress | BatchStatus::Finalizing | BatchStatus::Cancelling => {
                if let Some(timeout_secs) = config.timeout_secs {
                    let timeout = Duration::from_secs_f64(timeout_secs);
                    if started.elapsed() >= timeout {
                        return Err(BatchWaitError::Timeout { timeout_secs });
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(Duration::from_secs_f64(interval_secs)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::sleep(Duration::from_secs_f64(interval_secs)).await;
                let next =
                    (interval_secs as f32 * config.backoff_multiplier).min(config.max_interval_secs as f32) as f64;
                interval_secs = next;
            }
        }
    }
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl DefaultClient {
    /// Poll a batch until it reaches a terminal status (Completed, Failed, Expired, Cancelled).
    ///
    /// Uses exponential backoff with configurable initial interval, maximum interval, and backoff multiplier.
    /// Optionally supports a timeout that aborts polling if exceeded.
    ///
    /// # Errors
    ///
    /// Returns `BatchWaitError::Failed` if the batch reaches a failure terminal status.
    /// Returns `BatchWaitError::Timeout` if the configured timeout is exceeded.
    /// Returns `BatchWaitError::Client` for underlying client errors.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use liter_llm::client::{DefaultClient, ClientConfig, WaitForBatchConfig};
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = DefaultClient::new(ClientConfig::new("api-key"), None)?;
    /// let batch = client.wait_for_batch("b-123", WaitForBatchConfig::default()).await?;
    /// println!("Batch completed: {:?}", batch.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_batch(
        &self,
        batch_id: &str,
        config: WaitForBatchConfig,
    ) -> std::result::Result<BatchObject, BatchWaitError> {
        wait_for_batch_impl(self, batch_id, config).await
    }
}

/// Parse a single Responses API SSE `data:` payload into a [`ResponseStreamEvent`].
///
/// Unlike [`Provider::parse_stream_event`], this has no provider dispatch: it always
/// decodes the OpenAI Responses `response.*` event vocabulary. That is not evidence
/// that the format is portable — the Responses path is OpenAI-only (see
/// [`ResponseClient`](trait@ResponseClient)), so there is no second format to dispatch
/// on. Unrecognized `type` values decode into
/// [`ResponseStreamEvent::Unknown`] rather than failing (see
/// `ResponseStreamEvent`'s `Deserialize` impl), so this only returns `Err`
/// for payloads that are not valid JSON at all.
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn parse_response_stream_event(data: &str) -> Result<Option<ResponseStreamEvent>> {
    serde_json::from_str::<ResponseStreamEvent>(data)
        .map(Some)
        .map_err(|e| LiterLlmError::Streaming {
            message: format!("failed to parse Responses SSE data: {e}"),
        })
}

/// Reject a Responses URL that contains an empty path segment (`//`).
///
/// The four `ResponseClient` call sites pass `""` as the model to
/// [`Provider::build_url`] / [`Provider::build_stream_url`], because none of `create_response`,
/// `retrieve_response`, or `cancel_response` has a model available to give the URL-embedding
/// providers (see the `ResponseClient` trait doc). Most providers' `build_url` ignores the model
/// for the `/responses` path — it only fires for `chat/completions` and `embeddings` — so an
/// empty model is harmless there. Azure's `build_url` embeds the model unconditionally, so an
/// empty model turns into an empty path segment: `.../openai/deployments//responses`.
///
/// This checks the URL shape rather than `provider.name()` on purpose: naming a specific
/// provider would either allow-list `"openai"` and reject supported OpenAI-compatible gateways
/// running under a different provider name, or deny-list `"azure"` and silently miss any future
/// provider with the same URL-embedding bug. Checking for the empty segment catches exactly the
/// malformed case, from whichever provider produces it, and never fires for a provider whose
/// `build_url` output is well-formed. ~keep
///
/// One other configuration can trip this, so do not read the error as "this provider cannot do
/// Responses" without checking: [`ClientConfig::base_url`] is a public field, so a value set
/// directly rather than through [`ClientConfigBuilder::base_url`] or
/// [`ClientConfig::with_base_url`] skips their `trim_end_matches('/')`, and a trailing slash
/// then yields `.../v1//responses` here too. That configuration is already broken for every
/// other endpoint as well, so it should surface long before this check — but if this fires for
/// a provider that ought to work, inspect `base_url` before suspecting the provider. ~keep
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
fn reject_malformed_responses_url(url: &str, provider_name: &str) -> Result<()> {
    let path = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = path.split_once('/').map_or("", |(_, rest)| rest);
    let path = path.split(['?', '#']).next().unwrap_or("");
    if format!("/{path}").contains("//") {
        return Err(LiterLlmError::EndpointNotSupported {
            endpoint: "responses".into(),
            provider: provider_name.into(),
        });
    }
    Ok(())
}

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl ResponseClient for DefaultClient {
    fn create_response(&self, req: CreateResponseRequest) -> BoxFuture<'_, Result<ResponseObject>> {
        Box::pin(async move {
            self.refuse_unrouted_model(&req.model)?;
            let url = self.provider.build_url(self.provider.responses_path(), "");
            reject_malformed_responses_url(&url, self.provider.name())?;
            let body_json = response_request_body(&req)?;
            let body_bytes = bytes::Bytes::from(serde_json::to_vec(&body_json)?);

            let (auth_header, credential) = self.resolve_auth_header().await?;
            let all_headers = self.all_headers(credential.as_ref(), "POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                body_bytes,
                self.response_read_options(),
            )
            .await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn create_response_stream(
        &self,
        req: CreateResponseRequest,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Result<ResponseStreamEvent>>>> {
        Box::pin(async move {
            self.refuse_unrouted_model(&req.model)?;
            // ~keep Force streaming on the wire regardless of what the caller (or extra_body) set.
            let url = self.provider.build_stream_url(self.provider.responses_path(), "");
            reject_malformed_responses_url(&url, self.provider.name())?;
            let body_json = response_stream_request_body(req)?;
            let body_bytes = bytes::Bytes::from(serde_json::to_vec(&body_json)?);

            let (auth_header, credential) = self.resolve_auth_header().await?;
            let all_headers = self.all_headers(credential.as_ref(), "POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            http::streaming::post_stream_bounded(
                &self.http_stream,
                http::request::StreamingPost {
                    url: &url,
                    auth_header: auth,
                    extra_headers: &extra,
                    body: body_bytes,
                },
                parse_response_stream_event,
                Some(response_event_is_terminal),
                self.response_read_options(),
            )
            .await
        })
    }

    fn retrieve_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>> {
        let response_id = response_id.to_owned();
        Box::pin(async move {
            let base_url = self.provider.build_url(self.provider.responses_path(), "");
            reject_malformed_responses_url(&base_url, self.provider.name())?;
            let url = format!("{base_url}/{response_id}");
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let auth = auth_header.as_ref().map(str_pair);
            let all_headers = self.all_headers(credential.as_ref(), "GET", &url, &serde_json::Value::Null, &[])?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();

            let raw = http::request::get_json_raw_bounded(&self.http, &url, auth, &extra, self.response_read_options())
                .await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }

    fn cancel_response(&self, response_id: &str) -> BoxFuture<'_, Result<ResponseObject>> {
        let response_id = response_id.to_owned();
        Box::pin(async move {
            let base_url = self.provider.build_url(self.provider.responses_path(), "");
            reject_malformed_responses_url(&base_url, self.provider.name())?;
            let url = format!("{base_url}/{response_id}/cancel");
            let (auth_header, credential) = self.resolve_auth_header().await?;
            let body_json = serde_json::Value::Null;
            let body_bytes = bytes::Bytes::new();
            let all_headers = self.all_headers(credential.as_ref(), "POST", &url, &body_json, &body_bytes)?;
            let extra: Vec<(&str, &str)> = all_headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
            let auth = auth_header.as_ref().map(str_pair);

            let raw = http::request::post_json_raw_bounded(
                &self.http,
                &url,
                auth,
                &extra,
                body_bytes,
                self.response_read_options(),
            )
            .await?;
            serde_json::from_value::<ResponseObject>(raw).map_err(LiterLlmError::from)
        })
    }
}

#[cfg(all(test, any(feature = "native-http", feature = "wasm-http")))]
mod stream_termination_tests {
    use futures_util::StreamExt;

    use super::*;
    use crate::types::{FinishReason, StreamChoice};

    fn chunk(finish_reason: Option<FinishReason>) -> Result<ChatCompletionChunk> {
        Ok(ChatCompletionChunk {
            choices: vec![StreamChoice {
                finish_reason,
                ..Default::default()
            }],
            ..Default::default()
        })
    }

    async fn drain(items: Vec<Result<ChatCompletionChunk>>) -> Vec<Result<ChatCompletionChunk>> {
        RequireTerminalChunk::wrap(Box::pin(futures_util::stream::iter(items)), "anthropic".into())
            .collect()
            .await
    }

    #[tokio::test]
    async fn finish_reason_marks_the_stream_complete() {
        let items = drain(vec![chunk(None), chunk(Some(FinishReason::Stop)), chunk(None)]).await;
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(Result::is_ok));
    }

    #[tokio::test]
    async fn clean_end_without_finish_reason_is_a_streaming_error() {
        let items = drain(vec![chunk(None)]).await;
        assert_eq!(items.len(), 2);
        assert!(matches!(items[1], Err(LiterLlmError::Streaming { .. })));
    }

    #[tokio::test]
    async fn an_error_already_reported_is_not_followed_by_another() {
        let items = drain(vec![
            chunk(None),
            Err(LiterLlmError::Streaming {
                message: "cut mid-line".into(),
            }),
        ])
        .await;
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn only_first_party_protocols_require_a_final_event() {
        assert!(requires_terminal_chat_chunk(
            &provider::anthropic::AnthropicProvider::default()
        ));
        assert!(requires_terminal_chat_chunk(&provider::cohere::CohereProvider));
        assert!(requires_terminal_chat_chunk(
            &provider::ollama::OllamaProvider::default()
        ));
        // ~keep Ollama is found by the capability of its native provider, not by its name: a
        // custom provider called `ollama` is an OpenAI-compatible server.
        let named_ollama = OpenAiCompatibleProvider {
            name: "ollama".into(),
            base_url: "http://localhost:11434/v1".into(),
            env_var: None,
            model_prefixes: vec![],
            hint_provider: None,
        };
        assert!(!requires_terminal_chat_chunk(&named_ollama));
        assert!(!named_ollama.stream_tool_call_indexes_restart());
        // ~keep Bedrock is found by its event stream framing, not by its name.
        let bedrock = provider::bedrock::BedrockProvider::from_config(Some("us-east-1".into()), None, None, None, None);
        assert_eq!(bedrock.stream_format(), provider::StreamFormat::AwsEventStream);
        assert!(requires_terminal_chat_chunk(&bedrock));
        assert!(!requires_terminal_chat_chunk(
            provider::detect_provider("openai/gpt-4o")
                .expect("OpenAI is a built-in provider")
                .as_ref()
        ));
    }
}

#[cfg(all(test, any(feature = "native-http", feature = "wasm-http")))]
mod build_provider_tests {
    use super::*;
    use crate::client::config::ClientConfigBuilder;

    /// Revert line: reverting `reject_malformed_responses_url` in `crates/liter-llm/src/client/
    /// mod.rs` to always return `Ok(())` makes this test fail — it would no longer catch the
    /// empty `/openai/deployments//responses` segment Azure produces when no deployment name
    /// (model) is available, which is the case at all four `ResponseClient` call sites.
    #[test]
    fn reject_malformed_responses_url_rejects_azure_empty_deployment() {
        let provider = provider::azure::AzureProvider::with_base_url("https://resourceA.openai.azure.com");
        let url = provider.build_url(provider.responses_path(), "");

        assert!(
            url.contains("/openai/deployments//responses"),
            "test setup assumption broken, url = {url}"
        );

        let result = reject_malformed_responses_url(&url, provider.name());
        match result {
            Err(LiterLlmError::EndpointNotSupported {
                endpoint,
                provider: rejected_provider,
            }) => {
                assert_eq!(endpoint, "responses");
                assert_eq!(rejected_provider, "azure");
            }
            other => panic!("expected Err(EndpointNotSupported), got {other:?}"),
        }
    }

    /// Azure base URLs that already pin a deployment (`/openai/deployments/{name}`) never reach
    /// the model-embedding branch of `AzureProvider::build_url`, so the produced URL has no empty
    /// segment and must not be rejected.
    #[test]
    fn reject_malformed_responses_url_allows_azure_with_pinned_deployment() {
        let base_url = "https://resourceA.openai.azure.com/openai/deployments/my-gpt4-deployment";
        let provider = provider::azure::AzureProvider::with_base_url(base_url);
        let url = provider.build_url(provider.responses_path(), "");

        assert!(
            !url.contains("deployments//"),
            "test setup assumption broken: pinned-deployment URL should have no empty segment, url = {url}"
        );

        assert!(
            reject_malformed_responses_url(&url, provider.name()).is_ok(),
            "a pinned-deployment Azure URL must not be rejected, url = {url}"
        );
    }

    /// Bedrock, Vertex, and Google AI embed the model into `build_url` only for the
    /// `chat/completions` and `embeddings` paths; `responses_path()` matches neither, so the
    /// empty model passed at the four `ResponseClient` call sites must not be rejected for them.
    #[test]
    fn reject_malformed_responses_url_allows_bedrock_google_ai_and_vertex() {
        let bedrock = provider::bedrock::BedrockProvider::from_config(
            Some("us-east-1".into()),
            None,
            Some("test-access-key".into()),
            Some("test-secret-key".into()),
            None,
        );
        let bedrock_url = bedrock.build_url(bedrock.responses_path(), "");
        assert!(
            reject_malformed_responses_url(&bedrock_url, bedrock.name()).is_ok(),
            "bedrock url = {bedrock_url}"
        );

        let google_ai = provider::google_ai::GoogleAiProvider::default();
        let google_ai_url = google_ai.build_url(google_ai.responses_path(), "");
        assert!(
            reject_malformed_responses_url(&google_ai_url, google_ai.name()).is_ok(),
            "google ai url = {google_ai_url}"
        );

        let vertex = provider::vertex::VertexAiProvider::new("my-project", "us-central1");
        let vertex_url = vertex.build_url(vertex.responses_path(), "");
        assert!(
            reject_malformed_responses_url(&vertex_url, vertex.name()).is_ok(),
            "vertex url = {vertex_url}"
        );
    }

    #[test]
    fn azure_model_with_per_model_base_url_uses_azure_provider() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("https://resourceA.cognitiveservices.azure.com")
            .build();
        let p = build_provider(&config, Some("azure/gpt-5-mini")).expect("provider should build");
        assert_eq!(p.name(), "azure");
        let url = p.build_url("/chat/completions", "gpt-5-mini");
        assert!(
            url.starts_with("https://resourceA.cognitiveservices.azure.com/openai/deployments/gpt-5-mini/chat/completions?api-version="),
            "url = {url}"
        );
    }

    #[test]
    fn ollama_hint_without_base_url_uses_the_native_provider_at_the_default_address() {
        let config = ClientConfigBuilder::new("").build();
        for hint in ["ollama", "ollama_chat", "ollama/llama3"] {
            let p = build_provider(&config, Some(hint)).expect("provider should build");
            assert_eq!(p.name(), "ollama", "hint {hint}");
            assert_eq!(p.base_url(), "http://localhost:11434", "hint {hint}");
            assert_eq!(p.models_path(), "/api/tags", "hint {hint}");
        }
        let openai = build_provider(&config, Some("openai")).expect("provider should build");
        assert_eq!(openai.name(), "openai");
    }

    #[test]
    fn empty_base_url_on_an_ollama_model_uses_the_default_address() {
        let config = ClientConfigBuilder::new("test-key").base_url("").build();
        for hint in [Some("ollama"), Some("ollama/llama3")] {
            let client = DefaultClient::new(config.clone(), hint).expect("client should build");
            let p = client
                .resolve_provider_for_model("ollama/llama3")
                .expect("provider should resolve");
            assert_eq!(p.name(), "ollama", "hint {hint:?}");
            assert_eq!(p.base_url(), "http://localhost:11434", "hint {hint:?}");
        }
    }

    #[test]
    fn non_azure_model_with_base_url_uses_openai_compatible() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("http://localhost:11434/v1")
            .build();
        let p = build_provider(&config, Some("llama3.1:8b")).expect("provider should build");
        assert_eq!(p.name(), "custom");
        let url = p.build_url("/chat/completions", "llama3.1:8b");
        assert_eq!(url, "http://localhost:11434/v1/chat/completions");
    }

    #[test]
    fn base_url_path_is_joined_before_existing_query() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("http://localhost:11434/v1?token=secret")
            .build();
        let provider = build_provider(&config, Some("local-model")).expect("provider should build");
        let url = provider.build_url("/chat/completions", "local-model");
        assert_eq!(url, "http://localhost:11434/v1/chat/completions?token=secret");
    }

    #[test]
    fn native_provider_prefixes_keep_transforms_with_base_url() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("http://localhost:8080/native")
            .build();

        for (model, expected_name, expected_path) in [
            ("gemini/gemini-2.5-flash", "google_ai", ":generateContent"),
            ("google_ai/gemini-2.5-flash", "google_ai", ":generateContent"),
            ("vertex_ai/gemini-2.5-flash", "vertex_ai", ":generateContent"),
            ("bedrock/anthropic.claude-v2", "bedrock", "/model/"),
        ] {
            let provider = build_provider(&config, Some(model)).expect("provider should build");
            let bare_model = provider.strip_model_prefix(model);
            let url = provider.build_url(provider.chat_completions_path(), bare_model);
            assert_eq!(provider.name(), expected_name);
            assert!(url.starts_with("http://localhost:8080/native"), "url = {url}");
            assert!(url.contains(expected_path), "url = {url}");
        }
    }

    #[test]
    fn rerank_reply_of_a_cohere_model_keeps_its_results_array() {
        let original = serde_json::json!({
            "id": "r1",
            "results": [{"index": 1, "relevance_score": 0.9}],
            "meta": {"billed_units": {"search_units": 1}}
        });
        let mut body = original.clone();
        transform_rerank_reply(&provider::cohere::CohereProvider, "cohere/rerank-v3.5", &mut body)
            .expect("a Cohere rerank reply is not an error");
        assert_eq!(body, original);
    }

    #[test]
    fn rerank_reply_of_a_bedrock_model_maps_relevance_score() {
        let provider = build_provider(&ClientConfigBuilder::new("k").build(), None).expect("provider should build");
        let mut body = serde_json::json!({"results": [{"index": 1, "relevanceScore": 0.9}]});
        transform_rerank_reply(provider.as_ref(), "bedrock/amazon.rerank-v1:0", &mut body).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"results": [{"index": 1, "relevance_score": 0.9}]})
        );
    }

    #[test]
    fn rerank_reply_follows_the_hint_of_a_client_with_a_base_url() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("http://localhost:8080/v1")
            .build();
        let provider = build_provider(&config, Some("voyage")).expect("provider should build");
        let mut body = serde_json::json!({"data": [{"index": 0, "relevance_score": 0.5}]});
        transform_rerank_reply(provider.as_ref(), "rerank-2", &mut body).unwrap();
        assert!(body.get("results").is_some() && body.get("data").is_none());

        let unhinted = build_provider(&config, None).expect("provider should build");
        let mut body = serde_json::json!({"data": [{"index": 0, "relevance_score": 0.5}]});
        transform_rerank_reply(unhinted.as_ref(), "rerank-2", &mut body).unwrap();
        assert!(body.get("results").is_none() && body.get("data").is_some());
    }

    #[test]
    fn base_url_provider_strips_only_hint_provider_and_self_prefix() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("http://localhost:11434/v1")
            .build();

        let hinted = build_provider(&config, Some("openai")).expect("provider should build");
        assert_eq!(hinted.strip_model_prefix("openai/gpt-4o-mini"), "gpt-4o-mini");
        assert_eq!(hinted.strip_model_prefix("custom/my-model"), "my-model");
        assert_eq!(hinted.strip_model_prefix("anthropic/claude-x"), "anthropic/claude-x");
        assert_eq!(hinted.strip_model_prefix("meta-llama/llama-3"), "meta-llama/llama-3");
        assert_eq!(hinted.strip_model_prefix("llama3.1:8b"), "llama3.1:8b");
        assert_eq!(hinted.strip_model_prefix("openai/"), "openai/");

        let hinted_with_model = build_provider(&config, Some("openai/gpt-4o-mini")).expect("provider should build");
        assert_eq!(hinted_with_model.strip_model_prefix("openai/gpt-4o"), "gpt-4o");

        let unhinted = build_provider(&config, None).expect("provider should build");
        assert_eq!(unhinted.strip_model_prefix("openai/gpt-4o-mini"), "openai/gpt-4o-mini");
        assert_eq!(unhinted.strip_model_prefix("custom/my-model"), "my-model");
    }

    #[test]
    fn no_base_url_falls_through_to_detect_provider() {
        let config = ClientConfigBuilder::new("test-key").build();
        let p = build_provider(&config, Some("azure/gpt-4o")).expect("provider should build");
        assert_eq!(p.name(), "azure");
    }

    #[test]
    fn anthropic_model_with_per_model_base_url_uses_anthropic_provider() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("https://proxy.internal/anthropic/")
            .build();
        let p = build_provider(&config, Some("anthropic/claude-3-5-sonnet-20241022")).expect("provider should build");
        assert_eq!(p.name(), "anthropic");
        assert_eq!(p.base_url(), "https://proxy.internal/anthropic");
    }

    #[test]
    fn default_anthropic_provider_uses_official_base_url() {
        assert_eq!(
            provider::anthropic::AnthropicProvider::new().base_url(),
            "https://api.anthropic.com/v1"
        );
        assert_eq!(
            provider::anthropic::AnthropicProvider::default().base_url(),
            "https://api.anthropic.com/v1"
        );
    }

    #[test]
    fn extra_body_object_is_merged_and_overrides_existing_key() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            extra_body: Some(serde_json::json!({
                "thinking": {"type": "enabled"},
                "model": "extra-body-override"
            })),
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("extra_body").is_none(),
            "extra_body key must be removed from the final body"
        );
        assert_eq!(prepared.body_json["thinking"], serde_json::json!({"type": "enabled"}));
        assert_eq!(
            prepared.body_json["model"], "extra-body-override",
            "extra_body keys must override identically named top-level keys"
        );
    }

    fn cohere_client() -> DefaultClient {
        DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("cohere/command-r"))
            .expect("client construction should succeed")
    }

    #[test]
    fn embedding_request_takes_the_embedding_body_of_the_provider() {
        let req = EmbeddingRequest {
            model: "cohere/embed-english-v3.0".into(),
            input: crate::types::EmbeddingInput::Single("alpha".into()),
            ..Default::default()
        };

        let prepared = cohere_client()
            .prepare_embedding_request(&req)
            .expect("prepare_embedding_request should not fail");

        assert_eq!(prepared.url, "https://api.cohere.com/v2/embed");
        assert_eq!(prepared.body_json["texts"], serde_json::json!(["alpha"]));
        assert!(prepared.body_json.get("input").is_none());
    }

    #[test]
    fn cohere_embedding_request_merges_extra_body_over_the_rebuilt_body() {
        let req = EmbeddingRequest {
            model: "cohere/embed-english-v3.0".into(),
            input: crate::types::EmbeddingInput::Single("alpha".into()),
            extra_body: Some(serde_json::json!({"input_type": "search_query"})),
            ..Default::default()
        };

        let prepared = cohere_client()
            .prepare_embedding_request(&req)
            .expect("prepare_embedding_request should not fail");

        assert_eq!(prepared.body_json["input_type"], "search_query");
        assert_eq!(prepared.body_json["texts"], serde_json::json!(["alpha"]));
        assert!(prepared.body_json.get("extra_body").is_none());
    }

    #[test]
    fn non_embedding_bodies_with_input_are_not_rebuilt_as_embedding_bodies() {
        let client = cohere_client();
        let moderation = serde_json::json!({"model": "cohere/omni-moderation", "input": "some text"});
        let speech = serde_json::json!({"model": "cohere/tts-1", "input": "hello", "voice": "alloy"});

        let prepared = client
            .prepare_request(&moderation, |p| p.moderations_path(), "cohere/omni-moderation", None)
            .expect("moderation request should prepare");
        assert_eq!(prepared.body_json["input"], "some text");
        assert!(prepared.body_json.get("texts").is_none());

        let prepared = client
            .prepare_request(&speech, |p| p.audio_speech_path(), "cohere/tts-1", None)
            .expect("speech request should prepare");
        assert_eq!(prepared.body_json["voice"], "alloy");
        assert_eq!(prepared.body_json["input"], "hello");
    }

    #[test]
    fn extra_body_cannot_override_the_transport_controlled_stream_flag() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            extra_body: Some(serde_json::json!({"stream": false})),
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["stream"],
            serde_json::json!(true),
            "the caller-selected stream flag must win over extra_body"
        );
    }

    #[test]
    fn extra_body_non_object_is_dropped_without_reaching_the_body() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            extra_body: Some(serde_json::json!(["not", "an", "object"])),
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("extra_body").is_none(),
            "non-object extra_body must never reach the wire"
        );
    }

    #[test]
    fn anthropic_provider_leaves_no_extra_body_key_in_final_body() {
        let client = DefaultClient::new(
            ClientConfigBuilder::new("test-key").build(),
            Some("claude-3-5-sonnet-20241022"),
        )
        .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "claude-3-5-sonnet-20241022".into(),
            messages: vec![crate::types::Message::User(crate::types::UserMessage {
                content: crate::types::UserContent::Text("Hi".into()),
                name: None,
            })],
            max_tokens: Some(100),
            extra_body: Some(serde_json::json!({"reasoning_effort": "high"})),
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("extra_body").is_none(),
            "anthropic's own transform_request already strips extra_body"
        );
        assert_eq!(prepared.body_json["thinking"]["budget_tokens"], 16384);
    }

    fn base_response_request() -> CreateResponseRequest {
        CreateResponseRequest {
            model: "gpt-5".into(),
            input: serde_json::json!("What is the capital of France?"),
            ..Default::default()
        }
    }

    #[test]
    fn responses_extra_body_object_is_merged_and_overrides_existing_key() {
        let mut req = base_response_request();
        req.extra_body = Some(serde_json::json!({
            "reasoning": {"effort": "none"},
            "model": "extra-body-override"
        }));

        let body = response_request_body(&req).expect("body should serialize");

        assert!(
            body.get("extra_body").is_none(),
            "extra_body key must be removed from the final Responses body"
        );
        assert_eq!(body["reasoning"], serde_json::json!({"effort": "none"}));
        assert_eq!(
            body["model"], "extra-body-override",
            "extra_body keys must override identically named top-level keys, matching chat-path semantics"
        );
    }

    #[test]
    fn responses_extra_body_non_object_is_dropped_without_reaching_the_body() {
        let mut req = base_response_request();
        req.extra_body = Some(serde_json::json!(["not", "an", "object"]));

        let body = response_request_body(&req).expect("body should serialize");

        assert!(
            body.get("extra_body").is_none(),
            "non-object extra_body must never reach the wire"
        );
        assert_eq!(
            body["model"], "gpt-5",
            "a dropped extra_body must leave the real fields untouched"
        );
    }

    #[test]
    fn responses_streaming_path_also_merges_extra_body_and_keeps_stream_forced() {
        let mut req = base_response_request();
        req.extra_body = Some(serde_json::json!({
            "reasoning": {"effort": "none"},
            "stream": false
        }));

        let body = response_stream_request_body(req).expect("body should serialize");

        assert!(
            body.get("extra_body").is_none(),
            "extra_body key must be removed from the streaming Responses body too"
        );
        assert_eq!(body["reasoning"], serde_json::json!({"effort": "none"}));
        assert_eq!(
            body["stream"],
            serde_json::json!(true),
            "the streaming transport's stream flag must win over extra_body"
        );
    }

    /// When the resolved provider is Vertex AI and the caller supplied neither
    /// an explicit `api_key` nor a `credential_provider`, `DefaultClient::new`
    /// auto-installs `VertexAdcCredentialProvider`. This is the canonical auth
    /// path for GKE / Workload Identity deployments where short-lived OAuth2
    /// tokens come from the metadata server.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_ai_auto_installs_adc_provider_when_no_credentials_configured() {
        // ~keep SAFETY: serial_test::serial prevents concurrent env mutation.
        // ~keep Prior values are restored by the guard below.
        let prior_project = std::env::var("VERTEXAI_PROJECT").ok();
        let prior_location = std::env::var("VERTEXAI_LOCATION").ok();
        struct EnvGuard {
            prior_project: Option<String>,
            prior_location: Option<String>,
        }
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                // ~keep SAFETY: single-threaded restoration during test teardown.
                unsafe {
                    match &self.prior_project {
                        Some(v) => std::env::set_var("VERTEXAI_PROJECT", v),
                        None => std::env::remove_var("VERTEXAI_PROJECT"),
                    }
                    match &self.prior_location {
                        Some(v) => std::env::set_var("VERTEXAI_LOCATION", v),
                        None => std::env::remove_var("VERTEXAI_LOCATION"),
                    }
                }
            }
        }
        let _guard = EnvGuard {
            prior_project,
            prior_location,
        };
        // ~keep SAFETY: serial_test::serial ensures exclusive env access.
        unsafe {
            std::env::set_var("VERTEXAI_PROJECT", "test-project");
            std::env::set_var("VERTEXAI_LOCATION", "us-central1");
        }

        let config = ClientConfigBuilder::new("").load_env(false).build();
        assert!(
            config.credential_provider.is_none(),
            "input config should have no credential_provider"
        );

        let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.5-flash-lite"))
            .expect("DefaultClient::new should succeed for vertex with empty api_key");

        assert!(
            client.config.credential_provider.is_some(),
            "DefaultClient::new should auto-install VertexAdcCredentialProvider for vertex_ai when no credentials are configured"
        );
    }

    /// When the caller already supplied an `api_key`, the auto-install does
    /// not fire — pre-obtained tokens take precedence over ADC discovery.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_ai_explicit_api_key_skips_auto_install() {
        let prior_project = std::env::var("VERTEXAI_PROJECT").ok();
        struct EnvGuard(Option<String>);
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe {
                    match &self.0 {
                        Some(v) => std::env::set_var("VERTEXAI_PROJECT", v),
                        None => std::env::remove_var("VERTEXAI_PROJECT"),
                    }
                }
            }
        }
        let _guard = EnvGuard(prior_project);
        unsafe {
            std::env::set_var("VERTEXAI_PROJECT", "test-project");
        }

        let config = ClientConfigBuilder::new("ya29.pre-obtained-token")
            .load_env(false)
            .build();

        let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.5-flash-lite"))
            .expect("DefaultClient::new should succeed");

        assert!(
            client.config.credential_provider.is_none(),
            "auto-install must not fire when api_key is non-empty"
        );
    }

    /// When the caller explicitly supplied a `credential_provider`, the
    /// auto-install does not overwrite it.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_ai_explicit_credential_provider_skips_auto_install() {
        use std::sync::Arc;

        use crate::auth::{Credential, CredentialProvider, StaticTokenProvider};
        use secrecy::SecretString;

        let prior_project = std::env::var("VERTEXAI_PROJECT").ok();
        struct EnvGuard(Option<String>);
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe {
                    match &self.0 {
                        Some(v) => std::env::set_var("VERTEXAI_PROJECT", v),
                        None => std::env::remove_var("VERTEXAI_PROJECT"),
                    }
                }
            }
        }
        let _guard = EnvGuard(prior_project);
        unsafe {
            std::env::set_var("VERTEXAI_PROJECT", "test-project");
        }

        let explicit: Arc<dyn CredentialProvider> =
            Arc::new(StaticTokenProvider::new(SecretString::from("static-token".to_owned())));
        let explicit_marker = Arc::as_ptr(&explicit) as *const ();

        let config = ClientConfigBuilder::new("")
            .load_env(false)
            .credential_provider(Arc::clone(&explicit))
            .build();

        let client = DefaultClient::new(config, Some("vertex_ai/gemini-2.5-flash-lite"))
            .expect("DefaultClient::new should succeed");

        let installed = client
            .config
            .credential_provider
            .as_ref()
            .expect("explicit provider should survive auto-install path");
        let installed_marker = Arc::as_ptr(installed) as *const ();
        assert_eq!(
            installed_marker, explicit_marker,
            "auto-install must not overwrite an explicitly-supplied credential_provider"
        );

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let credential = rt.block_on(installed.resolve()).expect("resolve");
        match credential {
            Credential::BearerToken(t) => {
                use secrecy::ExposeSecret;
                assert_eq!(t.expose_secret(), "static-token");
            }
            _ => panic!("expected BearerToken"),
        }
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn vertex_client() -> DefaultClient {
        DefaultClient::new(
            ClientConfigBuilder::new("ya29.pre-obtained-token")
                .load_env(false)
                .build(),
            Some("vertex_ai/gemini-2.5-flash"),
        )
        .expect("DefaultClient::new should succeed")
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    struct VertexProjectGuard(Option<String>);

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    impl VertexProjectGuard {
        fn set() -> Self {
            let prior = std::env::var("VERTEXAI_PROJECT").ok();
            unsafe {
                std::env::set_var("VERTEXAI_PROJECT", "test-project");
            }
            Self(prior)
        }
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    impl Drop for VertexProjectGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VERTEXAI_PROJECT", value),
                    None => std::env::remove_var("VERTEXAI_PROJECT"),
                }
            }
        }
    }

    /// Vertex/Gemini rebuilds the body into native `generateContent` shape, which has no
    /// `stream` field. Restoring the transport flag after `transform_request` therefore shipped
    /// `"stream": false` on every non-streaming call, and Vertex rejects the whole request with
    /// HTTP 400 `Invalid JSON payload received. Unknown name "stream": Cannot find field.`
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_non_streaming_request_carries_no_stream_key() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = ChatCompletionRequest {
            model: "vertex_ai/gemini-2.5-flash".into(),
            messages: vec![],
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("stream").is_none(),
            "generateContent has no `stream` field; sending it is a hard 400, body was: {}",
            prepared.body_json
        );
    }

    /// Streaming against Vertex is selected by endpoint (`streamGenerateContent`), not by a body
    /// flag, so the streaming path must not reintroduce the key either.
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_streaming_request_carries_no_stream_key() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = ChatCompletionRequest {
            model: "vertex_ai/gemini-2.5-flash".into(),
            messages: vec![],
            ..Default::default()
        };

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(true))
            .expect("prepare_request should not fail");

        assert!(
            prepared.body_json.get("stream").is_none(),
            "streaming is endpoint-selected on Vertex; body was: {}",
            prepared.body_json
        );
    }

    /// A typed `ReasoningEffort::None` must reach the wire as each provider's own "thinking off"
    /// setting when the request goes through the client, not only when a provider test hands
    /// `transform_request` a hand-written JSON body.
    fn reasoning_none_request(model: &str) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: model.into(),
            messages: vec![crate::types::Message::User(crate::types::UserMessage {
                content: crate::types::UserContent::Text("Hi".into()),
                name: None,
            })],
            max_tokens: Some(100),
            reasoning_effort: Some(crate::types::ReasoningEffort::None),
            ..Default::default()
        }
    }

    #[test]
    fn anthropic_client_request_with_reasoning_effort_none_disables_thinking() {
        let model = "claude-sonnet-4-20250514";
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some(model))
            .expect("client construction should succeed");
        let req = reasoning_none_request(model);

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert_eq!(prepared.body_json["thinking"], serde_json::json!({"type": "disabled"}));
        assert!(prepared.body_json.get("reasoning_effort").is_none());
    }

    #[test]
    #[serial_test::serial]
    fn bedrock_client_request_with_reasoning_effort_none_disables_thinking() {
        let model = "bedrock/anthropic.claude-sonnet-4-20250514-v1:0";
        // Explicit credentials: building a Bedrock client without them reads the environment and
        // fails on a machine that has no AWS credentials.
        let config = ClientConfigBuilder::new("test-key")
            .bedrock_credentials("test-access-key", "test-secret-key", None)
            .build();
        let client = DefaultClient::new(config, Some(model)).expect("client construction should succeed");
        let req = reasoning_none_request(model);

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["additionalModelRequestFields"]["thinking"],
            serde_json::json!({"type": "disabled"})
        );
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_client_request_with_reasoning_effort_none_sets_zero_thinking_budget() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = reasoning_none_request("vertex_ai/gemini-2.5-flash");

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["generationConfig"]["thinkingConfig"],
            serde_json::json!({"thinkingBudget": 0}),
            "none sends a zero budget and requests no thought parts"
        );
    }

    #[test]
    fn google_ai_client_request_with_reasoning_effort_none_sets_zero_thinking_budget() {
        let model = "gemini/gemini-2.5-flash";
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some(model))
            .expect("client construction should succeed");
        let req = reasoning_none_request(model);

        let prepared = client
            .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(false))
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["generationConfig"]["thinkingConfig"],
            serde_json::json!({"thinkingBudget": 0}),
            "none sends a zero budget and requests no thought parts"
        );
    }

    /// Guard against over-correcting: providers whose wire format does carry `stream` must still
    /// get the transport-controlled flag, so the request cannot desync from the response parser.
    #[test]
    fn openai_shaped_provider_still_carries_the_stream_flag() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            ..Default::default()
        };

        for streaming in [false, true] {
            let prepared = client
                .prepare_request(&req, |p| p.chat_completions_path(), &req.model, Some(streaming))
                .expect("prepare_request should not fail");
            assert_eq!(
                prepared.body_json["stream"], streaming,
                "OpenAI-shaped bodies must keep the transport-controlled stream flag"
            );
            if streaming {
                assert_eq!(prepared.body_json["stream_options"]["include_usage"], true);
            } else {
                assert!(prepared.body_json.get("stream_options").is_none());
            }
        }
    }

    #[test]
    fn caller_can_disable_default_stream_usage_request() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("gpt-4"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![],
            stream_options: Some(crate::types::StreamOptions {
                include_usage: Some(false),
            }),
            ..Default::default()
        };
        let prepared = client
            .prepare_request(
                &req,
                |provider| provider.chat_completions_path(),
                &req.model,
                Some(true),
            )
            .expect("prepare_request should not fail");
        assert_eq!(prepared.body_json["stream_options"]["include_usage"], false);
    }

    #[test]
    fn openai_reasoning_models_translate_max_tokens() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("openai/gpt-5"))
            .expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "openai/gpt-5".into(),
            messages: vec![],
            max_tokens: Some(512),
            ..Default::default()
        };
        let prepared = client
            .prepare_request(
                &req,
                |provider| provider.chat_completions_path(),
                &req.model,
                Some(false),
            )
            .expect("prepare_request should not fail");
        assert_eq!(prepared.body_json["max_completion_tokens"], 512);
        assert!(prepared.body_json.get("max_tokens").is_none());
    }

    #[test]
    fn custom_openai_compatible_endpoint_keeps_max_tokens() {
        let config = ClientConfigBuilder::new("test-key")
            .base_url("http://localhost:8080/v1")
            .build();
        let client = DefaultClient::new(config, Some("gpt-5")).expect("client construction should succeed");
        let req = ChatCompletionRequest {
            model: "gpt-5".into(),
            messages: vec![],
            max_tokens: Some(512),
            ..Default::default()
        };
        let prepared = client
            .prepare_request(
                &req,
                |provider| provider.chat_completions_path(),
                &req.model,
                Some(false),
            )
            .expect("prepare_request should not fail");
        assert_eq!(prepared.body_json["max_tokens"], 512);
        assert!(prepared.body_json.get("max_completion_tokens").is_none());
    }

    #[test]
    fn empty_api_key_omits_authorization_header() {
        let config = ClientConfigBuilder::new("").load_env(false).build();
        let client = DefaultClient::new(config, Some("openai/gpt-4o")).expect("client construction");
        assert_eq!(client.cached_auth_header, None);
    }

    #[test]
    fn anthropic_final_stream_usage_is_cumulative() {
        let provider = crate::provider::anthropic::AnthropicProvider::new();
        let cumulative = std::sync::Mutex::new(ChatStreamState::default());
        let start = r#"{"type":"message_start","message":{"id":"msg_1","model":"claude-sonnet-4-5","usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":200}}}"#;
        let delta = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":7}}"#;

        parse_chat_stream_event(&provider, &cumulative, start).expect("start event");
        let final_chunk = parse_chat_stream_event(&provider, &cumulative, delta)
            .expect("delta event")
            .expect("final chunk");
        let usage = final_chunk.usage.expect("final cumulative usage");
        assert_eq!(usage.prompt_tokens, 310);
        assert_eq!(usage.completion_tokens, 7);
        assert_eq!(usage.total_tokens, 317);
        let details = usage.prompt_tokens_details.expect("cache details");
        assert_eq!(details.cached_tokens, 200);
        assert_eq!(details.cache_creation_tokens, 100);
    }

    fn embed_request(model: &str, extra_body: Option<serde_json::Value>) -> EmbeddingRequest {
        EmbeddingRequest {
            model: model.into(),
            input: crate::types::EmbeddingInput::Single("what is rust?".into()),
            extra_body,
            ..Default::default()
        }
    }

    fn bedrock_embed_client(model: &str) -> DefaultClient {
        let config = ClientConfigBuilder::new("test-key")
            .bedrock_credentials("test-access-key", "test-secret-key", None)
            .build();
        DefaultClient::new(config, Some(model)).expect("client construction should succeed")
    }

    #[test]
    #[serial_test::serial]
    fn bedrock_cohere_embed_keeps_search_document_when_the_caller_sets_no_input_type() {
        let model = "bedrock/cohere.embed-english-v3";
        let client = bedrock_embed_client(model);
        let req = embed_request(model, None);

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(prepared.body_json["input_type"], "search_document");
        assert_eq!(prepared.body_json["texts"], serde_json::json!(["what is rust?"]));
    }

    #[test]
    #[serial_test::serial]
    fn bedrock_cohere_embed_takes_input_type_and_other_fields_from_extra_body() {
        let model = "bedrock/cohere.embed-english-v3";
        let client = bedrock_embed_client(model);
        let req = embed_request(
            model,
            Some(serde_json::json!({"input_type": "search_query", "truncate": "END"})),
        );

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(prepared.body_json["input_type"], "search_query");
        assert_eq!(prepared.body_json["truncate"], "END");
        assert!(prepared.body_json.get("extra_body").is_none());
        let sent: serde_json::Value = serde_json::from_slice(&prepared.body_bytes).expect("body bytes are JSON");
        assert_eq!(sent, prepared.body_json, "the signed bytes carry the merged body");
    }

    #[test]
    #[serial_test::serial]
    fn bedrock_titan_embed_takes_extra_body_fields() {
        let model = "bedrock/amazon.titan-embed-text-v2:0";
        let client = bedrock_embed_client(model);
        let req = embed_request(model, Some(serde_json::json!({"normalize": false})));

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(prepared.body_json["inputText"], "what is rust?");
        assert_eq!(prepared.body_json["normalize"], false);
        assert!(prepared.body_json.get("extra_body").is_none());
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_embed_sends_dimensions_as_output_dimensionality() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = EmbeddingRequest {
            dimensions: Some(256),
            ..embed_request("vertex_ai/text-embedding-005", None)
        };

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["parameters"],
            serde_json::json!({"outputDimensionality": 256})
        );
        assert_eq!(
            prepared.body_json["instances"],
            serde_json::json!([{"content": "what is rust?"}])
        );
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_embed_without_dimensions_sends_no_parameters() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = embed_request("vertex_ai/text-embedding-005", None);

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert!(prepared.body_json.get("parameters").is_none());
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_embed_takes_extra_body_fields() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = embed_request(
            "vertex_ai/text-embedding-005",
            Some(serde_json::json!({"parameters": {"autoTruncate": false}})),
        );

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["parameters"],
            serde_json::json!({"autoTruncate": false})
        );
        assert!(prepared.body_json.get("extra_body").is_none());
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_embed_keeps_dimensions_when_extra_body_sets_parameters() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = EmbeddingRequest {
            dimensions: Some(256),
            ..embed_request(
                "vertex_ai/text-embedding-005",
                Some(serde_json::json!({"parameters": {"autoTruncate": false}})),
            )
        };

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["parameters"],
            serde_json::json!({"outputDimensionality": 256, "autoTruncate": false})
        );
        assert!(prepared.body_json.get("extra_body").is_none());
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    #[test]
    #[serial_test::serial]
    fn vertex_embed_extra_body_parameters_override_the_dimensions_key() {
        let _guard = VertexProjectGuard::set();
        let client = vertex_client();
        let req = EmbeddingRequest {
            dimensions: Some(256),
            ..embed_request(
                "vertex_ai/text-embedding-005",
                Some(serde_json::json!({"parameters": {"outputDimensionality": 512, "autoTruncate": false}})),
            )
        };

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json["parameters"],
            serde_json::json!({"outputDimensionality": 512, "autoTruncate": false})
        );
    }

    #[test]
    fn gemini_api_embed_takes_extra_body_fields_in_each_request() {
        for model in ["gemini/text-embedding-004", "google_ai/text-embedding-004"] {
            let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some(model))
                .expect("client construction should succeed");
            let req = EmbeddingRequest {
                input: crate::types::EmbeddingInput::Multiple(vec!["one".into(), "two".into()]),
                dimensions: Some(128),
                ..embed_request(
                    model,
                    Some(serde_json::json!({"taskType": "RETRIEVAL_QUERY", "title": "doc"})),
                )
            };

            let prepared = client
                .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
                .expect("prepare_request should not fail");

            let requests = prepared.body_json["requests"].as_array().expect("requests array");
            assert_eq!(requests.len(), 2, "{model}");
            for request in requests {
                assert_eq!(request["taskType"], "RETRIEVAL_QUERY", "{model}");
                assert_eq!(request["title"], "doc", "{model}");
                assert_eq!(request["outputDimensionality"], 128, "{model}");
            }
            assert!(prepared.body_json.get("extra_body").is_none(), "{model}");
            assert!(prepared.body_json.get("taskType").is_none(), "{model}");
        }
    }

    #[test]
    fn ollama_embed_takes_extra_body_fields() {
        let model = "ollama/nomic-embed-text";
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some(model))
            .expect("client construction should succeed");
        let req = embed_request(
            model,
            Some(serde_json::json!({"truncate": false, "keep_alive": "5m", "num_ctx": 8192})),
        );

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(prepared.body_json["options"]["num_ctx"], 8192);
        assert!(prepared.body_json.get("num_ctx").is_none());
        assert_eq!(prepared.body_json["truncate"], false);
        assert_eq!(prepared.body_json["keep_alive"], "5m");
        assert_eq!(prepared.body_json["input"], "what is rust?");
        assert!(prepared.body_json.get("extra_body").is_none());
    }

    /// Every non-chat request type accepts `extra_body` and sends its keys as body fields.
    #[test]
    fn non_chat_requests_merge_extra_body_into_the_provider_body() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("openai/model"))
            .expect("client construction should succeed");
        let extra = Some(serde_json::json!({"provider_option": "value"}));
        let model = "openai/model";

        let bodies = vec![
            (
                "embed",
                client.prepare_request(
                    &embed_request(model, extra.clone()),
                    |p| p.embeddings_path(),
                    model,
                    None,
                ),
            ),
            (
                "rerank",
                client.prepare_request(
                    &RerankRequest {
                        model: model.into(),
                        extra_body: extra.clone(),
                        ..Default::default()
                    },
                    |p| p.rerank_path(),
                    model,
                    None,
                ),
            ),
            (
                "image",
                client.prepare_request(
                    &CreateImageRequest {
                        prompt: "a cat".into(),
                        extra_body: extra.clone(),
                        ..Default::default()
                    },
                    |p| p.image_generations_path(),
                    model,
                    None,
                ),
            ),
            (
                "speech",
                client.prepare_request(
                    &CreateSpeechRequest {
                        model: model.into(),
                        extra_body: extra.clone(),
                        ..Default::default()
                    },
                    |p| p.audio_speech_path(),
                    model,
                    None,
                ),
            ),
            (
                "transcription",
                client.prepare_request(
                    &CreateTranscriptionRequest {
                        model: model.into(),
                        extra_body: extra.clone(),
                        ..Default::default()
                    },
                    |p| p.audio_transcriptions_path(),
                    model,
                    None,
                ),
            ),
            (
                "moderation",
                client.prepare_request(
                    &ModerationRequest {
                        extra_body: extra.clone(),
                        ..Default::default()
                    },
                    |p| p.moderations_path(),
                    model,
                    None,
                ),
            ),
            (
                "search",
                client.prepare_request(
                    &SearchRequest {
                        model: model.into(),
                        extra_body: extra.clone(),
                        ..Default::default()
                    },
                    |p| p.search_path(),
                    model,
                    None,
                ),
            ),
            (
                "ocr",
                client.prepare_request(
                    &OcrRequest {
                        model: model.into(),
                        extra_body: extra.clone(),
                        ..Default::default()
                    },
                    |p| p.ocr_path(),
                    model,
                    None,
                ),
            ),
        ];

        assert_eq!(bodies.len(), 8);
        for (kind, prepared) in bodies {
            let prepared = prepared.unwrap_or_else(|e| panic!("{kind}: prepare_request failed: {e}"));
            assert_eq!(prepared.body_json["provider_option"], "value", "{kind}: key is merged");
            assert!(
                prepared.body_json.get("extra_body").is_none(),
                "{kind}: wrapper is removed"
            );
        }
    }

    #[test]
    fn non_chat_request_without_extra_body_is_unchanged() {
        let client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some("openai/model"))
            .expect("client construction should succeed");
        let req = embed_request("openai/model", None);

        let prepared = client
            .prepare_request(&req, |p| p.embeddings_path(), &req.model, None)
            .expect("prepare_request should not fail");

        assert_eq!(
            prepared.body_json,
            serde_json::json!({"model": "model", "input": "what is rust?"})
        );
    }
}

/// Embed and rerank calls of a client that resolves the native Cohere provider.
///
/// A client with a `base_url` uses a generic provider, so these tests build the client for a
/// `cohere/` model and then point the native provider at a local server.
#[cfg(all(test, feature = "native-http", not(target_arch = "wasm32")))]
mod cohere_client_tests {
    use std::borrow::Cow;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    use serde_json::json;
    use serial_test::serial;

    use super::*;
    use crate::client::config::ClientConfigBuilder;
    use crate::provider::cohere::CohereProvider;
    use crate::provider::{OutboundPolicy, set_outbound_policy};
    use crate::types::EmbeddingInput;
    use crate::types::rerank::RerankDocument;

    static COHERE: CohereProvider = CohereProvider;

    /// The native Cohere provider at a local base URL. Each method that Cohere overrides calls
    /// Cohere's own, so the base URL is the only difference.
    struct LocalCohere {
        base_url: String,
    }

    impl Provider for LocalCohere {
        fn name(&self) -> &str {
            COHERE.name()
        }

        fn base_url(&self) -> &str {
            &self.base_url
        }

        fn env_var(&self) -> Option<&str> {
            COHERE.env_var()
        }

        fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
            COHERE.auth_header(api_key)
        }

        fn matches_model(&self, model: &str) -> bool {
            COHERE.matches_model(model)
        }

        fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
            COHERE.strip_model_prefix(model)
        }

        fn chat_completions_path(&self) -> &str {
            COHERE.chat_completions_path()
        }

        fn embeddings_path(&self) -> &str {
            COHERE.embeddings_path()
        }

        fn models_path(&self) -> &str {
            COHERE.models_path()
        }

        fn rerank_path(&self) -> &str {
            COHERE.rerank_path()
        }

        fn build_url(&self, endpoint_path: &str, model: &str) -> String {
            COHERE
                .build_url(endpoint_path, model)
                .replacen(COHERE.base_url(), &self.base_url, 1)
        }

        fn transform_request(&self, body: &mut serde_json::Value) -> Result<()> {
            COHERE.transform_request(body)
        }

        fn transform_embedding_request(&self, body: &mut serde_json::Value) -> Result<()> {
            COHERE.transform_embedding_request(body)
        }

        fn transform_embedding_response(&self, body: &mut serde_json::Value, request_model: &str) -> Result<()> {
            COHERE.transform_embedding_response(body, request_model)
        }

        fn parse_stream_event(&self, event_data: &str) -> Result<Option<ChatCompletionChunk>> {
            COHERE.parse_stream_event(event_data)
        }

        fn transform_response(&self, body: &mut serde_json::Value) -> Result<()> {
            COHERE.transform_response(body)
        }
    }

    /// A client built for `model`, with its native Cohere provider moved to `base_url`.
    fn local_cohere_client(model: &str, base_url: String) -> DefaultClient {
        let mut client = DefaultClient::new(ClientConfigBuilder::new("test-key").build(), Some(model))
            .expect("client construction should succeed");
        assert_eq!(
            client.provider.name(),
            "cohere",
            "a cohere/ model resolves the native provider"
        );
        client.provider = Arc::new(LocalCohere { base_url });
        client
    }

    /// The request line and the JSON body of each request a local server took.
    type Requests = Vec<(String, serde_json::Value)>;

    /// A local server that answers `count` requests with `reply`, one request per connection.
    fn serve(reply: &'static str, count: usize) -> (String, std::thread::JoinHandle<Requests>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the local server");
        let base_url = format!("http://{}/v2", listener.local_addr().expect("local address"));
        let handle = std::thread::spawn(move || {
            (0..count)
                .map(|_| {
                    let (stream, _) = listener.accept().expect("accept a request");
                    let mut reader = BufReader::new(stream);
                    let mut request_line = String::new();
                    reader.read_line(&mut request_line).expect("read the request line");
                    let mut length = 0;
                    loop {
                        let mut line = String::new();
                        reader.read_line(&mut line).expect("read a header line");
                        if line.trim_end().is_empty() {
                            break;
                        }
                        if let Some((name, value)) = line.split_once(':')
                            && name.eq_ignore_ascii_case("content-length")
                        {
                            length = value.trim().parse().expect("a numeric content length");
                        }
                    }
                    let mut body = vec![0; length];
                    reader.read_exact(&mut body).expect("read the request body");
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                        reply.len()
                    );
                    reader
                        .get_mut()
                        .write_all(response.as_bytes())
                        .expect("write the reply");
                    let body = serde_json::from_slice(&body).expect("a JSON request body");
                    (request_line.trim_end().to_owned(), body)
                })
                .collect()
        });
        (base_url, handle)
    }

    const EMBED_MODEL: &str = "cohere/embed-english-v3.0";
    const EMBED_REPLY: &str = r#"{"id":"e1","embeddings":{"float":[[0.5,0.25]]},"texts":["alpha"],"meta":{"billed_units":{"input_tokens":3}}}"#;

    fn embed_request() -> EmbeddingRequest {
        EmbeddingRequest {
            model: EMBED_MODEL.into(),
            input: EmbeddingInput::Single("alpha".into()),
            ..Default::default()
        }
    }

    /// Revert line: call `transform_response` in place of `transform_embedding_response` in
    /// `embed` to make this test fail.
    #[tokio::test]
    #[serial(outbound_policy)]
    async fn embed_of_a_cohere_model_sends_the_cohere_body_and_names_the_reply_model() {
        set_outbound_policy(OutboundPolicy::Off);
        let (base_url, server) = serve(EMBED_REPLY, 1);

        let reply = local_cohere_client(EMBED_MODEL, base_url)
            .embed(embed_request())
            .await
            .expect("the embed call should succeed");
        let requests = server.join().expect("join the local server");

        assert_eq!(requests[0].0, "POST /v2/embed HTTP/1.1");
        assert_eq!(requests[0].1["texts"], json!(["alpha"]));
        assert_eq!(reply.model, EMBED_MODEL);
        assert_eq!(reply.data.len(), 1);
        assert_eq!(reply.data[0].embedding, vec![0.5, 0.25]);
        assert_eq!(reply.usage.map(|usage| usage.prompt_tokens), Some(3));
    }

    /// Revert line: call `transform_response` in place of `transform_embedding_response` in
    /// `embed_raw` to make this test fail.
    #[tokio::test]
    #[serial(outbound_policy)]
    async fn embed_raw_of_a_cohere_model_sends_the_cohere_body_and_names_the_reply_model() {
        set_outbound_policy(OutboundPolicy::Off);
        let (base_url, server) = serve(EMBED_REPLY, 1);

        let exchange = local_cohere_client(EMBED_MODEL, base_url)
            .embed_raw(embed_request())
            .await
            .expect("the embed_raw call should succeed");
        let requests = server.join().expect("join the local server");

        assert_eq!(requests[0].0, "POST /v2/embed HTTP/1.1");
        assert_eq!(exchange.raw_request["texts"], json!(["alpha"]));
        assert_eq!(exchange.raw_response.as_ref().map(|raw| &raw["id"]), Some(&json!("e1")));
        assert_eq!(exchange.data.model, EMBED_MODEL);
        assert_eq!(exchange.data.data[0].embedding, vec![0.5, 0.25]);
    }

    const RERANK_MODEL: &str = "cohere/rerank-v3.5";
    const RERANK_REPLY: &str = r#"{"id":"r1","results":[{"index":1,"relevance_score":0.9},{"index":0,"relevance_score":0.1}],"meta":{"billed_units":{"search_units":1}}}"#;

    fn rerank_request() -> RerankRequest {
        RerankRequest {
            model: RERANK_MODEL.into(),
            query: "what is rust?".into(),
            documents: vec![
                RerankDocument::Text("a fungus".into()),
                RerankDocument::Text("a language".into()),
            ],
            top_n: Some(2),
            ..Default::default()
        }
    }

    fn assert_cohere_rerank_body(line: &str, body: &serde_json::Value) {
        assert_eq!(line, "POST /v2/rerank HTTP/1.1");
        assert_eq!(
            body,
            &json!({
                "model": "rerank-v3.5",
                "query": "what is rust?",
                "documents": ["a fungus", "a language"],
                "top_n": 2
            })
        );
    }

    fn assert_cohere_rerank_reply(reply: &RerankResponse) {
        assert_eq!(reply.id.as_deref(), Some("r1"));
        let ranked: Vec<(u32, f64)> = reply
            .results
            .iter()
            .map(|result| (result.index, result.relevance_score))
            .collect();
        assert_eq!(ranked, vec![(1, 0.9), (0, 0.1)]);
    }

    /// Revert line: pass `transform_embedding_request` in place of `transform_request` at the
    /// rerank call sites to make this test fail.
    #[tokio::test]
    #[serial(outbound_policy)]
    async fn rerank_of_a_cohere_model_sends_the_rerank_body_and_parses_the_reply() {
        set_outbound_policy(OutboundPolicy::Off);
        let (base_url, server) = serve(RERANK_REPLY, 2);
        let client = local_cohere_client(RERANK_MODEL, base_url);

        let reply = client
            .rerank(rerank_request())
            .await
            .expect("the rerank call should succeed");
        let exchange = client
            .rerank_raw(rerank_request())
            .await
            .expect("the rerank_raw call should succeed");
        let requests = server.join().expect("join the local server");

        assert_cohere_rerank_body(&requests[0].0, &requests[0].1);
        assert_cohere_rerank_reply(&reply);
        assert_cohere_rerank_body(&requests[1].0, &requests[1].1);
        assert_eq!(exchange.raw_request, requests[1].1);
        assert_cohere_rerank_reply(&exchange.data);
    }
}
