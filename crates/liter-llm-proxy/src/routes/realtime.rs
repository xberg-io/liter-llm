//! WebSocket proxy for the OpenAI Realtime API.
//!
//! # Route
//!
//! `GET /v1/realtime?model=<model>` — upgrades the HTTP connection to a
//! WebSocket and proxies messages bidirectionally between the API client and
//! the upstream provider's Realtime endpoint.
//!
//! # Message flow
//!
//! ```text
//! client  ──[WS message]──►  translate_outbound  ──►  upstream provider
//! client  ◄─[WS message]──   translate_inbound   ◄──  upstream provider
//! ```
//!
//! # Guardrails
//!
//! [`handle_session`] enforces the guardrail set configured under
//! `[[guardrails]]`, read from [`AppState::guardrails`] via
//! [`session_guardrails`].  It is the *same* [`GuardrailRegistry`] that
//! [`crate::service_pool::ServicePool`] layers into every model's unary Tower
//! stack — one `Arc`, obtained from [`ServicePool::guardrails`], so a
//! guardrail can never be live on one path and absent on the other.
//!
//! Every message is checked: client → upstream at the `GuardrailStage::Input`
//! stage, upstream → client at `GuardrailStage::OutputChunk`.  A `Block`
//! decision is returned to the client as an error event and the message is not
//! forwarded; a `Mutate` decision rewrites the payload that is forwarded.
//!
//! Note that stage semantics differ between the two paths: the unary path
//! inspects a serialized `LlmRequest`, while here the inspected payload is a
//! single realtime event.  A pattern-based guardrail works on both, but a CEL
//! expression written against a chat-completion shape will not evaluate
//! against a realtime event — and `CelGuardrail` fails closed by default, so
//! that misconfiguration blocks realtime traffic loudly rather than passing it
//! silently.
//!
//! [`AppState`]: crate::state::AppState
//! [`AppState::guardrails`]: crate::state::AppState::guardrails
//! [`GuardrailRegistry`]: liter_llm::guardrail::GuardrailRegistry
//! [`ServicePool::guardrails`]: crate::service_pool::ServicePool::guardrails
//!
//! # Cancellation
//!
//! When the client disconnects the upstream WebSocket is closed within
//! one event-loop iteration via a [`tokio_util::sync::CancellationToken`].
//! When [`crate::shutdown::ShutdownHandle`] is draining the same token is
//! cancelled, closing all active sessions.

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Instant;

use axum::Extension;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_util::sync::CancellationToken;

use liter_llm::guardrail::{Guardrail, GuardrailContext, GuardrailDecision, GuardrailStage};
use liter_llm::realtime::{RealtimeEvent, RealtimeTranslator};
use liter_llm::tenant::TenantId;
use liter_llm::tower::TENANT_ID_METADATA_KEY;
use liter_llm::tower::metrics::{record_realtime_bytes, record_realtime_event, record_realtime_session_duration};
use secrecy::{ExposeSecret, SecretString};

use crate::auth::KeyContext;
use crate::config::VirtualKeyConfig;
use crate::error::ProxyError;
use crate::shutdown::ShutdownHandle;
use crate::state::AppState;

/// Query parameters accepted by `GET /v1/realtime`.
#[derive(Debug, Deserialize)]
pub struct RealtimeQueryParams {
    /// The model to use for the realtime session (e.g. `gpt-4o-realtime-preview`).
    pub model: Option<String>,
}

/// `GET /v1/realtime` — upgrades to WebSocket and starts the bidirectional proxy.
///
/// The handler is intentionally thin: it resolves the upstream URL from the
/// configured model and delegates the actual proxying to [`run_proxy`].
///
/// # Security
///
/// - `KeyContext` is extracted from request extensions (populated by the
///   `validate_api_key` middleware) and checked against the requested model
///   **before** the WebSocket upgrade.  A 403 is returned immediately if the
///   virtual key is not allowed to access the model — the upgrade never
///   completes.
/// - The upstream credential is resolved from the virtual key's
///   `provider_credentials` list, **not** from the global `master_key`.  If
///   no matching credential is found for the `"openai"` provider (or for a
///   master-key caller without an explicit credential), the handler returns
///   503 rather than falling back to the master key.
///
/// # Rate limit and budget enforcement
///
/// Unlike unary endpoints, a realtime session never runs through
/// `ServicePool::get_service`'s Tower stack (there is no discrete
/// `LlmRequest`/`LlmResponse` per WebSocket message), so `KeyLimitLayer` and
/// `BudgetLedgerLayer` never see it. [`ServicePool::check_realtime_session_start`]
/// closes that gap at session establishment — **before** the upgrade — reusing
/// the SAME per-key rpm window and budget ledger unary calls use, rather than
/// a parallel realtime-only implementation. See that method's docs for what
/// it can and cannot enforce (it cannot meter the live session's own token
/// usage, only gate on rpm and already-recorded spend).
pub async fn realtime_websocket(
    ws: WebSocketUpgrade,
    Query(params): Query<RealtimeQueryParams>,
    State(state): State<AppState>,
    Extension(key_ctx): Extension<KeyContext>,
) -> impl IntoResponse {
    let model = params.model.unwrap_or_default();

    if !key_ctx.can_access_model(&model) {
        let err = ProxyError::forbidden(format!(
            "key '{}' is not allowed to access model '{model}'",
            key_ctx.redacted_id()
        ));
        return err.into_response();
    }

    if let Err(err) = state
        .service_pool
        .check_realtime_session_start(&key_ctx.tenant_id, &model)
        .await
    {
        return err.into_response();
    }

    // ~keep Each request uses one stable config snapshot even if hot-reload fires.
    let config = state.config.load();

    // ~keep Never fall back to master_key; that would let VK holders use the master billing key.
    let upstream_api_key: SecretString = match resolve_upstream_credential(&key_ctx, &config.keys, &model) {
        Some(key) => key,
        None => {
            let err = ProxyError::service_unavailable(format!(
                "no provider credential configured for model '{model}' — \
                 add [[keys.provider_credentials]] with provider = \"openai\" \
                 to the virtual key configuration"
            ));
            return err.into_response();
        }
    };

    let tenant_id = key_ctx.tenant_id.clone();

    ws.on_upgrade(move |socket| handle_session(socket, model, upstream_api_key, state, tenant_id))
        .into_response()
}

/// Resolve an upstream API key from the virtual key's provider credential pool.
///
/// Selection order:
/// 1. Any `provider_credentials` entry with `provider == "openai"` whose
///    `model_allowlist` includes `model` (or whose `model_allowlist` is `None`).
/// 2. First such entry when multiple match (callers should configure one per
///    model group or leave `model_allowlist` unset for a universal credential).
///
/// Returns `None` when:
/// - The caller authenticated as the master key (no VK config) and no explicit
///   credential is provided — returning `None` forces a 503 rather than leaking
///   `master_key` to the upstream.
/// - The VK has no `provider_credentials` entries for `"openai"`.
fn resolve_upstream_credential(
    key_ctx: &KeyContext,
    vk_configs: &[VirtualKeyConfig],
    model: &str,
) -> Option<SecretString> {
    // ~keep Master-key callers need explicit provider credentials; never leak master_key upstream.
    if key_ctx.is_master {
        return None;
    }

    let vk_config = vk_configs.iter().find(|vk| vk.key == key_ctx.key_id)?;

    vk_config
        .provider_credentials
        .iter()
        .find(|cred| {
            cred.provider == "openai"
                && match &cred.model_allowlist {
                    None => true,
                    Some(allowed) => allowed.iter().any(|m| m == model),
                }
        })
        .map(|cred| cred.api_key.clone())
}

/// Derive the cancellation token for one realtime session.
///
/// MUST return a CHILD of the shutdown token, never a clone. `run_proxy`
/// cancels whatever token it is given as soon as either relay loop exits —
/// that is every normal session close — and a clone shares cancellation state
/// with the process-wide token that
/// `axum::serve(..).with_graceful_shutdown(..)` awaits. Cloning here meant one
/// client hanging up a voice session shut down the entire proxy. A child is
/// still cancelled BY the parent, so draining continues to close live
/// sessions, but cancelling the child leaves the parent untouched. ~keep
fn session_cancel_token(shutdown: Option<&ShutdownHandle>) -> CancellationToken {
    shutdown.map_or_else(CancellationToken::default, |handle| {
        handle.cancellation_token().child_token()
    })
}

/// Spawned per WebSocket connection.  Opens the upstream connection using the
/// pre-resolved `upstream_api_key` and runs the bidirectional proxy until
/// either side closes.
///
/// The `upstream_api_key` is the resolved per-VK credential — the master key
/// is NEVER passed here (that check lives in [`realtime_websocket`]).
async fn handle_session(
    client_socket: WebSocket,
    model: String,
    upstream_api_key: SecretString,
    state: AppState,
    tenant_id: TenantId,
) {
    let session_start = Instant::now();

    let guardrail_metadata = session_guardrail_metadata(&tenant_id);

    let encoded_model: String = model
        .chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~') {
                vec![c]
            } else {
                format!("%{:02X}", c as u32).chars().collect()
            }
        })
        .collect();
    let upstream_url = format!("wss://api.openai.com/v1/realtime?model={encoded_model}");

    tracing::info!(
        model = %model,
        upstream_url = %upstream_url,
        "realtime session starting"
    );

    let upstream = match connect_upstream(&upstream_url, upstream_api_key.expose_secret()).await {
        Ok(ws) => ws,
        Err(err) => {
            tracing::warn!(error = %err, "failed to connect to upstream realtime endpoint");
            send_error_to_axum_socket(client_socket, "upstream_connection_failed", &err.to_string()).await;
            return;
        }
    };

    let cancel = session_cancel_token(state.shutdown.as_ref());

    run_proxy(
        client_socket,
        upstream,
        cancel,
        session_guardrails(&state),
        guardrail_metadata,
        "openai",
    )
    .await;

    let duration = session_start.elapsed().as_secs_f64();
    record_realtime_session_duration("openai", duration);
    tracing::info!(duration_secs = duration, "realtime session ended");
}

/// The guardrail set a realtime session enforces.
///
/// Taken from [`AppState::guardrails`] — the registry
/// [`crate::service_pool::ServicePool`] built at startup and layers into every
/// model's unary Tower stack — never rebuilt here, so realtime and unary
/// traffic are guarded by the same configured set.
///
/// [`run_proxy`] wants a `Vec` because it evaluates guardrails directly rather
/// than through `GuardrailRegistry::run_stage`: realtime frames are neither an
/// `LlmRequest` nor an `LlmResponse`, so the Tower layer does not apply.
///
/// [`AppState::guardrails`]: crate::state::AppState::guardrails
fn session_guardrails(state: &AppState) -> Vec<Arc<dyn Guardrail>> {
    state.guardrails.iter().map(Arc::clone).collect()
}

type UpstreamStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Connect to the upstream WebSocket endpoint using the pre-resolved API key.
///
/// # Security
///
/// `api_key` must be the per-VK credential resolved in [`realtime_websocket`].
/// This function must NEVER be called with the global `master_key` — that
/// check is enforced at the call-site in [`handle_session`] which receives
/// the key from [`realtime_websocket`] (never from `AppState.config`).
async fn connect_upstream(url: &str, api_key: &str) -> Result<UpstreamStream, String> {
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::http::Request;

    let request = Request::builder()
        .uri(url)
        .header("User-Agent", "liter-llm-proxy/realtime")
        .header("Authorization", format!("Bearer {api_key}"))
        .body(())
        .map_err(|e| format!("failed to build upstream request: {e}"))?;

    let (ws, _response) = connect_async(request)
        .await
        .map_err(|e| format!("upstream WebSocket handshake failed: {e}"))?;

    Ok(ws)
}

/// The `GuardrailContext::metadata` a realtime session presents to its guardrails.
///
/// The unary path gets this for free — `GuardrailService` derives it from
/// [`liter_llm::tower::types::LlmRequest::tenant_id`] on every call. The relay proxies raw
/// frames and has no `LlmRequest` to derive one from, so it must assemble the equivalent
/// map itself. Without it, a `deny_list` on `tenant_id` would block unary traffic and
/// silently allow realtime traffic: one configured set enforcing differently depending on
/// which door the request came through, which is the failure the shared registry exists to
/// prevent.
///
/// Uses [`TENANT_ID_METADATA_KEY`] rather than a local `"tenant_id"` literal so the two
/// sides cannot drift into different spellings. ~keep
fn session_guardrail_metadata(tenant_id: &TenantId) -> HashMap<String, String> {
    HashMap::from([(TENANT_ID_METADATA_KEY.to_owned(), tenant_id.as_ref().to_owned())])
}

/// Run the bidirectional proxy loop.
///
/// - Reads from `client_socket` (axum `WebSocket`) and forwards to `upstream`
///   (tungstenite `WebSocketStream`) after applying outbound guardrails.
/// - Reads from `upstream` and forwards to `client_socket` after applying
///   inbound guardrails.
/// - Returns when either side closes or `cancel` is triggered.
pub(crate) async fn run_proxy(
    client_socket: WebSocket,
    upstream: UpstreamStream,
    cancel: CancellationToken,
    guardrails: Vec<Arc<dyn Guardrail>>,
    metadata: HashMap<String, String>,
    provider: &'static str,
) {
    let translator = liter_llm::realtime::OpenAiRealtimeTranslator::new();

    let (client_tx, client_rx) = client_socket.split();
    let (upstream_tx, upstream_rx) = upstream.split();

    let client_tx = Arc::new(Mutex::new(client_tx));
    let upstream_tx = Arc::new(Mutex::new(upstream_tx));

    let guardrails = Arc::new(guardrails);
    let metadata = Arc::new(metadata);

    let c2u_ctx = PumpContext {
        client_tx: Arc::clone(&client_tx),
        upstream_tx: Arc::clone(&upstream_tx),
        cancel: cancel.clone(),
        guardrails: Arc::clone(&guardrails),
        metadata: Arc::clone(&metadata),
        translator: translator.clone(),
        provider,
    };
    let u2c_ctx = PumpContext {
        client_tx: Arc::clone(&client_tx),
        upstream_tx: Arc::clone(&upstream_tx),
        cancel: cancel.clone(),
        guardrails: Arc::clone(&guardrails),
        metadata: Arc::clone(&metadata),
        translator,
        provider,
    };

    let c2u = tokio::spawn(pump_client_to_upstream(client_rx, c2u_ctx));
    let u2c = tokio::spawn(pump_upstream_to_client(upstream_rx, u2c_ctx));

    let _ = tokio::join!(c2u, u2c);
}

type ClientSink = SplitSink<WebSocket, Message>;
type UpstreamSink = SplitSink<UpstreamStream, TungsteniteMessage>;

/// State shared by one direction of the proxy loop.
///
/// Both directions carry both sinks even though each only sends on one (plus
/// the client sink for error events). Sink lifetime is owned by `run_proxy`,
/// which holds its own `Arc`s until both tasks are joined, so the extra clone
/// never changes when a sink closes; keeping one shape for both directions is
/// simpler than two near-identical context types. Do not drop the unused sink
/// from one direction on the assumption that it affects close timing -- it
/// does not, and neither does keeping it.
struct PumpContext {
    client_tx: Arc<Mutex<ClientSink>>,
    upstream_tx: Arc<Mutex<UpstreamSink>>,
    cancel: CancellationToken,
    guardrails: Arc<Vec<Arc<dyn Guardrail>>>,
    metadata: Arc<HashMap<String, String>>,
    translator: liter_llm::realtime::OpenAiRealtimeTranslator,
    provider: &'static str,
}

/// Client -> upstream: apply `Input` guardrails and forward each message until
/// either side closes or `cancel` fires, then cancel the other direction.
async fn pump_client_to_upstream(mut stream: SplitStream<WebSocket>, ctx: PumpContext) {
    loop {
        tokio::select! {
            biased;
            _ = ctx.cancel.cancelled() => break,
            msg = stream.next() => {
                match msg {
                    None => break,
                    Some(Err(e)) => {
                        tracing::debug!(error = %e, "client socket error");
                        break;
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Text(text))) => {
                        if forward_client_text(&ctx, text.as_str()).await.is_break() {
                            break;
                        }
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        record_realtime_event(ctx.provider, "outbound", "binary");
                        record_realtime_bytes(ctx.provider, "outbound", bytes.len() as u64);
                        let mut tx = ctx.upstream_tx.lock().await;
                        if tx
                            .send(TungsteniteMessage::Binary(bytes.to_vec().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let mut tx = ctx.upstream_tx.lock().await;
                        let _ = tx.send(TungsteniteMessage::Ping(data.to_vec().into())).await;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                }
            }
        }
    }
    ctx.cancel.cancel();
}

/// Guardrail, translate and forward one client text frame upstream.
///
/// `Break` means the upstream send failed and the loop must stop; every
/// skipped frame (invalid JSON, blocked, untranslatable) is `Continue`.
async fn forward_client_text(ctx: &PumpContext, text: &str) -> ControlFlow<()> {
    let raw: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(error = %e, "client sent invalid JSON");
            return ControlFlow::Continue(());
        }
    };

    let raw = match apply_guardrails_input(ctx.guardrails.as_slice(), &raw, ctx.metadata.as_ref()).await {
        GuardrailOutcome::Allow(payload) => payload,
        GuardrailOutcome::Block { reason, code } => {
            let err_json = guardrail_error_json(code, &reason);
            let mut tx = ctx.client_tx.lock().await;
            let _ = tx.send(Message::Text(err_json.into())).await;
            return ControlFlow::Continue(());
        }
    };

    let event = match ctx.translator.translate_inbound(raw) {
        Ok(e) => e,
        Err(e) => {
            tracing::debug!(error = %e, "translate_inbound failed (c2u)");
            return ControlFlow::Continue(());
        }
    };

    let label = event_type_label(&event);

    let audio_bytes = audio_bytes_for_event(&event);

    let outbound = match ctx.translator.translate_outbound(&event) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(error = %e, "translate_outbound failed");
            return ControlFlow::Continue(());
        }
    };

    let wire = serde_json::to_string(&outbound).unwrap_or_default();

    record_realtime_event(ctx.provider, "outbound", label);
    if audio_bytes > 0 {
        record_realtime_bytes(ctx.provider, "outbound", audio_bytes);
    }

    let mut tx = ctx.upstream_tx.lock().await;
    if tx.send(TungsteniteMessage::Text(wire.into())).await.is_err() {
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(())
}

/// Upstream -> client: apply `OutputChunk` guardrails and forward each message
/// until either side closes or `cancel` fires, then cancel the other direction.
async fn pump_upstream_to_client(mut stream: SplitStream<UpstreamStream>, ctx: PumpContext) {
    loop {
        tokio::select! {
            biased;
            _ = ctx.cancel.cancelled() => break,
            msg = stream.next() => {
                match msg {
                    None => break,
                    Some(Err(e)) => {
                        tracing::debug!(error = %e, "upstream socket error");
                        break;
                    }
                    Some(Ok(TungsteniteMessage::Text(text))) => {
                        if forward_upstream_text(&ctx, text.as_str()).await.is_break() {
                            break;
                        }
                    }
                    Some(Ok(TungsteniteMessage::Binary(bytes))) => {
                        record_realtime_event(ctx.provider, "inbound", "binary");
                        record_realtime_bytes(ctx.provider, "inbound", bytes.len() as u64);
                        let mut tx = ctx.client_tx.lock().await;
                        if tx
                            .send(Message::Binary(bytes.to_vec().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Some(Ok(TungsteniteMessage::Ping(data))) => {
                        let mut tx = ctx.client_tx.lock().await;
                        let _ = tx.send(Message::Ping(data.to_vec().into())).await;
                    }
                    Some(Ok(TungsteniteMessage::Close(_))) => break,
                    Some(Ok(
                        TungsteniteMessage::Pong(_) | TungsteniteMessage::Frame(_),
                    )) => {}
                }
            }
        }
    }
    ctx.cancel.cancel();
}

/// Translate, guardrail and forward one upstream text frame to the client.
///
/// Invalid JSON is passed through verbatim, as before; a blocked frame becomes
/// an error event. `Break` means the client send failed.
async fn forward_upstream_text(ctx: &PumpContext, text: &str) -> ControlFlow<()> {
    let raw: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(
                error = %e,
                "upstream sent invalid JSON"
            );
            let mut tx = ctx.client_tx.lock().await;
            let _ = tx.send(Message::Text(text.to_owned().into())).await;
            return ControlFlow::Continue(());
        }
    };

    let event = match ctx.translator.translate_inbound(raw.clone()) {
        Ok(e) => e,
        Err(e) => {
            tracing::debug!(
                error = %e,
                "translate_inbound failed (u2c)"
            );
            return ControlFlow::Continue(());
        }
    };

    let label = event_type_label(&event);
    let audio_bytes = audio_bytes_for_event(&event);

    let forward_json = match apply_guardrails_output_chunk(ctx.guardrails.as_slice(), &raw, ctx.metadata.as_ref()).await
    {
        GuardrailOutcome::Allow(v) => serde_json::to_string(&v).unwrap_or_default(),
        GuardrailOutcome::Block { reason, code } => {
            let err = RealtimeEvent::Error {
                code: format!("{code}"),
                message: reason,
                event_id: None,
            };
            match ctx.translator.translate_outbound(&err) {
                Ok(v) => serde_json::to_string(&v).unwrap_or_default(),
                Err(_) => return ControlFlow::Continue(()),
            }
        }
    };

    record_realtime_event(ctx.provider, "inbound", label);
    if audio_bytes > 0 {
        record_realtime_bytes(ctx.provider, "inbound", audio_bytes);
    }

    let mut tx = ctx.client_tx.lock().await;
    if tx.send(Message::Text(forward_json.into())).await.is_err() {
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(())
}

/// Outcome of running guardrails on a payload.
pub(crate) enum GuardrailOutcome {
    /// Forward this payload — the original, or the rewrite a `Mutate`
    /// decision produced.
    Allow(serde_json::Value),
    /// Do not forward; return an error event to the client instead.
    Block { reason: String, code: u32 },
}

/// Render a guardrail `Block` as the realtime error event sent to the client.
pub(crate) fn guardrail_error_json(code: u32, reason: &str) -> String {
    let err = serde_json::json!({
        "type": "error",
        "error": {
            "code": format!("{code}"),
            "message": reason,
        }
    });
    serde_json::to_string(&err).unwrap_or_default()
}

/// Run the `Input` stage over a client → upstream event.
///
/// A `Mutate` decision rewrites the payload and evaluation continues against
/// the rewritten value, so the next guardrail inspects what will actually be
/// forwarded and a redaction guardrail redacts here exactly as it does on the
/// unary path. Dropping the rewrite and forwarding the original — as this
/// function did while no caller supplied a guardrail set — is how a redaction
/// guardrail comes to leak the content it was installed to remove. ~keep
pub(crate) async fn apply_guardrails_input(
    guardrails: &[Arc<dyn Guardrail>],
    payload: &serde_json::Value,
    metadata: &HashMap<String, String>,
) -> GuardrailOutcome {
    let mut current = payload.clone();

    for guardrail in guardrails.iter() {
        if !guardrail.supported_stages().contains(&GuardrailStage::Input) {
            continue;
        }
        let ctx = GuardrailContext {
            request: &current,
            response: None,
            chunk: None,
            metadata,
        };
        match guardrail.check(GuardrailStage::Input, &ctx).await {
            GuardrailDecision::Block { reason, code } => {
                return GuardrailOutcome::Block { reason, code };
            }
            GuardrailDecision::Mutate { new_payload } => {
                current = new_payload;
            }
            GuardrailDecision::Allow => {}
        }
    }

    GuardrailOutcome::Allow(current)
}

pub(crate) async fn apply_guardrails_output_chunk(
    guardrails: &[Arc<dyn Guardrail>],
    payload: &serde_json::Value,
    metadata: &HashMap<String, String>,
) -> GuardrailOutcome {
    let payload_str = serde_json::to_string(payload).unwrap_or_default();
    let mut current = payload.clone();

    for guardrail in guardrails.iter() {
        if !guardrail.supported_stages().contains(&GuardrailStage::OutputChunk) {
            continue;
        }
        let ctx = GuardrailContext {
            request: &current,
            response: None,
            chunk: Some(&payload_str),
            metadata,
        };
        match guardrail.check(GuardrailStage::OutputChunk, &ctx).await {
            GuardrailDecision::Block { reason, code } => {
                return GuardrailOutcome::Block { reason, code };
            }
            GuardrailDecision::Mutate { new_payload } => {
                current = new_payload;
            }
            GuardrailDecision::Allow => {}
        }
    }
    GuardrailOutcome::Allow(current)
}

pub(crate) fn event_type_label(event: &RealtimeEvent) -> &'static str {
    match event {
        RealtimeEvent::SessionCreated { .. } => "session.created",
        RealtimeEvent::SessionUpdated { .. } => "session.updated",
        RealtimeEvent::ConversationItemCreated { .. } => "conversation.item.created",
        RealtimeEvent::ConversationItemDeleted { .. } => "conversation.item.deleted",
        RealtimeEvent::ResponseCreated { .. } => "response.created",
        RealtimeEvent::ResponseDone { .. } => "response.done",
        RealtimeEvent::ResponseTextDelta { .. } => "response.text.delta",
        RealtimeEvent::ResponseTextDone { .. } => "response.text.done",
        RealtimeEvent::ResponseAudioDelta { .. } => "response.audio.delta",
        RealtimeEvent::ResponseAudioDone { .. } => "response.audio.done",
        RealtimeEvent::ResponseAudioTranscriptDelta { .. } => "response.audio_transcript.delta",
        RealtimeEvent::ResponseAudioTranscriptDone { .. } => "response.audio_transcript.done",
        RealtimeEvent::ResponseFunctionCallArgumentsDelta { .. } => "response.function_call_arguments.delta",
        RealtimeEvent::ResponseFunctionCallArgumentsDone { .. } => "response.function_call_arguments.done",
        RealtimeEvent::InputAudioBufferAppend { .. } => "input_audio_buffer.append",
        RealtimeEvent::InputAudioBufferCommit => "input_audio_buffer.commit",
        RealtimeEvent::InputAudioBufferClear => "input_audio_buffer.clear",
        RealtimeEvent::InputAudioBufferSpeechStarted { .. } => "input_audio_buffer.speech_started",
        RealtimeEvent::InputAudioBufferSpeechStopped { .. } => "input_audio_buffer.speech_stopped",
        RealtimeEvent::RateLimitsUpdated { .. } => "rate_limits.updated",
        RealtimeEvent::Error { .. } => "error",
        RealtimeEvent::Raw { .. } => "raw",
    }
}

/// Approximate audio byte count for an event (0 when not an audio event).
fn audio_bytes_for_event(event: &RealtimeEvent) -> u64 {
    match event {
        RealtimeEvent::InputAudioBufferAppend { audio_base64 } => (audio_base64.len() * 3 / 4) as u64,
        RealtimeEvent::ResponseAudioDelta { delta_base64, .. } => (delta_base64.len() * 3 / 4) as u64,
        _ => 0,
    }
}

async fn send_error_to_axum_socket(mut socket: WebSocket, code: &str, message: &str) {
    let err = serde_json::json!({
        "type": "error",
        "error": { "code": code, "message": message }
    });
    let _ = socket
        .send(Message::Text(serde_json::to_string(&err).unwrap_or_default().into()))
        .await;
    let _ = socket.close().await;
}

#[cfg(test)]
mod tests;
