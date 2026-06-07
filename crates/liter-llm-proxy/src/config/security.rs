//! Security configuration for the liter-llm proxy.
//!
//! Controls the outbound HTTP policy applied to custom provider URLs.  The
//! proxy-side default is [`OutboundPolicyKind::DenyPrivate`], which blocks
//! SSRF into cloud-metadata services and private networks.  Operators must
//! explicitly opt out to `off` when running in a fully-trusted environment.

use serde::Deserialize;

/// Which outbound-request policy the proxy enforces.
///
/// Maps directly to [`liter_llm::provider::OutboundPolicy`] after the config
/// has been parsed and an optional allowlist has been resolved to `Url` values.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutboundPolicyKind {
    /// No restrictions.  Use only when all registered providers are trusted.
    Off,
    /// Reject URLs that resolve to private / loopback / link-local / CGNAT
    /// addresses.  This is the **proxy default**.
    #[default]
    DenyPrivate,
    /// Only allow URLs whose origin matches one of `outbound_allowlist`.
    Allowlist,
}

/// Security-related proxy configuration.
///
/// ```toml
/// [security]
/// outbound_policy = "deny_private"   # default; also "off" or "allowlist"
/// outbound_allowlist = ["https://api.openai.com", "https://api.anthropic.com"]
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    /// Which outbound policy to enforce (default: `deny_private`).
    #[serde(default)]
    pub outbound_policy: OutboundPolicyKind,
    /// Allowed origins when `outbound_policy = "allowlist"`.
    ///
    /// Each entry must be a valid URL whose scheme, host, and port define the
    /// permitted origin.  Path components are ignored.
    #[serde(default)]
    pub outbound_allowlist: Vec<String>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml_fragment: &str) -> Result<SecurityConfig, String> {
        toml::from_str(toml_fragment).map_err(|e| e.to_string())
    }

    #[test]
    fn default_policy_is_deny_private() {
        let cfg = SecurityConfig::default();
        assert_eq!(cfg.outbound_policy, OutboundPolicyKind::DenyPrivate);
        assert!(cfg.outbound_allowlist.is_empty());
    }

    #[test]
    fn empty_section_defaults_to_deny_private() {
        let cfg = parse("").unwrap();
        assert_eq!(cfg.outbound_policy, OutboundPolicyKind::DenyPrivate);
    }

    #[test]
    fn off_policy_parses() {
        let cfg = parse(r#"outbound_policy = "off""#).unwrap();
        assert_eq!(cfg.outbound_policy, OutboundPolicyKind::Off);
    }

    #[test]
    fn allowlist_policy_with_hosts_parses() {
        let toml = r#"
outbound_policy = "allowlist"
outbound_allowlist = ["https://api.openai.com", "https://api.anthropic.com"]
"#;
        let cfg = parse(toml).unwrap();
        assert_eq!(cfg.outbound_policy, OutboundPolicyKind::Allowlist);
        assert_eq!(cfg.outbound_allowlist.len(), 2);
        assert_eq!(cfg.outbound_allowlist[0], "https://api.openai.com");
        assert_eq!(cfg.outbound_allowlist[1], "https://api.anthropic.com");
    }

    #[test]
    fn deny_private_explicit_parses() {
        let cfg = parse(r#"outbound_policy = "deny_private""#).unwrap();
        assert_eq!(cfg.outbound_policy, OutboundPolicyKind::DenyPrivate);
    }

    #[test]
    fn unknown_field_is_rejected() {
        assert!(parse("bogus = true").is_err());
    }
}
