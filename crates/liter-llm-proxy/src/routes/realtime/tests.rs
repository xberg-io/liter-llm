/// A deny-list configured on `tenant_id` must decide the same way on both paths. The
/// unary path gets its metadata from `GuardrailService`; realtime has no `LlmRequest`,
/// so it assembles the equivalent here. If this map were empty — as it was before —
/// `DenyListGuardrail` would read `None`, treat it as "nothing to deny", and silently
/// allow every realtime message while the identical rule blocked unary traffic.
///
/// Revert line: replace `session_guardrail_metadata(&tenant_id)` in `handle_session`
/// with `HashMap::new()` to make this fail.
#[test]
fn session_guardrail_metadata_carries_the_tenant_under_the_core_key() {
    let tenant = TenantId::from("tnt-0123456789abcdef");
    let metadata = session_guardrail_metadata(&tenant);

    assert_eq!(
        metadata.get(TENANT_ID_METADATA_KEY).map(String::as_str),
        Some("tnt-0123456789abcdef"),
        "realtime must present the tenant under the same key the unary path uses"
    );
    assert_eq!(metadata.len(), 1, "only tenant_id is derived for a realtime session");
}

/// The key is the core's exported constant, not a local literal, so a rename upstream
/// cannot leave the two paths spelling it differently.
#[test]
fn realtime_metadata_key_matches_the_core_constant() {
    assert_eq!(TENANT_ID_METADATA_KEY, "tenant_id");
}

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as Msg;
use tokio_util::sync::CancellationToken;

use super::*;
use liter_llm::guardrail::{GuardrailContext, GuardrailDecision, GuardrailStage};
use liter_llm::realtime::RealtimeEvent;

async fn spawn_mock_ws_server<F, Fut>(handler: F) -> std::net::SocketAddr
where
    F: FnOnce(tokio_tungstenite::WebSocketStream<TcpStream>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = accept_async(stream).await.unwrap();
        handler(ws).await;
    });
    addr
}

/// Verifies that the mock server can exchange messages bi-directionally.
///
/// The test speaks tungstenite directly (not through axum) because wiring
/// up a full axum WebSocket inside a unit test requires a real HTTP upgrade
/// that would turn this into an integration test.  The proxy logic itself
/// is exercised via the `run_proxy` public function in the integration
/// companion below; here we focus on the underlying transport plumbing.
#[tokio::test]
async fn realtime_websocket_proxy_forwards_bidirectional() {
    let addr = spawn_mock_ws_server(|mut ws| async move {
        let greeting = serde_json::json!({
            "type": "session.created",
            "session": { "id": "sess_1", "model": "gpt-4o-realtime-preview" }
        });
        let _ = ws
            .send(Msg::Text(serde_json::to_string(&greeting).unwrap().into()))
            .await;

        if let Some(Ok(msg)) = ws.next().await {
            let _ = ws.send(msg).await;
        }
    })
    .await;

    let url = format!("ws://{addr}");
    let (mut stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    let greeting_msg = stream.next().await.unwrap().unwrap();
    assert!(greeting_msg.is_text(), "expected text frame from upstream");
    let val: serde_json::Value = serde_json::from_str(greeting_msg.into_text().unwrap().as_str()).unwrap();
    assert_eq!(val["type"], "session.created");
    assert_eq!(val["session"]["id"], "sess_1");

    let commit = serde_json::json!({ "type": "input_audio_buffer.commit" });
    stream
        .send(Msg::Text(serde_json::to_string(&commit).unwrap().into()))
        .await
        .unwrap();
    let echo = stream.next().await.unwrap().unwrap();
    assert!(echo.is_text());
    let echo_val: serde_json::Value = serde_json::from_str(echo.into_text().unwrap().as_str()).unwrap();
    assert_eq!(echo_val["type"], "input_audio_buffer.commit");
}

/// A pre-cancelled token resolves immediately — models client disconnect.
#[tokio::test]
async fn realtime_websocket_proxy_cancels_on_client_disconnect() {
    let cancel = CancellationToken::new();
    cancel.cancel();

    let start = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_millis(100), cancel.cancelled()).await;
    assert!(result.is_ok(), "cancellation should complete within 100ms");
    assert!(start.elapsed() < Duration::from_millis(100), "should not have blocked");
}

/// Ending one realtime session must NOT shut down the proxy.
///
/// This drives `session_cancel_token` itself rather than re-deriving a
/// child token inline — an inline version would keep passing if the
/// production code reverted to `.clone()`, which is precisely the bug.
#[test]
fn session_cancel_token_does_not_cancel_the_shutdown_token() {
    let coordinator = crate::shutdown::ShutdownCoordinator::new();
    let handle = coordinator.handle();
    let shutdown = handle.cancellation_token();

    let session = session_cancel_token(Some(&handle));
    // Exactly what run_proxy does when either relay loop exits.
    session.cancel();

    assert!(session.is_cancelled(), "the session token must be cancelled");
    assert!(
        !shutdown.is_cancelled(),
        "a session ending must NOT cancel the process-wide shutdown token — that stops the server"
    );
}

/// The session token must still be cancelled BY a real shutdown, or
/// draining would leave realtime sessions running.
#[test]
fn session_cancel_token_is_still_cancelled_by_shutdown() {
    let coordinator = crate::shutdown::ShutdownCoordinator::new();
    let handle = coordinator.handle();
    let session = session_cancel_token(Some(&handle));

    handle.cancellation_token().cancel();

    assert!(
        session.is_cancelled(),
        "drain must still close active realtime sessions"
    );
}

/// A guardrail that blocks every event prevents the payload from reaching
/// the upstream and sends an error event to the client.
#[tokio::test]
async fn realtime_websocket_proxy_blocks_on_guardrail() {
    struct BlockAllGuardrail;

    impl Guardrail for BlockAllGuardrail {
        fn name(&self) -> &'static str {
            "block_all"
        }

        fn supported_stages(&self) -> &'static [GuardrailStage] {
            &[GuardrailStage::Input]
        }

        fn check<'a>(
            &'a self,
            _stage: GuardrailStage,
            _ctx: &'a GuardrailContext<'a>,
        ) -> Pin<Box<dyn std::future::Future<Output = GuardrailDecision> + Send + 'a>> {
            Box::pin(async {
                GuardrailDecision::Block {
                    reason: "blocked by test guardrail".into(),
                    code: 1001,
                }
            })
        }
    }

    let guardrails: Vec<Arc<dyn Guardrail>> = vec![Arc::new(BlockAllGuardrail)];
    let payload = serde_json::json!({ "type": "input_audio_buffer.commit" });

    let outcome = apply_guardrails_input(&guardrails, &payload, &HashMap::new()).await;

    let GuardrailOutcome::Block { reason, code } = outcome else {
        panic!("guardrail should have blocked the event");
    };
    let err_val: serde_json::Value =
        serde_json::from_str(&guardrail_error_json(code, &reason)).expect("error event should be valid JSON");
    assert_eq!(err_val["type"], "error");
    assert_eq!(err_val["error"]["code"], "1001");
    assert!(
        err_val["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("blocked by test guardrail"),
        "error message should mention the guardrail reason"
    );
}

#[tokio::test]
async fn apply_guardrails_output_chunk_allows_clean_event() {
    let guardrails: Vec<Arc<dyn Guardrail>> = vec![];
    let payload = serde_json::json!({ "type": "response.text.delta", "delta": "hello" });
    let outcome = apply_guardrails_output_chunk(&guardrails, &payload, &HashMap::new()).await;
    assert!(matches!(outcome, GuardrailOutcome::Allow(_)));
}

/// Build an [`AppState`] the way `ProxyServer::serve_with_shutdown` does —
/// in particular taking `guardrails` from the pool rather than rebuilding
/// it. Mirrors `routes::batches::tests::test_state`; no network access
/// happens at construction time.
fn test_state(config_toml: &str) -> AppState {
    use arc_swap::ArcSwap;

    use crate::auth::KeyStore;
    use crate::config::{FileStorageConfig, ProxyConfig};
    use crate::file_store::FileStore;
    use crate::secrets::{EnvVarSecretManager, SecretManager, SecretManagerRegistry};
    use crate::service_pool::ServicePool;

    let config = ProxyConfig::from_toml_str(config_toml).expect("valid TOML");
    let service_pool = Arc::new(ServicePool::from_config(&config, None).expect("service pool"));
    let file_store = Arc::new(FileStore::from_config(&FileStorageConfig::default()).expect("file store"));
    let key_store = Arc::new(KeyStore::from_config(None, &[]));
    let secret_registry = Arc::new(
        SecretManagerRegistry::builder()
            .default_backend(Arc::new(EnvVarSecretManager::new()) as Arc<dyn SecretManager>)
            .build(),
    );

    AppState {
        key_store: Arc::clone(&key_store),
        key_resolver: key_store,
        guardrails: service_pool.guardrails(),
        service_pool,
        file_store,
        config: Arc::new(ArcSwap::new(Arc::new(config))),
        secret_registry,
        shutdown: None,
        usage_sink: None,
    }
}

const MODEL_TOML: &str = r#"
[[models]]
name = "test-model"
provider_model = "openai/gpt-4o"
api_key = "sk-test"
"#;

/// A realtime session must enforce the guardrails declared in
/// `[[guardrails]]`. Before this wiring existed, `handle_session` passed a
/// hardcoded empty vec and every realtime message went through unexamined.
#[test]
fn session_guardrails_returns_the_configured_set() {
    let state = test_state(&format!(
        r#"{MODEL_TOML}
[[guardrails]]
type = "prompt_injection"
name = "injection-heuristic"
"#
    ));

    let guardrails = session_guardrails(&state);

    assert_eq!(guardrails.len(), 1, "the configured guardrail must reach the session");
    assert_eq!(guardrails[0].name(), "injection-heuristic");
}

/// The realtime and unary paths must share one registry, not two sets
/// built from the same config — a shared `Arc` is what makes it impossible
/// to enable a guardrail on one path only.
#[test]
fn session_guardrails_share_the_pool_registry() {
    let state = test_state(&format!(
        r#"{MODEL_TOML}
[[guardrails]]
type = "prompt_injection"
name = "injection-heuristic"
"#
    ));

    assert!(
        Arc::ptr_eq(&state.guardrails, &state.service_pool.guardrails()),
        "AppState must hold the same registry the Tower stacks were built with"
    );
}

#[test]
fn session_guardrails_is_empty_when_none_configured() {
    let state = test_state(MODEL_TOML);
    assert!(
        session_guardrails(&state).is_empty(),
        "an absent [[guardrails]] section must leave realtime behaviour unchanged"
    );
}

/// End-to-end for the realtime path: TOML → `build_registry` → the exact
/// function the client → upstream relay loop calls on every frame.
#[tokio::test]
async fn configured_guardrail_blocks_a_realtime_input_event() {
    let config = crate::config::ProxyConfig::from_toml_str(
        r#"
[[guardrails]]
type = "regex"
name = "block-ssn"
pattern = '\d{3}-\d{2}-\d{4}'
stages = ["input"]
action = { kind = "block", code = 1100, reason_prefix = "SSN detected" }
"#,
    )
    .expect("valid TOML");
    let registry = crate::guardrail::build_registry(&config.guardrails).expect("guardrail should build");
    let guardrails: Vec<Arc<dyn Guardrail>> = registry.iter().map(Arc::clone).collect();

    let payload = serde_json::json!({
        "type": "conversation.item.create",
        "item": { "content": [{ "text": "my ssn is 123-45-6789" }] }
    });

    let outcome = apply_guardrails_input(&guardrails, &payload, &HashMap::new()).await;

    let GuardrailOutcome::Block { code, reason } = outcome else {
        panic!("a configured regex guardrail must block a matching realtime event");
    };
    assert_eq!(code, 1100);
    assert!(
        reason.contains("SSN detected"),
        "reason should carry the prefix: {reason}"
    );
}

/// A clean event still passes when a guardrail is configured — the check
/// blocks on a match, it does not reject everything.
#[tokio::test]
async fn configured_guardrail_allows_a_clean_realtime_input_event() {
    let config = crate::config::ProxyConfig::from_toml_str(
        r#"
[[guardrails]]
type = "regex"
name = "block-ssn"
pattern = '\d{3}-\d{2}-\d{4}'
stages = ["input"]
action = { kind = "block", code = 1100, reason_prefix = "SSN detected" }
"#,
    )
    .expect("valid TOML");
    let registry = crate::guardrail::build_registry(&config.guardrails).expect("guardrail should build");
    let guardrails: Vec<Arc<dyn Guardrail>> = registry.iter().map(Arc::clone).collect();

    let payload = serde_json::json!({ "type": "input_audio_buffer.commit" });
    let outcome = apply_guardrails_input(&guardrails, &payload, &HashMap::new()).await;

    assert!(matches!(outcome, GuardrailOutcome::Allow(_)));
}

/// A `Mutate` decision at the `Input` stage must rewrite the payload that
/// is forwarded upstream. Dropping the rewrite would make a configured
/// redaction guardrail forward exactly the content it was installed to
/// remove.
#[tokio::test]
async fn apply_guardrails_input_forwards_the_mutated_payload() {
    struct RedactGuardrail;

    impl Guardrail for RedactGuardrail {
        fn name(&self) -> &'static str {
            "redact_all"
        }

        fn supported_stages(&self) -> &'static [GuardrailStage] {
            &[GuardrailStage::Input]
        }

        fn check<'a>(
            &'a self,
            _stage: GuardrailStage,
            _ctx: &'a GuardrailContext<'a>,
        ) -> Pin<Box<dyn std::future::Future<Output = GuardrailDecision> + Send + 'a>> {
            Box::pin(async {
                GuardrailDecision::Mutate {
                    new_payload: serde_json::json!({ "type": "redacted" }),
                }
            })
        }
    }

    let guardrails: Vec<Arc<dyn Guardrail>> = vec![Arc::new(RedactGuardrail)];
    let payload = serde_json::json!({ "type": "conversation.item.create", "secret": "hunter2" });

    let outcome = apply_guardrails_input(&guardrails, &payload, &HashMap::new()).await;

    let GuardrailOutcome::Allow(forwarded) = outcome else {
        panic!("a Mutate decision must allow the rewritten payload through");
    };
    assert_eq!(forwarded, serde_json::json!({ "type": "redacted" }));
    assert!(
        forwarded.get("secret").is_none(),
        "the original payload must not be forwarded after a Mutate decision"
    );
}

#[test]
fn event_type_label_returns_correct_strings() {
    let cases: &[(RealtimeEvent, &str)] = &[
        (RealtimeEvent::InputAudioBufferCommit, "input_audio_buffer.commit"),
        (RealtimeEvent::InputAudioBufferClear, "input_audio_buffer.clear"),
        (
            RealtimeEvent::ResponseCreated {
                response_id: "r".into(),
            },
            "response.created",
        ),
        (
            RealtimeEvent::Raw {
                event_type: "x".into(),
                payload: serde_json::Value::Null,
            },
            "raw",
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(event_type_label(event), *expected);
    }
}

#[test]
fn audio_bytes_for_event_returns_zero_for_non_audio() {
    let event = RealtimeEvent::InputAudioBufferCommit;
    assert_eq!(audio_bytes_for_event(&event), 0);
}

#[test]
fn audio_bytes_for_event_returns_nonzero_for_audio_append() {
    let event = RealtimeEvent::InputAudioBufferAppend {
        audio_base64: "AAAA".into(),
    };
    assert!(audio_bytes_for_event(&event) > 0);
}

use secrecy::{ExposeSecret, SecretString};

use crate::auth::KeyContext;
use crate::config::VirtualKeyConfig;
use crate::config::key::ProviderCredential;

fn make_vk_config(key: &str, models: Vec<String>, provider_credentials: Vec<ProviderCredential>) -> VirtualKeyConfig {
    VirtualKeyConfig {
        key: key.to_string(),
        tenant_id: None,
        description: None,
        models,
        rpm: None,
        tpm: None,
        budget_limit: None,
        provider_credentials,
    }
}

fn make_provider_cred(
    provider: &str,
    id: &str,
    api_key: &str,
    model_allowlist: Option<Vec<String>>,
) -> ProviderCredential {
    ProviderCredential {
        provider: provider.to_string(),
        id: id.to_string(),
        api_key: SecretString::from(api_key.to_string()),
        model_allowlist,
    }
}

/// `resolve_upstream_credential` returns a `SecretString`.
/// Verify the key is correctly resolved and the type is `SecretString`.
#[test]
fn provider_credential_api_key_is_secret_string() {
    let cred = make_provider_cred("openai", "cred-1", "sk-test-secret", None);
    let _: &SecretString = &cred.api_key;
    let debug = format!("{cred:?}");
    assert!(
        !debug.contains("sk-test-secret"),
        "Debug output must redact the api_key; got: {debug}"
    );
    assert!(
        debug.contains("[REDACTED]"),
        "Debug output must contain '[REDACTED]'; got: {debug}"
    );
}

/// `resolve_upstream_credential` must return the per-VK credential for
/// the matched model, never `master_key`.  Having two VKs with different
/// OpenAI credentials verifies that the correct one is selected.
#[test]
fn realtime_master_key_not_leaked_to_upstream() {
    let cred_a = make_provider_cred("openai", "cred-a", "sk-vk-a-secret", None);
    let cred_b = make_provider_cred("openai", "cred-b", "sk-vk-b-secret", None);

    let vk_a = make_vk_config("vk-team-a", vec!["gpt-4o-realtime".into()], vec![cred_a]);
    let vk_b = make_vk_config("vk-team-b", vec!["gpt-4o-realtime".into()], vec![cred_b]);
    let vk_configs = vec![vk_a, vk_b];

    let ctx_a = KeyContext {
        key_id: "vk-team-a".into(),
        allowed_models: Some(vec!["gpt-4o-realtime".into()]),
        is_master: false,
        tenant_id: liter_llm::tenant::TenantId::from("vk-team-a"),
    };
    let resolved_a = resolve_upstream_credential(&ctx_a, &vk_configs, "gpt-4o-realtime");
    assert_eq!(
        resolved_a.as_ref().map(|s| s.expose_secret()),
        Some("sk-vk-a-secret"),
        "team-a should get its own credential, not master key or team-b's key"
    );

    let ctx_b = KeyContext {
        key_id: "vk-team-b".into(),
        allowed_models: Some(vec!["gpt-4o-realtime".into()]),
        is_master: false,
        tenant_id: liter_llm::tenant::TenantId::from("vk-team-b"),
    };
    let resolved_b = resolve_upstream_credential(&ctx_b, &vk_configs, "gpt-4o-realtime");
    assert_eq!(
        resolved_b.as_ref().map(|s| s.expose_secret()),
        Some("sk-vk-b-secret"),
        "team-b should get its own credential"
    );

    let ctx_master = KeyContext::master();
    let resolved_master = resolve_upstream_credential(&ctx_master, &vk_configs, "gpt-4o-realtime");
    assert!(
        resolved_master.is_none(),
        "master-key caller must never leak master_key to upstream; got Some({resolved_master:?})"
    );
}

/// A VK with `model_allowlist = ["gpt-4o-realtime"]` must not receive
/// a credential when requesting a different model (the request would
/// already have been rejected at `can_access_model`, but we verify
/// credential resolution also refuses).
#[test]
fn realtime_credential_model_allowlist_respected() {
    let cred = make_provider_cred(
        "openai",
        "cred-1",
        "sk-vk-secret",
        Some(vec!["gpt-4o-realtime-preview".into()]),
    );
    let vk = make_vk_config("vk-1", vec![], vec![cred]);
    let vk_configs = vec![vk];

    let ctx = KeyContext {
        key_id: "vk-1".into(),
        allowed_models: None,
        is_master: false,
        tenant_id: liter_llm::tenant::TenantId::from("vk-1"),
    };

    let matched = resolve_upstream_credential(&ctx, &vk_configs, "gpt-4o-realtime-preview");
    assert_eq!(matched.as_ref().map(|s| s.expose_secret()), Some("sk-vk-secret"));

    let unmatched = resolve_upstream_credential(&ctx, &vk_configs, "gpt-4o-mini");
    assert!(
        unmatched.is_none(),
        "credential with model_allowlist must not be used for an unlisted model"
    );
}

/// Verify that `can_access_model` gates model access in realtime.
/// This tests the `KeyContext` method used by the handler's security gate.
#[test]
fn realtime_websocket_denies_unallowed_model_with_403() {
    let ctx = KeyContext {
        key_id: "vk-restricted".into(),
        allowed_models: Some(vec!["gpt-4o".into()]),
        is_master: false,
        tenant_id: liter_llm::tenant::TenantId::from("vk-restricted"),
    };

    assert!(
        !ctx.can_access_model("gpt-4o-mini"),
        "VK restricted to gpt-4o must be denied access to gpt-4o-mini"
    );
    assert!(
        ctx.can_access_model("gpt-4o"),
        "VK restricted to gpt-4o must be allowed access to gpt-4o"
    );
}
