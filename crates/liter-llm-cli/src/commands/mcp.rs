use std::path::PathBuf;

use clap::Args;
use liter_llm_proxy::config::ProxyConfig;

#[derive(Args)]
pub struct McpArgs {
    /// Path to config file.
    #[arg(long, short)]
    pub config: Option<PathBuf>,
    /// Transport mode: stdio or http.
    #[arg(long, default_value = "stdio")]
    pub transport: String,
    /// Host for HTTP transport.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Port for HTTP transport.
    #[arg(long, default_value_t = 3001)]
    pub port: u16,
}

pub async fn run(args: McpArgs) -> Result<(), String> {
    use std::sync::Arc;

    use liter_llm_proxy::auth::{KeyContext, KeyStore};
    use liter_llm_proxy::file_store::FileStore;
    use liter_llm_proxy::mcp::{LiterLlmMcp, McpTransportKind};
    use liter_llm_proxy::service_pool::ServicePool;
    use liter_llm_proxy::state::AppState;
    use rmcp::ServiceExt;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = if let Some(path) = &args.config {
        ProxyConfig::from_toml_file(path)?
    } else {
        ProxyConfig::discover()?.unwrap_or_default()
    };

    let service_pool = Arc::new(ServicePool::from_config(&config)?);
    let key_store = Arc::new(KeyStore::from_config(config.general.master_key.clone(), &config.keys));
    let file_store = Arc::new(FileStore::from_config(
        config.files.as_ref().unwrap_or(&Default::default()),
    )?);

    match args.transport.as_str() {
        "stdio" => {
            // Resolve the default KeyContext for the stdio transport.
            //
            // stdio has no per-request auth headers, so auth is established
            // once at startup via `[mcp]` config.  Failing to configure it
            // is a security misconfiguration; refuse to start.
            let default_ctx = match (&config.mcp.stdio_key_id, config.mcp.stdio_trust_local) {
                (Some(key_id), _) => {
                    let key_cfg = key_store.get(key_id).ok_or_else(|| {
                        format!(
                            "mcp.stdio_key_id '{key_id}' not found in the virtual key store; \
                             add it under [[keys]] in your config"
                        )
                    })?;
                    KeyContext::from_config(&key_cfg)
                }
                (None, true) => KeyContext::master(),
                (None, false) => {
                    return Err(
                        "stdio MCP transport requires authentication configuration; set either \
                         `mcp.stdio_key_id` (to bind a specific virtual key) or \
                         `mcp.stdio_trust_local = true` (for fully trusted local environments) \
                         in your liter-llm-proxy.toml"
                            .into(),
                    );
                }
            };

            let mcp = LiterLlmMcp::new(
                service_pool.clone(),
                file_store.clone(),
                default_ctx,
                McpTransportKind::Stdio,
            );

            tracing::info!("starting MCP server with stdio transport");
            let service = mcp
                .serve(rmcp::transport::stdio())
                .await
                .map_err(|e| format!("MCP stdio serve failed: {e}"))?;
            service.waiting().await.map_err(|e| format!("MCP server error: {e}"))?;
        }
        "http" => {
            use liter_llm_proxy::auth::validate_api_key;
            use rmcp::transport::streamable_http_server::StreamableHttpService;
            use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

            let addr: std::net::SocketAddr = format!("{}:{}", args.host, args.port)
                .parse()
                .map_err(|e| format!("invalid MCP listen address: {e}"))?;

            let app_state = AppState {
                key_store: key_store.clone(),
                service_pool: service_pool.clone(),
                file_store: file_store.clone(),
                config: Arc::new(config.clone()),
            };

            // For HTTP transport the actual KeyContext is resolved from the
            // per-request axum extensions injected by validate_api_key.
            // `KeyContext::master()` is used as a safe fallback; it should
            // never be reached in a correctly wired deployment (the middleware
            // will 401 before the MCP handler runs).
            let http_service = StreamableHttpService::new(
                move || {
                    let sp = service_pool.clone();
                    let fs = file_store.clone();
                    Ok(LiterLlmMcp::new(
                        sp,
                        fs,
                        KeyContext::master(),
                        McpTransportKind::Http,
                    ))
                },
                LocalSessionManager::default().into(),
                Default::default(),
            );

            // Wire the auth middleware so every request to /mcp is validated.
            let router = axum::Router::new()
                .nest_service("/mcp", http_service)
                .layer(axum::middleware::from_fn_with_state(
                    app_state.clone(),
                    validate_api_key,
                ))
                .with_state(app_state);

            tracing::info!("starting MCP server with HTTP transport on {addr}");
            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .map_err(|e| format!("failed to bind MCP HTTP {addr}: {e}"))?;
            axum::serve(listener, router)
                .await
                .map_err(|e| format!("MCP HTTP server error: {e}"))?;
        }
        other => return Err(format!("unknown transport '{other}', use 'stdio' or 'http'")),
    }

    Ok(())
}
