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
    ///
    /// Prefer the `LITER_LLM_MASTER_KEY` environment variable or the config file: a value passed
    /// here is visible to any local user in the process table.
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
    // ~keep Hold the guard for the whole server so the OTLP providers flush on exit.
    let _telemetry_guard = crate::telemetry::init(filter);

    let mut config = if let Some(path) = &args.config {
        ProxyConfig::from_toml_file(path)?
    } else {
        ProxyConfig::discover()?.unwrap_or_default()
    };

    config.server.host.clone_from(&args.host);
    config.server.port = args.port;
    if let Some(ref key) = args.master_key {
        config.general.master_key = Some(SecretString::from(key.clone()));
    }

    // ~keep An argv-supplied master key is world-readable via the process table (ps/procfs) for the
    // process lifetime; the env var and config-file paths are not. Detected by scanning argv, since a
    // value sourced from LITER_LLM_MASTER_KEY never appears there.
    if std::env::args().any(|a| a == "--master-key" || a.starts_with("--master-key=")) {
        tracing::warn!(
            "master key passed as a command-line argument is visible to any local user via the \
             process table; prefer the LITER_LLM_MASTER_KEY environment variable or the config file"
        );
    }

    // ~keep Warn when wildcard CORS plus public bind permits credentialed cross-origin requests.
    if config.server.cors_origins.iter().any(|o| o == "*") && config.server.host == "0.0.0.0" {
        tracing::warn!(
            "wildcard CORS (cors_origins=[\"*\"]) combined with host=0.0.0.0 exposes \
             credentialed cross-origin requests; restrict cors_origins or bind to 127.0.0.1"
        );
    }

    // ~keep Apply outbound policy before custom providers or requests can open SSRF paths.
    let policy = build_outbound_policy(&config)?;
    set_outbound_policy(policy);

    let watch_mode = resolve_watch_mode(&args)?;

    let coordinator = ShutdownCoordinator::new();
    coordinator.spawn_signal_handler();

    serve_until_shutdown(config, watch_mode, coordinator).await
}

/// Run the proxy until a shutdown signal arrives or the server stops by itself.
async fn serve_until_shutdown(
    config: ProxyConfig,
    watch_mode: WatchMode,
    coordinator: ShutdownCoordinator,
) -> Result<(), String> {
    let handle = coordinator.handle();

    let server_handle = handle.clone();
    let mut server_task = tokio::spawn(async move {
        ProxyServer::new(config)
            .with_watch_mode(watch_mode)
            .serve_with_shutdown(Some(server_handle))
            .await
    });

    // ~keep A server that fails during startup ends its task with no shutdown signal, so the wait
    // ~keep for the signal must also watch the task or the process stays alive without a listener.
    {
        let mut drain_handle = handle.clone();
        tokio::select! {
            () = drain_handle.wait_for_drain_start() => {}
            outcome = &mut server_task => return server_outcome(outcome),
        }
    }

    let phase = handle.phase();
    if phase >= ShutdownPhase::Draining {
        tracing::info!("Draining — waiting for in-flight requests to complete...");
    }

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

    server_outcome(server_task.await)
}

/// Turn the result of the server task into the result of the command.
fn server_outcome(outcome: Result<Result<(), String>, tokio::task::JoinError>) -> Result<(), String> {
    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            tracing::error!("server exited with error: {e}");
            Err(e)
        }
        Err(e) => {
            tracing::error!("server task panicked: {e}");
            Err(format!("server task panicked: {e}"))
        }
    }
}

/// Translate CLI flags into a [`WatchMode`].
fn resolve_watch_mode(args: &ApiArgs) -> Result<WatchMode, String> {
    if !args.watch {
        return Ok(WatchMode::Off);
    }

    if !args.etcd_endpoint.is_empty() {
        #[cfg(feature = "etcd-watch")]
        return Ok(WatchMode::Etcd {
            endpoints: args.etcd_endpoint.clone(),
            key: args.etcd_key.clone(),
        });

        #[cfg(not(feature = "etcd-watch"))]
        return Err("--etcd-endpoint requires the 'etcd-watch' feature. \
             Rebuild liter-llm-cli with --features etcd-watch."
            .to_owned());
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
pub(crate) fn build_outbound_policy(config: &ProxyConfig) -> Result<OutboundPolicy, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A model entry for a provider with no route and no `base_url` must stop startup with the
    /// refusal. Before, the command waited for a signal and printed the error only after SIGTERM.
    #[tokio::test]
    async fn startup_fails_promptly_for_a_model_with_no_route() {
        let config = ProxyConfig::from_toml_str(
            r#"
[server]
host = "127.0.0.1"
port = 0

[[models]]
name = "m1"
provider_model = "snowflake/arctic"
api_key = "sk-test"
"#,
        )
        .expect("valid TOML");

        let outcome = tokio::time::timeout(
            Duration::from_secs(20),
            serve_until_shutdown(config, WatchMode::Off, ShutdownCoordinator::new()),
        )
        .await
        .expect("startup must end without a shutdown signal");

        let message = outcome.expect_err("startup must fail");
        assert!(message.contains("failed to build client for model 'm1'"), "{message}");
        assert!(
            message.contains("provider `snowflake` has no built-in route"),
            "{message}"
        );
    }
}
