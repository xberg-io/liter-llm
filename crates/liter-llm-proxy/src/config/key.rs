use secrecy::SecretString;
use serde::Deserialize;

/// A single provider credential in a virtual key's credential pool.
///
/// When a virtual key's `provider_credentials` list is non-empty, the proxy
/// uses a [`crate::provider::InMemoryCredentialPool`] to rotate among these
/// credentials automatically on 429 / 5xx responses.
///
/// `api_key` is stored as a [`SecretString`] so the value is zeroed on drop
/// and redacted in `Debug` output.  Do **not** log or display this struct
/// directly — use `format!("{:?}", cred)` to verify redaction if needed.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCredential {
    /// The provider name this credential is for (e.g. `"openai"`, `"anthropic"`).
    pub provider: String,
    /// Opaque identifier for this credential within the pool.
    pub id: String,
    /// The raw API key — stored behind `SecretString`; zeroed on drop.
    pub api_key: SecretString,
    /// Optional list of model names this credential is allowed to serve.
    /// `null` / omitted means the credential is valid for all models.
    #[serde(default)]
    pub model_allowlist: Option<Vec<String>>,
}

impl std::fmt::Debug for ProviderCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderCredential")
            .field("provider", &self.provider)
            .field("id", &self.id)
            .field("api_key", &"[REDACTED]")
            .field("model_allowlist", &self.model_allowlist)
            .finish()
    }
}

/// A virtual API key with optional model restrictions, rate/budget limits,
/// and a per-provider credential pool for automatic key rotation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VirtualKeyConfig {
    pub key: String,
    pub description: Option<String>,
    /// Models this virtual key is allowed to access. Empty means all models.
    #[serde(default)]
    pub models: Vec<String>,
    pub rpm: Option<u32>,
    pub tpm: Option<u64>,
    pub budget_limit: Option<f64>,
    /// Per-provider credential pool.
    ///
    /// When non-empty, each provider listed here gets an
    /// [`crate::provider::InMemoryCredentialPool`] seeded with these entries.
    /// The proxy rotates among them automatically on 429 / 5xx responses.
    ///
    /// Example TOML:
    /// ```toml
    /// [[keys]]
    /// key = "vk-mykey"
    ///
    /// [[keys.provider_credentials]]
    /// provider = "openai"
    /// id = "key-1"
    /// api_key = "sk-..."
    ///
    /// [[keys.provider_credentials]]
    /// provider = "openai"
    /// id = "key-2"
    /// api_key = "sk-..."
    /// model_allowlist = ["gpt-4o"]
    /// ```
    #[serde(default)]
    pub provider_credentials: Vec<ProviderCredential>,
}
