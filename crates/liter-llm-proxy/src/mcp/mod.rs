pub mod errors;
pub mod params;
mod tools;

use std::sync::Arc;

use rmcp::handler::server::router::prompt::PromptRouter;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler, prompt, prompt_handler, prompt_router, tool_handler};

use liter_llm::cost::model_pricing;
use liter_llm::provider::all_providers;

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
    #[allow(dead_code)]
    prompt_router: PromptRouter<Self>,
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
            prompt_router: Self::prompt_router(),
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
    Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
}

// ─── Prompt templates ─────────────────────────────────────────────────────────

#[prompt_router]
impl LiterLlmMcp {
    /// Summarise a block of text concisely.
    #[prompt(name = "summarize", description = "Summarise a block of text concisely")]
    async fn summarize_prompt(
        &self,
        Parameters(args): Parameters<params::SummarizeArgs>,
    ) -> Result<Vec<PromptMessage>, rmcp::ErrorData> {
        let text = format!(
            "Summarise the following text concisely while preserving its key points:\n\n{}",
            args.text
        );
        Ok(vec![PromptMessage::new_text(Role::User, text)])
    }

    /// Translate text into a target language.
    #[prompt(name = "translate", description = "Translate text into a target language")]
    async fn translate_prompt(
        &self,
        Parameters(args): Parameters<params::TranslateArgs>,
    ) -> Result<Vec<PromptMessage>, rmcp::ErrorData> {
        let text = format!(
            "Translate the following text into {}. Output only the translation:\n\n{}",
            args.target_language, args.text
        );
        Ok(vec![PromptMessage::new_text(Role::User, text)])
    }

    /// Extract structured data from text following natural-language instructions.
    #[prompt(
        name = "extract",
        description = "Extract structured data from text following instructions"
    )]
    async fn extract_prompt(
        &self,
        Parameters(args): Parameters<params::ExtractArgs>,
    ) -> Result<Vec<PromptMessage>, rmcp::ErrorData> {
        let text = format!(
            "Extract information from the text below following these instructions: {}\n\nText:\n{}",
            args.instructions, args.text
        );
        Ok(vec![PromptMessage::new_text(Role::User, text)])
    }
}

// ─── Resource & completion catalog ──────────────────────────────────────────────

/// Resource URI for the configured-model list.
const RESOURCE_MODELS: &str = "liter-llm://models";
/// Resource URI for the built-in provider registry.
const RESOURCE_PROVIDERS: &str = "liter-llm://providers";
/// Resource-template prefix for per-model pricing.
const RESOURCE_PRICING_PREFIX: &str = "liter-llm://pricing/";
/// Resource-template prefix for per-provider detail.
const RESOURCE_PROVIDER_PREFIX: &str = "liter-llm://provider/";

/// Resolve a resource URI to its JSON body.
///
/// Pure (takes the configured `model_names` explicitly) so unit tests can
/// exercise it without constructing a live server, mirroring the
/// [`LiterLlmMcp::check_model_access`] helper pattern.
fn resource_body(uri: &str, model_names: &[&str]) -> Result<String, rmcp::ErrorData> {
    let to_json = |v: &serde_json::Value| {
        serde_json::to_string_pretty(v).map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    };
    if uri == RESOURCE_MODELS {
        return serde_json::to_string_pretty(model_names)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None));
    }
    if uri == RESOURCE_PROVIDERS {
        let providers = all_providers().map_err(to_error_data)?;
        return serde_json::to_string_pretty(providers)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None));
    }
    if let Some(model) = uri.strip_prefix(RESOURCE_PRICING_PREFIX) {
        let pricing = model_pricing(model)
            .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("no pricing for model '{model}'"), None))?;
        return to_json(&serde_json::json!({
            "model": model,
            "input_cost_per_token": pricing.input_cost_per_token,
            "output_cost_per_token": pricing.output_cost_per_token,
            "cache_read_input_token_cost": pricing.cache_read_input_token_cost,
            "cache_creation_input_token_cost": pricing.cache_creation_input_token_cost,
        }));
    }
    if let Some(name) = uri.strip_prefix(RESOURCE_PROVIDER_PREFIX) {
        let providers = all_providers().map_err(to_error_data)?;
        let provider = providers
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("unknown provider '{name}'"), None))?;
        return serde_json::to_string_pretty(provider)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None));
    }
    Err(rmcp::ErrorData::invalid_params(
        format!("unknown resource uri: {uri}"),
        None,
    ))
}

/// Suggest completions for a prompt or resource-template argument.
///
/// Keys off the argument NAME (not the reference) so it serves both prompt
/// arguments and resource-template variables. Returns up to 100 matches whose
/// lowercased value starts with `partial`. Pure for testability.
fn complete_values(arg_name: &str, partial: &str, model_names: &[&str]) -> Vec<String> {
    const MAX: usize = 100;
    let needle = partial.to_ascii_lowercase();
    match arg_name {
        "model" => model_names
            .iter()
            .filter(|m| m.to_ascii_lowercase().starts_with(&needle))
            .take(MAX)
            .map(|m| (*m).to_string())
            .collect(),
        "name" | "provider" => match all_providers() {
            Ok(providers) => providers
                .iter()
                .map(|p| p.name.clone())
                .filter(|n| n.to_ascii_lowercase().starts_with(&needle))
                .take(MAX)
                .collect(),
            Err(_) => Vec::new(),
        },
        _ => Vec::new(),
    }
}

// ─── ServerHandler implementation ────────────────────────────────────────────

#[tool_handler]
#[prompt_handler]
impl ServerHandler for LiterLlmMcp {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_prompts()
            .enable_resources()
            .enable_completions()
            .build();

        InitializeResult::new(capabilities)
            .with_server_info(Implementation::new("liter-llm", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "LiterLLM proxy — universal LLM API gateway with 143 providers. \
                 Use the chat tool to send completion requests, embed for embeddings, \
                 and the file/batch/response tools for management operations. \
                 Reusable prompt templates (summarize, translate, extract) and \
                 catalog resources (liter-llm://models, liter-llm://providers, \
                 liter-llm://pricing/{model}, liter-llm://provider/{name}) are also exposed.",
            )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let resources = vec![
            Resource::new(RESOURCE_MODELS, "Configured Models")
                .with_description("Model names configured in this proxy")
                .with_mime_type("application/json"),
            Resource::new(RESOURCE_PROVIDERS, "Provider Registry")
                .with_description("All built-in LLM providers")
                .with_mime_type("application/json"),
        ];
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        let models = self.service_pool.model_names();
        let json = resource_body(&request.uri, &models)?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            json,
            &request.uri,
        )]))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, rmcp::ErrorData> {
        let resource_templates = vec![
            ResourceTemplate::new("liter-llm://pricing/{model}", "Model Pricing")
                .with_description("Per-token pricing for a model"),
            ResourceTemplate::new("liter-llm://provider/{name}", "Provider Detail")
                .with_description("Configuration for a single provider"),
        ];
        Ok(ListResourceTemplatesResult {
            resource_templates,
            next_cursor: None,
            meta: None,
        })
    }

    async fn complete(
        &self,
        request: CompleteRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CompleteResult, rmcp::ErrorData> {
        let models = self.service_pool.model_names();
        let values = complete_values(&request.argument.name, &request.argument.value, &models);
        let completion =
            CompletionInfo::with_all_values(values).map_err(|e| rmcp::ErrorData::internal_error(e, None))?;
        Ok(CompleteResult::new(completion))
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
            provider_credentials: vec![],
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

    // ── Tool annotations: every tool advertises MCP hints ─────────────────

    /// Every exposed tool must carry `ToolAnnotations` with a human title and
    /// the open-world hint set (all tools reach external providers). Clients
    /// rely on these hints to decide auto-approval and presentation, so a tool
    /// shipped without them is a regression.
    #[test]
    fn every_tool_has_annotations_with_title_and_open_world() {
        let tools = LiterLlmMcp::tool_router().list_all();
        assert!(!tools.is_empty(), "tool router exposes no tools");
        for tool in &tools {
            let ann = tool
                .annotations
                .as_ref()
                .unwrap_or_else(|| panic!("tool '{}' is missing annotations", tool.name));
            assert!(
                ann.title.as_deref().is_some_and(|t| !t.is_empty()),
                "tool '{}' is missing an annotation title",
                tool.name
            );
            assert_eq!(
                ann.open_world_hint,
                Some(true),
                "tool '{}' must set open_world_hint (it calls external providers)",
                tool.name
            );
        }
    }

    /// Spot-check the read-only / destructive classification so the hints stay
    /// semantically correct: queries are read-only, `create_*` mutate without
    /// being destructive, and `delete_*`/`cancel_*` are destructive.
    #[test]
    fn tool_annotation_classification_is_correct() {
        let tools = LiterLlmMcp::tool_router().list_all();
        let by_name = |name: &str| {
            tools
                .iter()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("tool '{name}' not found"))
                .annotations
                .clone()
                .unwrap_or_else(|| panic!("tool '{name}' missing annotations"))
        };

        // Pure query tools are read-only.
        for name in ["chat", "embed", "list_models", "list_files", "retrieve_batch"] {
            assert_eq!(by_name(name).read_only_hint, Some(true), "{name} should be read-only");
        }

        // create_* mutate state but are not destructive.
        for name in ["create_file", "create_batch", "create_response"] {
            let ann = by_name(name);
            assert_eq!(ann.read_only_hint, Some(false), "{name} should not be read-only");
            assert_eq!(ann.destructive_hint, Some(false), "{name} should not be destructive");
        }

        // delete_* / cancel_* are destructive and idempotent.
        for name in ["delete_file", "cancel_batch", "cancel_response"] {
            let ann = by_name(name);
            assert_eq!(ann.read_only_hint, Some(false), "{name} should not be read-only");
            assert_eq!(ann.destructive_hint, Some(true), "{name} should be destructive");
            assert_eq!(ann.idempotent_hint, Some(true), "{name} should be idempotent");
        }
    }

    // ── Prompts: the three templates are registered ───────────────────────

    #[test]
    fn prompt_router_lists_all_templates() {
        let prompts = super::LiterLlmMcp::prompt_router().list_all();
        let names: Vec<&str> = prompts.iter().map(|p| p.name.as_ref()).collect();
        assert!(names.contains(&"summarize"), "missing summarize prompt: {names:?}");
        assert!(names.contains(&"translate"), "missing translate prompt: {names:?}");
        assert!(names.contains(&"extract"), "missing extract prompt: {names:?}");
        assert_eq!(names.len(), 3, "unexpected prompt set: {names:?}");
    }

    // ── Resources: pure body resolver ─────────────────────────────────────

    #[test]
    fn resource_body_models_lists_configured_names() {
        let models = ["openai/gpt-4o", "anthropic/claude-sonnet"];
        let json = super::resource_body(super::RESOURCE_MODELS, &models).expect("models resource");
        assert!(json.contains("openai/gpt-4o"), "models JSON missing entry: {json}");
        assert!(
            json.contains("anthropic/claude-sonnet"),
            "models JSON missing entry: {json}"
        );
    }

    #[test]
    fn resource_body_providers_is_nonempty_json_array() {
        let json = super::resource_body(super::RESOURCE_PROVIDERS, &[]).expect("providers resource");
        assert!(
            json.trim_start().starts_with('['),
            "providers should be a JSON array: {json}"
        );
        // The built-in registry ships many providers; openai is always present.
        assert!(json.contains("openai"), "providers JSON should include openai");
    }

    #[test]
    fn resource_body_unknown_uri_is_invalid_params() {
        let err = super::resource_body("liter-llm://nope", &[]).unwrap_err();
        assert!(
            err.message.contains("unknown resource uri"),
            "unexpected error: {}",
            err.message
        );
    }

    // ── Completion: pure value resolver ───────────────────────────────────

    #[test]
    fn complete_values_filters_models_by_prefix() {
        let models = ["openai/gpt-4o", "openai/gpt-4o-mini", "anthropic/claude-sonnet"];
        let out = super::complete_values("model", "openai/", &models);
        assert_eq!(out.len(), 2, "should match the two openai models: {out:?}");
        assert!(out.iter().all(|m| m.starts_with("openai/")));

        // Case-insensitive prefix.
        let out = super::complete_values("model", "ANTHRO", &models);
        assert_eq!(out, vec!["anthropic/claude-sonnet".to_string()]);

        // Unknown argument names yield nothing.
        assert!(super::complete_values("nonsense", "x", &models).is_empty());
    }

    #[test]
    fn complete_values_filters_provider_names() {
        // Provider names come from the global registry, independent of models.
        let out = super::complete_values("name", "openai", &[]);
        assert!(out.iter().any(|n| n == "openai"), "should suggest openai: {out:?}");
        assert!(out.iter().all(|n| n.starts_with("openai")));
    }
}
