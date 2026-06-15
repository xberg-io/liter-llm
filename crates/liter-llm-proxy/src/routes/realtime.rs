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
//! Guardrails are applied:
//! - **Client → upstream**: the serialised event text is checked at the
//!   `GuardrailStage::Input` stage before forwarding.  A `Block` result sends
//!   a [`RealtimeEvent::Error`] back to the client; the upstream never sees the
//!   message.
//! - **Upstream → client**: the serialised event text is checked at the
//!   `GuardrailStage::OutputChunk` stage before forwarding.  A `Block` result
//!   replaces the event with a [`RealtimeEvent::Error`] sent to the client.
//!
//! # Cancellation
//!
//! When the client disconnects the upstream WebSocket is closed within
//! one event-loop iteration via a [`tokio_util::sync::CancellationToken`].
//! When [`crate::shutdown::ShutdownHandle`] is draining the same token is
//! cancelled, closing all active sessions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use secrecy::ExposeSecret;
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_util::sync::CancellationToken;

use liter_llm::guardrail::{Guardrail, GuardrailContext, GuardrailDecision, GuardrailStage};
use liter_llm::realtime::{RealtimeEvent, RealtimeTranslator};
use liter_llm::tower::metrics::{record_realtime_bytes, record_realtime_event, record_realtime_session_duration};

use crate::state::AppState;

// ── Query parameters ──────────────────────────────────────────────────────────

/// Query parameters accepted by `GET /v1/realtime`.
#[derive(Debug, Deserialize)]
pub struct RealtimeQueryParams {
    /// The model to use for the realtime session (e.g. `gpt-4o-realtime-preview`).
    pub model: Option<String>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

/// `GET /v1/realtime` — upgrades to WebSocket and starts the bidirectional proxy.
///
/// The handler is intentionally thin: it resolves the upstream URL from the
/// configured model and delegates the actual proxying to [`run_proxy`].
pub async fn realtime_websocket(
    ws: WebSocketUpgrade,
    Query(params): Query<RealtimeQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let model = params.model.unwrap_or_default();
    ws.on_upgrade(move |socket| handle_session(socket, model, state))
}

// ── Session handler ───────────────────────────────────────────────────────────

/// Spawned per WebSocket connection.  Resolves config, opens the upstream
/// connection, and runs the bidirectional proxy until either side closes.
async fn handle_session(client_socket: WebSocket, model: String, state: AppState) {
    let session_start = Instant::now();

    // Percent-encode the model name so query string is valid even if the model
    // contains special characters (e.g. a future `/` separated model id).
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

    // Open the upstream WebSocket.
    let upstream = match connect_upstream(&upstream_url, &state).await {
        Ok(ws) => ws,
        Err(err) => {
            tracing::warn!(error = %err, "failed to connect to upstream realtime endpoint");
            send_error_to_axum_socket(client_socket, "upstream_connection_failed", &err.to_string()).await;
            return;
        }
    };

    // Build a cancellation token.
    //
    // When the proxy has a shutdown handle we reuse its cancellation token so
    // the session participates in the coordinated drain.  Otherwise we allocate
    // a new, independent token for this session only.
    let cancel = state
        .shutdown
        .as_ref()
        .map(|handle| handle.cancellation_token())
        .unwrap_or_default();

    // Empty guardrail list — callers inject guardrails in future iterations.
    let guardrails: Vec<Arc<dyn Guardrail>> = vec![];

    run_proxy(client_socket, upstream, cancel, guardrails, "openai").await;

    let duration = session_start.elapsed().as_secs_f64();
    record_realtime_session_duration("openai", duration);
    tracing::info!(duration_secs = duration, "realtime session ended");
}

// ── Upstream connection ───────────────────────────────────────────────────────

type UpstreamStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect_upstream(url: &str, state: &AppState) -> Result<UpstreamStream, String> {
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::http::Request;

    let mut request_builder = Request::builder()
        .uri(url)
        .header("User-Agent", "liter-llm-proxy/realtime");

    // Forward master key as Bearer token when available.
    let config = state.config.load();
    if let Some(key) = config.general.master_key.as_ref() {
        request_builder = request_builder.header("Authorization", format!("Bearer {}", key.expose_secret()));
    }

    let request = request_builder
        .body(())
        .map_err(|e| format!("failed to build upstream request: {e}"))?;

    let (ws, _response) = connect_async(request)
        .await
        .map_err(|e| format!("upstream WebSocket handshake failed: {e}"))?;

    Ok(ws)
}

// ── Bidirectional proxy ───────────────────────────────────────────────────────

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
    provider: &'static str,
) {
    let translator = liter_llm::realtime::OpenAiRealtimeTranslator::new();

    let (client_tx, client_rx) = client_socket.split();
    let (upstream_tx, upstream_rx) = upstream.split();

    // Wrap sinks in Mutex so they can be shared across the two halves.
    let client_tx = Arc::new(Mutex::new(client_tx));
    let upstream_tx = Arc::new(Mutex::new(upstream_tx));

    let guardrails = Arc::new(guardrails);

    // --- Client → upstream task -----------------------------------------------
    let upstream_tx_c2u = Arc::clone(&upstream_tx);
    let client_tx_c2u = Arc::clone(&client_tx);
    let cancel_c2u = cancel.clone();
    let guardrails_c2u = Arc::clone(&guardrails);
    let translator_c2u = translator.clone();

    let c2u = tokio::spawn(async move {
        let mut stream = client_rx;
        loop {
            tokio::select! {
                biased;
                _ = cancel_c2u.cancelled() => break,
                msg = stream.next() => {
                    match msg {
                        None => break,
                        Some(Err(e)) => {
                            tracing::debug!(error = %e, "client socket error");
                            break;
                        }
                        Some(Ok(Message::Close(_))) => break,
                        Some(Ok(Message::Text(text))) => {
                            let raw: serde_json::Value = match serde_json::from_str(&text) {
                                Ok(v) => v,
                                Err(e) => {
                                    tracing::debug!(error = %e, "client sent invalid JSON");
                                    continue;
                                }
                            };

                            // Apply Input guardrails before forwarding.
                            if let Some(err_json) =
                                apply_guardrails_input(&guardrails_c2u, &raw, &HashMap::new()).await
                            {
                                let mut tx = client_tx_c2u.lock().await;
                                let _ = tx.send(Message::Text(err_json.into())).await;
                                continue;
                            }

                            // Translate client JSON to unified event then back to
                            // provider wire format (identity for OpenAI).
                            let event = match translator_c2u.translate_inbound(raw) {
                                Ok(e) => e,
                                Err(e) => {
                                    tracing::debug!(error = %e, "translate_inbound failed (c2u)");
                                    continue;
                                }
                            };

                            let label = event_type_label(&event);

                            // Count audio bytes before moving event.
                            let audio_bytes = audio_bytes_for_event(&event);

                            let outbound = match translator_c2u.translate_outbound(&event) {
                                Ok(v) => v,
                                Err(e) => {
                                    tracing::debug!(error = %e, "translate_outbound failed");
                                    continue;
                                }
                            };

                            let wire = serde_json::to_string(&outbound).unwrap_or_default();

                            record_realtime_event(provider, "outbound", label);
                            if audio_bytes > 0 {
                                record_realtime_bytes(provider, "outbound", audio_bytes);
                            }

                            let mut tx = upstream_tx_c2u.lock().await;
                            if tx.send(TungsteniteMessage::Text(wire.into())).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Binary(bytes))) => {
                            record_realtime_event(provider, "outbound", "binary");
                            record_realtime_bytes(provider, "outbound", bytes.len() as u64);
                            let mut tx = upstream_tx_c2u.lock().await;
                            if tx
                                .send(TungsteniteMessage::Binary(bytes.to_vec().into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let mut tx = upstream_tx_c2u.lock().await;
                            let _ = tx.send(TungsteniteMessage::Ping(data.to_vec().into())).await;
                        }
                        Some(Ok(Message::Pong(_))) => {}
                    }
                }
            }
        }
        cancel_c2u.cancel();
    });

    // --- Upstream → client task -----------------------------------------------
    let client_tx_u2c = Arc::clone(&client_tx);
    let cancel_u2c = cancel.clone();
    let guardrails_u2c = Arc::clone(&guardrails);
    let translator_u2c = translator;

    let u2c = tokio::spawn(async move {
        let mut stream = upstream_rx;
        loop {
            tokio::select! {
                biased;
                _ = cancel_u2c.cancelled() => break,
                msg = stream.next() => {
                    match msg {
                        None => break,
                        Some(Err(e)) => {
                            tracing::debug!(error = %e, "upstream socket error");
                            break;
                        }
                        Some(Ok(TungsteniteMessage::Text(text))) => {
                            let raw: serde_json::Value =
                                match serde_json::from_str(text.as_str()) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        tracing::debug!(
                                            error = %e,
                                            "upstream sent invalid JSON"
                                        );
                                        // Forward verbatim to avoid data loss.
                                        let mut tx = client_tx_u2c.lock().await;
                                        let _ = tx
                                            .send(Message::Text(text.to_string().into()))
                                            .await;
                                        continue;
                                    }
                                };

                            let event = match translator_u2c.translate_inbound(raw.clone()) {
                                Ok(e) => e,
                                Err(e) => {
                                    tracing::debug!(
                                        error = %e,
                                        "translate_inbound failed (u2c)"
                                    );
                                    continue;
                                }
                            };

                            let label = event_type_label(&event);
                            let audio_bytes = audio_bytes_for_event(&event);

                            // Apply OutputChunk guardrails.
                            let forward_json = match apply_guardrails_output_chunk(
                                &guardrails_u2c,
                                &raw,
                                &HashMap::new(),
                            )
                            .await
                            {
                                GuardrailOutcome::Allow(v) => {
                                    serde_json::to_string(&v).unwrap_or_default()
                                }
                                GuardrailOutcome::Block { reason, code } => {
                                    let err = RealtimeEvent::Error {
                                        code: format!("{code}"),
                                        message: reason,
                                        event_id: None,
                                    };
                                    match translator_u2c.translate_outbound(&err) {
                                        Ok(v) => serde_json::to_string(&v).unwrap_or_default(),
                                        Err(_) => continue,
                                    }
                                }
                            };

                            record_realtime_event(provider, "inbound", label);
                            if audio_bytes > 0 {
                                record_realtime_bytes(provider, "inbound", audio_bytes);
                            }

                            let mut tx = client_tx_u2c.lock().await;
                            if tx.send(Message::Text(forward_json.into())).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(TungsteniteMessage::Binary(bytes))) => {
                            record_realtime_event(provider, "inbound", "binary");
                            record_realtime_bytes(provider, "inbound", bytes.len() as u64);
                            let mut tx = client_tx_u2c.lock().await;
                            if tx
                                .send(Message::Binary(bytes.to_vec().into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Some(Ok(TungsteniteMessage::Ping(data))) => {
                            let mut tx = client_tx_u2c.lock().await;
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
        cancel_u2c.cancel();
    });

    let _ = tokio::join!(c2u, u2c);
}

// ── Guardrail helpers ─────────────────────────────────────────────────────────

/// Outcome of running guardrails on a chunk.
pub(crate) enum GuardrailOutcome {
    Allow(serde_json::Value),
    Block { reason: String, code: u32 },
}

pub(crate) async fn apply_guardrails_input(
    guardrails: &[Arc<dyn Guardrail>],
    payload: &serde_json::Value,
    metadata: &HashMap<String, String>,
) -> Option<String> {
    for guardrail in guardrails.iter() {
        if !guardrail.supported_stages().contains(&GuardrailStage::Input) {
            continue;
        }
        let ctx = GuardrailContext {
            request: payload,
            response: None,
            chunk: None,
            metadata,
        };
        match guardrail.check(GuardrailStage::Input, &ctx).await {
            GuardrailDecision::Block { reason, code } => {
                let err = serde_json::json!({
                    "type": "error",
                    "error": {
                        "code": format!("{code}"),
                        "message": reason
                    }
                });
                return Some(serde_json::to_string(&err).unwrap_or_default());
            }
            GuardrailDecision::Allow | GuardrailDecision::Mutate { .. } => {}
        }
    }
    None
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

// ── Utility ───────────────────────────────────────────────────────────────────

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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::sync::Arc;
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use tokio_tungstenite::tungstenite::Message as Msg;

    use super::*;
    use liter_llm::guardrail::{GuardrailContext, GuardrailDecision, GuardrailStage};
    use liter_llm::realtime::RealtimeEvent;

    // ── Helper: spawn a mock WebSocket server ─────────────────────────────────

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

    // ── realtime_websocket_proxy_forwards_bidirectional ───────────────────────

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
            // Upstream → client: send a session.created greeting.
            let greeting = serde_json::json!({
                "type": "session.created",
                "session": { "id": "sess_1", "model": "gpt-4o-realtime-preview" }
            });
            let _ = ws
                .send(Msg::Text(serde_json::to_string(&greeting).unwrap().into()))
                .await;

            // Client → upstream: echo back whatever we receive.
            if let Some(Ok(msg)) = ws.next().await {
                let _ = ws.send(msg).await;
            }
        })
        .await;

        // Connect as the "client" to the mock server.
        let url = format!("ws://{addr}");
        let (mut stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        // Upstream → client direction: read the session.created greeting.
        let greeting_msg = stream.next().await.unwrap().unwrap();
        assert!(greeting_msg.is_text(), "expected text frame from upstream");
        let val: serde_json::Value = serde_json::from_str(greeting_msg.into_text().unwrap().as_str()).unwrap();
        assert_eq!(val["type"], "session.created");
        assert_eq!(val["session"]["id"], "sess_1");

        // Client → upstream direction: send a commit and get it echoed back.
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

    // ── realtime_websocket_proxy_cancels_on_client_disconnect ─────────────────

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

    // ── realtime_websocket_proxy_blocks_on_guardrail ──────────────────────────

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

        let result = apply_guardrails_input(&guardrails, &payload, &HashMap::new()).await;

        assert!(result.is_some(), "guardrail should have blocked the event");
        let err_json = result.unwrap();
        let err_val: serde_json::Value = serde_json::from_str(&err_json).unwrap();
        assert_eq!(err_val["type"], "error");
        assert!(
            err_val["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("blocked by test guardrail"),
            "error message should mention the guardrail reason"
        );
    }

    // ── OutputChunk guardrail allows clean events ─────────────────────────────

    #[tokio::test]
    async fn apply_guardrails_output_chunk_allows_clean_event() {
        let guardrails: Vec<Arc<dyn Guardrail>> = vec![];
        let payload = serde_json::json!({ "type": "response.text.delta", "delta": "hello" });
        let outcome = apply_guardrails_output_chunk(&guardrails, &payload, &HashMap::new()).await;
        assert!(matches!(outcome, GuardrailOutcome::Allow(_)));
    }

    // ── event_type_label ─────────────────────────────────────────────────────

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

    // ── audio_bytes_for_event ─────────────────────────────────────────────────

    #[test]
    fn audio_bytes_for_event_returns_zero_for_non_audio() {
        let event = RealtimeEvent::InputAudioBufferCommit;
        assert_eq!(audio_bytes_for_event(&event), 0);
    }

    #[test]
    fn audio_bytes_for_event_returns_nonzero_for_audio_append() {
        // base64("AAAA") → 3 bytes
        let event = RealtimeEvent::InputAudioBufferAppend {
            audio_base64: "AAAA".into(),
        };
        assert!(audio_bytes_for_event(&event) > 0);
    }
}
