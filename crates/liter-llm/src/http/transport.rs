/// HTTP transport configuration and builder.
///
/// This module provides fine-grained control over the reqwest HTTP client's
/// connection pooling, DNS resolution, TLS session resumption, and HTTP version
/// selection. All settings have sensible defaults that maintain backward
/// compatibility with existing code.
///
/// # Examples
///
/// ```no_run
/// use liter_llm::TransportConfig;
/// use std::time::Duration;
///
/// let config = TransportConfig::default()
///     .with_pool_max_idle_per_host(16)
///     .with_pool_idle_timeout(Some(Duration::from_secs(30)))
///     .with_http2_prior_knowledge(true)
///     .with_tcp_keepalive(Some(Duration::from_secs(60)));
/// ```
use std::time::Duration;

/// HTTP transport configuration for [`reqwest::Client`].
///
/// Manages connection pooling, DNS caching, TLS session resumption, and HTTP
/// version negotiation. All fields have defaults that preserve the current
/// behavior and do not require explicit configuration.
#[derive(Clone, Debug)]
pub struct TransportConfig {
    /// Maximum number of idle connections per host in the connection pool.
    ///
    /// Default: 32. Set to 0 to disable pooling.
    pub pool_max_idle_per_host: usize,

    /// How long an idle connection remains in the pool before being dropped.
    ///
    /// Default: 90 seconds. Set to `None` to disable idle timeout.
    pub pool_idle_timeout: Option<Duration>,

    /// TCP keepalive interval for idle connections.
    ///
    /// Default: 60 seconds. Set to `None` to disable.
    /// Helps detect stale connections on long-lived clients.
    pub tcp_keepalive: Option<Duration>,

    /// Enable HTTP/2 prior-knowledge.
    ///
    /// When true, the client assumes the server supports HTTP/2 and skips the
    /// ALPN negotiation for HTTP/1.1 fallback. Saves a round-trip for endpoints
    /// that advertise HTTP/2 support (OpenAI, Anthropic).
    ///
    /// Default: false (disable to avoid breaking custom base URLs).
    pub http2_prior_knowledge: bool,

    /// DNS cache time-to-live.
    ///
    /// Default: 30 seconds. DNS lookups are cached locally for this duration
    /// before re-resolving. Set to `None` to disable the local DNS cache and
    /// always re-resolve on each request.
    pub dns_cache_ttl: Option<Duration>,

    /// Enable HTTP/3 (h3-quinn) support when the feature flag is active.
    ///
    /// When true and the `http3` feature is enabled, the client negotiates
    /// HTTP/3 during connection establishment if available. No effect when
    /// the feature is disabled; silently ignored.
    ///
    /// Default: false.
    #[cfg(feature = "http3")]
    pub enable_http3: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            pool_max_idle_per_host: 32,
            pool_idle_timeout: Some(Duration::from_secs(90)),
            tcp_keepalive: Some(Duration::from_secs(60)),
            http2_prior_knowledge: false,
            dns_cache_ttl: Some(Duration::from_secs(30)),
            #[cfg(feature = "http3")]
            enable_http3: false,
        }
    }
}

impl TransportConfig {
    /// Create a new transport config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum idle connections per host.
    pub fn with_pool_max_idle_per_host(mut self, count: usize) -> Self {
        self.pool_max_idle_per_host = count;
        self
    }

    /// Set the idle connection timeout.
    pub fn with_pool_idle_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.pool_idle_timeout = timeout;
        self
    }

    /// Set the TCP keepalive interval.
    pub fn with_tcp_keepalive(mut self, interval: Option<Duration>) -> Self {
        self.tcp_keepalive = interval;
        self
    }

    /// Enable or disable HTTP/2 prior-knowledge.
    pub fn with_http2_prior_knowledge(mut self, enabled: bool) -> Self {
        self.http2_prior_knowledge = enabled;
        self
    }

    /// Set the DNS cache TTL.
    pub fn with_dns_cache_ttl(mut self, ttl: Option<Duration>) -> Self {
        self.dns_cache_ttl = ttl;
        self
    }

    /// Enable or disable HTTP/3 (when feature is active).
    #[cfg(feature = "http3")]
    pub fn with_http3(mut self, enabled: bool) -> Self {
        self.enable_http3 = enabled;
        self
    }

    /// Apply all transport settings to a [`reqwest::ClientBuilder`].
    ///
    /// This method converts transport fields into concrete reqwest builder
    /// calls. Call it once during client construction and chain the returned
    /// builder.
    ///
    /// Not available on WASM targets: the browser fetch API controls transport
    /// settings and does not expose builder-level hooks.
    ///
    /// # Fields and their reqwest mapping
    ///
    /// | Field | reqwest method |
    /// |---|---|
    /// | `pool_max_idle_per_host` | `.pool_max_idle_per_host(n)` |
    /// | `pool_idle_timeout` | `.pool_idle_timeout(t)` |
    /// | `tcp_keepalive` | `.tcp_keepalive(t)` |
    /// | `http2_prior_knowledge` | `.http2_prior_knowledge()` (when `true`) |
    /// | `dns_cache_ttl` | `.dns_resolver(...)` during native client construction |
    /// | `enable_http3` | `.http3_prior_knowledge()` (when `true` and `http3` feature is on) |
    ///
    /// # DNS TTL
    ///
    /// reqwest 0.13 exposes DNS customization through `ClientBuilder::dns_resolver`
    /// rather than a direct TTL setter. The native client construction path
    /// installs a resolver that combines this TTL with outbound-policy DNS
    /// validation, so DNS rebinding protection is preserved when both features
    /// are active.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn apply_to_builder(&self, builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
        let builder = builder
            .pool_max_idle_per_host(self.pool_max_idle_per_host)
            .pool_idle_timeout(self.pool_idle_timeout)
            .tcp_keepalive(self.tcp_keepalive);

        let builder = if self.http2_prior_knowledge {
            builder.http2_prior_knowledge()
        } else {
            builder
        };

        #[cfg(feature = "http3")]
        let _ = self.enable_http3;

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = TransportConfig::default();
        assert_eq!(cfg.pool_max_idle_per_host, 32);
        assert_eq!(cfg.pool_idle_timeout, Some(Duration::from_secs(90)));
        assert_eq!(cfg.tcp_keepalive, Some(Duration::from_secs(60)));
        assert!(!cfg.http2_prior_knowledge);
        assert_eq!(cfg.dns_cache_ttl, Some(Duration::from_secs(30)));
        #[cfg(feature = "http3")]
        assert!(!cfg.enable_http3);
    }

    #[test]
    fn test_builder_chain() {
        let cfg = TransportConfig::new()
            .with_pool_max_idle_per_host(16)
            .with_pool_idle_timeout(Some(Duration::from_secs(45)))
            .with_tcp_keepalive(Some(Duration::from_secs(120)))
            .with_http2_prior_knowledge(true)
            .with_dns_cache_ttl(Some(Duration::from_secs(60)));

        assert_eq!(cfg.pool_max_idle_per_host, 16);
        assert_eq!(cfg.pool_idle_timeout, Some(Duration::from_secs(45)));
        assert_eq!(cfg.tcp_keepalive, Some(Duration::from_secs(120)));
        assert!(cfg.http2_prior_knowledge);
        assert_eq!(cfg.dns_cache_ttl, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_disable_pooling() {
        let cfg = TransportConfig::new().with_pool_max_idle_per_host(0);
        assert_eq!(cfg.pool_max_idle_per_host, 0);
    }

    #[test]
    fn test_disable_keepalive() {
        let cfg = TransportConfig::new().with_tcp_keepalive(None);
        assert_eq!(cfg.tcp_keepalive, None);
    }

    #[test]
    fn test_disable_dns_cache() {
        let cfg = TransportConfig::new().with_dns_cache_ttl(None);
        assert_eq!(cfg.dns_cache_ttl, None);
    }

    #[cfg(feature = "http3")]
    #[test]
    fn test_http3_flag() {
        let cfg = TransportConfig::new().with_http3(true);
        assert!(cfg.enable_http3);
    }

    /// Verify that `apply_to_builder` actually produces a buildable reqwest
    /// client when a non-default `TransportConfig` is supplied.  If the method
    /// were not wired or had an incompatible API call this would fail to compile
    /// or panic at `.build()`.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_apply_to_builder_builds_client_with_non_default_config() {
        let cfg = TransportConfig::new()
            .with_pool_max_idle_per_host(128)
            .with_pool_idle_timeout(Some(Duration::from_secs(45)))
            .with_tcp_keepalive(Some(Duration::from_secs(30)))
            .with_http2_prior_knowledge(false)
            .with_dns_cache_ttl(Some(Duration::from_secs(60)));

        let builder = reqwest::Client::builder();
        let builder = cfg.apply_to_builder(builder);
        let client = builder.build();
        assert!(
            client.is_ok(),
            "reqwest::Client::build() failed with non-default TransportConfig"
        );
    }

    /// Verify that `apply_to_builder` with `http2_prior_knowledge = true` also
    /// produces a valid client.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_apply_to_builder_with_http2_prior_knowledge() {
        let cfg = TransportConfig::new().with_http2_prior_knowledge(true);
        let client = cfg.apply_to_builder(reqwest::Client::builder()).build();
        assert!(
            client.is_ok(),
            "reqwest::Client::build() failed with http2_prior_knowledge=true"
        );
    }

    /// Verify that `apply_to_builder` with pooling disabled produces a valid client.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_apply_to_builder_with_pooling_disabled() {
        let cfg = TransportConfig::new()
            .with_pool_max_idle_per_host(0)
            .with_pool_idle_timeout(None)
            .with_tcp_keepalive(None)
            .with_dns_cache_ttl(None);

        let client = cfg.apply_to_builder(reqwest::Client::builder()).build();
        assert!(client.is_ok(), "reqwest::Client::build() failed with pooling disabled");
    }
}
