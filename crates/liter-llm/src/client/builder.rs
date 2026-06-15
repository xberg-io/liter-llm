//! Type-state builder for [`DefaultClient`].
//!
//! Use [`ClientBuilder`] when you want compile-time enforcement that both an
//! API key and a provider name have been supplied before the client is
//! constructed.  Calling [`build`](ClientBuilder::build) without setting both
//! fields is a **compile error**, not a runtime error.
//!
//! [`ClientConfigBuilder`](super::config::ClientConfigBuilder) remains
//! available for cases where the key or provider are not known until runtime
//! (e.g. when loading from environment variables without an explicit provider
//! hint).
//!
//! # Examples
//!
//! ```rust,no_run
//! # #[cfg(any(feature = "native-http", feature = "wasm-http"))]
//! # {
//! use liter_llm::ClientBuilder;
//!
//! # async fn run() -> liter_llm::Result<()> {
//! let client = ClientBuilder::new()
//!     .api_key("sk-…")
//!     .provider("openai")
//!     .build()?;
//! # Ok(())
//! # }
//! # }
//! ```

use std::sync::Arc;
use std::time::Duration;

use secrecy::SecretString;

use crate::auth::CredentialProvider;
#[cfg(any(feature = "native-http", feature = "wasm-http"))]
use crate::error::Result;
use crate::http::transport::TransportConfig;
#[cfg(feature = "tower")]
use crate::tower::{BudgetConfig, CacheConfig, CacheStore, LlmHook, RateLimitConfig};

// ── Type-state markers ───────────────────────────────────────────────────────

/// Marker: no API key has been set on this builder.
pub struct NoApiKey;
/// Marker: an API key has been set on this builder.
pub struct WithApiKey;
/// Marker: no provider has been set on this builder.
pub struct NoProvider;
/// Marker: a provider name (model hint) has been set on this builder.
pub struct WithProvider;

// ── Builder ──────────────────────────────────────────────────────────────────

/// Type-state builder for [`DefaultClient`](super::DefaultClient).
///
/// The two type parameters `K` and `P` track whether the required fields have
/// been supplied:
///
/// - `K` is [`NoApiKey`] until [`api_key`](Self::api_key) is called, then
///   [`WithApiKey`].
/// - `P` is [`NoProvider`] until [`provider`](Self::provider) is called, then
///   [`WithProvider`].
///
/// [`build`](Self::build) is only available when both are [`WithApiKey`] and
/// [`WithProvider`].  Attempting to call it in any other state is a **compile
/// error**.
///
/// All optional knobs (`base_url`, `timeout`, `transport`, …) are available
/// at any point in the chain and do not affect the type-state.
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(any(feature = "native-http", feature = "wasm-http"))]
/// # {
/// use liter_llm::ClientBuilder;
///
/// # async fn run() -> liter_llm::Result<()> {
/// let client = ClientBuilder::new()
///     .api_key("sk-…")
///     .provider("openai")
///     .timeout(std::time::Duration::from_secs(30))
///     .build()?;
/// # Ok(())
/// # }
/// # }
/// ```
#[must_use = "call .build() to construct the client"]
pub struct ClientBuilder<K = NoApiKey, P = NoProvider> {
    api_key: SecretString,
    provider_hint: String,
    base_url: Option<String>,
    timeout: Duration,
    max_retries: u32,
    transport: TransportConfig,
    load_env: bool,
    credential_provider: Option<Arc<dyn CredentialProvider>>,
    #[cfg(feature = "tower")]
    cache_config: Option<CacheConfig>,
    #[cfg(feature = "tower")]
    cache_store: Option<Arc<dyn CacheStore>>,
    #[cfg(feature = "tower")]
    budget_config: Option<BudgetConfig>,
    #[cfg(feature = "tower")]
    hooks: Vec<Arc<dyn LlmHook>>,
    #[cfg(feature = "tower")]
    cooldown_duration: Option<Duration>,
    #[cfg(feature = "tower")]
    rate_limit_config: Option<RateLimitConfig>,
    #[cfg(feature = "tower")]
    health_check_interval: Option<Duration>,
    #[cfg(feature = "tower")]
    enable_cost_tracking: bool,
    #[cfg(feature = "tower")]
    enable_tracing: bool,
    _key_state: std::marker::PhantomData<K>,
    _provider_state: std::marker::PhantomData<P>,
}

impl ClientBuilder<NoApiKey, NoProvider> {
    /// Create a new builder with no key and no provider set.
    ///
    /// Call [`api_key`](Self::api_key) and [`provider`](Self::provider) to
    /// advance the type state, then call [`build`] when both are set.
    pub fn new() -> Self {
        Self {
            api_key: SecretString::from(String::new()),
            provider_hint: String::new(),
            base_url: None,
            timeout: Duration::from_secs(60),
            max_retries: 3,
            transport: TransportConfig::default(),
            load_env: false,
            credential_provider: None,
            #[cfg(feature = "tower")]
            cache_config: None,
            #[cfg(feature = "tower")]
            cache_store: None,
            #[cfg(feature = "tower")]
            budget_config: None,
            #[cfg(feature = "tower")]
            hooks: Vec::new(),
            #[cfg(feature = "tower")]
            cooldown_duration: None,
            #[cfg(feature = "tower")]
            rate_limit_config: None,
            #[cfg(feature = "tower")]
            health_check_interval: None,
            #[cfg(feature = "tower")]
            enable_cost_tracking: false,
            #[cfg(feature = "tower")]
            enable_tracing: false,
            _key_state: std::marker::PhantomData,
            _provider_state: std::marker::PhantomData,
        }
    }
}

impl Default for ClientBuilder<NoApiKey, NoProvider> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, P> ClientBuilder<K, P> {
    /// Set the API key for authentication.
    ///
    /// Accepts any value that converts into a [`SecretString`] (including
    /// `&str` and `String`).  The secret is zeroed on drop.
    ///
    /// Calling this method transitions the `K` type parameter from
    /// [`NoApiKey`] to [`WithApiKey`].
    pub fn api_key(self, key: impl Into<String>) -> ClientBuilder<WithApiKey, P> {
        ClientBuilder {
            api_key: SecretString::from(key.into()),
            provider_hint: self.provider_hint,
            base_url: self.base_url,
            timeout: self.timeout,
            max_retries: self.max_retries,
            transport: self.transport,
            load_env: self.load_env,
            credential_provider: self.credential_provider,
            #[cfg(feature = "tower")]
            cache_config: self.cache_config,
            #[cfg(feature = "tower")]
            cache_store: self.cache_store,
            #[cfg(feature = "tower")]
            budget_config: self.budget_config,
            #[cfg(feature = "tower")]
            hooks: self.hooks,
            #[cfg(feature = "tower")]
            cooldown_duration: self.cooldown_duration,
            #[cfg(feature = "tower")]
            rate_limit_config: self.rate_limit_config,
            #[cfg(feature = "tower")]
            health_check_interval: self.health_check_interval,
            #[cfg(feature = "tower")]
            enable_cost_tracking: self.enable_cost_tracking,
            #[cfg(feature = "tower")]
            enable_tracing: self.enable_tracing,
            _key_state: std::marker::PhantomData,
            _provider_state: self._provider_state,
        }
    }

    /// Set the provider by name or model-hint string.
    ///
    /// The string is forwarded to provider auto-detection in the same way
    /// that the `model_hint` argument to `DefaultClient::new` works.  Use
    /// a slash-prefixed name (e.g. `"openai"`, `"groq/llama3-70b"`,
    /// `"anthropic"`) or a bare model name.
    ///
    /// Calling this method transitions the `P` type parameter from
    /// [`NoProvider`] to [`WithProvider`].
    pub fn provider(self, provider_name: impl Into<String>) -> ClientBuilder<K, WithProvider> {
        ClientBuilder {
            api_key: self.api_key,
            provider_hint: provider_name.into(),
            base_url: self.base_url,
            timeout: self.timeout,
            max_retries: self.max_retries,
            transport: self.transport,
            load_env: self.load_env,
            credential_provider: self.credential_provider,
            #[cfg(feature = "tower")]
            cache_config: self.cache_config,
            #[cfg(feature = "tower")]
            cache_store: self.cache_store,
            #[cfg(feature = "tower")]
            budget_config: self.budget_config,
            #[cfg(feature = "tower")]
            hooks: self.hooks,
            #[cfg(feature = "tower")]
            cooldown_duration: self.cooldown_duration,
            #[cfg(feature = "tower")]
            rate_limit_config: self.rate_limit_config,
            #[cfg(feature = "tower")]
            health_check_interval: self.health_check_interval,
            #[cfg(feature = "tower")]
            enable_cost_tracking: self.enable_cost_tracking,
            #[cfg(feature = "tower")]
            enable_tracing: self.enable_tracing,
            _key_state: self._key_state,
            _provider_state: std::marker::PhantomData,
        }
    }

    /// Override the provider base URL for all requests.
    ///
    /// Trailing slashes are trimmed automatically.  When set, provider
    /// auto-detection is bypassed and all requests go to this URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        let url = url.into();
        self.base_url = Some(url.trim_end_matches('/').to_string());
        self
    }

    /// Set the per-request timeout (default: 60 s).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries on 429 / 5xx responses (default: 3).
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the HTTP transport configuration.
    ///
    /// Controls connection pooling, TCP keepalive, DNS caching, and HTTP
    /// version selection.
    pub fn transport(mut self, config: TransportConfig) -> Self {
        self.transport = config;
        self
    }

    /// Enable or disable automatic API key loading from environment variables.
    ///
    /// When `true`, the client will attempt to read the provider's designated
    /// environment variable (e.g. `OPENAI_API_KEY`) if no key was set via
    /// [`api_key`](Self::api_key).  Defaults to `false` in [`ClientBuilder`]
    /// (unlike [`ClientConfigBuilder`](super::config::ClientConfigBuilder)
    /// which defaults to `true`) because the type-state design signals that
    /// the key should be provided explicitly.
    pub fn load_env(mut self, enabled: bool) -> Self {
        self.load_env = enabled;
        self
    }

    /// Set a dynamic credential provider for token-based or refreshable auth.
    ///
    /// When configured, the client calls `resolve()` before each request
    /// instead of using the static API key for authentication.
    pub fn credential_provider(mut self, provider: Arc<dyn CredentialProvider>) -> Self {
        self.credential_provider = Some(provider);
        self
    }

    /// Set the response cache configuration for the Tower middleware stack.
    #[cfg(feature = "tower")]
    pub fn cache(mut self, config: CacheConfig) -> Self {
        self.cache_config = Some(config);
        self
    }

    /// Set a custom cache store backend for the Tower cache middleware.
    #[cfg(feature = "tower")]
    pub fn cache_store(mut self, store: Arc<dyn CacheStore>) -> Self {
        self.cache_store = Some(store);
        self
    }

    /// Set the budget enforcement configuration for the Tower middleware stack.
    #[cfg(feature = "tower")]
    pub fn budget(mut self, config: BudgetConfig) -> Self {
        self.budget_config = Some(config);
        self
    }

    /// Add a single hook to the Tower hooks middleware stack.
    #[cfg(feature = "tower")]
    pub fn hook(mut self, hook: Arc<dyn LlmHook>) -> Self {
        self.hooks.push(hook);
        self
    }

    /// Set the full list of hooks for the Tower hooks middleware stack.
    #[cfg(feature = "tower")]
    pub fn hooks(mut self, hooks: Vec<Arc<dyn LlmHook>>) -> Self {
        self.hooks = hooks;
        self
    }

    /// Set the cooldown duration after transient errors.
    #[cfg(feature = "tower")]
    pub fn cooldown(mut self, duration: Duration) -> Self {
        self.cooldown_duration = Some(duration);
        self
    }

    /// Set per-model rate limiting configuration.
    #[cfg(feature = "tower")]
    pub fn rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = Some(config);
        self
    }

    /// Set the background health check interval.
    #[cfg(feature = "tower")]
    pub fn health_check(mut self, interval: Duration) -> Self {
        self.health_check_interval = Some(interval);
        self
    }

    /// Enable or disable per-request cost tracking.
    #[cfg(feature = "tower")]
    pub fn cost_tracking(mut self, enabled: bool) -> Self {
        self.enable_cost_tracking = enabled;
        self
    }

    /// Enable or disable OpenTelemetry-compatible tracing spans.
    #[cfg(feature = "tower")]
    pub fn tracing(mut self, enabled: bool) -> Self {
        self.enable_tracing = enabled;
        self
    }
}

// ── Terminal: build() is only available with WithApiKey + WithProvider ────────

#[cfg(any(feature = "native-http", feature = "wasm-http"))]
impl ClientBuilder<WithApiKey, WithProvider> {
    /// Consume the builder and construct a [`DefaultClient`](super::DefaultClient).
    ///
    /// This method is only available when both [`api_key`](ClientBuilder::api_key)
    /// and [`provider`](ClientBuilder::provider) have been called.  The
    /// compiler enforces this at the type level.
    ///
    /// # Errors
    ///
    /// Returns `LiterLlmError::Authentication` if the provider requires an
    /// environment variable that is unset (only relevant when the API key
    /// supplied via [`api_key`] is an empty string and `load_env` is `true`).
    ///
    /// Returns `LiterLlmError::Http` if the underlying HTTP client cannot be
    /// constructed.
    pub fn build(self) -> Result<super::DefaultClient> {
        use super::config::ClientConfig;

        let config = ClientConfig {
            api_key: self.api_key,
            base_url: self.base_url,
            timeout: self.timeout,
            max_retries: self.max_retries,
            extra_headers: Vec::new(),
            credential_provider: self.credential_provider,
            load_env: self.load_env,
            transport: self.transport,
            #[cfg(feature = "tower")]
            cache_config: self.cache_config,
            #[cfg(feature = "tower")]
            cache_store: self.cache_store,
            #[cfg(feature = "tower")]
            budget_config: self.budget_config,
            #[cfg(feature = "tower")]
            hooks: self.hooks,
            #[cfg(feature = "tower")]
            cooldown_duration: self.cooldown_duration,
            #[cfg(feature = "tower")]
            rate_limit_config: self.rate_limit_config,
            #[cfg(feature = "tower")]
            health_check_interval: self.health_check_interval,
            #[cfg(feature = "tower")]
            enable_cost_tracking: self.enable_cost_tracking,
            #[cfg(feature = "tower")]
            enable_tracing: self.enable_tracing,
        };

        // Use the provider_hint for model detection; an empty hint defaults to
        // OpenAI (same semantics as DefaultClient::new with None hint).
        let hint = if self.provider_hint.is_empty() {
            None
        } else {
            Some(self.provider_hint.as_str())
        };

        super::DefaultClient::new(config, hint)
    }
}
