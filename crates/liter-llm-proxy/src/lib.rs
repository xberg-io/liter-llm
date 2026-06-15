pub mod auth;
pub mod config;
pub mod error;
pub mod file_store;
pub mod mcp;
pub mod openapi;
pub mod routes;
pub mod service_pool;
pub mod shutdown;
pub mod state;
pub mod streaming;

#[cfg(test)]
#[ctor::ctor(unsafe)]
fn init_crypto_for_unit_tests() {
    liter_llm::ensure_crypto_provider();
}

use std::net::SocketAddr;
use std::sync::Arc;

use config::ProxyConfig;
use state::AppState;

/// Builder for the liter-llm proxy server.
///
/// Constructs the shared [`AppState`] from a [`ProxyConfig`], builds the axum
/// router, and serves on the configured address.
pub struct ProxyServer {
    config: ProxyConfig,
}

impl ProxyServer {
    /// Create a new proxy server with the given configuration.
    pub fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    /// Build the application state, assemble the router, and start serving.
    ///
    /// Accepts an optional [`shutdown::ShutdownHandle`] to integrate with the
    /// [`shutdown::ShutdownCoordinator`].  When provided, the server stops
    /// accepting new connections as soon as the handle's cancellation token is
    /// cancelled (i.e. when the coordinator enters `Draining`).  When
    /// `None` the server falls back to listening for Ctrl-C only.
    pub async fn serve_with_shutdown(self, shutdown_handle: Option<shutdown::ShutdownHandle>) -> Result<(), String> {
        liter_llm::ensure_crypto_provider();
        let service_pool = service_pool::ServicePool::from_config(&self.config)?;
        let key_store = auth::KeyStore::from_config(self.config.general.master_key.clone(), &self.config.keys);
        let file_store = file_store::FileStore::from_config(self.config.files.as_ref().unwrap_or(&Default::default()))?;

        let state = AppState {
            key_store: Arc::new(key_store),
            service_pool: Arc::new(service_pool),
            file_store: Arc::new(file_store),
            config: Arc::new(self.config),
            shutdown: shutdown_handle.clone(),
        };

        let addr: SocketAddr = format!("{}:{}", state.config.server.host, state.config.server.port)
            .parse()
            .map_err(|e| format!("invalid listen address: {e}"))?;

        let router = routes::build_router(state);

        tracing::info!("liter-llm proxy listening on {addr}");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("failed to bind {addr}: {e}"))?;

        match shutdown_handle {
            Some(handle) => {
                // Use the coordinator's cancellation token so axum drains on
                // the first signal, not just on process exit.
                let token = handle.cancellation_token();
                axum::serve(listener, router)
                    .with_graceful_shutdown(async move { token.cancelled().await })
                    .await
                    .map_err(|e| format!("server error: {e}"))?;
            }
            None => {
                // Fallback: ctrl-c only (legacy / test usage).
                let shutdown = async {
                    let _ = tokio::signal::ctrl_c().await;
                    tracing::info!("shutdown signal received");
                };
                axum::serve(listener, router)
                    .with_graceful_shutdown(shutdown)
                    .await
                    .map_err(|e| format!("server error: {e}"))?;
            }
        }

        Ok(())
    }

    /// Convenience wrapper that uses the coordinator-less fallback (Ctrl-C).
    ///
    /// Kept for backward compatibility with any code that was calling
    /// `ProxyServer::new(config).serve().await` directly.
    pub async fn serve(self) -> Result<(), String> {
        self.serve_with_shutdown(None).await
    }
}
