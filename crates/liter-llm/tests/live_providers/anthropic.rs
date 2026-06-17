use futures_util::StreamExt;
use liter_llm::{ClientConfigBuilder, DefaultClient, LiterLlmError, LlmClient};

use super::{anthropic_client, assert_chat_response_valid, require_env};

fn chat_request(model: &str, prompt: &str, max_tokens: u64) -> liter_llm::ChatCompletionRequest {
    serde_json::from_value(serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
    }))
    .unwrap()
}

const MODEL: &str = "claude-haiku-4-5-20251001";

// ── Basic chat ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn chat_basic() {
    let key = require_env!("ANTHROPIC_API_KEY");
    let client = anthropic_client(&key);

    let resp = client
        .chat(chat_request(MODEL, "Say hello in one word.", 16))
        .await
        .unwrap();

    assert_chat_response_valid(&resp, "anthropic/chat_basic");
    let usage = resp.usage.as_ref().expect("usage should be present");
    assert!(usage.prompt_tokens > 0, "prompt_tokens should be > 0");
    assert!(usage.total_tokens > 0, "total_tokens should be > 0");
}

// ── Streaming ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn chat_stream() {
    let key = require_env!("ANTHROPIC_API_KEY");
    let client = anthropic_client(&key);

    let mut stream = client
        .chat_stream(chat_request(MODEL, "Say hello in one word.", 16))
        .await
        .unwrap();

    let mut content = String::new();
    let mut chunk_count = 0u32;
    let mut saw_finish = false;
    let mut errors: Vec<String> = Vec::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                chunk_count += 1;
                if let Some(choice) = chunk.choices.first() {
                    if let Some(text) = &choice.delta.content {
                        content.push_str(text);
                    }
                    if choice.finish_reason.is_some() {
                        saw_finish = true;
                    }
                }
            }
            Err(e) => {
                errors.push(format!("{e}"));
            }
        }
        if chunk_count > 200 {
            break;
        }
    }

    assert!(errors.is_empty(), "stream produced errors: {errors:?}");
    assert!(chunk_count >= 1, "should receive at least 1 chunk");
    assert!(!content.is_empty(), "concatenated content should be non-empty");
    assert!(saw_finish, "should see a finish_reason in the stream");
}

// ── Multi-turn conversation ─────────────────────────────────────────────────

#[tokio::test]
async fn chat_multi_turn() {
    let key = require_env!("ANTHROPIC_API_KEY");
    let client = anthropic_client(&key);

    let req: liter_llm::ChatCompletionRequest = serde_json::from_value(serde_json::json!({
        "model": MODEL,
        "messages": [
            {"role": "user", "content": "My name is Alice."},
            {"role": "assistant", "content": "Hello Alice!"},
            {"role": "user", "content": "What is my name? Reply with just the name."},
        ],
        "max_tokens": 16,
    }))
    .unwrap();

    let resp = client.chat(req).await.unwrap();

    let content = resp.choices[0].message.text().unwrap_or_default();
    assert!(
        content.to_lowercase().contains("alice"),
        "expected response to contain 'alice', got: {content}"
    );
}

// ── System message ──────────────────────────────────────────────────────────

#[tokio::test]
async fn chat_system_message() {
    let key = require_env!("ANTHROPIC_API_KEY");
    let client = anthropic_client(&key);

    let req: liter_llm::ChatCompletionRequest = serde_json::from_value(serde_json::json!({
        "model": MODEL,
        "messages": [
            {"role": "system", "content": "You are a pirate. Always say 'Arrr'."},
            {"role": "user", "content": "Hello"},
        ],
        "max_tokens": 32,
    }))
    .unwrap();

    let resp = client.chat(req).await.unwrap();
    assert_chat_response_valid(&resp, "anthropic/chat_system_message");
}

// ── Tool calling ────────────────────────────────────────────────────────────

#[tokio::test]
async fn chat_tool_calling() {
    let key = require_env!("ANTHROPIC_API_KEY");
    let client = anthropic_client(&key);

    let req: liter_llm::ChatCompletionRequest = serde_json::from_value(serde_json::json!({
        "model": MODEL,
        "messages": [
            {"role": "user", "content": "What is the weather in Paris?"},
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather for a location",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string", "description": "City name"}
                    },
                    "required": ["location"]
                }
            }
        }],
        "tool_choice": "required",
        "max_tokens": 256,
    }))
    .unwrap();

    let resp = client.chat(req).await.unwrap();

    let choice = &resp.choices[0];
    let tool_calls = choice.message.tool_calls.as_ref().expect("should have tool_calls");
    assert!(!tool_calls.is_empty(), "tool_calls should not be empty");

    let call = &tool_calls[0];
    assert_eq!(call.function.name, "get_weather");

    // Verify arguments is valid JSON containing "location"
    let args: serde_json::Value =
        serde_json::from_str(&call.function.arguments).expect("tool call arguments should be valid JSON");
    assert!(
        args.get("location").is_some(),
        "arguments should contain 'location', got: {args}"
    );
}

// ── Tool calling with streaming ─────────────────────────────────────────────

#[tokio::test]
async fn chat_tool_calling_stream() {
    let key = require_env!("ANTHROPIC_API_KEY");
    let client = anthropic_client(&key);

    let req: liter_llm::ChatCompletionRequest = serde_json::from_value(serde_json::json!({
        "model": MODEL,
        "messages": [
            {"role": "user", "content": "What is the weather in London?"},
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather for a location",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                }
            }
        }],
        "tool_choice": "required",
        "max_tokens": 256,
    }))
    .unwrap();

    let mut stream = client.chat_stream(req).await.unwrap();

    let mut saw_tool_call = false;
    let mut tool_name = String::new();
    let mut tool_args = String::new();
    let mut chunk_count = 0u32;

    while let Some(result) = stream.next().await {
        let chunk = result.unwrap();
        chunk_count += 1;
        if let Some(choice) = chunk.choices.first()
            && let Some(tool_calls) = &choice.delta.tool_calls
        {
            for tc in tool_calls {
                saw_tool_call = true;
                if let Some(ref f) = tc.function {
                    if let Some(ref name) = f.name {
                        tool_name = name.clone();
                    }
                    if let Some(ref args) = f.arguments {
                        tool_args.push_str(args);
                    }
                }
            }
        }
        if chunk_count > 200 {
            break;
        }
    }

    assert!(saw_tool_call, "stream should contain tool call chunks");
    assert_eq!(tool_name, "get_weather");
    assert!(!tool_args.is_empty(), "tool arguments should be non-empty");
}

// ── Max tokens / length finish reason ───────────────────────────────────────

#[tokio::test]
async fn chat_max_tokens_truncation() {
    let key = require_env!("ANTHROPIC_API_KEY");
    let client = anthropic_client(&key);

    // Request a long response but cap at 5 tokens
    let resp = client
        .chat(chat_request(MODEL, "Write a 500 word essay about the ocean.", 5))
        .await
        .unwrap();

    assert!(
        resp.choices[0].finish_reason == Some(liter_llm::FinishReason::Length),
        "expected Length finish_reason for truncated response, got: {:?}",
        resp.choices[0].finish_reason
    );
}

// ── Error: invalid API key ──────────────────────────────────────────────────

#[tokio::test]
async fn error_invalid_key() {
    let _key = require_env!("ANTHROPIC_API_KEY");

    let config = ClientConfigBuilder::new("sk-ant-invalid-key-for-testing").build();
    let client = DefaultClient::new(config, Some("anthropic/claude-haiku-4-5-20251001")).unwrap();

    let err = client.chat(chat_request(MODEL, "Hello", 16)).await.unwrap_err();

    assert!(
        matches!(err, LiterLlmError::Authentication { .. }),
        "expected Authentication error, got: {err:?}"
    );
}
