use std::collections::HashMap;

/// Opaque tenant identifier.
///
/// A transparent `String` newtype that implements common traits so it can be
/// used as a `HashMap` key, compared for equality, and serialized.
#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TenantId(pub String);

impl AsRef<str> for TenantId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for TenantId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TenantId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// Identity and attributes associated with a single request.
///
/// Carried on [`LlmRequest`][crate::tower::types::LlmRequest] via
/// [`LlmRequest::with_tenant_id`] so every Tower layer can read the tenant
/// without re-resolving from raw credentials.
#[derive(Clone, Debug)]
pub struct TenantContext {
    /// Identifies the organisation (tenant) that owns this request.
    pub tenant_id: TenantId,
    /// Optional end-user identifier within the tenant.
    pub user_id: Option<String>,
    /// Arbitrary key-value attributes (e.g. `"environment"`, `"region"`).
    pub attributes: HashMap<String, String>,
}

impl TenantContext {
    /// Create a new context for the given tenant with no user or extra attributes.
    pub fn new(tenant_id: impl Into<TenantId>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            user_id: None,
            attributes: HashMap::new(),
        }
    }

    /// Attach an end-user identifier to this context.
    #[must_use]
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Attach a single key-value attribute.
    #[must_use]
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}
