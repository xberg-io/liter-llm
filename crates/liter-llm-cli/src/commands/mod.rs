pub mod api;
pub mod config_check;
pub mod mcp;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Start the OpenAI-compatible proxy server.
    Api(api::ApiArgs),
    /// Start the MCP server exposing LLM operations as tools.
    Mcp(mcp::McpArgs),
    /// Validate a proxy config file without starting the server.
    ///
    /// Prints one JSON line (`{"valid": true}`, or `{"valid": false, "code": ..., "message": ...}`)
    /// and exits non-zero when the config is invalid or unreadable.
    ConfigCheck(config_check::ConfigCheckArgs),
}
