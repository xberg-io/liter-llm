//! Coverage for model routing, URL building, stream parsing, embeddings and signing.

use serde_json::json;

use serial_test::serial;

use super::*;
use crate::provider::Provider;
use crate::types::chat::FinishReason;

use super::tests::provider;

#[test]
#[serial]
fn strip_model_prefix() {
    let p = provider();
    assert_eq!(p.strip_model_prefix("bedrock/anthropic.claude-3"), "anthropic.claude-3");
    assert_eq!(p.strip_model_prefix("anthropic.claude-3"), "anthropic.claude-3");
}

#[test]
#[serial]
fn matches_model() {
    let p = provider();
    assert!(p.matches_model("bedrock/anthropic.claude-3"));
    assert!(!p.matches_model("anthropic.claude-3"));
    assert!(!p.matches_model("gpt-4"));
}

#[test]
#[serial]
fn stream_format_is_eventstream() {
    let p = provider();
    assert_eq!(p.stream_format(), StreamFormat::AwsEventStream);
}

#[test]
#[serial]
fn build_stream_url_chat_completions() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let p = provider();
    let url = p.build_stream_url("/chat/completions", "anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(
        url,
        "https://bedrock-runtime.us-east-1.amazonaws.com/model/anthropic.claude-3-sonnet-20240229-v1%3A0/converse-stream"
    );
}

#[test]
#[serial]
fn build_stream_url_non_chat_falls_back() {
    let p = provider();
    let url = p.build_stream_url("/embeddings", "amazon.titan-embed-text-v1");
    assert_eq!(
        url,
        "https://bedrock-runtime.us-east-1.amazonaws.com/model/amazon.titan-embed-text-v1/invoke"
    );
}

#[test]
fn parse_stream_event_message_start() {
    let chunk = parse_bedrock_stream_event("messageStart", r#"{"role":"assistant"}"#)
        .expect("parse should not fail")
        .expect("should yield a chunk");
    assert_eq!(chunk.choices[0].delta.role.as_deref(), Some("assistant"));
}

#[test]
fn parse_stream_event_text_delta() {
    let chunk = parse_bedrock_stream_event(
        "contentBlockDelta",
        r#"{"contentBlockIndex":0,"delta":{"text":"Hello world"}}"#,
    )
    .expect("parse should not fail")
    .expect("should yield a chunk");
    assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello world"));
}

#[test]
fn parse_stream_event_tool_use_start() {
    let chunk = parse_bedrock_stream_event(
        "contentBlockStart",
        r#"{"contentBlockIndex":0,"start":{"toolUse":{"toolUseId":"call_123","name":"get_weather"}}}"#,
    )
    .expect("parse should not fail")
    .expect("should yield a chunk");
    let tc = &chunk.choices[0]
        .delta
        .tool_calls
        .as_ref()
        .expect("tool_calls should be present")[0];
    assert_eq!(tc.id.as_deref(), Some("call_123"));
    assert_eq!(
        tc.function
            .as_ref()
            .expect("function should be present")
            .name
            .as_deref(),
        Some("get_weather")
    );
}

#[test]
fn parse_stream_event_tool_use_input_delta() {
    let chunk = parse_bedrock_stream_event(
        "contentBlockDelta",
        r#"{"contentBlockIndex":0,"delta":{"toolUse":{"input":"{\"city\":\"Berlin\"}"}}}"#,
    )
    .expect("parse should not fail")
    .expect("should yield a chunk");
    let tc = &chunk.choices[0]
        .delta
        .tool_calls
        .as_ref()
        .expect("tool_calls should be present")[0];
    assert_eq!(
        tc.function
            .as_ref()
            .expect("function should be present")
            .arguments
            .as_deref(),
        Some("{\"city\":\"Berlin\"}")
    );
}

#[test]
fn parse_stream_event_message_stop() {
    let chunk = parse_bedrock_stream_event("messageStop", r#"{"stopReason":"end_turn"}"#)
        .expect("parse should not fail")
        .expect("should yield a chunk");
    assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Stop));
}

#[test]
fn parse_stream_event_metadata_usage() {
    let chunk = parse_bedrock_stream_event("metadata", r#"{"usage":{"inputTokens":42,"outputTokens":10}}"#)
        .expect("parse should not fail")
        .expect("should yield a chunk");
    let usage = chunk.usage.expect("usage should be present");
    assert_eq!(usage.prompt_tokens, 42);
    assert_eq!(usage.completion_tokens, 10);
}

#[test]
fn parse_stream_event_content_block_stop_returns_none() {
    let result =
        parse_bedrock_stream_event("contentBlockStop", r#"{"contentBlockIndex":0}"#).expect("parse should not fail");
    assert!(result.is_none());
}

#[test]
fn parse_stream_event_unknown_returns_none() {
    let result = parse_bedrock_stream_event("futureEventType", r#"{}"#).expect("parse should not fail");
    assert!(result.is_none());
}

#[test]
#[serial]
fn transform_request_reasoning_effort_low() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "Think step by step."}],
        "reasoning_effort": "low",
        "max_tokens": 1000
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let amf = &body["additionalModelRequestFields"];
    assert_eq!(amf["thinking"]["type"], "enabled");
    assert_eq!(amf["thinking"]["budget_tokens"], 1024);
}

#[test]
#[serial]
fn transform_request_reasoning_effort_medium() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "Think."}],
        "reasoning_effort": "medium"
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["additionalModelRequestFields"]["thinking"]["budget_tokens"], 4096);
}

#[test]
#[serial]
fn transform_request_reasoning_effort_high() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "Think hard."}],
        "reasoning_effort": "high"
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["additionalModelRequestFields"]["thinking"]["budget_tokens"], 16384);
}

#[test]
#[serial]
fn transform_request_no_reasoning_effort_omits_amf() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}]
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert!(body.get("additionalModelRequestFields").is_none());
}

#[test]
#[serial]
fn transform_request_document_content_part() {
    let p = provider();
    let mut body = json!({
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Summarize this document."},
                {
                    "type": "document",
                    "document": {
                        "data": "JVBERi0xLjQ=",
                        "media_type": "application/pdf"
                    }
                }
            ]
        }]
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let content = body["messages"][0]["content"]
        .as_array()
        .expect("content should be an array");
    assert_eq!(content.len(), 2);

    assert_eq!(content[0]["text"], "Summarize this document.");

    let doc = &content[1]["document"];
    assert_eq!(doc["name"], "doc");
    assert_eq!(doc["format"], "pdf");
    assert_eq!(doc["source"]["bytes"], "JVBERi0xLjQ=");
}

#[test]
#[serial]
fn transform_request_document_csv_format() {
    let p = provider();
    let mut body = json!({
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "document",
                    "document": {
                        "data": "Y29sMSxjb2wy",
                        "media_type": "text/csv"
                    }
                }
            ]
        }]
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let doc = &body["messages"][0]["content"][0]["document"];
    assert_eq!(doc["format"], "csv");
}

#[test]
#[serial]
fn transform_request_guardrails() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hello"}],
        "extra_body": {
            "guardrailConfig": {
                "guardrailIdentifier": "my-guardrail-id",
                "guardrailVersion": "DRAFT",
                "trace": "enabled"
            }
        }
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let gc = &body["guardrailConfig"];
    assert_eq!(gc["guardrailIdentifier"], "my-guardrail-id");
    assert_eq!(gc["guardrailVersion"], "DRAFT");
    assert_eq!(gc["trace"], "enabled");
}

#[test]
#[serial]
fn transform_request_no_guardrails_omits_config() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hello"}]
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert!(body.get("guardrailConfig").is_none());
}

#[test]
#[serial]
fn transform_request_json_object_response_format() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "Give me JSON."}],
        "response_format": {"type": "json_object"}
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let system = body["system"].as_array().expect("system should be an array");
    let has_json_instruction = system
        .iter()
        .any(|s| s["text"].as_str().unwrap_or("").contains("valid JSON"));
    assert!(has_json_instruction, "should inject JSON instruction in system");
}

#[test]
#[serial]
fn transform_request_json_schema_response_format() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "Give me structured data."}],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "my_schema",
                "schema": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    }
                }
            }
        }
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let system = body["system"].as_array().expect("system should be an array");
    let json_instruction = system
        .iter()
        .find(|s| s["text"].as_str().unwrap_or("").contains("valid JSON"))
        .expect("JSON instruction block should be present");
    let text = json_instruction["text"].as_str().expect("text should be a string");
    assert!(
        text.contains("conforms to this schema"),
        "should include schema reference: {text}"
    );
    assert!(text.contains("\"name\""), "should include the schema content: {text}");
}

#[test]
#[serial]
fn transform_request_text_response_format_no_injection() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hello"}],
        "response_format": {"type": "text"}
    });
    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert!(body.get("system").is_none());
}

#[test]
#[serial]
fn apply_cross_region_prefix_when_set() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::set_var("BEDROCK_CROSS_REGION", "us") };
    let result = super::apply_cross_region_prefix("anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(result, "us.anthropic.claude-3-sonnet-20240229-v1:0");
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
}

#[test]
#[serial]
fn apply_cross_region_prefix_no_double_prefix() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::set_var("BEDROCK_CROSS_REGION", "eu") };
    let result = super::apply_cross_region_prefix("eu.anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(
        result, "eu.anthropic.claude-3-sonnet-20240229-v1:0",
        "should not double-prefix"
    );
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
}

#[test]
#[serial]
fn apply_cross_region_prefix_unset() {
    // ~keep SAFETY: env vars are process-global; `#[serial]` ensures no parallel mutation.
    unsafe { std::env::remove_var("BEDROCK_CROSS_REGION") };
    let result = super::apply_cross_region_prefix("anthropic.claude-3-sonnet-20240229-v1:0");
    assert_eq!(result, "anthropic.claude-3-sonnet-20240229-v1:0");
}

#[test]
fn reasoning_effort_budget_tokens() {
    assert_eq!(super::reasoning_effort_to_budget_tokens("low"), 1024);
    assert_eq!(super::reasoning_effort_to_budget_tokens("medium"), 4096);
    assert_eq!(super::reasoning_effort_to_budget_tokens("high"), 16384);
    assert_eq!(super::reasoning_effort_to_budget_tokens("unknown"), 4096);
}

#[test]
fn format_from_media_type_extraction() {
    assert_eq!(super::format_from_media_type("application/pdf"), "pdf");
    assert_eq!(super::format_from_media_type("text/csv"), "csv");
    assert_eq!(
        super::format_from_media_type("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "vnd.openxmlformats-officedocument.wordprocessingml.document"
    );
    assert_eq!(super::format_from_media_type("pdf"), "pdf");
}

#[test]
#[serial]
fn transform_request_titan_embedding_input_is_not_mangled_into_empty_messages() {
    // ~keep Regression test: before the embed/chat branch, `transform_request`
    // ~keep unconditionally rebuilt the body as `{"messages": []}` for any request,
    // ~keep silently discarding an embedding request's `input` entirely.
    let p = provider();
    let mut body = json!({
        "model": "amazon.titan-embed-text-v1",
        "input": "hello world"
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["inputText"], "hello world");
    assert!(
        body.get("messages").is_none(),
        "embedding body must not gain a messages field"
    );
}

/// Titan's synchronous invoke takes one input, but OpenAI's contract is that
/// `data[]` parallels `input[]`. Quietly embedding only the first text would
/// hand back one vector for an N-element batch — misaligned embeddings that
/// surface much later as bad retrieval, not as an error. Reject instead.
#[test]
#[serial]
fn transform_request_titan_embedding_batch_input_is_rejected_not_truncated() {
    let p = provider();
    let mut body = json!({
        "model": "amazon.titan-embed-text-v2:0",
        "input": ["first", "second"]
    });

    let err = p
        .transform_request(&mut body)
        .expect_err("a batched Titan embedding request must be rejected, not silently truncated");

    assert_eq!(err.status_code(), 400);
    assert!(
        err.to_string().contains("single input per call"),
        "the error must explain the single-input limit, got: {err}"
    );
}

#[test]
#[serial]
fn transform_request_titan_embedding_single_element_array_is_accepted() {
    let p = provider();
    let mut body = json!({
        "model": "amazon.titan-embed-text-v2:0",
        "input": ["only"]
    });

    p.transform_request(&mut body)
        .expect("a single-element batch is within Titan's one-input limit");

    assert_eq!(body["inputText"], "only");
}

#[test]
#[serial]
fn transform_request_embedding_multimodal_input_is_rejected() {
    let p = provider();
    let mut body = json!({
        "model": "amazon.titan-embed-text-v2:0",
        "input": [{"type": "image_url", "image_url": {"url": "https://example.com/image.png"}}]
    });

    let error = p
        .transform_request(&mut body)
        .expect_err("multimodal input should be rejected");
    assert_eq!(error.status_code(), 400);
    assert!(error.to_string().contains("text input only"));
}

#[test]
#[serial]
fn transform_request_cohere_embedding_maps_to_texts_and_input_type() {
    let p = provider();
    let mut body = json!({
        "model": "cohere.embed-english-v3",
        "input": ["one", "two"]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let texts = body["texts"].as_array().expect("texts should be an array");
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0], "one");
    assert_eq!(texts[1], "two");
    assert_eq!(body["input_type"], "search_document");
}

#[test]
#[serial]
fn transform_request_unknown_embedding_model_is_rejected() {
    let p = provider();
    let mut body = json!({
        "model": "unknown-vendor.embed-v1",
        "input": "hello"
    });

    let err = p.transform_request(&mut body).unwrap_err();
    assert!(
        err.to_string().contains("unsupported Bedrock embedding model"),
        "got: {err}"
    );
}

/// Regression guard: the embed/chat dispatch must only trigger for
/// input-without-messages bodies. A normal chat body (messages present) must
/// still go through the unchanged Converse transform.
#[test]
#[serial]
fn transform_request_chat_path_unaffected_by_embedding_branch() {
    let p = provider();
    let mut body = json!({
        "model": "anthropic.claude-3-sonnet",
        "messages": [{"role": "user", "content": "Hello!"}]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"][0]["text"], "Hello!");
    assert!(body.get("inputText").is_none());
}

#[test]
#[serial]
fn transform_response_titan_embedding_response_normalized() {
    let p = provider();
    let mut body = json!({
        "embedding": [0.1, 0.2, 0.3],
        "inputTextTokenCount": 4
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["object"], "list");
    assert_eq!(body["data"][0]["embedding"], json!([0.1, 0.2, 0.3]));
    assert_eq!(body["data"][0]["index"], 0);
    assert_eq!(body["usage"]["prompt_tokens"], 4);
}

#[test]
#[serial]
fn transform_response_cohere_embedding_response_normalized() {
    let p = provider();
    let mut body = json!({
        "embeddings": [[0.1, 0.2], [0.3, 0.4]],
        "id": "abc",
        "response_type": "embeddings_floats",
        "texts": ["one", "two"]
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    let data = body["data"].as_array().expect("data should be an array");
    assert_eq!(data.len(), 2);
    assert_eq!(data[0]["embedding"], json!([0.1, 0.2]));
    assert_eq!(data[1]["embedding"], json!([0.3, 0.4]));
    assert_eq!(data[1]["index"], 1);
}

/// Regression guard: a normal chat response (no `embedding`/`embeddings` key)
/// must still go through the unchanged Converse response transform.
#[test]
#[serial]
fn transform_response_chat_path_unaffected_by_embedding_branch() {
    let p = provider();
    let mut body = json!({
        "requestId": "req-456",
        "stopReason": "end_turn",
        "output": {
            "message": {
                "role": "assistant",
                "content": [{"text": "hi"}]
            }
        },
        "usage": {"inputTokens": 1, "outputTokens": 1}
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["object"], "chat.completion");
    assert!(body.get("data").is_none());
}
