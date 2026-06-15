use serde::Serialize;

use crate::client::BoxStream;
use crate::error::Result;
use crate::tenant::TenantId;
use crate::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest, TranscriptionResponse};
use crate::types::image::{CreateImageRequest, ImagesResponse};
use crate::types::moderation::{ModerationRequest, ModerationResponse};
use crate::types::ocr::{OcrRequest, OcrResponse};
use crate::types::rerank::{RerankRequest, RerankResponse};
use crate::types::search::{SearchRequest, SearchResponse};
use crate::types::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ModelsListResponse, Usage,
};

/// Discriminant enum for [`LlmRequest`].
///
/// Use the associated constructors on [`LlmRequest`] (e.g. [`LlmRequest::Chat`])
/// rather than constructing this directly; they return a fully initialised
/// `LlmRequest` with `tenant_id: None`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(alef, alef(skip))]
pub enum LlmRequestKind {
    /// Non-streaming chat completion.
    Chat(ChatCompletionRequest),
    /// Streaming chat completion — yields a stream of chunks.
    ChatStream(ChatCompletionRequest),
    /// Text embedding.
    Embed(EmbeddingRequest),
    /// List available models from the provider.
    ListModels,
    /// Image generation.
    ImageGenerate(CreateImageRequest),
    /// Text-to-speech audio generation.
    Speech(CreateSpeechRequest),
    /// Audio transcription.
    Transcribe(CreateTranscriptionRequest),
    /// Content moderation.
    Moderate(ModerationRequest),
    /// Document reranking.
    Rerank(RerankRequest),
    /// Web/document search.
    Search(SearchRequest),
    /// Document OCR.
    Ocr(OcrRequest),
}

/// The request passed through the Tower `Service` stack.
///
/// Constructed via the associated constructors (e.g. [`LlmRequest::Chat`]).
/// Attach a tenant identity with [`LlmRequest::with_tenant_id`] and read it
/// back with [`LlmRequest::tenant_id`].
///
/// Serializes as the inner [`LlmRequestKind`] (tenant_id and idempotency_key
/// are infra metadata, not provider payload).
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct LlmRequest {
    /// The request payload and discriminant.
    pub kind: LlmRequestKind,
    /// Optional tenant identity propagated through the Tower stack.
    pub tenant_id: Option<TenantId>,
    /// Optional idempotency key (OpenAI `Idempotency-Key` convention).
    ///
    /// When set, [`crate::tower::idempotency::IdempotencyLayer`] deduplicates
    /// requests sharing the same key and body within the configured TTL.
    /// Requests with the same key but a different body return
    /// [`crate::error::LiterLlmError::IdempotencyConflict`].
    pub idempotency_key: Option<String>,
}

impl serde::Serialize for LlmRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.kind.serialize(serializer)
    }
}

// ── Constructors ──────────────────────────────────────────────────────────────

// Constructors mirror the old enum variant syntax (`LlmRequest::Chat(r)`) so
// that all existing call sites continue to compile without modification.
// The non_snake_case names are intentional and match the previous enum variants.
#[allow(non_snake_case)]
#[cfg_attr(alef, alef(skip))]
impl LlmRequest {
    /// Non-streaming chat completion.
    #[must_use]
    pub fn Chat(req: ChatCompletionRequest) -> Self {
        Self {
            kind: LlmRequestKind::Chat(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Streaming chat completion.
    #[must_use]
    pub fn ChatStream(req: ChatCompletionRequest) -> Self {
        Self {
            kind: LlmRequestKind::ChatStream(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Text embedding.
    #[must_use]
    pub fn Embed(req: EmbeddingRequest) -> Self {
        Self {
            kind: LlmRequestKind::Embed(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// List available models.
    #[must_use]
    pub fn ListModels() -> Self {
        Self {
            kind: LlmRequestKind::ListModels,
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Image generation.
    #[must_use]
    pub fn ImageGenerate(req: CreateImageRequest) -> Self {
        Self {
            kind: LlmRequestKind::ImageGenerate(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Text-to-speech audio generation.
    #[must_use]
    pub fn Speech(req: CreateSpeechRequest) -> Self {
        Self {
            kind: LlmRequestKind::Speech(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Audio transcription.
    #[must_use]
    pub fn Transcribe(req: CreateTranscriptionRequest) -> Self {
        Self {
            kind: LlmRequestKind::Transcribe(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Content moderation.
    #[must_use]
    pub fn Moderate(req: ModerationRequest) -> Self {
        Self {
            kind: LlmRequestKind::Moderate(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Document reranking.
    #[must_use]
    pub fn Rerank(req: RerankRequest) -> Self {
        Self {
            kind: LlmRequestKind::Rerank(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Web/document search.
    #[must_use]
    pub fn Search(req: SearchRequest) -> Self {
        Self {
            kind: LlmRequestKind::Search(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }

    /// Document OCR.
    #[must_use]
    pub fn Ocr(req: OcrRequest) -> Self {
        Self {
            kind: LlmRequestKind::Ocr(req),
            tenant_id: None,
            idempotency_key: None,
        }
    }
}

// ── Accessors and builder ─────────────────────────────────────────────────────

#[cfg_attr(alef, alef(skip))]
impl LlmRequest {
    /// OpenTelemetry GenAI `gen_ai.operation.name` value for this request.
    #[must_use]
    pub fn operation_name(&self) -> &'static str {
        match &self.kind {
            LlmRequestKind::Chat(_) | LlmRequestKind::ChatStream(_) => "chat",
            LlmRequestKind::Embed(_) => "embeddings",
            LlmRequestKind::ListModels => "list_models",
            LlmRequestKind::ImageGenerate(_) => "image_generate",
            LlmRequestKind::Speech(_) => "speech",
            LlmRequestKind::Transcribe(_) => "transcribe",
            LlmRequestKind::Moderate(_) => "moderate",
            LlmRequestKind::Rerank(_) => "rerank",
            LlmRequestKind::Search(_) => "search",
            LlmRequestKind::Ocr(_) => "ocr",
        }
    }

    /// Human-readable name of the request type; used as a span / metric label.
    #[must_use]
    pub fn request_type(&self) -> &'static str {
        match &self.kind {
            LlmRequestKind::Chat(_) => "chat",
            LlmRequestKind::ChatStream(_) => "chat_stream",
            LlmRequestKind::Embed(_) => "embeddings",
            LlmRequestKind::ListModels => "list_models",
            LlmRequestKind::ImageGenerate(_) => "image_generate",
            LlmRequestKind::Speech(_) => "speech",
            LlmRequestKind::Transcribe(_) => "transcribe",
            LlmRequestKind::Moderate(_) => "moderate",
            LlmRequestKind::Rerank(_) => "rerank",
            LlmRequestKind::Search(_) => "search",
            LlmRequestKind::Ocr(_) => "ocr",
        }
    }

    /// Return the model name embedded in the request, if any.
    #[must_use]
    pub fn model(&self) -> Option<&str> {
        match &self.kind {
            LlmRequestKind::Chat(r) | LlmRequestKind::ChatStream(r) => Some(r.model.as_str()),
            LlmRequestKind::Embed(r) => Some(r.model.as_str()),
            LlmRequestKind::ImageGenerate(r) => r.model.as_deref(),
            LlmRequestKind::Speech(r) => Some(r.model.as_str()),
            LlmRequestKind::Transcribe(r) => Some(r.model.as_str()),
            LlmRequestKind::Moderate(r) => r.model.as_deref(),
            LlmRequestKind::Rerank(r) => Some(r.model.as_str()),
            LlmRequestKind::Search(r) => Some(r.model.as_str()),
            LlmRequestKind::Ocr(r) => Some(r.model.as_str()),
            LlmRequestKind::ListModels => None,
        }
    }

    /// Attach a tenant identifier to this request.
    ///
    /// Consumed and returns `Self` so this can be chained with constructors.
    #[must_use]
    pub fn with_tenant_id(mut self, tenant_id: impl Into<TenantId>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Return the tenant identifier, if one was attached.
    #[must_use]
    pub fn tenant_id(&self) -> Option<&TenantId> {
        self.tenant_id.as_ref()
    }

    /// Attach an idempotency key to this request.
    ///
    /// When set, [`crate::tower::idempotency::IdempotencyLayer`] stores the
    /// response keyed by this string.  Subsequent requests with the same key
    /// and body return the stored response without calling the inner service.
    ///
    /// Consumed and returns `Self` so this can be chained with constructors.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// The response variant returned through the tower `Service` stack.
#[cfg_attr(alef, alef(skip))]
pub enum LlmResponse {
    /// Non-streaming chat completion.
    Chat(ChatCompletionResponse),
    /// Streaming chat completion.
    ChatStream(BoxStream<'static, Result<ChatCompletionChunk>>),
    /// Text embedding.
    Embed(EmbeddingResponse),
    /// Model list.
    ListModels(ModelsListResponse),
    /// Image generation.
    ImageGenerate(ImagesResponse),
    /// Text-to-speech audio (raw bytes).
    Speech(bytes::Bytes),
    /// Audio transcription.
    Transcribe(TranscriptionResponse),
    /// Content moderation.
    Moderate(ModerationResponse),
    /// Document reranking.
    Rerank(RerankResponse),
    /// Search results.
    Search(SearchResponse),
    /// OCR results.
    Ocr(OcrResponse),
}

#[cfg_attr(alef, alef(skip))]
impl LlmResponse {
    /// Return the usage data from the response, if present.
    ///
    /// Streaming, model-list, and non-chat responses do not carry aggregated
    /// usage data and always return `None`.
    #[must_use]
    pub fn usage(&self) -> Option<&Usage> {
        match self {
            Self::Chat(r) => r.usage.as_ref(),
            Self::Embed(r) => r.usage.as_ref(),
            Self::Ocr(r) => r.usage.as_ref(),
            Self::ChatStream(_)
            | Self::ListModels(_)
            | Self::ImageGenerate(_)
            | Self::Speech(_)
            | Self::Transcribe(_)
            | Self::Moderate(_)
            | Self::Rerank(_)
            | Self::Search(_) => None,
        }
    }
}

impl std::fmt::Debug for LlmResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Chat(r) => f.debug_tuple("Chat").field(r).finish(),
            Self::ChatStream(_) => f.write_str("ChatStream(<stream>)"),
            Self::Embed(r) => f.debug_tuple("Embed").field(r).finish(),
            Self::ListModels(r) => f.debug_tuple("ListModels").field(r).finish(),
            Self::ImageGenerate(r) => f.debug_tuple("ImageGenerate").field(r).finish(),
            Self::Speech(b) => f
                .debug_tuple("Speech")
                .field(&format_args!("<{} bytes>", b.len()))
                .finish(),
            Self::Transcribe(r) => f.debug_tuple("Transcribe").field(r).finish(),
            Self::Moderate(r) => f.debug_tuple("Moderate").field(r).finish(),
            Self::Rerank(r) => f.debug_tuple("Rerank").field(r).finish(),
            Self::Search(r) => f.debug_tuple("Search").field(r).finish(),
            Self::Ocr(r) => f.debug_tuple("Ocr").field(r).finish(),
        }
    }
}
