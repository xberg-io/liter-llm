//! Integration tests for the guardrail plugin system.
//!
//! These tests exercise cross-module behaviour: the registry + built-in
//! primitives together, and the Tower layer wrapping a mock client.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use regex::Regex;

use super::builtin::{AllowListGuardrail, DenyListGuardrail, OnMatch, PromptInjectionHeuristic, RegexGuardrail};
use super::registry::GuardrailRegistry;
use super::{GuardrailContext, GuardrailDecision, GuardrailStage};

fn empty_meta() -> HashMap<String, String> {
    HashMap::new()
}

fn meta_with(key: &str, val: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(key.to_string(), val.to_string());
    m
}

fn null_request() -> serde_json::Value {
    serde_json::Value::Null
}

fn chat_request(model: &str) -> serde_json::Value {
    serde_json::json!({ "model": model, "messages": [{"role": "user", "content": "hello"}] })
}

fn make_ctx<'a>(
    request: &'a serde_json::Value,
    response: Option<&'a serde_json::Value>,
    chunk: Option<&'a str>,
    meta: &'a HashMap<String, String>,
) -> GuardrailContext<'a> {
    GuardrailContext {
        request,
        response,
        chunk,
        metadata: meta,
    }
}

// ── Registry ordering and short-circuit ───────────────────────────────────────

#[tokio::test]
async fn registry_ordering_first_block_wins() {
    let mut registry = GuardrailRegistry::new();

    // Guard 1: deny-list (will block "banned")
    let deny_list: HashSet<String> = ["banned"].iter().map(|s| s.to_string()).collect();
    registry.register(Arc::new(DenyListGuardrail::new("deny-1", deny_list, "user_id")));

    // Guard 2: allow-list (would also block, but never reached)
    let allow_list: HashSet<String> = ["premium"].iter().map(|s| s.to_string()).collect();
    registry.register(Arc::new(AllowListGuardrail::new("allow-1", allow_list, "tier")));

    let req = null_request();
    let meta = meta_with("user_id", "banned");
    let ctx = make_ctx(&req, None, None, &meta);
    let decision = registry.run_stage(GuardrailStage::Input, &ctx).await;

    match decision {
        GuardrailDecision::Block { code, .. } => {
            // 1003 = DenyListGuardrail code; confirms first guardrail blocked.
            assert_eq!(code, 1003, "deny-list should be the one that blocked");
        }
        other => panic!("expected Block, got {other:?}"),
    }
}

#[tokio::test]
async fn registry_all_pass_returns_allow() {
    let mut registry = GuardrailRegistry::new();
    registry.register(Arc::new(PromptInjectionHeuristic::new("inj")));
    registry.register(Arc::new(PromptInjectionHeuristic::new("inj2")));

    let req = chat_request("gpt-4");
    let meta = empty_meta();
    let ctx = make_ctx(&req, None, None, &meta);
    let decision = registry.run_stage(GuardrailStage::Input, &ctx).await;
    assert!(decision.is_allow(), "benign prompt should pass all guardrails");
}

#[tokio::test]
async fn registry_skips_wrong_stage_guardrails() {
    let mut registry = GuardrailRegistry::new();
    // DenyListGuardrail only runs at Input. At Output it should be skipped.
    let deny_list: HashSet<String> = ["banned"].iter().map(|s| s.to_string()).collect();
    registry.register(Arc::new(DenyListGuardrail::new("deny", deny_list, "user_id")));

    let req = null_request();
    let meta = meta_with("user_id", "banned");
    let ctx = make_ctx(&req, None, None, &meta);

    // At Output stage — deny-list doesn't run — should Allow.
    let decision = registry.run_stage(GuardrailStage::Output, &ctx).await;
    assert!(decision.is_allow(), "deny-list only applies to Input stage");
}

// ── Built-in primitives integration ───────────────────────────────────────────

#[tokio::test]
async fn regex_guardrail_and_deny_list_compose_correctly() {
    static STAGES: &[GuardrailStage] = &[GuardrailStage::Input];
    let pattern = Regex::new(r"(?i)DROP TABLE").unwrap();
    let mut registry = GuardrailRegistry::new();
    registry.register(Arc::new(RegexGuardrail::new(
        "sql-injection",
        pattern,
        OnMatch::Block {
            code: 1100,
            reason_prefix: "SQL injection".to_string(),
        },
        STAGES,
    )));

    // Benign prompt — regex doesn't fire.
    let req = chat_request("gpt-4");
    let meta = empty_meta();
    let ctx = make_ctx(&req, None, None, &meta);
    assert!(registry.run_stage(GuardrailStage::Input, &ctx).await.is_allow());

    // Injection attempt — regex fires.
    let bad_req = serde_json::json!({
        "model": "gpt-4",
        "messages": [{ "role": "user", "content": "DROP TABLE users;" }]
    });
    let ctx2 = make_ctx(&bad_req, None, None, &meta);
    let decision = registry.run_stage(GuardrailStage::Input, &ctx2).await;
    assert!(decision.is_block(), "SQL injection should be blocked");
}

// ── Tower layer integration ───────────────────────────────────────────────────

#[cfg(feature = "tower")]
mod tower_integration {
    use std::collections::HashMap;
    use std::sync::Arc;

    use regex::Regex;
    use tower::{Layer, Service};

    use super::*;
    use crate::guardrail::builtin::{OnMatch, RegexGuardrail};
    use crate::guardrail::registry::GuardrailRegistry;
    use crate::tower::guardrail::GuardrailLayer;
    use crate::tower::service::LlmService;
    use crate::tower::tests_common::{MockClient, chat_req};
    use crate::tower::types::LlmRequest;
    use crate::error::LiterLlmError;

    #[allow(dead_code)]
    static INPUT_STAGES: &[GuardrailStage] = &[GuardrailStage::Input];

    #[tokio::test]
    async fn guardrail_layer_input_block_short_circuits_inner_service() {
        // Build a registry with a deny-list.
        let mut registry = GuardrailRegistry::new();
        let deny_list: std::collections::HashSet<String> = ["banned"].iter().map(|s| s.to_string()).collect();
        registry.register(Arc::new(DenyListGuardrail::new("deny", deny_list, "user_id")));

        let mock = MockClient::ok();
        let call_count = Arc::clone(&mock.call_count);
        let inner = LlmService::new(mock);

        let mut meta = HashMap::new();
        meta.insert("user_id".to_string(), "banned".to_string());

        let mut svc = GuardrailLayer::new(Arc::new(registry), meta).layer(inner);
        let err = svc
            .call(LlmRequest::Chat(chat_req("gpt-4")))
            .await
            .expect_err("should be blocked by guardrail");

        assert!(matches!(err, LiterLlmError::HookRejected { .. }), "guardrail block should map to HookRejected");
        // Inner service must NOT have been called.
        use std::sync::atomic::Ordering;
        assert_eq!(call_count.load(Ordering::SeqCst), 0, "inner service should not be called");
    }

    #[tokio::test]
    async fn guardrail_layer_allows_clean_request() {
        let registry = GuardrailRegistry::new(); // Empty — everything passes.
        let mock = MockClient::ok();
        let inner = LlmService::new(mock);
        let meta = HashMap::new();

        let mut svc = GuardrailLayer::new(Arc::new(registry), meta).layer(inner);
        let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(result.is_ok(), "empty registry should allow all requests");
    }

    #[tokio::test]
    async fn guardrail_layer_output_block_returns_error() {
        // Block responses that contain the word "confidential" in the output.
        static STAGES: &[GuardrailStage] = &[GuardrailStage::Output];
        let pattern = Regex::new(r"(?i)confidential").unwrap();
        let mut registry = GuardrailRegistry::new();
        registry.register(Arc::new(RegexGuardrail::new(
            "no-confidential-output",
            pattern,
            OnMatch::Block {
                code: 1200,
                reason_prefix: "confidential leak".to_string(),
            },
            STAGES,
        )));

        // MockClient::ok() returns "Hello!" as the response content,
        // which does not contain "confidential" — so this should pass.
        let mock = MockClient::ok();
        let inner = LlmService::new(mock);
        let meta = HashMap::new();

        let mut svc = GuardrailLayer::new(Arc::new(registry), meta).layer(inner);
        // Should succeed because "Hello!" is not confidential.
        let result = svc.call(LlmRequest::Chat(chat_req("gpt-4"))).await;
        assert!(result.is_ok(), "non-confidential response should pass output guardrail");
    }
}
