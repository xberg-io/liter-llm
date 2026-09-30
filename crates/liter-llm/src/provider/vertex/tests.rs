use serde_json::json;

use super::*;
use crate::provider::Provider;

pub(super) fn provider() -> VertexAiProvider {
    VertexAiProvider::new("test-project", "us-central1")
}

pub(super) fn provider_without_project() -> VertexAiProvider {
    VertexAiProvider {
        base_url: String::new(),
    }
}

#[test]
fn validate_succeeds_with_project() {
    let p = provider();
    assert!(p.validate().is_ok());
}

#[test]
fn validate_fails_without_project() {
    let p = provider_without_project();
    let err = p.validate().unwrap_err();
    assert!(
        err.to_string().contains("VERTEXAI_PROJECT"),
        "error should mention VERTEXAI_PROJECT"
    );
}

#[test]
fn gemini_embedding_rejects_multimodal_input() {
    let mut body = json!({
        "input": [
            {"type": "text", "text": "caption"},
            {"type": "image_url", "image_url": {"url": "https://example.com/image.png"}}
        ]
    });

    let error = transform_gemini_embed_request(&mut body).expect_err("multimodal input should be rejected");
    assert_eq!(error.status_code(), 400);
    assert!(error.to_string().contains("text input only"));
}

#[test]
fn vertex_embedding_rejects_multimodal_input() {
    let mut body = json!({
        "input": [{"type": "image_base64", "image_base64": "data:image/png;base64,aW1hZ2U="}]
    });

    let error = transform_vertex_embed_request(&mut body).expect_err("multimodal input should be rejected");
    assert_eq!(error.status_code(), 400);
    assert!(error.to_string().contains("text input only"));
}

#[test]
fn base_url_constructed_from_project_and_location() {
    let p = provider();
    assert_eq!(
        p.base_url(),
        "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1"
    );
}

#[test]
fn base_url_custom_location() {
    let p = VertexAiProvider::new("my-proj", "europe-west1");
    assert_eq!(
        p.base_url(),
        "https://europe-west1-aiplatform.googleapis.com/v1/projects/my-proj/locations/europe-west1"
    );
}

#[test]
fn build_url_returns_empty_without_base() {
    let p = provider_without_project();
    let url = p.build_url("/chat/completions", "gemini-2.0-flash");
    assert!(url.is_empty(), "should return empty string without a base URL");
}

#[test]
fn build_url_chat_completions() {
    let p = provider();
    let url = p.build_url("/chat/completions", "gemini-2.0-flash");
    assert!(url.ends_with("/publishers/google/models/gemini-2.0-flash:generateContent"));
}

#[test]
fn build_url_embeddings() {
    let p = provider();
    let url = p.build_url("/embeddings", "text-embedding-004");
    assert!(url.ends_with("/publishers/google/models/text-embedding-004:predict"));
}

/// Streaming must target `:streamGenerateContent`.  `build_stream_url`
/// previously reused `build_url`, so it streamed from `:generateContent`,
/// which ignores `alt=sse` and returns one ordinary JSON body — the caller
/// got a misleading "SSE stream truncated" error, never any content.
#[test]
fn build_stream_url_targets_stream_generate_content() {
    let p = provider();
    let url = p.build_stream_url("/chat/completions", "gemini-2.0-flash");

    assert!(
        url.ends_with("/publishers/google/models/gemini-2.0-flash:streamGenerateContent?alt=sse"),
        "got {url}"
    );
}

/// The empty-base guard must still short-circuit before the rewrite.
#[test]
fn build_stream_url_returns_empty_without_project() {
    let p = provider_without_project();

    assert!(p.build_stream_url("/chat/completions", "gemini-2.0-flash").is_empty());
}

#[test]
fn transform_request_basic_chat() {
    let p = provider();
    let mut body = json!({
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello!"}
        ],
        "max_tokens": 200,
        "temperature": 0.5
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(
        body["systemInstruction"]["parts"][0]["text"],
        "You are a helpful assistant."
    );

    assert_eq!(body["contents"][0]["role"], "user");
    assert_eq!(body["contents"][0]["parts"][0]["text"], "Hello!");

    assert_eq!(body["generationConfig"]["maxOutputTokens"], 200);
    assert_eq!(body["generationConfig"]["temperature"], 0.5);
}

/// `max_completion_tokens` maps to `maxOutputTokens` when `max_tokens` is absent.
/// Previously untested even though the mapping itself predates this fix.
#[test]
fn transform_request_max_completion_tokens_maps_to_max_output_tokens() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "max_completion_tokens": 512
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["generationConfig"]["maxOutputTokens"], 512);
}

/// `logprobs`/`top_logprobs` map to Gemini's `responseLogprobs`/`logprobs` generationConfig
/// fields rather than being silently discarded by the wholesale body rebuild.
#[test]
fn transform_request_logprobs_maps_to_response_logprobs() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "logprobs": true,
        "top_logprobs": 5
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["generationConfig"]["responseLogprobs"], true);
    assert_eq!(body["generationConfig"]["logprobs"], 5);
}

#[test]
fn transform_request_logprobs_false_maps_explicitly() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "logprobs": false
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["generationConfig"]["responseLogprobs"], false);
    assert!(body["generationConfig"].get("logprobs").is_none());
}

/// `audio` and `web_search_options` have no Gemini equivalent; the request must still
/// succeed but the fields must not appear anywhere on the wire body.
#[test]
fn transform_request_audio_and_web_search_options_dropped_not_forwarded() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "audio": {"voice": "alloy", "format": "wav"},
        "web_search_options": {"search_context_size": "medium"}
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert!(body.get("audio").is_none());
    assert!(body.get("web_search_options").is_none());
}

/// `service_tier`, `store`, `metadata` and `prediction` have no Gemini equivalent and no
/// wire-safety risk (unlike Anthropic's colliding `metadata`), so they stay silently
/// dropped by the wholesale body rebuild — this pins that as intentional, not an oversight.
#[test]
fn transform_request_cosmetic_openai_fields_dropped() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "service_tier": "flex",
        "store": true,
        "metadata": {"run": "nightly"},
        "prediction": {"type": "content", "content": "draft"}
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    for key in &["service_tier", "store", "metadata", "prediction"] {
        assert!(body.get(key).is_none(), "`{key}` should not be forwarded to Gemini");
    }
}

#[test]
fn transform_request_assistant_becomes_model_role() {
    let p = provider();
    let mut body = json!({
        "messages": [
            {"role": "user", "content": "Hi"},
            {"role": "assistant", "content": "Hello there!"}
        ]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["contents"][1]["role"], "model");
    assert_eq!(body["contents"][1]["parts"][0]["text"], "Hello there!");
}

#[test]
fn transform_request_with_tool_calls() {
    let p = provider();
    let mut body = json!({
        "messages": [
            {"role": "user", "content": "What is the weather in Berlin?"},
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "get_weather", "arguments": "{\"city\":\"Berlin\"}"}
                }]
            },
            {
                "role": "tool",
                "name": "get_weather",
                "tool_call_id": "call_1",
                "content": "Sunny, 22°C"
            }
        ]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let contents = body["contents"].as_array().expect("contents should be an array");
    assert_eq!(contents.len(), 3);

    let model_turn = &contents[1];
    assert_eq!(model_turn["role"], "model");
    let fn_call = &model_turn["parts"][0]["functionCall"];
    assert_eq!(fn_call["name"], "get_weather");
    assert_eq!(fn_call["args"]["city"], "Berlin");

    let tool_turn = &contents[2];
    assert_eq!(tool_turn["role"], "user");
    let fn_resp = &tool_turn["parts"][0]["functionResponse"];
    assert_eq!(fn_resp["name"], "get_weather");
}

#[test]
fn transform_request_stop_sequences() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "stop": ["END", "STOP"]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let stop_seqs = body["generationConfig"]["stopSequences"]
        .as_array()
        .expect("stopSequences should be an array");
    assert_eq!(stop_seqs.len(), 2);
    assert_eq!(stop_seqs[0], "END");
    assert_eq!(stop_seqs[1], "STOP");
}

#[test]
fn transform_request_reasoning_effort_maps_to_thinking_config() {
    // ~keep Regression test for #52: `reasoning_effort` was previously dropped
    // entirely by Gemini's request transform.
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "reasoning_effort": "high"
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["generationConfig"]["thinkingConfig"]["thinkingBudget"], 16384);
    assert_eq!(body["generationConfig"]["thinkingConfig"]["includeThoughts"], true);
}

#[test]
fn transform_request_no_reasoning_effort_omits_thinking_config() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert!(body["generationConfig"]["thinkingConfig"].is_null());
}

#[test]
fn transform_request_safety_settings_from_extra_body() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "extra_body": {
            "safety_settings": [
                {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_MEDIUM_AND_ABOVE"},
                {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_ONLY_HIGH"}
            ]
        }
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let settings = body["safetySettings"]
        .as_array()
        .expect("safetySettings should be an array");
    assert_eq!(settings.len(), 2);
    assert_eq!(settings[0]["category"], "HARM_CATEGORY_HATE_SPEECH");
    assert_eq!(settings[0]["threshold"], "BLOCK_MEDIUM_AND_ABOVE");
    assert_eq!(settings[1]["category"], "HARM_CATEGORY_DANGEROUS_CONTENT");
}

#[test]
fn transform_request_grounding_config_adds_google_search() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "What happened today?"}],
        "extra_body": {
            "grounding_config": {}
        }
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let tools = body["tools"].as_array().expect("tools should be an array");
    assert!(
        tools.iter().any(|t| t.get("google_search_retrieval").is_some()),
        "tools should contain google_search_retrieval"
    );
}

#[test]
fn transform_request_google_search_retrieval_with_existing_tools() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "tools": [{"type": "function", "function": {"name": "f", "parameters": {}}}],
        "extra_body": {
            "google_search_retrieval": {}
        }
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let tools = body["tools"].as_array().expect("tools should be an array");
    assert_eq!(tools.len(), 2);
    assert!(tools[0].get("functionDeclarations").is_some());
    assert!(tools[1].get("google_search_retrieval").is_some());
}

#[test]
fn transform_request_cached_content_from_extra_body() {
    let p = provider();
    let cached = "projects/xxx/locations/xxx/cachedContents/abc123";
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "extra_body": {
            "cached_content": cached
        }
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["cachedContent"], cached);
}

#[test]
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

    let parts = body["contents"][0]["parts"]
        .as_array()
        .expect("parts should be an array");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0]["text"], "Summarize this document.");
    assert_eq!(parts[1]["inlineData"]["mimeType"], "application/pdf");
    assert_eq!(parts[1]["inlineData"]["data"], "JVBERi0xLjQ=");
}

#[test]
fn transform_response_basic() {
    let p = provider();
    let mut body = json!({
        "responseId": "resp-gemini-123",
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{"text": "Hello from Gemini!"}]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {
            "promptTokenCount": 8,
            "candidatesTokenCount": 6
        }
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["id"], "resp-gemini-123");
    assert_eq!(body["choices"][0]["message"]["content"], "Hello from Gemini!");
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert_eq!(body["usage"]["prompt_tokens"], 8);
    assert_eq!(body["usage"]["completion_tokens"], 6);
    assert_eq!(body["usage"]["total_tokens"], 14);
}

#[test]
fn transform_response_thought_part_routes_to_reasoning_content_not_visible() {
    // ~keep Regression test for #52: a Gemini "thought" part must never leak into
    // the visible `content`; it must be routed to `reasoning_content` instead.
    let p = provider();
    let mut body = json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [
                    {"text": "Let me think about this...", "thought": true},
                    {"text": "The answer is 42."}
                ]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 3, "candidatesTokenCount": 5}
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["choices"][0]["message"]["content"], "The answer is 42.");
    assert_eq!(
        body["choices"][0]["message"]["reasoning_content"],
        "Let me think about this..."
    );
}

#[test]
fn transform_response_thought_only_has_null_content() {
    let p = provider();
    let mut body = json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{"text": "still thinking...", "thought": true}]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1}
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    assert!(body["choices"][0]["message"]["content"].is_null());
    assert_eq!(body["choices"][0]["message"]["reasoning_content"], "still thinking...");
}
