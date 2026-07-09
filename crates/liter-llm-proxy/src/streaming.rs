use std::convert::Infallible;
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::stream::StreamExt;
use liter_llm::Result as LlmResult;
use liter_llm::client::BoxStream;
use liter_llm::types::ChatCompletionChunk;

/// Convert a chat completion chunk stream into an SSE response.
///
/// Each chunk is serialized to JSON and sent as an SSE `data:` event.
/// After the stream completes, a final `data: [DONE]` event is sent
/// (matching OpenAI's convention).
pub fn sse_response(stream: BoxStream<'static, LlmResult<ChatCompletionChunk>>) -> Response {
    let sse_stream = stream
        .map(|result| -> std::result::Result<Event, Infallible> {
            match result {
                Ok(chunk) => Ok(Event::default().json_data(&chunk).unwrap_or_else(|_| {
                    Event::default()
                        .data(r#"{"error":{"message":"chunk serialization failed","type":"InternalError"}}"#)
                })),
                Err(e) => {
                    let proxy_err: crate::error::ProxyError = e.into();
                    Ok(Event::default().data(proxy_err.to_sse_payload()))
                }
            }
        })
        .chain(futures_util::stream::once(async {
            Ok::<_, Infallible>(Event::default().data("[DONE]"))
        }));

    Sse::new(sse_stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures_util::stream;
    use http_body_util::BodyExt;
    use liter_llm::error::LiterLlmError;
    use liter_llm::types::chat::{StreamChoice, StreamDelta};

    type LlmResult<T> = std::result::Result<T, LiterLlmError>;

    /// Build a minimal `ChatCompletionChunk` for testing.
    fn make_chunk(id: &str, content: &str) -> ChatCompletionChunk {
        ChatCompletionChunk {
            id: id.to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "test-model".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some(content.to_string()),
                    tool_calls: None,
                    function_call: None,
                    refusal: None,
                },
                finish_reason: None,
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        }
    }

    /// Extract the response body as a string.
    async fn body_string(response: Response) -> String {
        let bytes = axum::body::Body::new(response.into_body())
            .collect()
            .await
            .expect("body collection should not fail")
            .to_bytes();
        String::from_utf8(bytes.to_vec()).expect("body should be valid UTF-8")
    }

    /// Parse SSE events from a raw body string, returning only the `data:` payloads.
    fn parse_sse_data(body: &str) -> Vec<String> {
        body.split("\n\n")
            .filter(|block| !block.is_empty())
            .filter_map(|block| {
                block
                    .lines()
                    .find(|line| line.starts_with("data:"))
                    .map(|line| line.strip_prefix("data:").unwrap_or("").trim().to_string())
            })
            .collect()
    }

    #[tokio::test]
    async fn sse_response_emits_chunks_and_done_sentinel() {
        let chunks: Vec<LlmResult<ChatCompletionChunk>> = vec![
            Ok(make_chunk("c1", "Hello")),
            Ok(make_chunk("c2", " world")),
            Ok(make_chunk("c3", "!")),
        ];
        let mock_stream: BoxStream<'static, LlmResult<ChatCompletionChunk>> = Box::pin(stream::iter(chunks));

        let response = sse_response(mock_stream);

        assert_eq!(
            response
                .headers()
                .get("content-type")
                .expect("content-type header should be present")
                .to_str()
                .expect("content-type should be valid ASCII"),
            "text/event-stream"
        );

        let body = body_string(response).await;
        let events = parse_sse_data(&body);

        assert_eq!(events.len(), 4, "expected 4 events, got: {events:?}");

        let first: serde_json::Value = serde_json::from_str(&events[0]).expect("first event should be valid JSON");
        assert_eq!(first["id"], "c1");
        assert_eq!(first["choices"][0]["delta"]["content"], "Hello");

        assert_eq!(events[3], "[DONE]");
    }

    #[tokio::test]
    async fn sse_response_handles_error_chunks() {
        let chunks: Vec<LlmResult<ChatCompletionChunk>> = vec![
            Ok(make_chunk("c1", "partial")),
            Err(LiterLlmError::Streaming {
                message: "connection reset".into(),
            }),
        ];
        let mock_stream: BoxStream<'static, LlmResult<ChatCompletionChunk>> = Box::pin(stream::iter(chunks));

        let response = sse_response(mock_stream);
        let body = body_string(response).await;
        let events = parse_sse_data(&body);

        assert_eq!(events.len(), 3, "expected 3 events, got: {events:?}");

        assert!(
            events[1].contains("connection reset"),
            "error event should contain the error message, got: {}",
            events[1]
        );

        assert_eq!(events[2], "[DONE]");
    }

    #[tokio::test]
    async fn sse_error_payload_is_valid_json() {
        let chunks: Vec<LlmResult<ChatCompletionChunk>> = vec![Err(LiterLlmError::Streaming {
            message: "connection reset".into(),
        })];
        let mock_stream: BoxStream<'static, LlmResult<ChatCompletionChunk>> = Box::pin(stream::iter(chunks));

        let response = sse_response(mock_stream);
        let body = body_string(response).await;
        let events = parse_sse_data(&body);

        assert_eq!(events.len(), 2, "expected 2 events, got: {events:?}");

        let value: serde_json::Value = serde_json::from_str(&events[0]).expect("SSE error payload must be valid JSON");

        assert_eq!(value["error"]["type"], "Streaming", "error type mismatch: {value:?}");

        let msg_len = value["error"]["message"]
            .as_str()
            .expect("message must be a string")
            .chars()
            .count();
        assert!(msg_len <= 200, "message length {msg_len} exceeds 200 chars");
    }
}
