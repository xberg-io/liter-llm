//! The `ollama/` provider calls Ollama's own API (`/api/chat`, `/api/embed`)
//! so that `num_ctx`, `top_k` and `repeat_penalty` reach the model through `options`.
//! An Ollama hint routes a bare model there whatever the `base_url` is; one trailing `/v1`
//! comes off the address. The `openai/` prefix with an Ollama `base_url` keeps the
//! OpenAI-compatible path, where Ollama ignores them.

#![cfg(feature = "native-http")]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;

use futures_util::StreamExt;
use liter_llm::LiterLlmError;
use liter_llm::client::{ClientConfigBuilder, DefaultClient, LlmClient, LlmClientRaw};
use liter_llm::types::{ChatCompletionChunk, ChatCompletionRequest, EmbeddingRequest, FinishReason};
use serde_json::{Value, json};

/// What the server saw: the request line and the parsed JSON body.
struct Seen {
    request_line: String,
    body: Value,
}

/// Serve one response, then close the connection, and report the request.
fn server(status: u16, content_type: &'static str, body: String) -> (String, thread::JoinHandle<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind response server");
    let base_url = format!("http://{}", listener.local_addr().expect("server address"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        reader.read_line(&mut request_line).expect("read request line");
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read request headers");
            if line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                content_length = value.trim().parse().expect("request content length");
            }
        }
        let mut raw = vec![0; content_length];
        reader.read_exact(&mut raw).expect("read request body");
        write!(
            stream,
            "HTTP/1.1 {status} X\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write response");
        Seen {
            request_line: request_line.trim().to_owned(),
            body: if raw.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&raw).expect("request body is JSON")
            },
        }
    });
    (base_url, handle)
}

fn client(base_url: String) -> DefaultClient {
    let config = ClientConfigBuilder::new("").base_url(base_url).max_retries(0).build();
    DefaultClient::new(config, None).expect("create client")
}

fn chat_request(model: &str) -> ChatCompletionRequest {
    serde_json::from_value(json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hello"}],
        "temperature": 0.2,
        "max_tokens": 16,
        "stop": ["END"],
        "seed": 7,
        "extra_body": {"num_ctx": 512, "top_k": 5, "repeat_penalty": 1.1, "keep_alive": "10m"}
    }))
    .expect("chat request")
}

const CHAT_REPLY: &str = r#"{"model":"qwen2:0.5b","created_at":"2026-01-01T00:00:00Z","message":{"role":"assistant","content":"Hi there"},"done":true,"done_reason":"stop","prompt_eval_count":5,"eval_count":8}"#;

#[tokio::test]
async fn chat_goes_to_the_native_path_with_options() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    let response = client(url)
        .chat(chat_request("ollama/qwen2:0.5b"))
        .await
        .expect("chat succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1");
    assert_eq!(
        seen.body,
        json!({
            "model": "qwen2:0.5b",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": false,
            "keep_alive": "10m",
            "options": {
                "temperature": 0.2, "num_predict": 16, "stop": ["END"], "seed": 7,
                "num_ctx": 512, "top_k": 5, "repeat_penalty": 1.1
            }
        })
    );
    assert_eq!(response.model, "qwen2:0.5b");
    assert_eq!(response.choices[0].message.text().as_deref(), Some("Hi there"));
    assert_eq!(response.choices[0].finish_reason, Some(FinishReason::Stop));
    let usage = response.usage.expect("usage from the token counts");
    assert_eq!(
        (usage.prompt_tokens, usage.completion_tokens, usage.total_tokens),
        (5, 8, 13)
    );
}

#[tokio::test]
async fn options_in_extra_body_merge_with_typed_options() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    let mut request = chat_request("ollama/qwen2:0.5b");
    request.extra_body = Some(json!({"options": {"num_ctx": 1024, "mirostat": 1}, "top_k": 3}));
    client(url).chat(request).await.expect("chat succeeds");
    let options = seen.join().expect("server done").body["options"].clone();
    assert_eq!(options["num_ctx"], 1024);
    assert_eq!(options["mirostat"], 1);
    assert_eq!(options["top_k"], 3);
    assert_eq!(options["temperature"], 0.2, "typed field survives next to the extras");
}

#[tokio::test]
async fn ollama_chat_alias_uses_the_native_path_and_strips_the_prefix() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    client(url)
        .chat(chat_request("ollama_chat/qwen2:0.5b"))
        .await
        .expect("chat succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1");
    assert_eq!(seen.body["model"], "qwen2:0.5b");
}

#[tokio::test]
async fn openai_prefix_keeps_the_compatible_path() {
    let reply = r#"{"id":"c","object":"chat.completion","created":1,"model":"m","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"ok"}}]}"#;
    let (url, seen) = server(200, "application/json", reply.to_owned());
    client(url).chat(chat_request("openai/m")).await.expect("chat succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /chat/completions HTTP/1.1");
    assert_eq!(
        seen.body["num_ctx"], 512,
        "compatible path sends the extras at the top level"
    );
}

#[tokio::test]
async fn tool_calls_round_trip() {
    let reply = r#"{"model":"m","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"get_weather","arguments":{"city":"Tokyo"}}}]},"done":true,"done_reason":"stop","prompt_eval_count":3,"eval_count":4}"#;
    let (url, seen) = server(200, "application/json", reply.to_owned());
    let request: ChatCompletionRequest = serde_json::from_value(json!({
        "model": "ollama/m",
        "messages": [
            {"role": "user", "content": "weather?"},
            {"role": "assistant", "content": null, "tool_calls": [
                {"id": "call_1", "type": "function", "function": {"name": "get_weather", "arguments": "{\"city\":\"Rome\"}"}}]},
            {"role": "tool", "tool_call_id": "call_1", "content": "20 C"}
        ],
        "tools": [{"type": "function", "function": {"name": "get_weather", "parameters": {"type": "object"}}}],
        "response_format": {"type": "json_object"}
    }))
    .expect("request");
    let response = client(url).chat(request).await.expect("chat succeeds");
    let body = seen.join().expect("server done").body;
    assert_eq!(body["format"], "json");
    assert_eq!(body["tools"][0]["function"]["name"], "get_weather");
    assert_eq!(
        body["messages"][1]["tool_calls"][0]["function"]["arguments"],
        json!({"city": "Rome"})
    );
    assert_eq!(
        body["messages"][2],
        json!({"role": "tool", "content": "20 C", "tool_name": "get_weather"})
    );
    let choice = &response.choices[0];
    assert_eq!(choice.finish_reason, Some(FinishReason::ToolCalls));
    let call = &choice.message.tool_calls.as_ref().expect("tool calls")[0];
    assert_eq!(call.function.name, "get_weather");
    assert_eq!(call.function.arguments, r#"{"city":"Tokyo"}"#);
    assert!(!call.id.is_empty());
}

#[tokio::test]
async fn images_are_sent_as_base64_without_the_data_url_prefix() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    let request: ChatCompletionRequest = serde_json::from_value(json!({
        "model": "ollama/m",
        "messages": [{"role": "user", "content": [
            {"type": "text", "text": "what is this?"},
            {"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAA"}}]}]
    }))
    .expect("request");
    client(url).chat(request).await.expect("chat succeeds");
    let message = seen.join().expect("server done").body["messages"][0].clone();
    assert_eq!(
        message,
        json!({"role": "user", "content": "what is this?", "images": ["AAAA"]})
    );
}

#[tokio::test]
async fn length_stop_maps_to_length() {
    let reply = CHAT_REPLY.replace(r#""done_reason":"stop""#, r#""done_reason":"length""#);
    let (url, _seen) = server(200, "application/json", reply);
    let response = client(url).chat(chat_request("ollama/m")).await.expect("chat succeeds");
    assert_eq!(response.choices[0].finish_reason, Some(FinishReason::Length));
}

#[tokio::test]
async fn error_body_becomes_a_typed_error_with_ollama_message() {
    let (url, _seen) = server(
        404,
        "application/json",
        r#"{"error":"model \"nope\" not found, try pulling it first"}"#.to_owned(),
    );
    let error = client(url)
        .chat(chat_request("ollama/nope"))
        .await
        .expect_err("404 is an error");
    assert!(
        matches!(&error, LiterLlmError::NotFound { message } if message == "model \"nope\" not found, try pulling it first"),
        "got {error:?}"
    );
}

fn ndjson(lines: &[&str]) -> String {
    lines.iter().map(|line| format!("{line}\n")).collect()
}

async fn stream_items(body: String) -> (Seen, Vec<Result<ChatCompletionChunk, LiterLlmError>>) {
    let (url, seen) = server(200, "application/x-ndjson", body);
    let stream = client(url)
        .chat_stream(chat_request("ollama/m"))
        .await
        .expect("stream opens");
    let items = stream.collect::<Vec<_>>().await;
    (seen.join().expect("server done"), items)
}

const PART_1: &str = r#"{"model":"m","created_at":"t","message":{"role":"assistant","content":"Hel"},"done":false}"#;
const PART_2: &str = r#"{"model":"m","created_at":"t","message":{"role":"assistant","content":"lo"},"done":false}"#;
const LAST: &str = r#"{"model":"m","created_at":"t","message":{"role":"assistant","content":""},"done":true,"done_reason":"stop","prompt_eval_count":5,"eval_count":2}"#;

#[tokio::test]
async fn stream_reads_ndjson_lines() {
    let (seen, items) = stream_items(ndjson(&[PART_1, PART_2, LAST])).await;
    assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1");
    assert_eq!(seen.body["stream"], true);
    assert_eq!(seen.body["options"]["num_ctx"], 512);
    let chunks: Vec<_> = items.into_iter().collect::<Result<_, _>>().expect("no stream errors");
    let text: String = chunks
        .iter()
        .filter_map(|c| c.choices.first().and_then(|choice| choice.delta.content.clone()))
        .collect();
    assert_eq!(text, "Hello");
    let last = chunks.last().expect("final chunk");
    assert_eq!(last.choices[0].finish_reason, Some(FinishReason::Stop));
    let usage = last.usage.as_ref().expect("usage on the final chunk");
    assert_eq!(
        (usage.prompt_tokens, usage.completion_tokens, usage.total_tokens),
        (5, 2, 7)
    );
}

#[tokio::test]
async fn raw_stream_reads_ndjson_lines() {
    let (url, seen) = server(200, "application/x-ndjson", ndjson(&[PART_1, PART_2, LAST]));
    let exchange = client(url)
        .chat_stream_raw(chat_request("ollama/m"))
        .await
        .expect("raw stream opens");
    let items = exchange.stream.collect::<Vec<_>>().await;
    assert_eq!(
        seen.join().expect("server done").request_line,
        "POST /api/chat HTTP/1.1"
    );
    let chunks: Vec<_> = items.into_iter().collect::<Result<_, _>>().expect("no stream errors");
    let text: String = chunks
        .iter()
        .filter_map(|c| c.choices.first().and_then(|choice| choice.delta.content.clone()))
        .collect();
    assert_eq!(text, "Hello");
    assert_eq!(
        chunks.last().expect("final chunk").choices[0].finish_reason,
        Some(FinishReason::Stop)
    );
}

#[tokio::test]
async fn stream_tool_calls_get_distinct_indexes_and_finish_with_tool_calls() {
    let call = |name: &str| {
        format!(
            r#"{{"model":"m","message":{{"role":"assistant","content":"","tool_calls":[{{"function":{{"name":"{name}","arguments":{{"a":1}}}}}}]}},"done":false}}"#
        )
    };
    let (first, second) = (call("one"), call("two"));
    let (_, items) = stream_items(ndjson(&[&first, &second, LAST])).await;
    let chunks: Vec<_> = items.into_iter().collect::<Result<_, _>>().expect("no stream errors");
    let calls: Vec<_> = chunks
        .iter()
        .flat_map(|c| c.choices.iter())
        .filter_map(|choice| choice.delta.tool_calls.clone())
        .flatten()
        .collect();
    assert_eq!(calls.len(), 2);
    assert_eq!((calls[0].index, calls[1].index), (0, 1));
    assert_ne!(calls[0].id, calls[1].id);
    assert_eq!(
        calls[1].function.as_ref().and_then(|f| f.arguments.as_deref()),
        Some(r#"{"a":1}"#)
    );
    let last = chunks.last().expect("final chunk");
    assert_eq!(last.choices[0].finish_reason, Some(FinishReason::ToolCalls));
}

#[tokio::test]
async fn stream_thinking_becomes_reasoning_content() {
    let think = r#"{"model":"m","message":{"role":"assistant","content":"","thinking":"hmm"},"done":false}"#;
    let (_, items) = stream_items(ndjson(&[think, PART_1, LAST])).await;
    let chunks: Vec<_> = items.into_iter().collect::<Result<_, _>>().expect("no stream errors");
    assert_eq!(chunks[0].choices[0].delta.reasoning_content.as_deref(), Some("hmm"));
}

#[tokio::test]
async fn stream_cut_before_the_done_line_is_an_error() {
    let (_, items) = stream_items(ndjson(&[PART_1, PART_2])).await;
    let last = items.last().expect("items");
    assert!(
        matches!(last, Err(LiterLlmError::Streaming { message }) if message.contains("final event")),
        "got {last:?}"
    );
}

#[tokio::test]
async fn stream_error_line_is_an_error() {
    let (_, items) = stream_items(ndjson(&[PART_1, r#"{"error":"out of memory"}"#])).await;
    let last = items.last().expect("items");
    assert!(
        matches!(last, Err(error) if error.to_string().contains("out of memory")),
        "got {last:?}"
    );
}

#[tokio::test]
async fn embed_goes_to_the_native_path() {
    let reply = r#"{"model":"all-minilm","embeddings":[[0.1,0.2],[0.3,0.4]],"prompt_eval_count":4}"#;
    let (url, seen) = server(200, "application/json", reply.to_owned());
    let request: EmbeddingRequest = serde_json::from_value(json!({
        "model": "ollama/all-minilm", "input": ["a", "b"], "dimensions": 2
    }))
    .expect("request");
    let response = client(url).embed(request).await.expect("embed succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /api/embed HTTP/1.1");
    assert_eq!(
        seen.body,
        json!({"model": "all-minilm", "input": ["a", "b"], "dimensions": 2})
    );
    assert_eq!(response.model, "all-minilm");
    assert_eq!(response.data.len(), 2);
    assert_eq!(response.data[1].index, 1);
    assert_eq!(response.data[1].embedding, vec![0.3, 0.4]);
    assert_eq!(response.usage.expect("usage").prompt_tokens, 4);
}

#[tokio::test]
async fn list_models_reads_the_native_tags_reply() {
    let reply = r#"{"models":[{"name":"qwen2:0.5b","model":"qwen2:0.5b","modified_at":"2026-01-01T00:00:00Z","size":1,"digest":"d1"},{"name":"all-minilm:latest","model":"all-minilm:latest","modified_at":"2026-01-02T00:00:00Z","size":2,"digest":"d2"}]}"#;
    let (url, seen) = server(200, "application/json", reply.to_owned());
    let config = ClientConfigBuilder::new("").base_url(url).max_retries(0).build();
    let client = DefaultClient::new(config, Some("ollama/qwen2:0.5b")).expect("create client");
    let response = client.list_models().await.expect("list_models succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "GET /api/tags HTTP/1.1");
    assert_eq!(response.object, "list");
    let ids: Vec<&str> = response.data.iter().map(|model| model.id.as_str()).collect();
    assert_eq!(ids, ["qwen2:0.5b", "all-minilm:latest"]);
    assert!(
        response
            .data
            .iter()
            .all(|model| model.object == "model" && model.owned_by == "ollama")
    );
}

fn chat_request_with(model: &str, extra: Value) -> ChatCompletionRequest {
    let mut body = json!({"model": model, "messages": [{"role": "user", "content": "Hello"}]});
    body.as_object_mut()
        .expect("object")
        .extend(extra.as_object().expect("extra object").clone());
    serde_json::from_value(body).expect("chat request")
}

#[tokio::test]
async fn error_text_of_another_provider_is_the_raw_body() {
    let raw = r#"{"error":"model not found"}"#;
    let (url, _seen) = server(404, "application/json", raw.to_owned());
    let error = client(url)
        .chat(chat_request("openai/m"))
        .await
        .expect_err("404 is an error");
    assert!(
        matches!(&error, LiterLlmError::NotFound { message } if message == raw),
        "got {error:?}"
    );
}

#[tokio::test]
async fn reasoning_effort_becomes_think() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    client(url)
        .chat(chat_request_with("ollama/m", json!({"reasoning_effort": "high"})))
        .await
        .expect("chat succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.body["think"], "high");
    assert!(seen.body.get("reasoning_effort").is_none());
}

#[tokio::test]
async fn extra_body_think_beats_reasoning_effort() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    client(url)
        .chat(chat_request_with(
            "ollama/m",
            json!({"reasoning_effort": "high", "extra_body": {"think": false}}),
        ))
        .await
        .expect("chat succeeds");
    assert_eq!(seen.join().expect("server done").body["think"], false);
}

#[tokio::test]
async fn logprobs_reach_ollama_and_come_back() {
    let reply = CHAT_REPLY.replace(
        r#""done":true"#,
        r#""logprobs":[{"token":"Hi","logprob":-0.1,"top_logprobs":[]}],"done":true"#,
    );
    let (url, seen) = server(200, "application/json", reply);
    let response = client(url)
        .chat(chat_request_with(
            "ollama/m",
            json!({"logprobs": true, "top_logprobs": 2}),
        ))
        .await
        .expect("chat succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.body["logprobs"], true);
    assert_eq!(seen.body["top_logprobs"], 2);
    let logprobs = response.choices[0].logprobs.as_ref().expect("logprobs in the reply");
    assert_eq!(logprobs["content"][0]["token"], "Hi");
}

#[tokio::test]
async fn fields_without_a_native_equivalent_are_not_sent() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    client(url)
        .chat(chat_request_with(
            "ollama/m",
            json!({"n": 2, "user": "u", "logit_bias": {"1": 1.0}, "parallel_tool_calls": true,
                   "service_tier": "auto", "store": true, "stream_options": {"include_usage": true},
                   "metadata": {"k": "v"}}),
        ))
        .await
        .expect("chat succeeds");
    let body = seen.join().expect("server done").body;
    let keys: Vec<&str> = body.as_object().expect("object").keys().map(String::as_str).collect();
    assert_eq!(keys, ["messages", "model", "stream"], "got {keys:?}");
}

#[tokio::test]
async fn embed_drops_encoding_format_and_user() {
    let reply = r#"{"model":"m","embeddings":[[0.1]],"prompt_eval_count":1}"#;
    let (url, seen) = server(200, "application/json", reply.to_owned());
    let request: EmbeddingRequest =
        serde_json::from_value(json!({"model": "ollama/m", "input": "a", "encoding_format": "float", "user": "u"}))
            .expect("embed request");
    client(url).embed(request).await.expect("embed succeeds");
    let body = seen.join().expect("server done").body;
    let keys: Vec<&str> = body.as_object().expect("object").keys().map(String::as_str).collect();
    assert_eq!(keys, ["input", "model"], "got {keys:?}");
}

#[tokio::test]
async fn bare_hint_lists_models_from_the_native_tags_path() {
    for hint in ["ollama", "ollama_chat"] {
        let (url, seen) = server(200, "application/json", r#"{"models":[]}"#.to_owned());
        let config = ClientConfigBuilder::new("").base_url(url).max_retries(0).build();
        let client = DefaultClient::new(config, Some(hint)).expect("create client");
        client.list_models().await.expect("list_models succeeds");
        assert_eq!(seen.join().expect("server done").request_line, "GET /api/tags HTTP/1.1");
    }
}

#[tokio::test]
async fn stream_that_stops_on_the_token_limit_finishes_with_length() {
    let last = LAST.replace(r#""done_reason":"stop""#, r#""done_reason":"length""#);
    let (_, items) = stream_items(ndjson(&[PART_1, &last])).await;
    let chunks: Vec<_> = items.into_iter().collect::<Result<_, _>>().expect("no stream errors");
    let finish = chunks.last().and_then(|chunk| chunk.choices[0].finish_reason.clone());
    assert_eq!(finish, Some(FinishReason::Length));
}

#[tokio::test]
async fn unterminated_last_line_is_called_ndjson_not_sse() {
    let (_, items) = stream_items(format!("{PART_1}\n{PART_2}")).await;
    let last = items.last().expect("items");
    assert!(
        matches!(last, Err(LiterLlmError::Streaming { message }) if message.starts_with("NDJSON stream truncated")),
        "got {last:?}"
    );
}

#[tokio::test]
async fn tool_call_only_reply_keeps_empty_content() {
    let reply = r#"{"model":"m","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"f","arguments":{}}}]},"done":true,"done_reason":"stop"}"#;
    let (url, _seen) = server(200, "application/json", reply.to_owned());
    let response = client(url).chat(chat_request("ollama/m")).await.expect("chat succeeds");
    let message = serde_json::to_value(&response.choices[0].message).expect("message json");
    assert_eq!(message["content"], "");
}

#[tokio::test]
async fn list_models_reads_modified_at_into_created() {
    let reply = r#"{"models":[{"name":"a","modified_at":"2026-01-01T00:00:00Z"},{"name":"b","modified_at":"2026-01-01T01:00:00+01:00"},{"name":"c"}]}"#;
    let (url, _seen) = server(200, "application/json", reply.to_owned());
    let config = ClientConfigBuilder::new("").base_url(url).max_retries(0).build();
    let client = DefaultClient::new(config, Some("ollama/a")).expect("create client");
    let created: Vec<u64> = client
        .list_models()
        .await
        .expect("list_models succeeds")
        .data
        .iter()
        .map(|model| model.created)
        .collect();
    assert_eq!(created, [1_767_225_600, 1_767_225_600, 0]);
}

const EMBED_REPLY: &str = r#"{"model":"m","embeddings":[[0.1,0.2]],"prompt_eval_count":1}"#;

/// Build a client hinted at Ollama for the server at `url`, with `suffix` after the address.
fn hinted_client(hint: &str, url: &str, suffix: &str) -> DefaultClient {
    let config = ClientConfigBuilder::new("")
        .base_url(format!("{url}{suffix}"))
        .max_retries(0)
        .build();
    DefaultClient::new(config, Some(hint)).expect("create client")
}

#[tokio::test]
async fn hinted_client_sends_a_bare_model_to_the_native_api_whatever_the_base_url_is() {
    for hint in ["ollama", "ollama_chat", "ollama/other", "ollama_chat/other"] {
        for suffix in ["", "/", "/v1", "/v1/"] {
            let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
            hinted_client(hint, &url, suffix)
                .chat(chat_request("m"))
                .await
                .expect("chat succeeds");
            let seen = seen.join().expect("server done");
            assert_eq!(
                seen.request_line, "POST /api/chat HTTP/1.1",
                "hint {hint}, base_url suffix {suffix:?}"
            );
            assert_eq!(seen.body["model"], "m", "hint {hint}, base_url suffix {suffix:?}");
            assert_eq!(
                seen.body["options"]["num_ctx"], 512,
                "hint {hint}, base_url suffix {suffix:?}"
            );

            let (url, seen) = server(200, "application/json", EMBED_REPLY.to_owned());
            let request: EmbeddingRequest =
                serde_json::from_value(json!({"model": "m", "input": "a"})).expect("embed request");
            hinted_client(hint, &url, suffix)
                .embed(request)
                .await
                .expect("embed succeeds");
            let seen = seen.join().expect("server done");
            assert_eq!(
                seen.request_line, "POST /api/embed HTTP/1.1",
                "hint {hint}, base_url suffix {suffix:?}"
            );
            assert_eq!(seen.body["model"], "m", "hint {hint}, base_url suffix {suffix:?}");

            let (url, seen) = server(200, "application/json", r#"{"models":[]}"#.to_owned());
            hinted_client(hint, &url, suffix)
                .list_models()
                .await
                .expect("list_models succeeds");
            let seen = seen.join().expect("server done");
            assert_eq!(
                seen.request_line, "GET /api/tags HTTP/1.1",
                "hint {hint}, base_url suffix {suffix:?}"
            );
        }
    }
}

#[tokio::test]
async fn prefixed_model_with_a_v1_base_url_goes_to_the_native_path() {
    for model in ["ollama/m", "ollama_chat/m"] {
        let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
        hinted_client("openai", &url, "/v1")
            .chat(chat_request(model))
            .await
            .expect("chat succeeds");
        let seen = seen.join().expect("server done");
        assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1", "model {model}");
        assert_eq!(seen.body["model"], "m", "model {model}");

        let (url, seen) = server(200, "application/json", EMBED_REPLY.to_owned());
        let request: EmbeddingRequest =
            serde_json::from_value(json!({"model": model, "input": "a"})).expect("embed request");
        client(format!("{url}/v1"))
            .embed(request)
            .await
            .expect("embed succeeds");
        let seen = seen.join().expect("server done");
        assert_eq!(seen.request_line, "POST /api/embed HTTP/1.1", "model {model}");
        assert_eq!(seen.body["model"], "m", "model {model}");
    }
}

const COMPATIBLE_CHAT_REPLY: &str = r#"{"id":"c","object":"chat.completion","created":1,"model":"m","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"ok"}}]}"#;
const COMPATIBLE_EMBED_REPLY: &str = r#"{"object":"list","data":[{"object":"embedding","index":0,"embedding":[0.1,0.2]}],"model":"m","usage":{"prompt_tokens":1,"total_tokens":1}}"#;

/// Only a bare model goes to the native API of a hinted client. A model with another provider's
/// prefix goes to the OpenAI-compatible endpoint at the `base_url` as given, with the model name
/// as given, as a client without the hint sends it; `custom/` comes off as it does there.
#[tokio::test]
async fn hinted_client_sends_a_prefixed_model_to_the_compatible_path() {
    for hint in ["ollama", "ollama_chat", "ollama/other"] {
        for suffix in ["", "/v1"] {
            for (model, sent) in [
                ("openai/m", "openai/m"),
                ("anthropic/x", "anthropic/x"),
                ("groq/m", "groq/m"),
                ("snowflake/m", "snowflake/m"),
                ("openrouter/anthropic/claude", "openrouter/anthropic/claude"),
                ("custom/m", "m"),
            ] {
                let (url, seen) = server(200, "application/json", COMPATIBLE_CHAT_REPLY.to_owned());
                hinted_client(hint, &url, suffix)
                    .chat(chat_request(model))
                    .await
                    .expect("chat succeeds");
                let seen = seen.join().expect("server done");
                let case = format!("hint {hint}, base_url suffix {suffix:?}, model {model}");
                assert_eq!(
                    seen.request_line,
                    format!("POST {suffix}/chat/completions HTTP/1.1"),
                    "{case}"
                );
                assert_eq!(seen.body["model"], sent, "{case}");
                assert_eq!(seen.body["num_ctx"], 512, "{case}: extras stay at the top level");
                assert!(seen.body.get("options").is_none(), "{case}: no native options");
            }
        }
    }

    let (url, seen) = server(200, "application/json", COMPATIBLE_EMBED_REPLY.to_owned());
    let request: EmbeddingRequest =
        serde_json::from_value(json!({"model": "openai/m", "input": "a"})).expect("embed request");
    hinted_client("ollama", &url, "/v1")
        .embed(request)
        .await
        .expect("embed succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /v1/embeddings HTTP/1.1");
    assert_eq!(seen.body["model"], "openai/m");

    // ~keep A slash alone makes no provider prefix: these are Ollama model names, so they are bare.
    for model in ["hf.co/user/model", "library/llama3"] {
        let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
        hinted_client("ollama", &url, "/v1")
            .chat(chat_request(model))
            .await
            .expect("chat succeeds");
        let seen = seen.join().expect("server done");
        assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1", "model {model}");
        assert_eq!(seen.body["model"], model, "model {model}");
        assert_eq!(seen.body["options"]["num_ctx"], 512, "model {model}");
    }
}

/// The `ollama/` and `ollama_chat/` prefixes name the native API on a hinted client too: the
/// prefix comes off and the model goes to `/api/chat`, not to the compatible path.
#[tokio::test]
async fn hinted_client_sends_an_ollama_prefixed_model_to_the_native_api() {
    for model in ["ollama/m", "ollama_chat/m"] {
        for suffix in ["", "/v1"] {
            let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
            hinted_client("ollama", &url, suffix)
                .chat(chat_request(model))
                .await
                .expect("chat succeeds");
            let seen = seen.join().expect("server done");
            let case = format!("base_url suffix {suffix:?}, model {model}");
            assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1", "{case}");
            assert_eq!(seen.body["model"], "m", "{case}");
            assert_eq!(seen.body["options"]["num_ctx"], 512, "{case}");
        }
    }
}

/// A prefix the client does not know is no provider prefix: `acme/m` is bare and goes native.
/// Once a custom provider named `acme` is registered, `acme/m` goes to the compatible path as
/// given, as a client without the hint sends it.
#[tokio::test]
async fn custom_provider_prefix_goes_to_the_compatible_path_once_registered() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    hinted_client("ollama", &url, "/v1")
        .chat(chat_request("acme/m"))
        .await
        .expect("chat succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1", "no acme provider");
    assert_eq!(seen.body["model"], "acme/m", "no acme provider");
    assert_eq!(seen.body["options"]["num_ctx"], 512, "no acme provider");

    liter_llm::register_custom_provider(liter_llm::CustomProviderConfig {
        name: "acme".into(),
        base_url: "https://api.acme.example/v1".into(),
        auth_header: liter_llm::AuthHeaderFormat::Bearer,
        model_prefixes: vec!["acme/".into()],
    })
    .expect("register the custom provider");
    for suffix in ["", "/v1"] {
        let (url, seen) = server(200, "application/json", COMPATIBLE_CHAT_REPLY.to_owned());
        hinted_client("ollama", &url, suffix)
            .chat(chat_request("acme/m"))
            .await
            .expect("chat succeeds");
        let seen = seen.join().expect("server done");
        let case = format!("acme registered, base_url suffix {suffix:?}");
        assert_eq!(
            seen.request_line,
            format!("POST {suffix}/chat/completions HTTP/1.1"),
            "{case}"
        );
        assert_eq!(seen.body["model"], "acme/m", "{case}");
        assert_eq!(seen.body["num_ctx"], 512, "{case}: extras stay at the top level");
        assert!(seen.body.get("options").is_none(), "{case}: no native options");
    }
    assert!(liter_llm::unregister_custom_provider("acme").expect("unregister the custom provider"));
}

/// Send `model` through a client hinted at Ollama and report what the server saw. The server
/// answers `reply`; the call result is returned beside it so a wrong route shows its path first.
async fn hinted_chat(suffix: &str, model: &str, reply: &str) -> (Seen, Result<(), LiterLlmError>) {
    let (url, seen) = server(200, "application/json", reply.to_owned());
    let result = hinted_client("ollama", &url, suffix)
        .chat(chat_request(model))
        .await
        .map(|_| ());
    (seen.join().expect("server done"), result)
}

/// Provider prefixes are case sensitive: `Openai/m` names no provider, so a hinted client sends
/// it to the native API as a bare model, with the name as given.
#[tokio::test]
async fn capitalised_builtin_prefix_is_a_bare_model_on_a_hinted_client() {
    for model in ["Openai/m", "OPENAI/m", "Groq/m", "Anthropic/x"] {
        for suffix in ["", "/v1"] {
            let (seen, result) = hinted_chat(suffix, model, CHAT_REPLY).await;
            let case = format!("base_url suffix {suffix:?}, model {model}");
            assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1", "{case}");
            assert_eq!(seen.body["model"], model, "{case}");
            assert_eq!(seen.body["options"]["num_ctx"], 512, "{case}");
            result.expect("chat succeeds");
        }
    }
    // ~keep Control: the lower case prefix names a provider, so the same client takes the compatible path.
    let (seen, result) = hinted_chat("", "openai/m", COMPATIBLE_CHAT_REPLY).await;
    assert_eq!(seen.request_line, "POST /chat/completions HTTP/1.1", "control openai/m");
    result.expect("chat succeeds");
}

/// A registered custom provider name is matched case sensitively: `Casecorp/m` is a bare model
/// on a hinted client while `casecorp` is registered, and `casecorp/m` is not.
#[tokio::test]
async fn custom_provider_name_is_matched_case_sensitively_on_a_hinted_client() {
    liter_llm::register_custom_provider(liter_llm::CustomProviderConfig {
        name: "casecorp".into(),
        base_url: "https://api.casecorp.example/v1".into(),
        auth_header: liter_llm::AuthHeaderFormat::Bearer,
        model_prefixes: vec!["casecorp/".into()],
    })
    .expect("register the custom provider");
    for model in ["Casecorp/m", "CASECORP/m"] {
        for suffix in ["", "/v1"] {
            let (seen, result) = hinted_chat(suffix, model, CHAT_REPLY).await;
            let case = format!("base_url suffix {suffix:?}, model {model}");
            assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1", "{case}");
            assert_eq!(seen.body["model"], model, "{case}");
            result.expect("chat succeeds");
        }
    }
    // ~keep Control: the registered spelling takes the compatible path, so the registration is live here.
    let (seen, result) = hinted_chat("", "casecorp/m", COMPATIBLE_CHAT_REPLY).await;
    assert_eq!(
        seen.request_line, "POST /chat/completions HTTP/1.1",
        "control casecorp/m"
    );
    result.expect("chat succeeds");
    assert!(liter_llm::unregister_custom_provider("casecorp").expect("unregister the custom provider"));
}

/// The `ollama/` and `ollama_chat/` prefixes are case sensitive: on a client without a hint,
/// `Ollama/m` names no provider and goes to the OpenAI-compatible path with the name as given.
#[tokio::test]
async fn capitalised_ollama_prefix_keeps_the_compatible_path_without_a_hint() {
    for model in ["Ollama/m", "OLLAMA/m", "Ollama_Chat/m", "OLLAMA_CHAT/m"] {
        for suffix in ["", "/v1"] {
            let (url, seen) = server(200, "application/json", COMPATIBLE_CHAT_REPLY.to_owned());
            let result = client(format!("{url}{suffix}")).chat(chat_request(model)).await;
            let seen = seen.join().expect("server done");
            let case = format!("base_url suffix {suffix:?}, model {model}");
            assert_eq!(
                seen.request_line,
                format!("POST {suffix}/chat/completions HTTP/1.1"),
                "{case}"
            );
            assert_eq!(seen.body["model"], model, "{case}");
            assert!(seen.body.get("options").is_none(), "{case}: no native options");
            result.expect("chat succeeds");
        }
    }
    // ~keep Control: the lower case prefix names the native provider on the same client.
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    client(url).chat(chat_request("ollama/m")).await.expect("chat succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1", "control ollama/m");
}

/// Ollama hints are case sensitive: a client built with the hint `OLLAMA` or `Ollama/llama3` is
/// a plain OpenAI-compatible client, so a bare model goes to the compatible path, not native.
#[tokio::test]
async fn capitalised_ollama_hint_is_a_plain_compatible_client() {
    for hint in [
        "OLLAMA",
        "Ollama",
        "OLLAMA_CHAT",
        "Ollama_Chat",
        "Ollama/llama3",
        "OLLAMA_CHAT/m",
    ] {
        for suffix in ["", "/v1"] {
            let (url, seen) = server(200, "application/json", COMPATIBLE_CHAT_REPLY.to_owned());
            let result = hinted_client(hint, &url, suffix).chat(chat_request("m")).await;
            let seen = seen.join().expect("server done");
            let case = format!("hint {hint}, base_url suffix {suffix:?}");
            assert_eq!(
                seen.request_line,
                format!("POST {suffix}/chat/completions HTTP/1.1"),
                "{case}"
            );
            assert_eq!(seen.body["model"], "m", "{case}");
            assert_eq!(seen.body["num_ctx"], 512, "{case}: extras stay at the top level");
            assert!(seen.body.get("options").is_none(), "{case}: no native options");
            result.expect("chat succeeds");
        }
    }
    // ~keep Control: the lower case hints name Ollama, so the same bare model goes native.
    for hint in ["ollama", "ollama/llama3"] {
        let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
        hinted_client(hint, &url, "")
            .chat(chat_request("m"))
            .await
            .expect("chat succeeds");
        let seen = seen.join().expect("server done");
        assert_eq!(seen.request_line, "POST /api/chat HTTP/1.1", "control hint {hint}");
    }
}

/// A custom provider registered under the name `ollama` is an OpenAI-compatible server, not the
/// native provider: its stream keeps the compatible end-of-stream rule, whose error names the
/// missing final event and not the provider, and its tool call indexes. The native provider's own cut stream is `stream_cut_before_the_done_line_is_an_error`.
#[tokio::test]
async fn custom_provider_named_ollama_keeps_the_compatible_stream_rules() {
    let tool_call = |arguments: &str| {
        format!(
            r#"data: {{"id":"1","object":"chat.completion.chunk","created":1,"model":"m","choices":[{{"index":0,"delta":{{"role":"assistant","tool_calls":[{{"index":0,"id":"call_a","type":"function","function":{{"name":"f","arguments":"{arguments}"}}}}]}},"finish_reason":null}}]}}"#
        )
    };
    let body = format!("{}\n\n{}\n\n", tool_call(r#"{\"a\""#), tool_call(r#":1}"#));
    let (url, seen) = server(200, "text/event-stream", body);
    liter_llm::register_custom_provider(liter_llm::CustomProviderConfig {
        name: "ollama".into(),
        base_url: format!("{url}/v1"),
        auth_header: liter_llm::AuthHeaderFormat::None,
        model_prefixes: vec!["myol-".into()],
    })
    .expect("register the custom provider");
    let config = ClientConfigBuilder::new("k").max_retries(0).build();
    let client = DefaultClient::new(config, None).expect("create client");
    let stream = client.chat_stream(chat_request("myol-x")).await.expect("stream opens");
    let items = stream.collect::<Vec<_>>().await;
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /v1/chat/completions HTTP/1.1");
    let (last, content) = items.split_last().expect("stream yields items");
    let Err(LiterLlmError::Streaming { message }) = last else {
        panic!("a compatible stream cut without [DONE] or a finish chunk must end in an error, got {last:?}");
    };
    assert!(
        message.contains("before its final event"),
        "the generic reader error names the missing final event: {message}"
    );
    assert!(
        !message.starts_with("ollama"),
        "not the native provider's error: {message}"
    );
    let chunks: Vec<_> = content
        .iter()
        .map(|item| item.as_ref().expect("content before the cut arrives"))
        .collect();
    let calls: Vec<_> = chunks
        .iter()
        .flat_map(|c| c.choices.iter())
        .filter_map(|choice| choice.delta.tool_calls.clone())
        .flatten()
        .collect();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        (calls[0].index, calls[1].index),
        (0, 0),
        "the indexes are the server's: both fragments belong to one call"
    );
    assert_eq!(calls[1].id.as_deref(), Some("call_a"));
    assert!(chunks.iter().all(|c| c.choices[0].finish_reason.is_none()));
}

#[tokio::test]
async fn openai_prefix_with_a_v1_base_url_keeps_the_compatible_path() {
    let openai_reply = COMPATIBLE_CHAT_REPLY;
    for hint in ["openai", "openai/m"] {
        let (url, seen) = server(200, "application/json", openai_reply.to_owned());
        hinted_client(hint, &url, "/v1")
            .chat(chat_request("openai/m"))
            .await
            .expect("chat succeeds");
        let seen = seen.join().expect("server done");
        assert_eq!(seen.request_line, "POST /v1/chat/completions HTTP/1.1", "hint {hint}");
        assert_eq!(seen.body["model"], "m", "hint {hint}: the openai/ prefix comes off");
        assert_eq!(seen.body["num_ctx"], 512, "hint {hint}: extras stay at the top level");
    }
    let (url, seen) = server(200, "application/json", openai_reply.to_owned());
    client(format!("{url}/v1"))
        .chat(chat_request("openai/m"))
        .await
        .expect("chat succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /v1/chat/completions HTTP/1.1", "no hint");
    assert_eq!(seen.body["model"], "openai/m", "no hint: the model is sent as given");
}

const NOT_FOUND_BODY: &str = r#"{"error":"model \"nope\" not found, try pulling it first"}"#;
const NOT_FOUND_TEXT: &str = "model \"nope\" not found, try pulling it first";

fn assert_ollama_text(error: LiterLlmError, entry_point: &str) {
    assert!(
        matches!(&error, LiterLlmError::NotFound { message } if message == NOT_FOUND_TEXT),
        "{entry_point}: got {error:?}"
    );
}

fn embed_request() -> EmbeddingRequest {
    serde_json::from_value(json!({"model": "ollama/nope", "input": ["a"]})).expect("embed request")
}

#[tokio::test]
async fn every_entry_point_shows_the_plain_error_text() {
    let reply = || server(404, "application/json", NOT_FOUND_BODY.to_owned()).0;

    let error = client(reply()).chat_stream(chat_request("ollama/nope")).await;
    assert_ollama_text(error.err().expect("stream open fails"), "chat_stream");

    let error = client(reply()).embed(embed_request()).await;
    assert_ollama_text(error.expect_err("embed fails"), "embed");

    let config = ClientConfigBuilder::new("").base_url(reply()).max_retries(0).build();
    let hinted = DefaultClient::new(config, Some("ollama")).expect("create client");
    assert_ollama_text(hinted.list_models().await.expect_err("list fails"), "list_models");

    let error = client(reply()).chat_raw(chat_request("ollama/nope")).await;
    assert_ollama_text(error.expect_err("raw chat fails"), "chat_raw");

    let error = client(reply()).chat_stream_raw(chat_request("ollama/nope")).await;
    assert_ollama_text(error.err().expect("raw stream open fails"), "chat_stream_raw");

    let error = client(reply()).embed_raw(embed_request()).await;
    assert_ollama_text(error.expect_err("raw embed fails"), "embed_raw");
}

#[tokio::test]
async fn every_reasoning_effort_reaches_ollama_as_its_think_level() {
    for (effort, think) in [
        ("minimal", json!("low")),
        ("low", json!("low")),
        ("medium", json!("medium")),
        ("high", json!("high")),
        ("max", json!("high")),
        ("none", json!(false)),
    ] {
        let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
        client(url)
            .chat(chat_request_with("ollama/m", json!({"reasoning_effort": effort})))
            .await
            .expect("chat succeeds");
        let body = seen.join().expect("server done").body;
        assert_eq!(body["think"], think, "reasoning_effort {effort}");
        assert!(body.get("reasoning_effort").is_none(), "reasoning_effort {effort}");
    }
}

#[tokio::test]
async fn image_data_url_without_base64_is_refused() {
    let request: ChatCompletionRequest = serde_json::from_value(json!({
        "model": "ollama/m",
        "messages": [{"role": "user", "content": [
            {"type": "image_url", "image_url": {"url": "data:image/svg+xml,%3Csvg%2F%3E"}}]}]
    }))
    .expect("request");
    // ~keep No server listens here: the request must be refused before anything is sent.
    let error = client("http://127.0.0.1:9".into())
        .chat(request)
        .await
        .expect_err("a data URL without ;base64 is refused");
    assert!(
        matches!(&error, LiterLlmError::BadRequest { message, .. } if message.contains("base64")),
        "got {error:?}"
    );
}

#[tokio::test]
async fn null_option_values_are_not_sent() {
    let (url, seen) = server(200, "application/json", CHAT_REPLY.to_owned());
    let mut request = chat_request("ollama/m");
    request.extra_body = Some(json!({
        "num_ctx": null,
        "keep_alive": null,
        "top_k": 5,
        "options": {"num_thread": null, "mirostat": 1}
    }));
    client(url).chat(request).await.expect("chat succeeds");
    let body = seen.join().expect("server done").body;
    assert!(body.get("keep_alive").is_none(), "{body}");
    let options = body["options"].clone();
    assert!(options.get("num_ctx").is_none(), "{options}");
    assert!(options.get("num_thread").is_none(), "{options}");
    assert_eq!(options["top_k"], 5, "{options}");
    assert_eq!(options["mirostat"], 1, "{options}");
}

#[tokio::test]
async fn raw_stream_cut_before_the_done_line_is_an_error() {
    let (url, _seen) = server(200, "application/x-ndjson", ndjson(&[PART_1, PART_2]));
    let exchange = client(url)
        .chat_stream_raw(chat_request("ollama/m"))
        .await
        .expect("raw stream opens");
    let items = exchange.stream.collect::<Vec<_>>().await;
    let (last, content) = items.split_last().expect("items");
    assert!(
        content.iter().all(Result::is_ok),
        "content before the cut arrives: {items:?}"
    );
    assert!(
        matches!(last, Err(LiterLlmError::Streaming { message }) if message.contains("final event")),
        "got {last:?}"
    );
}

/// Send an `ollama/m` embed request with the given `extra_body` and return what the server saw.
async fn embed_with_extra_body(model: &str, input: Value, extra_body: Value) -> Seen {
    let reply = r#"{"model":"m","embeddings":[[0.1,0.2]],"prompt_eval_count":1}"#;
    let (url, seen) = server(200, "application/json", reply.to_owned());
    let mut request: EmbeddingRequest =
        serde_json::from_value(json!({"model": model, "input": input})).expect("embed request");
    request.extra_body = Some(extra_body);
    client(url).embed(request).await.expect("embed succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /api/embed HTTP/1.1");
    seen
}

#[tokio::test]
async fn embed_extra_body_top_level_keys_stay_at_the_top() {
    let seen = embed_with_extra_body(
        "ollama/m",
        json!(["a", "b"]),
        json!({"truncate": false, "keep_alive": "10m"}),
    )
    .await;
    assert_eq!(
        seen.body,
        json!({"model": "m", "input": ["a", "b"], "truncate": false, "keep_alive": "10m"})
    );
}

#[tokio::test]
async fn embed_extra_body_bare_setting_reaches_options() {
    let seen = embed_with_extra_body("ollama/m", json!("a"), json!({"num_ctx": 8192})).await;
    assert_eq!(
        seen.body,
        json!({"model": "m", "input": "a", "options": {"num_ctx": 8192}})
    );
}

#[tokio::test]
async fn embed_extra_body_explicit_options_win_over_bare_settings() {
    let extra = json!({"num_ctx": 1, "num_thread": 2, "options": {"num_ctx": 8192}});
    let seen = embed_with_extra_body("ollama_chat/m", json!("a"), extra).await;
    assert_eq!(
        seen.body,
        json!({"model": "m", "input": "a", "options": {"num_ctx": 8192, "num_thread": 2}})
    );
}

#[tokio::test]
async fn embed_extra_body_all_kinds_together() {
    let extra = json!({"truncate": false, "keep_alive": 0, "options": {"num_ctx": 4096}, "num_thread": 2});
    let seen = embed_with_extra_body("ollama/m", json!("a"), extra).await;
    assert_eq!(
        seen.body,
        json!({"model": "m", "input": "a", "truncate": false, "keep_alive": 0,
               "options": {"num_ctx": 4096, "num_thread": 2}})
    );
}

#[tokio::test]
async fn embed_extra_body_null_members_are_not_sent() {
    let seen = embed_with_extra_body("ollama/m", json!("a"), json!({"keep_alive": null, "num_ctx": null})).await;
    assert_eq!(seen.body, json!({"model": "m", "input": "a"}));
}

#[tokio::test]
async fn embed_extra_body_key_that_names_a_request_field_replaces_it() {
    let seen = embed_with_extra_body("ollama/m", json!("a"), json!({"model": "other", "dimensions": 7})).await;
    assert_eq!(seen.body, json!({"model": "other", "input": "a", "dimensions": 7}));
}

#[tokio::test]
async fn embed_extra_body_input_replaces_the_input_and_stays_out_of_options() {
    let seen = embed_with_extra_body("ollama/m", json!("a"), json!({"input": "b"})).await;
    assert_eq!(seen.body, json!({"model": "m", "input": "b"}));
}

#[tokio::test]
async fn embed_extra_body_that_is_not_an_object_is_ignored() {
    for extra in [json!("keep_alive"), json!(5), json!([1, 2]), json!(true), Value::Null] {
        let seen = embed_with_extra_body("ollama/m", json!("a"), extra.clone()).await;
        assert_eq!(seen.body, json!({"model": "m", "input": "a"}), "extra_body {extra}");
    }
}

#[tokio::test]
async fn embed_extra_body_reaches_a_hint_client_with_a_bare_model() {
    let reply = r#"{"model":"m","embeddings":[[0.1]],"prompt_eval_count":1}"#;
    let (url, seen) = server(200, "application/json", reply.to_owned());
    let config = ClientConfigBuilder::new("").base_url(url).max_retries(0).build();
    let client = DefaultClient::new(config, Some("ollama/m")).expect("client");
    let mut request: EmbeddingRequest = serde_json::from_value(json!({"model": "m", "input": "a"})).expect("request");
    request.extra_body = Some(json!({"num_ctx": 8192}));
    client.embed(request).await.expect("embed succeeds");
    let seen = seen.join().expect("server done");
    assert_eq!(seen.request_line, "POST /api/embed HTTP/1.1");
    assert_eq!(
        seen.body,
        json!({"model": "m", "input": "a", "options": {"num_ctx": 8192}})
    );
}

#[tokio::test]
async fn embed_extra_body_does_not_lift_multimodal_input_refusal() {
    let (url, _seen) = server(200, "application/json", "{}".to_owned());
    let mut request: EmbeddingRequest =
        serde_json::from_value(json!({"model": "ollama/m", "input": [{"type": "text", "text": "a"}]}))
            .expect("request");
    request.extra_body = Some(json!({"truncate": false}));
    let error = client(url)
        .embed(request)
        .await
        .expect_err("multimodal input is refused");
    assert!(matches!(error, LiterLlmError::BadRequest { .. }), "{error}");
}
