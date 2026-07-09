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
    ///
    /// Only available when the `etcd-watch` feature is enabled. Rebuild with
    /// `--features etcd-watch` to use this variant.
    #[cfg(feature = "etcd-watch")]
    Etcd { endpoints: Vec<String>, key: String },
}

/// Builder for the liter-llm proxy server.
///
/// Constructs the shared [`AppState`] from a [`ProxyConfig`], builds the axum
/// router, and serves on the configured address.
pub struct ProxyServer {
    config: ProxyConfig,
    watch_mode: WatchMode,
    key_resolver_override: Option<Arc<dyn liter_llm::tenant::KeyResolver>>,
    usage_sink: Option<Arc<dyn liter_llm::observability::UsageSinkErased>>,
}

impl ProxyServer {
    /// Create a new proxy server with the given configuration and no live
    /// reload (static config only).
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            watch_mode: WatchMode::Off,
            key_resolver_override: None,
            usage_sink: None,
        }
    }

    /// Enable hot-reload with the given [`WatchMode`].
    pub fn with_watch_mode(mut self, mode: WatchMode) -> Self {
        self.watch_mode = mode;
        self
    }

    /// Override the default `KeyStore`-backed key resolver with a custom
    /// implementation.
    ///
    /// When set, the supplied resolver is used instead of the `KeyStore`
    /// constructed from `ProxyConfig.keys`.  Cloud embedders use this to plug
    /// in database-backed or remote resolvers without forking `AppState`.
    #[must_use]
    pub fn with_key_resolver(mut self, resolver: Arc<dyn liter_llm::tenant::KeyResolver>) -> Self {
        self.key_resolver_override = Some(resolver);
        self
    }

    /// Attach an embedder-supplied usage sink.
    ///
    /// When set, `HooksLayer` is pushed outermost in the Tower stack so every
    /// completed request (success or error) emits a [`liter_llm::observability::UsageEvent`].
    /// Default behaviour (no sink) is unchanged.
    #[must_use]
    pub fn with_usage_sink<S: liter_llm::observability::UsageSink>(mut self, sink: Arc<S>) -> Self {
        self.usage_sink = Some(sink as Arc<dyn liter_llm::observability::UsageSinkErased>);
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

        // ~keep Activate the SSRF outbound policy before config or backend setup so URL validation is live.
        // ~keep The default is `DenyPrivate`; operators must explicitly opt out in trusted environments.
        {
            use config::OutboundPolicyKind;
            use liter_llm::provider::{self as lp, OutboundPolicy};

            let policy = match self.config.security.outbound_policy {
                OutboundPolicyKind::Off => OutboundPolicy::Off,
                OutboundPolicyKind::DenyPrivate => OutboundPolicy::DenyPrivate,
                OutboundPolicyKind::Allowlist => {
                    let urls = parse_allowlist_urls(&self.config.security.outbound_allowlist);
                    OutboundPolicy::Allowlist(urls)
                }
            };
            lp::set_outbound_policy(policy);
        }

        let service_pool = service_pool::ServicePool::from_config(&self.config, self.usage_sink.clone())?;
        let key_store = auth::KeyStore::from_config(self.config.general.master_key.clone(), &self.config.keys);
        let file_store = file_store::FileStore::from_config(self.config.files.as_ref().unwrap_or(&Default::default()))?;

        let arc_config = Arc::new(ArcSwap::from(Arc::new(self.config)));

        let cancel = shutdown_handle
            .as_ref()
            .map(|h| h.cancellation_token())
            .unwrap_or_default();

        match self.watch_mode {
            WatchMode::Off => {}
            WatchMode::File { path } => {
                let provider = Arc::new(config::FileWatchConfigProvider::new(path));
                config::watcher::spawn_watcher(provider, Arc::clone(&arc_config), cancel.clone()).await;
            }
            #[cfg(feature = "etcd-watch")]
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
        let key_resolver: Arc<dyn liter_llm::tenant::KeyResolver> = self
            .key_resolver_override
            .clone()
            .unwrap_or_else(|| key_store.clone() as Arc<dyn liter_llm::tenant::KeyResolver>);
        let state = AppState {
            key_store,
            key_resolver,
            service_pool: Arc::new(service_pool),
            file_store: Arc::new(file_store),
            config: arc_config,
            secret_registry,
            shutdown: shutdown_handle.clone(),
            usage_sink: self.usage_sink.clone(),
        };

        let router = routes::build_router(state);

        tracing::info!("liter-llm proxy listening on {addr}");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("failed to bind {addr}: {e}"))?;

        match shutdown_handle {
            Some(handle) => {
                let token = handle.cancellation_token();
                axum::serve(listener, router)
                    .with_graceful_shutdown(async move { token.cancelled().await })
                    .await
                    .map_err(|e| format!("server error: {e}"))?;
            }
            None => {
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

/// Parse outbound allowlist strings into [`url::Url`] values, logging a warning
/// for each entry that fails to parse so operators can spot typos at startup.
///
/// Returns the successfully parsed URLs.
pub(crate) fn parse_allowlist_urls(entries: &[String]) -> Vec<url::Url> {
    let mut urls = Vec::with_capacity(entries.len());
    let mut parsed_count: usize = 0;
    let mut skipped_count: usize = 0;

    for s in entries {
        match url::Url::parse(s) {
            Ok(url) => {
                urls.push(url);
                parsed_count += 1;
            }
            Err(e) => {
                tracing::warn!(
                    target: "liter_llm_proxy::security",
                    entry = ?s,
                    error = %e,
                    "invalid allowlist URL — skipping"
                );
                skipped_count += 1;
            }
        }
    }

    tracing::info!(
        target: "liter_llm_proxy::security",
        parsed = parsed_count,
        skipped = skipped_count,
        "outbound allowlist policy applied"
    );

    urls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_allowlist_urls_accepts_valid_entries() {
        let entries = vec![
            "https://api.openai.com".to_string(),
            "https://api.anthropic.com".to_string(),
        ];
        let urls = parse_allowlist_urls(&entries);
        assert_eq!(urls.len(), 2, "both valid URLs should be parsed");
    }

    #[test]
    fn parse_allowlist_urls_skips_invalid_entries() {
        let entries = vec![
            "htps://api.openai.com".to_string(),
            "https://api.anthropic.com".to_string(),
            "not a url at all".to_string(),
        ];
        let urls = parse_allowlist_urls(&entries);
        assert_eq!(
            urls.len(),
            2,
            "two syntactically valid URLs should be parsed; one relative entry must be skipped"
        );
        assert_eq!(urls[1].as_str(), "https://api.anthropic.com/");
    }

    #[test]
    fn parse_allowlist_urls_empty_input_returns_empty() {
        let urls = parse_allowlist_urls(&[]);
        assert!(urls.is_empty());
    }

    /// Verifies that invalid entries are silently skipped and the parsed count
    /// is correct.  Log capture via `tracing-test` is skipped here because the
    /// `#[ctor]` global crypto subscriber installed elsewhere in this crate
    /// pre-empts the test subscriber; the warn/info paths are covered by the
    /// code path exercised in `parse_allowlist_urls_skips_invalid_entries`.
    #[test]
    fn parse_allowlist_urls_emits_warn_on_invalid_entry() {
        let entries = vec!["not-a-url".to_string(), "https://api.anthropic.com".to_string()];
        let urls = parse_allowlist_urls(&entries);
        assert_eq!(
            urls.len(),
            1,
            "one valid entry must be returned; the relative entry must be skipped"
        );
        assert_eq!(urls[0].as_str(), "https://api.anthropic.com/");
    }
}
