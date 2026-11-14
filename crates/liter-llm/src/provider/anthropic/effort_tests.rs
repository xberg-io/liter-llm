//! `reasoning_effort` per Claude model family (#473): model id parsing, effort levels each
//! model accepts, `max_tokens` headroom and beta headers.

use serde_json::{Value, json};

use super::request::ClaudeModel;
use super::*;

use super::tests::provider;

fn transformed(model: &str, effort: &str, max_tokens: Option<u64>) -> Value {
    let mut body = json!({
        "model": model,
        "messages": [{"role": "user", "content": "Think"}],
        "reasoning_effort": effort
    });
    if let Some(max_tokens) = max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    provider()
        .transform_request(&mut body)
        .expect("transform_request should not fail");
    body
}

#[test]
fn parse_reads_family_and_version_from_every_id_form() {
    for (model, family, version) in [
        ("anthropic/claude-opus-4-6", "opus", (4, 6)),
        ("claude-opus-4-6@20260101", "opus", (4, 6)),
        ("claude-opus-4-6[1m]", "opus", (4, 6)),
        ("claude-opus-4-6-latest", "opus", (4, 6)),
        ("anthropic.claude-sonnet-5-5:0", "sonnet", (5, 5)),
        ("claude-sonnet-5-5-20261001", "sonnet", (5, 5)),
        ("claude-opus-4-20250514", "opus", (4, 0)),
        ("claude-opus-5-latest", "opus", (5, 0)),
        ("claude-opus-5@20260101", "opus", (5, 0)),
    ] {
        let parsed = ClaudeModel::parse(model).unwrap_or_else(|| panic!("{model} should parse"));
        assert_eq!((parsed.family, parsed.version), (family, version), "{model}");
    }
    assert!(ClaudeModel::parse("claude-3-7-sonnet-20250219").is_none());
    assert!(ClaudeModel::parse("my-proxy-alias").is_none());
}

#[test]
fn ids_with_suffixes_get_adaptive_thinking() {
    for model in [
        "claude-opus-4-6@20260101",
        "claude-opus-4-6[1m]",
        "anthropic.claude-sonnet-5-5:0",
    ] {
        assert_eq!(
            transformed(model, "high", None)["thinking"],
            json!({"type": "adaptive"}),
            "{model}"
        );
    }
}

#[test]
fn an_unknown_family_from_claude_5_on_gets_adaptive_thinking() {
    assert_eq!(
        transformed("claude-nova-5", "high", None)["thinking"],
        json!({"type": "adaptive"})
    );
    assert_eq!(
        transformed("claude-nova-4-5", "high", None)["thinking"],
        json!({"type": "enabled", "budget_tokens": 16384})
    );
}

#[test]
fn effort_levels_map_to_the_levels_each_model_accepts() {
    for (model, effort, expected) in [
        ("claude-opus-4-6", "xhigh", "max"),
        ("claude-sonnet-4-6", "xhigh", "max"),
        ("claude-opus-5-5", "xhigh", "xhigh"),
        ("claude-haiku-5-5", "minimal", "low"),
        ("claude-opus-5-5", "turbo", "medium"),
    ] {
        assert_eq!(
            transformed(model, effort, Some(100))["output_config"]["effort"],
            expected,
            "{model} {effort}"
        );
    }
}

#[test]
fn xhigh_on_a_budget_model_gets_a_budget_between_high_and_max() {
    let budget = |effort| transformed("claude-haiku-4-5", effort, None)["thinking"]["budget_tokens"].clone();
    let (high, xhigh, max) = (budget("high"), budget("xhigh"), budget("max"));
    assert!(
        high.as_u64() < xhigh.as_u64() && xhigh.as_u64() < max.as_u64(),
        "{high} {xhigh} {max}"
    );
}

#[test]
fn adaptive_thinking_gets_max_tokens_headroom_unless_the_caller_set_it() {
    for (effort, floor) in [("medium", 16_000), ("high", 16_000), ("xhigh", 64_000), ("max", 64_000)] {
        let max_tokens = transformed("claude-nova-5", effort, None)["max_tokens"].as_u64();
        assert!(max_tokens >= Some(floor), "{effort}: {max_tokens:?}");
    }
    assert_eq!(transformed("claude-nova-5", "max", Some(100))["max_tokens"], 100);
}

#[test]
fn adaptive_and_between_tools_thinking_send_no_thinking_beta_header() {
    for (model, effort) in [("claude-opus-5-5", "high"), ("claude-sonnet-5-5", "none")] {
        let body = transformed(model, effort, None);
        let headers = provider().dynamic_headers(&body);
        assert!(
            headers.iter().all(|(_, value)| !value.contains("thinking-2025-04-14")),
            "{model}: {headers:?}"
        );
    }
}

#[test]
fn parse_rejects_a_date_in_the_major_position() {
    assert!(ClaudeModel::parse("claude-sonnet-20250514").is_none());
    assert_eq!(
        transformed("claude-sonnet-20250514", "high", None)["thinking"],
        json!({"type": "enabled", "budget_tokens": 16384})
    );
}

#[test]
fn effort_levels_are_case_insensitive() {
    assert_eq!(
        transformed("claude-opus-5-5", "High", Some(100))["output_config"]["effort"],
        "high"
    );
    assert_eq!(
        transformed("claude-opus-4-6", "XHIGH", Some(100))["output_config"]["effort"],
        "max"
    );
    assert_eq!(
        transformed("claude-haiku-4-5", "Max", None)["thinking"]["budget_tokens"],
        32768
    );
    assert_eq!(
        transformed("claude-sonnet-5-5", "None", None)["thinking"],
        json!({"type": "between_tools"})
    );
}

#[test]
fn the_adaptive_max_tokens_floor_never_exceeds_the_model_output_limit() {
    assert_eq!(super::request::adaptive_max_tokens_floor("max", Some(32_000)), 32_000);
    assert_eq!(super::request::adaptive_max_tokens_floor("max", Some(128_000)), 64_000);
    assert_eq!(super::request::adaptive_max_tokens_floor("high", None), 16_000);
    assert_eq!(super::request::adaptive_max_tokens_floor("xhigh", None), 64_000);
}
