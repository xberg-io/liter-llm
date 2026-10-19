use std::path::PathBuf;

use clap::Args;
use liter_llm_proxy::config::ProxyConfig;
use serde_json::{Value, json};

#[derive(Args)]
pub struct ConfigCheckArgs {
    /// Path to the proxy config file to validate.
    #[arg(long, short)]
    pub config: PathBuf,
}

/// Validate a proxy config without binding a listener, printing one JSON line.
///
/// Exits non-zero when the config is invalid or unreadable, so the command can
/// gate CI, container entrypoints and init steps.
pub fn run(args: ConfigCheckArgs) -> Result<(), String> {
    let report = check(&args.config);
    // ~keep The JSON line on stdout is the command's machine-readable result, not a diagnostic.
    #[expect(clippy::print_stdout, reason = "config-check prints its JSON result to stdout")]
    {
        println!("{report}");
    }
    if report["valid"] == Value::Bool(true) {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

/// Check the config at `path` and describe the outcome.
///
/// Only the first line of an error is reported: later lines of a TOML parse
/// error quote the offending source, which can hold an inline or interpolated
/// secret. A read failure reports only the I/O error kind.
fn check(path: &std::path::Path) -> Value {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => return invalid("read_failed", &e.kind().to_string()),
    };
    let result =
        ProxyConfig::from_toml_str(&content).and_then(|config| super::api::build_outbound_policy(&config).map(drop));
    match result {
        Ok(()) => json!({ "valid": true }),
        Err(message) => invalid("invalid_config", &message),
    }
}

fn invalid(code: &str, message: &str) -> Value {
    let first_line = message.lines().next().unwrap_or_default().trim();
    json!({ "valid": false, "code": code, "message": first_line })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_toml(toml: &str) -> Value {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("liter-llm-proxy.toml");
        std::fs::write(&path, toml).expect("write config");
        check(&path)
    }

    #[test]
    fn valid_config_reports_valid() {
        let report = check_toml("[general]\nmaster_key = \"sk-test\"\n");
        assert_eq!(report, json!({ "valid": true }));
    }

    #[test]
    fn parse_error_reports_only_its_first_line() {
        let report = check_toml("[general]\nmaster_key = \"sk-inline-secret\"\nnot_a_field = 1\n");
        assert_eq!(report["valid"], false);
        assert_eq!(report["code"], "invalid_config");
        let message = report["message"].as_str().expect("message string");
        assert!(!message.contains('\n'), "message must be a single line: {message}");
        assert!(
            !message.contains("sk-inline-secret"),
            "config values must never be echoed: {message}"
        );
    }

    #[test]
    fn invalid_outbound_allowlist_is_rejected() {
        let report = check_toml("[security]\noutbound_policy = \"allowlist\"\n");
        assert_eq!(report["valid"], false);
        assert_eq!(report["code"], "invalid_config");
    }

    #[test]
    fn missing_file_reports_read_failed() {
        let report = check(std::path::Path::new("/nonexistent/liter-llm-proxy.toml"));
        assert_eq!(
            report,
            json!({ "valid": false, "code": "read_failed", "message": "entity not found" })
        );
    }
}
