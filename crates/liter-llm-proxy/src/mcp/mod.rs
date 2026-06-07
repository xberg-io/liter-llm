pub mod errors;
pub mod params;

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler, tool, tool_handler, tool_router};
use tower::Service;

use liter_llm::client::{BatchClient, FileClient, ResponseClient};
use liter_llm::tower::types::{LlmRequest, LlmResponse};
use liter_llm::types::audio::{CreateSpeechRequest, CreateTranscriptionRequest};
use liter_llm::types::batch::{BatchListQuery, CreateBatchRequest};
use liter_llm::types::files::{CreateFileRequest, FileListQuery, FilePurpose};
use liter_llm::types::image::CreateImageRequest;
use liter_llm::types::moderation::ModerationRequest;
use liter_llm::types::ocr::{OcrDocument, OcrRequest};
use liter_llm::types::rerank::{RerankDocument, RerankRequest};
use liter_llm::types::responses::CreateResponseRequest;
use liter_llm::types::search::SearchRequest;
use liter_llm::types::{ChatCompletionRequest, EmbeddingRequest};

use crate::auth::KeyContext;
use crate::file_store::FileStore;
use crate::service_pool::ServicePool;

use self::errors::to_error_data;

/// Which transport is in use for this MCP server instance.
///
/// Controls how [`LiterLlmMcp`] resolves the [`KeyContext`] for each tool
/// invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpTransportKind {
    /// HTTP-based transports (`streamable_http`, SSE).
    ///
    /// rmcp 1.7's `StreamableHttpService` injects the axum
    /// `http::request::Parts` (including all request extensions) into
    /// `RequestContext.extensions` before calling any tool handler.  The
    /// `validate_api_key` axum middleware inserts a [`KeyContext`] into those
    /// request extensions, so we recover it via:
    ///
    /// ```text
    /// ctx.extensions
    ///     .get::<http::request::Parts>()
    ///     .and_then(|p| p.extensions.get::<KeyContext>())
    /// ```
    Http,

    /// Stdio transport — single long-lived process, no per-request headers.
    ///
    /// The `default_ctx` configured at startup is used for every tool call.
    Stdio,
}

/// MCP server exposing the liter-llm proxy as a set of callable tools.
///
/// Each tool corresponds to an LLM API endpoint (chat, embed, image
/// generation, etc.) or a management operation (files, batches, responses).
#[derive(Clone)]
pub struct LiterLlmMcp {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    service_pool: Arc<ServicePool>,
    #[allow(dead_code)]
    file_store: Arc<FileStore>,
    /// Context used when transport_kind is Stdio, or as a last-resort fallback
    /// for Http when the middleware did not run (should never happen in
    /// production — a warning is logged).
    default_ctx: KeyContext,
    transport_kind: McpTransportKind,
}

impl LiterLlmMcp {
    /// Create a new MCP server backed by the given service pool, file store,
    /// default key context and transport kind.
    pub fn new(
        service_pool: Arc<ServicePool>,
        file_store: Arc<FileStore>,
        default_ctx: KeyContext,
        transport_kind: McpTransportKind,
    ) -> Self {
        Self {
            tool_router: Self::tool_router(),
            service_pool,
            file_store,
            default_ctx,
            transport_kind,
        }
    }

    // ── Auth helpers ──────────────────────────────────────────────────────

    /// Resolve the [`KeyContext`] for a tool invocation.
    ///
    /// For HTTP transports rmcp's `StreamableHttpService` puts the axum
    /// `http::request::Parts` into `RequestContext.extensions`.  The
    /// `validate_api_key` middleware inserts a [`KeyContext`] into the request
    /// extensions before rmcp wraps them, so we recover it from there.
    ///
    /// For stdio there is no request — fall back to the `default_ctx`
    /// configured at startup.
    fn resolve_ctx(&self, ctx: &RequestContext<RoleServer>) -> KeyContext {
        if self.transport_kind == McpTransportKind::Http {
            if let Some(parts) = ctx.extensions.get::<http::request::Parts>()
                && let Some(key_ctx) = parts.extensions.get::<KeyContext>()
            {
                return key_ctx.clone();
            }
            // Middleware did not run — this should never happen in a correctly
            // wired HTTP deployment.
            tracing::warn!(
                "MCP HTTP tool called without a KeyContext in request extensions; \
                 falling back to default_ctx — check that validate_api_key middleware is wired"
            );
        }
        self.default_ctx.clone()
    }

    /// Guard for model-routed tools.
    ///
    /// Returns `invalid_params` if the resolved key may not access `model`.
    fn require_model_access(
        &self,
        ctx: &RequestContext<RoleServer>,
        model: &str,
    ) -> Result<KeyContext, rmcp::ErrorData> {
        let key_ctx = self.resolve_ctx(ctx);
        Self::check_model_access(&key_ctx, model)?;
        Ok(key_ctx)
    }

    /// Guard for master-only tools (file / batch / response management).
    ///
    /// These tools bypass model routing via `first_client()`.  Restricting
    /// them to master keys prevents a virtual key from seeing another
    /// tenant's batches or files.
    fn require_master(&self, ctx: &RequestContext<RoleServer>, tool: &str) -> Result<KeyContext, rmcp::ErrorData> {
        let key_ctx = self.resolve_ctx(ctx);
        Self::check_master_access(&key_ctx, tool)?;
        Ok(key_ctx)
    }

    /// Pure check: returns `invalid_params` if `key_ctx` may not access `model`.
    ///
    /// Separated from the context-resolution step so that unit tests can
    /// exercise the guard logic without needing a live [`RequestContext`].
    fn check_model_access(key_ctx: &KeyContext, model: &str) -> Result<(), rmcp::ErrorData> {
        if !key_ctx.can_access_model(model) {
            return Err(rmcp::ErrorData::invalid_params(
                format!("key '{}' is not allowed to access model '{model}'", key_ctx.key_id),
                None,
            ));
        }
        Ok(())
    }

    /// Pure check: returns `invalid_params` if `key_ctx` is not a master key.
    ///
    /// Separated from the context-resolution step so that unit tests can
    /// exercise the guard logic without needing a live [`RequestContext`].
    fn check_master_access(key_ctx: &KeyContext, tool: &str) -> Result<(), rmcp::ErrorData> {
        if !key_ctx.is_master {
            return Err(rmcp::ErrorData::invalid_params(
                format!(
                    "tool '{tool}' requires master-key access; key '{}' is restricted",
                    key_ctx.key_id
                ),
                None,
            ));
        }
        Ok(())
    }
}

// ─── Helper ──────────────────────────────────────────────────────────────────

/// Serialize a value to pretty JSON and wrap it in a successful `CallToolResult`.
fn json_success<T: serde::Serialize>(value: &T) -> Result<CallToolResult, rmcp::ErrorData> {
    let json = serde_json::to_string_pretty(value).map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

// ─── Tool implementations ────────────────────────────────────────────────────

#[tool_router]
impl LiterLlmMcp {
    // ── Chat & Embeddings ────────────────────────────────────────────────

    #[tool(description = "Send a chat completion request to an LLM")]
    async fn chat(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::ChatParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_model_access(&ctx, &params.model)?;

        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": params.model,
            "messages": params.messages,
            "temperature": params.temperature,
            "max_tokens": params.max_tokens,
        }))
        .map_err(|e| rmcp::ErrorData::invalid_params(e.to_string(), None))?;

        let mut svc = self
            .service_pool
            .get_service(&params.model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::Chat(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::Chat(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    #[tool(description = "Generate text embeddings for the given input")]
    async fn embed(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::EmbedParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_model_access(&ctx, &params.model)?;

        let req: EmbeddingRequest = serde_json::from_value(serde_json::json!({
            "model": params.model,
            "input": params.input,
        }))
        .map_err(|e| rmcp::ErrorData::invalid_params(e.to_string(), None))?;

        let mut svc = self
            .service_pool
            .get_service(&params.model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::Embed(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::Embed(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    #[tool(description = "List available models from configured providers")]
    async fn list_models(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(_params): Parameters<params::EmptyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // Any authenticated key may list models — just ensure auth ran.
        let _key_ctx = self.resolve_ctx(&ctx);

        // Try to list models from the first available service.
        let model_names = self.service_pool.model_names();
        if model_names.is_empty() {
            return Err(rmcp::ErrorData::internal_error("no models configured", None));
        }

        let first_model = model_names[0];
        let mut svc = self
            .service_pool
            .get_service(first_model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::ListModels).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::ListModels(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    // ── Image generation ─────────────────────────────────────────────────

    #[tool(description = "Generate images from a text prompt")]
    async fn generate_image(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::ImageParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // When a model is specified, enforce access; otherwise fall back to
        // the first configured model and enforce access against that.
        let effective_model = match params.model.as_deref() {
            Some(m) => m.to_owned(),
            None => {
                let names = self.service_pool.model_names();
                names
                    .first()
                    .ok_or_else(|| rmcp::ErrorData::internal_error("no models configured", None))?
                    .to_string()
            }
        };
        self.require_model_access(&ctx, &effective_model)?;

        let req = CreateImageRequest {
            prompt: params.prompt,
            model: params.model.clone(),
            n: params.n,
            size: params.size,
            quality: None,
            style: None,
            response_format: None,
            user: None,
        };

        let mut svc = self
            .service_pool
            .get_service(&effective_model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::ImageGenerate(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::ImageGenerate(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    // ── Audio ────────────────────────────────────────────────────────────

    #[tool(description = "Generate speech audio from text (text-to-speech)")]
    async fn speech(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::SpeechParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_model_access(&ctx, &params.model)?;

        let req = CreateSpeechRequest {
            model: params.model.clone(),
            input: params.input,
            voice: params.voice,
            response_format: None,
            speed: None,
        };

        let mut svc = self
            .service_pool
            .get_service(&params.model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::Speech(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::Speech(bytes) => {
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Audio generated ({} bytes). Base64: {}",
                    bytes.len(),
                    b64
                ))]))
            }
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    #[tool(description = "Transcribe audio to text (speech-to-text)")]
    async fn transcribe(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::TranscribeParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_model_access(&ctx, &params.model)?;

        let req = CreateTranscriptionRequest {
            model: params.model.clone(),
            file: params.file_base64,
            language: None,
            prompt: None,
            response_format: None,
            temperature: None,
        };

        let mut svc = self
            .service_pool
            .get_service(&params.model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::Transcribe(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::Transcribe(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    // ── Moderation ───────────────────────────────────────────────────────

    #[tool(description = "Check content against moderation policies")]
    async fn moderate(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::ModerateParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // When a model is specified, enforce access; otherwise fall back to
        // the first configured model and enforce access against that.
        let effective_model = match params.model.as_deref() {
            Some(m) => m.to_owned(),
            None => {
                let names = self.service_pool.model_names();
                names
                    .first()
                    .ok_or_else(|| rmcp::ErrorData::internal_error("no models configured", None))?
                    .to_string()
            }
        };
        self.require_model_access(&ctx, &effective_model)?;

        let req: ModerationRequest = serde_json::from_value(serde_json::json!({
            "input": params.input,
            "model": params.model,
        }))
        .map_err(|e| rmcp::ErrorData::invalid_params(e.to_string(), None))?;

        let mut svc = self
            .service_pool
            .get_service(&effective_model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::Moderate(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::Moderate(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    // ── Rerank ───────────────────────────────────────────────────────────

    #[tool(description = "Rerank documents by relevance to a query")]
    async fn rerank(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::RerankParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_model_access(&ctx, &params.model)?;

        let req = RerankRequest {
            model: params.model.clone(),
            query: params.query,
            documents: params.documents.into_iter().map(RerankDocument::Text).collect(),
            top_n: None,
            return_documents: None,
        };

        let mut svc = self
            .service_pool
            .get_service(&params.model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::Rerank(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::Rerank(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    // ── Search ───────────────────────────────────────────────────────────

    #[tool(description = "Perform a web or document search")]
    async fn search(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::SearchParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_model_access(&ctx, &params.model)?;

        let req = SearchRequest {
            model: params.model.clone(),
            query: params.query,
            max_results: None,
            search_domain_filter: None,
            country: None,
        };

        let mut svc = self
            .service_pool
            .get_service(&params.model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::Search(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::Search(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    // ── OCR ──────────────────────────────────────────────────────────────

    #[tool(description = "Extract text from an image or document via OCR")]
    async fn ocr(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::OcrParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_model_access(&ctx, &params.model)?;

        let document = if let Some(url) = params.image_url {
            OcrDocument::Url { url }
        } else if let Some(data) = params.image_base64 {
            let media_type = params.media_type.unwrap_or_else(|| "image/png".to_string());
            OcrDocument::Base64 { data, media_type }
        } else {
            return Err(rmcp::ErrorData::invalid_params(
                "either image_url or image_base64 must be provided",
                None,
            ));
        };

        let req = OcrRequest {
            model: params.model.clone(),
            document,
            pages: None,
            include_image_base64: None,
        };

        let mut svc = self
            .service_pool
            .get_service(&params.model)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let resp = svc.call(LlmRequest::Ocr(req)).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::Ocr(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    // ── File operations ──────────────────────────────────────────────────

    #[tool(description = "Upload a file to the LLM provider")]
    async fn create_file(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::CreateFileParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "create_file")?;

        let purpose: FilePurpose = serde_json::from_value(serde_json::Value::String(params.purpose)).map_err(|e| {
            rmcp::ErrorData::invalid_params(
                format!("invalid purpose (expected assistants, batch, fine-tune, or vision): {e}"),
                None,
            )
        })?;

        let req = CreateFileRequest {
            file: params.file_base64,
            purpose,
            filename: Some(params.filename),
        };

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.create_file(req).await.map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "List uploaded files")]
    async fn list_files(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::ListFilesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "list_files")?;

        let query = if params.purpose.is_some() || params.limit.is_some() {
            Some(FileListQuery {
                purpose: params.purpose,
                limit: params.limit,
                after: None,
            })
        } else {
            None
        };

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.list_files(query).await.map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "Retrieve metadata for an uploaded file")]
    async fn retrieve_file(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::FileIdParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "retrieve_file")?;

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.retrieve_file(&params.file_id).await.map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "Delete an uploaded file")]
    async fn delete_file(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::FileIdParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "delete_file")?;

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.delete_file(&params.file_id).await.map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "Retrieve the raw content of an uploaded file")]
    async fn file_content(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::FileIdParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "file_content")?;

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let bytes = client.file_content(&params.file_id).await.map_err(to_error_data)?;

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "File content ({} bytes). Base64: {b64}",
            bytes.len()
        ))]))
    }

    // ── Batch operations ─────────────────────────────────────────────────

    #[tool(description = "Create a new batch processing job")]
    async fn create_batch(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::CreateBatchParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "create_batch")?;

        let req = CreateBatchRequest {
            input_file_id: params.input_file_id,
            endpoint: params.endpoint,
            completion_window: params.completion_window,
            metadata: None,
        };

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.create_batch(req).await.map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "List batch processing jobs")]
    async fn list_batches(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::ListBatchesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "list_batches")?;

        let query = if params.limit.is_some() || params.after.is_some() {
            Some(BatchListQuery {
                limit: params.limit,
                after: params.after,
            })
        } else {
            None
        };

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.list_batches(query).await.map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "Retrieve a batch processing job by ID")]
    async fn retrieve_batch(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::BatchIdParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "retrieve_batch")?;

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.retrieve_batch(&params.batch_id).await.map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "Cancel an in-progress batch processing job")]
    async fn cancel_batch(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::BatchIdParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "cancel_batch")?;

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.cancel_batch(&params.batch_id).await.map_err(to_error_data)?;
        json_success(&result)
    }

    // ── Response operations ──────────────────────────────────────────────

    #[tool(description = "Create a new response (Responses API)")]
    async fn create_response(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::CreateResponseParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_model_access(&ctx, &params.model)?;

        let req = CreateResponseRequest {
            model: params.model,
            input: params.input,
            instructions: None,
            tools: None,
            temperature: None,
            max_output_tokens: None,
            metadata: None,
        };

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client.create_response(req).await.map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "Retrieve a response by ID (Responses API)")]
    async fn retrieve_response(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::ResponseIdParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "retrieve_response")?;

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client
            .retrieve_response(&params.response_id)
            .await
            .map_err(to_error_data)?;
        json_success(&result)
    }

    #[tool(description = "Cancel an in-progress response (Responses API)")]
    async fn cancel_response(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<params::ResponseIdParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.require_master(&ctx, "cancel_response")?;

        let client = self
            .service_pool
            .first_client()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let result = client
            .cancel_response(&params.response_id)
            .await
            .map_err(to_error_data)?;
        json_success(&result)
    }
}

// ─── ServerHandler implementation ────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for LiterLlmMcp {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::default();
        capabilities.tools = Some(ToolsCapability::default());

        InitializeResult::new(capabilities)
            .with_server_info(Implementation::new("liter-llm", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "LiterLLM proxy — universal LLM API gateway with 142+ providers. \
                 Use the chat tool to send completion requests, embed for embeddings, \
                 and the file/batch/response tools for management operations.",
            )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::auth::KeyContext;
    use crate::config::VirtualKeyConfig;

    use super::LiterLlmMcp;

    fn restricted_ctx(key_id: &str, models: Vec<String>) -> KeyContext {
        let cfg = VirtualKeyConfig {
            key: key_id.to_string(),
            description: None,
            models,
            rpm: None,
            tpm: None,
            budget_limit: None,
        };
        KeyContext::from_config(&cfg)
    }

    // ── check_model_access: restricted key blocked for unlisted model ─────

    #[test]
    fn restricted_key_rejected_for_unlisted_model_in_chat() {
        let key_ctx = restricted_ctx("vk-test", vec!["gpt-4o".to_string()]);
        let result = LiterLlmMcp::check_model_access(&key_ctx, "claude-sonnet");
        assert!(result.is_err(), "should reject unlisted model");
        let err = result.unwrap_err();
        let msg = &err.message;
        assert!(msg.contains("vk-test"), "error must name the key: {msg}");
        assert!(msg.contains("claude-sonnet"), "error must name the model: {msg}");
    }

    // ── check_model_access: master key allows any model ───────────────────

    #[test]
    fn master_ctx_allows_chat_for_any_model() {
        let key_ctx = KeyContext::master();
        assert!(LiterLlmMcp::check_model_access(&key_ctx, "claude-sonnet").is_ok());
        assert!(LiterLlmMcp::check_model_access(&key_ctx, "some-random-model").is_ok());
    }

    // ── check_model_access: allowed model passes ──────────────────────────

    #[test]
    fn restricted_key_allowed_for_listed_model() {
        let key_ctx = restricted_ctx("vk-test", vec!["gpt-4o".to_string(), "claude-opus".to_string()]);
        assert!(LiterLlmMcp::check_model_access(&key_ctx, "gpt-4o").is_ok());
        assert!(LiterLlmMcp::check_model_access(&key_ctx, "claude-opus").is_ok());
        assert!(LiterLlmMcp::check_model_access(&key_ctx, "other-model").is_err());
    }

    // ── check_master_access: non-master rejected for master-only tools ────

    #[test]
    fn non_master_ctx_rejected_for_create_file() {
        let key_ctx = restricted_ctx("vk-limited", vec!["gpt-4o".to_string()]);
        let result = LiterLlmMcp::check_master_access(&key_ctx, "create_file");
        assert!(result.is_err(), "restricted key must be rejected for create_file");
        let msg = &result.unwrap_err().message;
        assert!(msg.contains("create_file"), "error must name the tool: {msg}");
        assert!(msg.contains("vk-limited"), "error must name the key: {msg}");
        assert!(msg.contains("master-key"), "error must mention master-key: {msg}");
    }

    // ── check_master_access: master key allowed for master-only tools ──────

    #[test]
    fn master_ctx_allowed_for_master_only_tool() {
        let key_ctx = KeyContext::master();
        assert!(LiterLlmMcp::check_master_access(&key_ctx, "list_files").is_ok());
        assert!(LiterLlmMcp::check_master_access(&key_ctx, "create_batch").is_ok());
    }

    // ── check_model_access: unrestricted virtual key (empty models) ───────

    #[test]
    fn unrestricted_key_allows_any_model() {
        // models: [] → no restriction (same as master for model access).
        let key_ctx = restricted_ctx("vk-all-models", vec![]);
        assert!(LiterLlmMcp::check_model_access(&key_ctx, "gpt-4o").is_ok());
        assert!(LiterLlmMcp::check_model_access(&key_ctx, "claude-opus").is_ok());
    }

    // ── check_master_access: unrestricted virtual key still blocked ───────

    #[test]
    fn unrestricted_key_still_blocked_for_master_only_tool() {
        // An unrestricted virtual key has wide model access but is NOT master.
        let key_ctx = restricted_ctx("vk-all-models", vec![]);
        let result = LiterLlmMcp::check_master_access(&key_ctx, "list_files");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("list_files"));
    }
}
