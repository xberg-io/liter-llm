//! MCP tool implementations (`#[tool_router]`).
//!
//! Split out of `mcp/mod.rs` to keep each file under the repository
//! line-count ceiling. As a child module of `mcp`, this file retains access
//! to the private helpers (`resolve_ctx`, `require_model_access`,
//! `require_master`, `json_success`) defined on [`LiterLlmMcp`] in `mod.rs`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::{RoleServer, tool, tool_router};
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

use super::errors::to_error_data;
use super::{LiterLlmMcp, json_success, params};

#[tool_router(vis = "pub(super)")]
impl LiterLlmMcp {
    // ── Chat & Embeddings ────────────────────────────────────────────────

    #[tool(
        description = "Send a chat completion request to an LLM",
        annotations(title = "Chat Completion", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Generate text embeddings for the given input",
        annotations(title = "Generate Embeddings", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "List available models from configured providers",
        annotations(title = "List Models", read_only_hint = true, open_world_hint = true)
    )]
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

        let resp = svc.call(LlmRequest::ListModels()).await.map_err(to_error_data)?;

        match resp {
            LlmResponse::ListModels(r) => json_success(&r),
            other => Err(rmcp::ErrorData::internal_error(
                format!("unexpected response variant: {other:?}"),
                None,
            )),
        }
    }

    // ── Image generation ─────────────────────────────────────────────────

    #[tool(
        description = "Generate images from a text prompt",
        annotations(title = "Generate Images", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Generate speech audio from text (text-to-speech)",
        annotations(title = "Text to Speech", read_only_hint = true, open_world_hint = true)
    )]
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
                Ok(CallToolResult::success(vec![ContentBlock::text(format!(
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

    #[tool(
        description = "Transcribe audio to text (speech-to-text)",
        annotations(title = "Transcribe Audio", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Check content against moderation policies",
        annotations(title = "Moderate Content", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Rerank documents by relevance to a query",
        annotations(title = "Rerank Documents", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Perform a web or document search",
        annotations(title = "Search", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Extract text from an image or document via OCR",
        annotations(title = "OCR Extract", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Upload a file to the LLM provider",
        annotations(
            title = "Upload File",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
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

    #[tool(
        description = "List uploaded files",
        annotations(title = "List Files", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Retrieve metadata for an uploaded file",
        annotations(title = "Retrieve File", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Delete an uploaded file",
        annotations(
            title = "Delete File",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
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

    #[tool(
        description = "Retrieve the raw content of an uploaded file",
        annotations(title = "Get File Content", read_only_hint = true, open_world_hint = true)
    )]
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
        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
            "File content ({} bytes). Base64: {b64}",
            bytes.len()
        ))]))
    }

    // ── Batch operations ─────────────────────────────────────────────────

    #[tool(
        description = "Create a new batch processing job",
        annotations(
            title = "Create Batch",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
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

    #[tool(
        description = "List batch processing jobs",
        annotations(title = "List Batches", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Retrieve a batch processing job by ID",
        annotations(title = "Retrieve Batch", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Cancel an in-progress batch processing job",
        annotations(
            title = "Cancel Batch",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
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

    #[tool(
        description = "Create a new response (Responses API)",
        annotations(
            title = "Create Response",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
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

    #[tool(
        description = "Retrieve a response by ID (Responses API)",
        annotations(title = "Retrieve Response", read_only_hint = true, open_world_hint = true)
    )]
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

    #[tool(
        description = "Cancel an in-progress response (Responses API)",
        annotations(
            title = "Cancel Response",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
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
