//! `liter-llm config-check` prints one JSON line and signals validity through
//! its exit status, without binding a listener.

use std::process::Command;

use serde_json::{Value, json};

fn config_check(args: &[&str]) -> (Option<i32>, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_liter-llm"))
        .arg("config-check")
        .args(args)
        .output()
        .expect("run liter-llm");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert_eq!(
        stdout.lines().count(),
        1,
        "exactly one JSON line expected, got {stdout:?}"
    );
    let report = serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    (output.status.code(), report)
}

#[test]
fn valid_config_exits_zero() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("proxy.toml");
    std::fs::write(&path, "[general]\nmaster_key = \"sk-test\"\n").expect("write config");

    let (code, report) = config_check(&["--config", path.to_str().expect("UTF-8 path")]);
    assert_eq!(code, Some(0));
    assert_eq!(report, json!({ "valid": true }));
}

#[test]
fn invalid_config_exits_non_zero_without_echoing_values() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("proxy.toml");
    std::fs::write(&path, "[general]\nmaster_key = \"sk-inline-secret\"\nbogus = true\n").expect("write config");

    let (code, report) = config_check(&["--config", path.to_str().expect("UTF-8 path")]);
    assert_eq!(code, Some(1));
    assert_eq!(report["valid"], false);
    assert_eq!(report["code"], "invalid_config");
    assert!(!report.to_string().contains("sk-inline-secret"), "got {report}");
}

#[test]
fn unreadable_config_reports_read_failed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing.toml");

    let (code, report) = config_check(&["--config", missing.to_str().expect("UTF-8 path")]);
    assert_eq!(code, Some(1));
    assert_eq!(report["code"], "read_failed");
}
