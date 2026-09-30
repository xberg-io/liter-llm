//! Coverage for response normalization, tool calls, streaming, embeddings,
//! grounding and multimodal output.

use serde_json::json;

use super::tests::provider;
use super::*;
use crate::provider::Provider;

#[test]
fn transform_response_tool_calls_have_unique_ids() {
    let p = provider();
    let mut body = json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [
                    {
                        "functionCall": {
                            "name": "get_weather",
                            "args": {"city": "Berlin"}
                        }
                    },
                    {
                        "functionCall": {
                            "name": "get_weather",
                            "args": {"city": "Paris"}
                        }
                    }
                ]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 5}
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    let tool_calls = body["choices"][0]["message"]["tool_calls"]
        .as_array()
        .expect("tool_calls should be an array");
    assert_eq!(tool_calls.len(), 2);

    let id0 = tool_calls[0]["id"].as_str().expect("id should be a string");
    let id1 = tool_calls[1]["id"].as_str().expect("id should be a string");
    assert_ne!(id0, id1, "tool call IDs must be unique even for the same function");
    assert!(id0.starts_with("call_get_weather_"));
    assert!(id1.starts_with("call_get_weather_"));

    let args0: serde_json::Value = serde_json::from_str(
        tool_calls[0]["function"]["arguments"]
            .as_str()
            .expect("arguments should be a string"),
    )
    .expect("arguments should be valid JSON");
    let args1: serde_json::Value = serde_json::from_str(
        tool_calls[1]["function"]["arguments"]
            .as_str()
            .expect("arguments should be a string"),
    )
    .expect("arguments should be valid JSON");
    assert_eq!(args0["city"], "Berlin");
    assert_eq!(args1["city"], "Paris");
}

#[test]
fn transform_response_single_tool_call() {
    let p = provider();
    let mut body = json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{
                    "functionCall": {
                        "name": "get_weather",
                        "args": {"city": "Berlin"}
                    }
                }]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 5}
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    let tool_calls = body["choices"][0]["message"]["tool_calls"]
        .as_array()
        .expect("tool_calls should be an array");
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
    let id = tool_calls[0]["id"].as_str().expect("id should be a string");
    assert!(
        id.starts_with("call_get_weather_"),
        "id should start with call_get_weather_, got: {id}"
    );
}

#[test]
fn transform_response_finish_reason_mapping() {
    let p = provider();

    for (gemini_reason, expected_oai_reason) in [
        ("STOP", "stop"),
        ("MAX_TOKENS", "length"),
        ("SAFETY", "content_filter"),
        ("RECITATION", "content_filter"),
        ("BLOCKLIST", "content_filter"),
        ("PROHIBITED_CONTENT", "content_filter"),
        ("UNKNOWN_FUTURE_REASON", "stop"),
    ] {
        let mut body = json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": ""}]},
                "finishReason": gemini_reason
            }],
            "usageMetadata": {"promptTokenCount": 0, "candidatesTokenCount": 0}
        });
        p.transform_response(&mut body)
            .expect("transform_response should not fail");
        assert_eq!(
            body["choices"][0]["finish_reason"], expected_oai_reason,
            "Gemini finishReason '{gemini_reason}' should map to '{expected_oai_reason}'"
        );
    }
}

#[test]
fn transform_response_grounding_metadata_preserved() {
    let p = provider();
    let mut body = json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{"text": "grounded answer"}]
            },
            "finishReason": "STOP",
            "groundingMetadata": {
                "searchEntryPoint": {"renderedContent": "<html>...</html>"},
                "groundingChunks": [{"web": {"uri": "https://example.com", "title": "Example"}}]
            }
        }],
        "usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 3}
    });

    p.transform_response(&mut body)
        .expect("transform_response should not fail");

    assert_eq!(body["choices"][0]["message"]["content"], "grounded answer");
    assert!(
        body.get("_grounding_metadata").is_some(),
        "grounding metadata should be preserved"
    );
    assert!(
        body["_grounding_metadata"]["groundingChunks"]
            .as_array()
            .expect("groundingChunks should be an array")
            .len()
            == 1
    );
}

#[test]
fn parse_stream_event_empty_returns_none() {
    let p = provider();
    let result = p.parse_stream_event("").expect("parse_stream_event should not fail");
    assert!(result.is_none());
}

#[test]
fn parse_stream_event_done_is_handled_at_sse_level() {
    let p = provider();
    let result = p.parse_stream_event("[DONE]");
    assert!(
        result.is_err(),
        "[DONE] is not valid JSON and should error if it reaches the provider"
    );
}

#[test]
fn parse_stream_event_basic_chunk() {
    let p = provider();
    let event_data = r#"{
        "candidates": [{
            "content": {"role": "model", "parts": [{"text": "Hello"}]},
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 2}
    }"#;

    let chunk = p
        .parse_stream_event(event_data)
        .expect("parse_stream_event should not fail")
        .expect("should yield a chunk");

    assert_eq!(chunk.object, "chat.completion.chunk");
    assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
}

#[test]
fn parse_stream_event_thought_part_routes_to_reasoning_content() {
    // ~keep Regression test for #52: the streaming path must also route "thought"
    // parts to `reasoning_content`, not just the non-streaming response transform.
    let p = provider();
    let event_data = r#"{
        "candidates": [{
            "content": {"role": "model", "parts": [{"text": "pondering...", "thought": true}]},
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1}
    }"#;

    let chunk = p
        .parse_stream_event(event_data)
        .expect("parse_stream_event should not fail")
        .expect("should yield a chunk");

    assert_eq!(chunk.choices[0].delta.content, None);
    assert_eq!(
        chunk.choices[0].delta.reasoning_content.as_deref(),
        Some("pondering...")
    );
}

#[test]
fn strip_model_prefix() {
    let p = provider();
    assert_eq!(p.strip_model_prefix("vertex_ai/gemini-2.0-flash"), "gemini-2.0-flash");
    assert_eq!(p.strip_model_prefix("gemini-2.0-flash"), "gemini-2.0-flash");
}

#[test]
fn matches_model() {
    let p = provider();
    assert!(p.matches_model("vertex_ai/gemini-2.0-flash"));
    assert!(!p.matches_model("gemini-2.0-flash"));
    assert!(!p.matches_model("gpt-4"));
}

#[test]
fn transform_request_multimodal_user_content() {
    let p = provider();
    let mut body = json!({
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "What is in this image?"},
                {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/abc=="}}
            ]
        }]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let parts = body["contents"][0]["parts"]
        .as_array()
        .expect("parts should be an array");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0]["text"], "What is in this image?");
    assert_eq!(parts[1]["inlineData"]["mimeType"], "image/jpeg");
    assert_eq!(parts[1]["inlineData"]["data"], "/9j/abc==");
}

#[test]
fn transform_request_multimodal_url_image() {
    let p = provider();
    let mut body = json!({
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Describe this."},
                {"type": "image_url", "image_url": {"url": "https://example.com/image.jpg"}}
            ]
        }]
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    let parts = body["contents"][0]["parts"]
        .as_array()
        .expect("parts should be an array");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[1]["fileData"]["fileUri"], "https://example.com/image.jpg");
}

#[test]
fn transform_request_response_format_json_object() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "response_format": {"type": "json_object"}
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["generationConfig"]["responseMimeType"], "application/json");
}

#[test]
fn transform_request_response_format_json_schema() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "test",
                "schema": {"type": "object", "properties": {"name": {"type": "string"}}}
            }
        }
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["generationConfig"]["responseMimeType"], "application/json");
    assert_eq!(body["generationConfig"]["responseSchema"]["type"], "object");
}

#[test]
fn transform_request_tool_choice_auto() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "tools": [{"type": "function", "function": {"name": "f", "parameters": {}}}],
        "tool_choice": "auto"
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "AUTO");
}

#[test]
fn transform_request_tool_choice_none() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "tools": [{"type": "function", "function": {"name": "f", "parameters": {}}}],
        "tool_choice": "none"
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "NONE");
}

#[test]
fn transform_request_tool_choice_required() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "tools": [{"type": "function", "function": {"name": "f", "parameters": {}}}],
        "tool_choice": "required"
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "ANY");
}

#[test]
fn transform_request_tool_choice_specific_function() {
    let p = provider();
    let mut body = json!({
        "messages": [{"role": "user", "content": "hi"}],
        "tools": [{"type": "function", "function": {"name": "get_weather", "parameters": {}}}],
        "tool_choice": {"type": "function", "function": {"name": "get_weather"}}
    });

    p.transform_request(&mut body)
        .expect("transform_request should not fail");

    assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "ANY");
    assert_eq!(
        body["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"][0],
        "get_weather"
    );
}

#[test]
fn convert_user_content_string() {
    let content = json!("Hello!");
    let parts = convert_user_content_to_gemini(Some(&content));
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["text"], "Hello!");
}

#[test]
fn convert_user_content_array_with_image() {
    let content = json!([
        {"type": "text", "text": "What is this?"},
        {"type": "image_url", "image_url": {"url": "data:image/png;base64,iVBOR"}}
    ]);
    let parts = convert_user_content_to_gemini(Some(&content));
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0]["text"], "What is this?");
    assert_eq!(parts[1]["inlineData"]["mimeType"], "image/png");
    assert_eq!(parts[1]["inlineData"]["data"], "iVBOR");
}

#[test]
fn convert_user_content_none() {
    let parts = convert_user_content_to_gemini(None);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["text"], "");
}

#[test]
fn convert_user_content_document_part() {
    let content = json!([
        {"type": "text", "text": "Read this PDF."},
        {"type": "document", "document": {"data": "base64data==", "media_type": "application/pdf"}}
    ]);
    let parts = convert_user_content_to_gemini(Some(&content));
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0]["text"], "Read this PDF.");
    assert_eq!(parts[1]["inlineData"]["mimeType"], "application/pdf");
    assert_eq!(parts[1]["inlineData"]["data"], "base64data==");
}

#[test]
fn translate_tool_choice_string_values() {
    let auto = translate_tool_choice(Some(&json!("auto"))).expect("auto choice should translate");
    assert_eq!(auto["functionCallingConfig"]["mode"], "AUTO");

    let none = translate_tool_choice(Some(&json!("none"))).expect("none choice should translate");
    assert_eq!(none["functionCallingConfig"]["mode"], "NONE");

    let required = translate_tool_choice(Some(&json!("required"))).expect("required choice should translate");
    assert_eq!(required["functionCallingConfig"]["mode"], "ANY");
}

#[test]
fn translate_tool_choice_specific_function() {
    let tc = json!({"type": "function", "function": {"name": "my_fn"}});
    let result = translate_tool_choice(Some(&tc)).expect("specific tool choice should translate");
    assert_eq!(result["functionCallingConfig"]["mode"], "ANY");
    assert_eq!(result["functionCallingConfig"]["allowedFunctionNames"][0], "my_fn");
}

#[test]
fn translate_tool_choice_none_input() {
    assert!(translate_tool_choice(None).is_none());
}

#[test]
fn transform_response_inline_data_image_emits_output_image() {
    let mut body = json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "inlineData": {
                        "mimeType": "image/png",
                        "data": "aGk="
                    }
                }]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 5}
    });
    transform_gemini_response(&mut body).expect("transform must succeed");

    let content = body
        .pointer("/choices/0/message/content")
        .expect("content must be present");
    assert!(content.is_array(), "content must be a parts array, got: {content}");
    let parts = content.as_array().expect("array");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["type"], "output_image");
    assert_eq!(parts[0]["image_url"]["url"], "data:image/png;base64,aGk=");
}

#[test]
fn transform_response_inline_data_audio_emits_output_audio() {
    let mut body = json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "inlineData": {
                        "mimeType": "audio/wav",
                        "data": "aGk="
                    }
                }]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 3}
    });
    transform_gemini_response(&mut body).expect("transform must succeed");

    let content = body
        .pointer("/choices/0/message/content")
        .expect("content must be present");
    assert!(content.is_array(), "content must be a parts array");
    let parts = content.as_array().expect("array");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["type"], "output_audio");
    assert_eq!(parts[0]["audio"]["data"], "aGk=");
    assert_eq!(parts[0]["audio"]["format"], "wav");
}

#[test]
fn transform_response_text_only_back_compat() {
    let mut body = json!({
        "candidates": [{
            "content": {
                "parts": [{"text": "Hello!"}]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 3}
    });
    transform_gemini_response(&mut body).expect("transform must succeed");

    let content = body
        .pointer("/choices/0/message/content")
        .expect("content must be present");
    assert!(
        content.is_string(),
        "text-only response must be a scalar string, got: {content}"
    );
    assert_eq!(content.as_str().unwrap(), "Hello!");
}

#[test]
fn transform_request_response_modalities_translated() {
    let mut body = json!({
        "model": "gemini-2.0-flash",
        "messages": [{"role": "user", "content": "hi"}],
        "modalities": ["text", "image"]
    });
    transform_gemini_request(&mut body).expect("transform must succeed");

    let modalities = body
        .pointer("/generationConfig/responseModalities")
        .expect("responseModalities must be set");
    assert!(modalities.is_array());
    let arr = modalities.as_array().expect("array");
    assert!(arr.contains(&json!("TEXT")), "expected TEXT in {arr:?}");
    assert!(arr.contains(&json!("IMAGE")), "expected IMAGE in {arr:?}");
}
