use std::path::PathBuf;

use clap::Args;
use liter_llm::provider::{OutboundPolicy, set_outbound_policy};
use liter_llm_proxy::ProxyServer;
use liter_llm_proxy::config::{OutboundPolicyKind, ProxyConfig};
use secrecy::SecretString;

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
    if let Some(key) = args.master_key {
        config.general.master_key = Some(SecretString::from(key));
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

    ProxyServer::new(config).serve().await
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
                .map(|s| {
                    url::Url::parse(s)
                        .map_err(|e| format!("invalid URL in outbound_allowlist ({s:?}): {e}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(OutboundPolicy::Allowlist(urls))
        }
    }
}
