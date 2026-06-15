pub mod auth;
pub mod config;
pub mod error;
pub mod file_store;
pub mod mcp;
pub mod openapi;
pub mod provider;
pub mod routes;
pub mod secrets;
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

use arc_swap::ArcSwap;
use config::ProxyConfig;
use state::AppState;

/// Hot-reload mode selected by the `--watch` CLI flag.
#[derive(Debug, Clone)]
pub enum WatchMode {
    /// No live reload — use a `StaticFileConfigProvider`.
    Off,
    /// Watch a local TOML file for changes via `notify`.
    File { path: std::path::PathBuf },
    /// Watch an etcd key prefix for changes.
    Etcd { endpoints: Vec<String>, key: String },
}

/// Builder for the liter-llm proxy server.
///
/// Constructs the shared [`AppState`] from a [`ProxyConfig`], builds the axum
/// router, and serves on the configured address.
pub struct ProxyServer {
    config: ProxyConfig,
    watch_mode: WatchMode,
}

impl ProxyServer {
    /// Create a new proxy server with the given configuration and no live
    /// reload (static config only).
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            watch_mode: WatchMode::Off,
        }
    }

    /// Enable hot-reload with the given [`WatchMode`].
    pub fn with_watch_mode(mut self, mode: WatchMode) -> Self {
        self.watch_mode = mode;
        self
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

        let arc_config = Arc::new(ArcSwap::from(Arc::new(self.config)));

        // Spawn the hot-reload background watcher when requested.
        let cancel = shutdown_handle
            .as_ref()
            .map(|h| h.cancellation_token())
            .unwrap_or_default();

        match self.watch_mode {
            WatchMode::Off => {
                // No watcher — arc_config is static.
            }
            WatchMode::File { path } => {
                let provider = Arc::new(config::FileWatchConfigProvider::new(path));
                config::watcher::spawn_watcher(provider, Arc::clone(&arc_config), cancel.clone()).await;
            }
            WatchMode::Etcd { endpoints, key } => {
                let provider = config::EtcdConfigProvider::connect(endpoints, key)
                    .await
                    .map_err(|e| format!("etcd connect failed: {e}"))?;
                config::watcher::spawn_watcher(Arc::new(provider), Arc::clone(&arc_config), cancel.clone()).await;
            }
        }

        let addr: SocketAddr = {
            let cfg = arc_config.load();
            format!("{}:{}", cfg.server.host, cfg.server.port)
                .parse()
                .map_err(|e| format!("invalid listen address: {e}"))?
        };

        // Build the default secret registry with the env backend as the
        // fallback. AWS and Vault backends are added when the corresponding
        // features are enabled and configured.
        let secret_registry = Arc::new(
            secrets::SecretManagerRegistry::builder()
                .register(
                    "env",
                    Arc::new(secrets::EnvVarSecretManager::new()) as Arc<dyn secrets::SecretManager>,
                )
                .default_backend(Arc::new(secrets::EnvVarSecretManager::new()) as Arc<dyn secrets::SecretManager>)
                .build(),
        );

        let key_store = Arc::new(key_store);
        let key_resolver: Arc<dyn liter_llm::tenant::KeyResolver> =
            key_store.clone() as Arc<dyn liter_llm::tenant::KeyResolver>;
        let state = AppState {
            key_store,
            key_resolver,
            service_pool: Arc::new(service_pool),
            file_store: Arc::new(file_store),
            config: arc_config,
            secret_registry,
            shutdown: shutdown_handle.clone(),
        };

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
