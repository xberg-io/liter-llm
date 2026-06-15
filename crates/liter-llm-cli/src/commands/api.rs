use std::path::PathBuf;
use std::time::Duration;

use clap::Args;
use liter_llm::provider::{OutboundPolicy, set_outbound_policy};
use liter_llm_proxy::ProxyServer;
use liter_llm_proxy::WatchMode;
use liter_llm_proxy::config::{OutboundPolicyKind, ProxyConfig};
use liter_llm_proxy::shutdown::{ShutdownCoordinator, ShutdownPhase};
use secrecy::SecretString;

/// Drain progress log interval.
const DRAIN_LOG_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Args)]
pub struct ApiArgs {
    /// Path to config file (default: auto-discover liter-llm-proxy.toml).
    #[arg(long, short)]
    pub config: Option<PathBuf>,
    /// Override bind host.
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,
    /// Override bind port.
    #[arg(long, short, default_value_t = 4000)]
    pub port: u16,
    /// Master API key (overrides config/env).
    #[arg(long, env = "LITER_LLM_MASTER_KEY")]
    pub master_key: Option<String>,
    /// Enable debug logging.
    #[arg(long)]
    pub debug: bool,
    /// Enable hot-reload of the configuration file.
    ///
    /// When `--config` is set, watches that file for changes and reloads
    /// automatically on every save.  When `--etcd-endpoint` is also set,
    /// uses etcd instead of file watching.
    #[arg(long)]
    pub watch: bool,
    /// etcd endpoint URL(s) for distributed config (comma-separated).
    ///
    /// Example: `http://127.0.0.1:2379`
    ///
    /// When provided together with `--watch`, the proxy subscribes to the
    /// etcd key at `--etcd-key` for live configuration updates.
    #[arg(long, value_delimiter = ',', env = "LITER_LLM_ETCD_ENDPOINTS")]
    pub etcd_endpoint: Vec<String>,
    /// etcd key to watch for proxy configuration (default: `/liter-llm/config`).
    ///
    /// Only used when `--etcd-endpoint` is set.
    #[arg(long, default_value = "/liter-llm/config", env = "LITER_LLM_ETCD_KEY")]
    pub etcd_key: String,
}

pub async fn run(args: ApiArgs) -> Result<(), String> {
    let filter = if args.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .init();

    let mut config = if let Some(path) = &args.config {
        ProxyConfig::from_toml_file(path)?
    } else {
        ProxyConfig::discover()?.unwrap_or_default()
    };

    // Apply CLI overrides (highest precedence).
    config.server.host.clone_from(&args.host);
    config.server.port = args.port;
    if let Some(ref key) = args.master_key {
        config.general.master_key = Some(SecretString::from(key.clone()));
    }

    // Warn when wildcard CORS is combined with a public-facing bind address.
    // This combination allows any origin to make credentialed cross-origin
    // requests, which is a common misconfiguration.
    if config.server.cors_origins.iter().any(|o| o == "*") && config.server.host == "0.0.0.0" {
        tracing::warn!(
            "wildcard CORS (cors_origins=[\"*\"]) combined with host=0.0.0.0 exposes \
             credentialed cross-origin requests; restrict cors_origins or bind to 127.0.0.1"
        );
    }

    // Apply the outbound policy BEFORE any custom providers are registered or
    // requests are accepted.  The library default is Off; the proxy default
    // (via SecurityConfig) is DenyPrivate, blocking SSRF into metadata services
    // and private networks.
    let policy = build_outbound_policy(&config)?;
    set_outbound_policy(policy);

    // Resolve the watch mode from CLI flags.
    let watch_mode = resolve_watch_mode(&args)?;

    // ── Shutdown coordinator ──────────────────────────────────────────────
    let coordinator = ShutdownCoordinator::new();
    let handle = coordinator.handle();

    // Signal handler: SIGTERM / SIGINT → Draining; second signal → Aborted.
    coordinator.spawn_signal_handler();

    // Axum server — drains via the cancellation token.
    let server_handle = handle.clone();
    let server_task = tokio::spawn(async move {
        ProxyServer::new(config)
            .with_watch_mode(watch_mode)
            .serve_with_shutdown(Some(server_handle))
            .await
    });

    // Wait until we enter Draining (or beyond) before logging drain progress.
    {
        let mut drain_handle = handle.clone();
        drain_handle.wait_for_drain_start().await;
    }

    let phase = handle.phase();
    if phase >= ShutdownPhase::Draining {
        tracing::info!("Draining — waiting for in-flight requests to complete...");
    }

    // Wait for the coordinator to finish draining all subsystems, emitting a
    // progress log every DRAIN_LOG_INTERVAL so operators know draining is
    // in progress.
    let final_phase = drain_with_progress_log(coordinator, DRAIN_LOG_INTERVAL).await;

    match final_phase {
        ShutdownPhase::Drained => {
            tracing::info!("graceful shutdown complete");
        }
        ShutdownPhase::Aborted => {
            tracing::error!("shutdown aborted — some in-flight requests may have been dropped");
        }
        other => {
            tracing::warn!(?other, "unexpected final shutdown phase");
        }
    }

    // Await the server task to collect any error it returned.
    match server_task.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::error!("server exited with error: {e}");
            return Err(e);
        }
        Err(e) => {
            tracing::error!("server task panicked: {e}");
            return Err(format!("server task panicked: {e}"));
        }
    }

    Ok(())
}

/// Translate CLI flags into a [`WatchMode`].
fn resolve_watch_mode(args: &ApiArgs) -> Result<WatchMode, String> {
    if !args.watch {
        return Ok(WatchMode::Off);
    }

    if !args.etcd_endpoint.is_empty() {
        return Ok(WatchMode::Etcd {
            endpoints: args.etcd_endpoint.clone(),
            key: args.etcd_key.clone(),
        });
    }

    let path = args
        .config
        .clone()
        .ok_or_else(|| "--watch requires --config <path> or --etcd-endpoint <url>".to_owned())?;

    Ok(WatchMode::File { path })
}

/// Drive `coordinator` to completion while emitting a log line every `interval`.
async fn drain_with_progress_log(coordinator: ShutdownCoordinator, interval: Duration) -> ShutdownPhase {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let drain_fut = coordinator.wait_for_drained();
    tokio::pin!(drain_fut);

    loop {
        tokio::select! {
            phase = &mut drain_fut => return phase,
            _ = ticker.tick() => {
                tracing::info!("still draining in-flight requests...");
            }
        }
    }
}

/// Translate the proxy security config into a [`OutboundPolicy`].
fn build_outbound_policy(config: &ProxyConfig) -> Result<OutboundPolicy, String> {
    let security = &config.security;
    match security.outbound_policy {
        OutboundPolicyKind::Off => Ok(OutboundPolicy::Off),
        OutboundPolicyKind::DenyPrivate => Ok(OutboundPolicy::DenyPrivate),
        OutboundPolicyKind::Allowlist => {
            if security.outbound_allowlist.is_empty() {
                return Err(
                    "security.outbound_policy = \"allowlist\" requires at least one entry in \
                     security.outbound_allowlist"
                        .into(),
                );
            }
            let urls = security
                .outbound_allowlist
                .iter()
                .map(|s| url::Url::parse(s).map_err(|e| format!("invalid URL in outbound_allowlist ({s:?}): {e}")))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(OutboundPolicy::Allowlist(urls))
        }
    }
}
