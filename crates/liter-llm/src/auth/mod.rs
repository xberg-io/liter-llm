#[cfg(feature = "azure-auth")]
pub mod azure_ad;
#[cfg(feature = "bedrock-auth")]
pub mod bedrock_sts;
#[cfg(feature = "copilot-auth")]
pub mod github_copilot;
#[cfg(feature = "vertex-adc")]
pub mod vertex_adc;
#[cfg(feature = "vertex-auth")]
pub mod vertex_oauth;

use std::sync::Arc;

use secrecy::SecretString;

use crate::client::BoxFuture;
use crate::error::Result;

/// Dynamic credential provider for providers that use token-based auth
/// (Azure AD, Vertex OAuth2) or refreshable credentials (AWS STS).
///
/// Implementations handle caching, refresh, and expiry internally.
/// The client calls `resolve()` before each request when a credential
/// provider is configured.
#[cfg_attr(alef, alef(skip))]
pub trait CredentialProvider: Send + Sync {
    /// Retrieve a valid credential.
    ///
    /// Implementations should cache credentials and only refresh when
    /// expired or about to expire.
    fn resolve(&self) -> BoxFuture<'_, Result<Credential>>;
}

/// Blanket implementation so `Arc<dyn CredentialProvider>` is itself a
/// `CredentialProvider`, making it convenient to share providers across
/// clients.
impl CredentialProvider for Arc<dyn CredentialProvider> {
    fn resolve(&self) -> BoxFuture<'_, Result<Credential>> {
        (**self).resolve()
    }
}

/// A resolved credential ready for use in request authentication.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub enum Credential {
    /// Bearer token (Azure AD, Vertex OAuth2, generic OIDC).
    BearerToken(SecretString),
    /// AWS credentials for SigV4 signing.
    AwsCredentials {
        /// Access key ID half of the SigV4 credential pair.
        access_key_id: SecretString,
        /// Secret access key half of the SigV4 credential pair.
        secret_access_key: SecretString,
        /// Optional session token for STS-issued temporary credentials.
        session_token: Option<SecretString>,
    },
}

/// A static credential provider that always returns the same bearer token.
/// Useful for testing or when tokens are managed externally.
#[cfg_attr(alef, alef(skip))]
pub struct StaticTokenProvider {
    token: SecretString,
}

impl StaticTokenProvider {
    /// Build a [`StaticTokenProvider`] that always resolves to `token`.
    #[must_use]
    pub fn new(token: SecretString) -> Self {
        Self { token }
    }
}

impl CredentialProvider for StaticTokenProvider {
    fn resolve(&self) -> BoxFuture<'_, Result<Credential>> {
        let token = self.token.clone();
        Box::pin(async move { Ok(Credential::BearerToken(token)) })
    }
}
