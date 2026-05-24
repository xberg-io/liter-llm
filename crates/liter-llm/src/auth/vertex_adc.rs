//! Google Vertex AI ADC (Application Default Credentials) provider.
//!
//! Obtains a short-lived OAuth2 access token without a service-account JSON key
//! file, making it suitable for GKE Workload Identity, Cloud Run, and Compute
//! Engine deployments.
//!
//! # Token acquisition order
//!
//! 1. **GKE / Compute Engine metadata server** — sends a `GET` to
//!    `http://169.254.169.254/computeMetadata/v1/instance/service-accounts/default/token`
//!    with `Metadata-Flavor: Google`.  This is the fastest path for pods running
//!    under Workload Identity.
//!
//! 2. **`gcp_auth` ADC discovery** — falls back to the `gcp_auth::provider()`
//!    function which tries, in order: `GOOGLE_APPLICATION_CREDENTIALS` file,
//!    `~/.config/gcloud/application_default_credentials.json`, the metadata
//!    server again, and finally `gcloud auth print-access-token`.  This covers
//!    local development and Cloud Run environments.
//!
//! Tokens are cached using the same `RwLock<Option<CachedToken>>` + 5-minute
//! pre-expiry buffer as [`super::vertex_oauth`].
//!
//! # Environment variables
//!
//! | Variable | Description |
//! |----------|-------------|
//! | `VERTEX_AI_SCOPE` | OAuth scope (defaults to `https://www.googleapis.com/auth/cloud-platform`) |
//! | `GOOGLE_APPLICATION_CREDENTIALS` | Path to a SA JSON key (used by the `gcp_auth` fallback path) |
//!
//! # Usage
//!
//! ```rust,ignore
//! use liter_llm::auth::vertex_adc::VertexAdcCredentialProvider;
//! use liter_llm::client::ClientConfigBuilder;
//! use std::sync::Arc;
//!
//! let provider = VertexAdcCredentialProvider::new();
//! let config = ClientConfigBuilder::new("")
//!     .credential_provider(Arc::new(provider))
//!     .build();
//! ```

use std::time::Instant;

use secrecy::SecretString;
use tokio::sync::RwLock;

use super::{Credential, CredentialProvider};
use crate::client::BoxFuture;
use crate::error::LiterLlmError;

/// Default OAuth2 scope for Google Cloud Platform / Vertex AI.
const DEFAULT_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

/// GKE / Compute Engine metadata server token endpoint.
const METADATA_TOKEN_URL: &str = "http://169.254.169.254/computeMetadata/v1/instance/service-accounts/default/token";

/// Required header that the metadata server validates before returning a token.
const METADATA_FLAVOR_HEADER: &str = "Metadata-Flavor";
const METADATA_FLAVOR_VALUE: &str = "Google";

/// Minimum remaining lifetime before a cached token is considered expired.
const EXPIRY_BUFFER_SECS: u64 = 300;

/// Assumed GCP access-token lifetime for tokens obtained via the `gcp_auth` fallback.
///
/// GCP access tokens are issued with a 3600-second lifetime.  Using this as
/// the cache ceiling means our EXPIRY_BUFFER_SECS (300) guard triggers a
/// refresh after ~3300 s — safely before the actual expiry.
const GCP_TOKEN_LIFETIME_SECS: u64 = 3600;

/// Cached token with acquisition time and lifetime.
struct CachedToken {
    token: SecretString,
    acquired_at: Instant,
    expires_in_secs: u64,
}

impl CachedToken {
    /// Returns `true` if the token is still valid with the safety buffer applied.
    fn is_valid(&self) -> bool {
        let elapsed = self.acquired_at.elapsed().as_secs();
        elapsed + EXPIRY_BUFFER_SECS < self.expires_in_secs
    }
}

/// Google Vertex AI ADC credential provider.
///
/// Prefers the Compute Engine / GKE metadata server and falls back to
/// `gcp_auth`'s full ADC discovery chain so the same type works in
/// development, Cloud Run, and GKE without per-environment configuration.
pub struct VertexAdcCredentialProvider {
    scope: String,
    /// Overridable metadata server base URL (set to the real 169.254.169.254
    /// address in production; injected during tests to point at a mock server).
    metadata_token_url: String,
    /// When `false`, skip the `gcp_auth` fallback path.  Used in tests to
    /// exercise the error path without triggering real ADC discovery.
    use_gcp_auth_fallback: bool,
    cached: RwLock<Option<CachedToken>>,
    http_client: reqwest::Client,
}

impl std::fmt::Debug for VertexAdcCredentialProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VertexAdcCredentialProvider")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

impl VertexAdcCredentialProvider {
    /// Create a provider with the default scope.
    ///
    /// The scope defaults to `https://www.googleapis.com/auth/cloud-platform`
    /// but can be overridden via the `VERTEX_AI_SCOPE` environment variable or
    /// by calling [`VertexAdcCredentialProvider::with_scope`].
    #[must_use]
    pub fn new() -> Self {
        let scope = std::env::var("VERTEX_AI_SCOPE").unwrap_or_else(|_| DEFAULT_SCOPE.to_owned());
        Self {
            scope,
            metadata_token_url: METADATA_TOKEN_URL.to_owned(),
            use_gcp_auth_fallback: true,
            cached: RwLock::new(None),
            http_client: reqwest::Client::new(),
        }
    }

    /// Override the OAuth2 scope (default: `https://www.googleapis.com/auth/cloud-platform`).
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = scope.into();
        self
    }

    /// Override the HTTP client used for metadata server requests.
    #[must_use]
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Override the metadata server token URL.
    ///
    /// This is intended for testing — in production the URL is always
    /// `http://169.254.169.254/computeMetadata/v1/instance/service-accounts/default/token`.
    #[must_use]
    pub fn with_metadata_url(metadata_base_url: impl Into<String>) -> Self {
        let scope = std::env::var("VERTEX_AI_SCOPE").unwrap_or_else(|_| DEFAULT_SCOPE.to_owned());
        // Append the metadata path to the supplied base URL.
        let base = metadata_base_url.into();
        let metadata_token_url = format!(
            "{}/computeMetadata/v1/instance/service-accounts/default/token",
            base.trim_end_matches('/')
        );
        Self {
            scope,
            metadata_token_url,
            use_gcp_auth_fallback: true,
            cached: RwLock::new(None),
            http_client: reqwest::Client::new(),
        }
    }

    /// Disable the `gcp_auth` fallback path.
    ///
    /// When set, the provider returns an error instead of falling back to ADC
    /// discovery if the metadata server is unreachable or returns a non-success
    /// status.  Useful in tests that exercise the error path in isolation.
    #[must_use]
    pub fn without_gcp_auth_fallback(mut self) -> Self {
        self.use_gcp_auth_fallback = false;
        self
    }

    /// Attempt to fetch a token from the GKE / Compute Engine metadata server.
    ///
    /// Returns `None` when the metadata server is not reachable (i.e. we are
    /// not running on GCP) so the caller can fall back to ADC discovery.
    async fn fetch_from_metadata_server(&self) -> Option<CachedToken> {
        let response = self
            .http_client
            .get(&self.metadata_token_url)
            .header(METADATA_FLAVOR_HEADER, METADATA_FLAVOR_VALUE)
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                status = response.status().as_u16(),
                "metadata server returned non-success status; will try gcp_auth ADC fallback"
            );
            return None;
        }

        let body = response.text().await.ok()?;
        let parsed: MetadataTokenResponse = serde_json::from_str(&body).ok()?;

        #[cfg(feature = "tracing")]
        tracing::info!("obtained access token from metadata server");

        Some(CachedToken {
            token: SecretString::from(parsed.access_token),
            acquired_at: Instant::now(),
            expires_in_secs: parsed.expires_in,
        })
    }

    /// Fetch a token via the `gcp_auth` ADC discovery chain.
    ///
    /// This covers: `GOOGLE_APPLICATION_CREDENTIALS` file,
    /// `~/.config/gcloud/application_default_credentials.json`, metadata server
    /// (second attempt), and `gcloud auth print-access-token`.
    async fn fetch_from_gcp_auth(&self) -> Result<CachedToken, LiterLlmError> {
        let provider = gcp_auth::provider().await.map_err(|e| LiterLlmError::Authentication {
            message: format!("gcp_auth ADC discovery failed: {e}"),
            status: 401,
        })?;

        let scopes = &[self.scope.as_str()];
        let token = provider
            .token(scopes)
            .await
            .map_err(|e| LiterLlmError::Authentication {
                message: format!("gcp_auth token acquisition failed: {e}"),
                status: 401,
            })?;

        #[cfg(feature = "tracing")]
        tracing::info!("obtained access token via gcp_auth ADC discovery");

        Ok(CachedToken {
            token: SecretString::from(token.as_str().to_owned()),
            acquired_at: Instant::now(),
            expires_in_secs: GCP_TOKEN_LIFETIME_SECS,
        })
    }

    /// Fetch a fresh token, preferring the metadata server over gcp_auth ADC.
    async fn fetch_token(&self) -> Result<CachedToken, LiterLlmError> {
        if let Some(cached) = self.fetch_from_metadata_server().await {
            return Ok(cached);
        }

        if self.use_gcp_auth_fallback {
            #[cfg(feature = "tracing")]
            tracing::debug!("metadata server not available; trying gcp_auth ADC discovery");
            self.fetch_from_gcp_auth().await
        } else {
            Err(LiterLlmError::Authentication {
                message: "Vertex AI ADC: metadata server unavailable and gcp_auth fallback is disabled".into(),
                status: 401,
            })
        }
    }
}

impl Default for VertexAdcCredentialProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialProvider for VertexAdcCredentialProvider {
    fn resolve(&self) -> BoxFuture<'_, crate::error::Result<Credential>> {
        Box::pin(async move {
            // Fast path: read lock to check cache.
            {
                let guard = self.cached.read().await;
                if let Some(ref cached) = *guard
                    && cached.is_valid()
                {
                    #[cfg(feature = "tracing")]
                    tracing::debug!("returning cached Vertex AI ADC token");
                    return Ok(Credential::BearerToken(cached.token.clone()));
                }
            }

            // Slow path: write lock to refresh.
            let mut guard = self.cached.write().await;

            // Double-check after acquiring write lock to avoid a redundant fetch
            // when two tasks race to the slow path simultaneously.
            if let Some(ref cached) = *guard
                && cached.is_valid()
            {
                #[cfg(feature = "tracing")]
                tracing::debug!("returning cached Vertex AI ADC token (post-lock check)");
                return Ok(Credential::BearerToken(cached.token.clone()));
            }

            let fresh = self.fetch_token().await?;
            let token = fresh.token.clone();
            *guard = Some(fresh);

            Ok(Credential::BearerToken(token))
        })
    }
}

/// Deserialised response from the GCE metadata server token endpoint.
#[derive(serde::Deserialize)]
struct MetadataTokenResponse {
    access_token: String,
    expires_in: u64,
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use secrecy::SecretString;

    use super::*;

    // ── CachedToken validity ─────────────────────────────────────────────────

    #[test]
    fn cached_token_is_valid_with_plenty_of_time() {
        let cached = CachedToken {
            token: SecretString::from("tok".to_owned()),
            acquired_at: Instant::now(),
            expires_in_secs: 3600,
        };
        assert!(cached.is_valid());
    }

    #[test]
    fn cached_token_is_expired_at_zero_lifetime() {
        let cached = CachedToken {
            token: SecretString::from("tok".to_owned()),
            acquired_at: Instant::now(),
            expires_in_secs: 0,
        };
        assert!(!cached.is_valid());
    }

    #[test]
    fn cached_token_is_expired_within_buffer() {
        let cached = CachedToken {
            token: SecretString::from("tok".to_owned()),
            acquired_at: Instant::now(),
            // Less than EXPIRY_BUFFER_SECS remaining.
            expires_in_secs: 200,
        };
        assert!(!cached.is_valid());
    }

    // ── Constructor and scope override ────────────────────────────────────────

    #[test]
    #[serial_test::serial(vertex_adc_env)]
    fn default_scope_is_cloud_platform() {
        // Temporarily unset the env var to test the hard-coded default.
        let _guard = EnvGuard::new("VERTEX_AI_SCOPE", None);
        let provider = VertexAdcCredentialProvider::new();
        assert_eq!(provider.scope, DEFAULT_SCOPE);
    }

    #[test]
    #[serial_test::serial(vertex_adc_env)]
    fn scope_override_via_env_var() {
        let _guard = EnvGuard::new("VERTEX_AI_SCOPE", Some("https://custom.scope/"));
        let provider = VertexAdcCredentialProvider::new();
        assert_eq!(provider.scope, "https://custom.scope/");
    }

    #[test]
    #[serial_test::serial(vertex_adc_env)]
    fn with_scope_overrides_scope() {
        let _guard = EnvGuard::new("VERTEX_AI_SCOPE", None);
        let provider = VertexAdcCredentialProvider::new().with_scope("https://my.scope/");
        assert_eq!(provider.scope, "https://my.scope/");
    }

    #[test]
    #[serial_test::serial(vertex_adc_env)]
    fn default_impl_equals_new() {
        let _guard = EnvGuard::new("VERTEX_AI_SCOPE", None);
        let provider: VertexAdcCredentialProvider = Default::default();
        assert_eq!(provider.scope, DEFAULT_SCOPE);
    }

    #[test]
    #[serial_test::serial(vertex_adc_env)]
    fn with_metadata_url_appends_token_path() {
        let _guard = EnvGuard::new("VERTEX_AI_SCOPE", None);
        let provider = VertexAdcCredentialProvider::with_metadata_url("http://127.0.0.1:12345");
        assert_eq!(
            provider.metadata_token_url,
            "http://127.0.0.1:12345/computeMetadata/v1/instance/service-accounts/default/token"
        );
    }

    #[test]
    #[serial_test::serial(vertex_adc_env)]
    fn with_metadata_url_trailing_slash_is_normalised() {
        let _guard = EnvGuard::new("VERTEX_AI_SCOPE", None);
        let provider = VertexAdcCredentialProvider::with_metadata_url("http://127.0.0.1:12345/");
        assert_eq!(
            provider.metadata_token_url,
            "http://127.0.0.1:12345/computeMetadata/v1/instance/service-accounts/default/token"
        );
    }

    // ── RAII env var guard for test isolation ─────────────────────────────────

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn new(key: &'static str, value: Option<&str>) -> Self {
            let original = std::env::var(key).ok();
            // SAFETY: tests that use EnvGuard are single-threaded (unit tests,
            // not spawning additional threads that read this variable).  Mutating
            // env vars is inherently unsafe in multi-threaded code; acceptable
            // here because we restore the original value on drop and each guard
            // covers a short, non-concurrent scope.
            unsafe {
                match value {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: same invariant as `EnvGuard::new` — restoring during drop
            // in a single-threaded test context.
            unsafe {
                match &self.original {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    // ── Live / integration tests (ignored by default) ─────────────────────────

    #[tokio::test]
    #[ignore] // Requires a GCP metadata server or configured ADC.
    async fn live_metadata_server_or_adc_returns_bearer_token() {
        let provider = VertexAdcCredentialProvider::new();
        let credential = provider.resolve().await.expect("token acquisition failed");
        assert!(matches!(credential, Credential::BearerToken(_)));
    }
}
