use serde::Deserialize;

/// MCP transport configuration.
///
/// Controls authentication behaviour for the `mcp` CLI sub-command.
///
/// For HTTP transports (`--transport http`) auth is always enforced through
/// the `validate_api_key` axum middleware — this section is ignored.
///
/// For the stdio transport a KeyContext must be established at startup time
/// because there is no per-request auth header:
///
/// * `stdio_key_id` — restrict stdio to an existing virtual key.
/// * `stdio_trust_local` — treat the stdio process as master-key access
///   (use only in fully trusted local environments).
///
/// Exactly one of the two options must be set; refusing to start otherwise
/// is intentional — running an MCP server with no context is a security
/// misconfiguration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpConfig {
    /// Trust the stdio transport as if it were the master key.
    ///
    /// Only set this to `true` in fully trusted, local-only environments.
    #[serde(default)]
    pub stdio_trust_local: bool,

    /// Use a specific virtual key as the default context for the stdio
    /// transport.  The key must exist in `[[keys]]`.
    pub stdio_key_id: Option<String>,
}
