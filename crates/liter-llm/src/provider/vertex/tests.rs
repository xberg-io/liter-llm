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
fn gemini_embedding_preserves_batch_order_and_dimensions() {
    let mut body = json!({
        "model": "gemini-embedding-001",
        "input": ["first", "second", "third"],
        "dimensions": 768
    });
    transform_gemini_embed_request(&mut body).expect("text batch should be accepted");
    let requests = body["requests"].as_array().expect("batch requests must be emitted");
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0]["content"]["parts"][0]["text"], "first");
    assert_eq!(requests[1]["content"]["parts"][0]["text"], "second");
    assert_eq!(requests[2]["content"]["parts"][0]["text"], "third");
    assert_eq!(requests[0]["model"], "models/gemini-embedding-001");
    assert_eq!(requests[0]["outputDimensionality"], 768);
}

#[test]
fn vertex_embedding_preserves_every_batch_input() {
    let mut body = json!({"input": ["first", "second", "third"]});
    transform_vertex_embed_request(&mut body).expect("text batch should be accepted");
    assert_eq!(body["instances"].as_array().map(Vec::len), Some(3));
    assert_eq!(body["instances"][2]["content"], "third");
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
fn transform_request_reasoning_effort_none_sets_zero_thinking_budget() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "reasoning_effort": "none"
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(
        body["generationConfig"]["thinkingConfig"],
        json!({"thinkingBudget": 0}),
        "none sends a zero budget and requests no thought parts"
    );
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

/// Revert line: remove the `stream_error_event` call from `parse_gemini_stream_event`
/// to make this test fail (the event then decodes as an empty chunk).
#[test]
fn gemini_stream_error_event_raises_the_http_error_kind() {
    let cases = [
        (r#"{"code":429,"message":"quota","status":"RESOURCE_EXHAUSTED"}"#, 429),
        (r#"{"code":503,"message":"down","status":"UNAVAILABLE"}"#, 503),
        (r#"{"code":401,"message":"bad key","status":"UNAUTHENTICATED"}"#, 401),
        (r#"{"code":400,"message":"bad input","status":"INVALID_ARGUMENT"}"#, 400),
    ];
    for (error, status) in cases {
        let event = format!(r#"{{"error":{error}}}"#);
        let err = provider().parse_stream_event(&event).unwrap_err();
        let http = crate::error::LiterLlmError::from_status(status, &event, None);
        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(&http),
            "{error}: stream gave {err:?}, HTTP {status} gives {http:?}"
        );
        assert_eq!(err.status_code(), status, "{error}");
    }
}

/// Revert line: remove the `status` name map from `google_error_status` to make this
/// test fail. The numeric `code` wins when it is an error status; a string `code`
/// reads as a number; a `code` that is no error status gives way to the `status` name.
#[test]
fn gemini_stream_error_event_edge_inputs_keep_the_http_error_kind() {
    let cases = [
        (r#"{"message":"quota","status":"RESOURCE_EXHAUSTED"}"#, 429),
        (r#"{"message":"down","status":"UNAVAILABLE"}"#, 503),
        (r#"{"message":"slow","status":"DEADLINE_EXCEEDED"}"#, 504),
        (r#"{"code":"429","message":"quota"}"#, 429),
        (r#"{"code":200,"message":"odd","status":"RESOURCE_EXHAUSTED"}"#, 429),
        // A code that is no error status and no status name gives the default 400.
        (r#"{"code":200,"message":"odd"}"#, 400),
    ];
    for (error, status) in cases {
        let event = format!(r#"{{"error":{error}}}"#);
        let err = provider().parse_stream_event(&event).unwrap_err();
        assert_eq!(err.status_code(), status, "{error}: {err:?}");
    }
    let err = provider()
        .parse_stream_event(r#"{"error":{"message":"quota","status":"RESOURCE_EXHAUSTED"}}"#)
        .unwrap_err();
    assert!(
        matches!(err, crate::error::LiterLlmError::RateLimited { .. }),
        "got: {err:?}"
    );
}

#[test]
fn gemini_stream_error_event_keeps_message_and_retry_after() {
    let event = r#"{"error":{"code":429,"message":"quota gone","status":"RESOURCE_EXHAUSTED","retry_after":11}}"#;
    let err = provider().parse_stream_event(event).unwrap_err();
    assert!(
        matches!(err, crate::error::LiterLlmError::RateLimited { .. }),
        "got: {err:?}"
    );
    assert!(err.to_string().contains("quota gone"), "got: {err}");
    assert_eq!(err.retry_after(), Some(std::time::Duration::from_secs(11)));
}

#[test]
fn gemini_stream_chunk_without_error_still_parses() {
    let event = r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"Hi"}]}}]}"#;
    let chunk = provider().parse_stream_event(event).unwrap().expect("a chunk");
    assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hi"));
}

#[test]
fn build_url_image_generations_uses_the_imagen_predict_method() {
    let p = provider();
    let url = p.build_url("/images/generations", "imagen-3.0-generate-002");
    assert_eq!(
        url,
        "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1/publishers/google/models/imagen-3.0-generate-002:predict"
    );
}

#[test]
fn build_url_list_models_uses_the_publisher_model_listing() {
    let p = provider();
    assert_eq!(
        p.build_url("/models", ""),
        "https://us-central1-aiplatform.googleapis.com/v1beta1/publishers/google/models"
    );
}

#[test]
fn build_url_list_models_keeps_the_host_of_a_base_url_override() {
    let scoped = VertexAiProvider::with_base_url("https://proxy.example/v1/projects/p/locations/l/");
    assert_eq!(
        scoped.build_url("/models", ""),
        "https://proxy.example/v1beta1/publishers/google/models"
    );
    let bare = VertexAiProvider::with_base_url("http://127.0.0.1:9000");
    assert_eq!(
        bare.build_url("/models", ""),
        "http://127.0.0.1:9000/v1beta1/publishers/google/models"
    );
}

#[test]
fn transform_request_builds_the_imagen_body() {
    let mut body = json!({"model": "imagen-3.0-generate-002", "prompt": "a cat", "n": 3, "size": "1024x1024"});
    provider().transform_request(&mut body).unwrap();
    assert_eq!(
        body,
        json!({
            "instances": [{"prompt": "a cat"}],
            "parameters": {"sampleCount": 3, "aspectRatio": "1:1"}
        })
    );
}

#[test]
fn transform_request_imagen_maps_each_supported_aspect_ratio() {
    for (size, ratio) in [
        ("1024x1024", "1:1"),
        ("768x1024", "3:4"),
        ("1024x768", "4:3"),
        ("1080x1920", "9:16"),
        ("1920x1080", "16:9"),
    ] {
        let mut body = json!({"prompt": "p", "size": size});
        provider().transform_request(&mut body).unwrap();
        assert_eq!(body["parameters"]["aspectRatio"], ratio, "{size}");
    }
}

#[test]
fn transform_request_imagen_rejects_a_size_without_an_aspect_ratio_and_a_url_format() {
    for body in [
        json!({"prompt": "p", "size": "1000x300"}),
        json!({"prompt": "p", "size": "large"}),
        json!({"prompt": "p", "response_format": "url"}),
    ] {
        let mut body = body;
        assert!(provider().transform_request(&mut body).is_err());
    }
}

#[test]
fn transform_response_maps_imagen_predictions_to_images() {
    let mut body = json!({
        "predictions": [
            {"bytesBase64Encoded": "aGk=", "mimeType": "image/png", "prompt": "a tabby cat"},
            {"raiFilteredReason": "blocked"}
        ]
    });
    provider().transform_response(&mut body).unwrap();
    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["b64_json"], "aGk=");
    assert_eq!(data[0]["revised_prompt"], "a tabby cat");
    assert!(body["created"].as_u64().unwrap() > 0);
}

#[test]
fn transform_response_still_maps_embedding_predictions() {
    let mut body = json!({"predictions": [{"embeddings": {"values": [0.5, 0.25]}}]});
    provider().transform_response(&mut body).unwrap();
    assert_eq!(body["data"][0]["embedding"], json!([0.5, 0.25]));
}

#[test]
fn transform_response_maps_publisher_models_to_a_model_list() {
    let mut body = json!({"publisherModels": [{"name": "publishers/google/models/gemini-2.0-flash"}]});
    provider().transform_response(&mut body).unwrap();
    assert_eq!(
        body,
        json!({
            "object": "list",
            "data": [{"id": "gemini-2.0-flash", "object": "model", "created": 0, "owned_by": "google"}]
        })
    );
}

#[test]
fn chat_request_with_a_prompt_stays_a_chat_request() {
    let mut body = json!({
        "model": "gemini-2.0-flash",
        "messages": [{"role": "user", "content": "hi"}],
        "prompt": "a cat"
    });
    provider().transform_request(&mut body).unwrap();
    assert!(body.get("contents").is_some());
    assert!(body.get("instances").is_none());
}

#[test]
fn transform_response_of_predictions_that_the_filter_removed_has_no_images() {
    let mut body = json!({"predictions": [{"raiFilteredReason": "blocked"}, {"raiFilteredReason": "blocked"}]});
    provider().transform_response(&mut body).unwrap();
    assert_eq!(body["data"], json!([]));
    assert!(body["created"].as_u64().unwrap() > 0);
}

/// Collects the messages of the warnings that a closure logs.
struct WarningLog(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl tracing::field::Visit for WarningLog {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0.lock().unwrap().push(format!("{value:?}"));
        }
    }
}

struct WarningSubscriber(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl tracing::Subscriber for WarningSubscriber {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        if *event.metadata().level() == tracing::Level::WARN {
            event.record(&mut WarningLog(self.0.clone()));
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

#[test]
fn imagen_request_warns_for_each_field_it_drops() {
    let warnings = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut body = json!({"prompt": "p", "quality": "hd", "style": "vivid", "user": "u", "n": 1});
    tracing::subscriber::with_default(WarningSubscriber(warnings.clone()), || {
        provider().transform_request(&mut body).unwrap();
    });
    let warnings = warnings.lock().unwrap();
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    assert!(warnings[0].contains("`quality`"), "{warnings:?}");
    assert!(warnings[1].contains("`style`"), "{warnings:?}");
    assert!(warnings[2].contains("`user`"), "{warnings:?}");
    assert!(body["parameters"].get("quality").is_none());
}

#[test]
fn transform_response_names_the_publisher_of_each_model() {
    let mut body = json!({"publisherModels": [
        {"name": "publishers/meta/models/llama-3"},
        {"name": "gemini-bare"},
        {}
    ]});
    provider().transform_response(&mut body).unwrap();
    assert_eq!(
        body["data"],
        json!([
            {"id": "llama-3", "object": "model", "created": 0, "owned_by": "meta"},
            {"id": "gemini-bare", "object": "model", "created": 0, "owned_by": "google"},
            {"id": "", "object": "model", "created": 0, "owned_by": "google"}
        ])
    );
}

#[test]
fn next_models_page_url_adds_the_encoded_page_token() {
    let p = provider();
    let url = "https://h/v1beta1/publishers/google/models";
    assert_eq!(
        p.next_models_page_url(url, &json!({"nextPageToken": "ab+/="}))
            .as_deref(),
        Some("https://h/v1beta1/publishers/google/models?pageToken=ab%2B%2F%3D")
    );
    assert_eq!(
        p.next_models_page_url(&format!("{url}?pageToken=old"), &json!({"nextPageToken": "new"}))
            .as_deref(),
        Some("https://h/v1beta1/publishers/google/models?pageToken=new")
    );
}

#[test]
fn next_models_page_url_is_none_on_the_last_page() {
    let p = provider();
    assert_eq!(
        p.next_models_page_url("https://h/m", &json!({"publisherModels": []})),
        None
    );
    assert_eq!(
        p.next_models_page_url("https://h/m", &json!({"nextPageToken": ""})),
        None
    );
}
