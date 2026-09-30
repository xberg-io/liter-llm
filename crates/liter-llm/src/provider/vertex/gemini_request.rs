//! OpenAI chat request -> Gemini `generateContent` request translation.

use serde_json::{Map, Value, json};

use super::{
    convert_user_content_to_gemini, reasoning_effort_to_thinking_budget, transform_gemini_embed_request,
    translate_tool_choice,
};
use crate::error::Result;

/// Convert an OpenAI-style chat request to Gemini `generateContent` format.
///
/// Key translations:
/// - System messages → `systemInstruction.parts[]`.
/// - Assistant role → `model` role.
/// - Tool calls → `functionCall` parts; tool results → `functionResponse` parts.
/// - Generation parameters → `generationConfig`.
/// - Multimodal content arrays → Gemini's `inlineData` / `fileData` format.
/// - `response_format` → `generationConfig.responseMimeType`.
/// - `tool_choice` → `toolConfig.functionCallingConfig.mode`.
/// - `extra_body.safety_settings` → top-level `safetySettings` array.
/// - `extra_body.grounding_config` / `google_search_retrieval` → `tools` entry.
/// - `extra_body.cached_content` → top-level `cachedContent` field.
/// - `ContentPart::Document` → `inlineData` with the document's MIME type.
pub(crate) fn transform_gemini_request(body: &mut Value) -> Result<()> {
    if body.get("input").is_some() && body.get("messages").is_none() {
        return transform_gemini_embed_request(body);
    }

    let extra_body = body
        .as_object_mut()
        .and_then(|o| o.remove("extra_body"))
        .and_then(|v| match v {
            Value::Object(map) => Some(map),
            _ => None,
        });

    let messages = body
        .as_object_mut()
        .and_then(|o| o.remove("messages"))
        .and_then(|v| match v {
            Value::Array(arr) => Some(arr),
            _ => None,
        })
        .unwrap_or_default();

    let (system_parts, contents) = convert_messages(&messages);
    let gen_config = generation_config(body);
    let mut tools_value = function_declarations(body);
    let tool_config = translate_tool_choice(body.get("tool_choice"));
    let (safety_settings, cached_content) = extra_body_settings(extra_body.as_ref(), &mut tools_value);

    warn_on_dropped_params(body);

    let mut new_body = json!({"contents": contents});
    if !system_parts.is_empty() {
        new_body["systemInstruction"] = json!({"parts": system_parts});
    }
    if let Some(obj) = gen_config.as_object()
        && !obj.is_empty()
    {
        new_body["generationConfig"] = gen_config;
    }
    if let Some(tools) = tools_value {
        new_body["tools"] = tools;
    }
    if let Some(tc) = tool_config {
        new_body["toolConfig"] = tc;
    }
    if let Some(ss) = safety_settings {
        new_body["safetySettings"] = ss;
    }
    if let Some(cc) = cached_content {
        new_body["cachedContent"] = cc;
    }

    *body = new_body;
    Ok(())
}

/// Split OpenAI messages into Gemini `systemInstruction` parts and `contents`;
/// unknown roles are dropped.
fn convert_messages(messages: &[Value]) -> (Vec<Value>, Vec<Value>) {
    let mut system_parts: Vec<Value> = vec![];
    let mut contents: Vec<Value> = vec![];

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        let content = msg.get("content");

        match role {
            "system" | "developer" => {
                if let Some(text) = content.and_then(|c| c.as_str()) {
                    system_parts.push(json!({"text": text}));
                }
            }
            "user" => {
                let parts = convert_user_content_to_gemini(content);
                contents.push(json!({"role": "user", "parts": parts}));
            }
            "assistant" => contents.push(model_message(msg, content)),
            "tool" => {
                let name = msg.get("name").and_then(|n| n.as_str()).unwrap_or("tool");
                let result_content = content.cloned().unwrap_or(json!(null));
                contents.push(json!({
                    "role": "user",
                    "parts": [{
                        "functionResponse": {
                            "name": name,
                            "response": {"result": result_content}
                        }
                    }]
                }));
            }
            _ => {}
        }
    }

    (system_parts, contents)
}

/// An assistant turn as a Gemini `model` turn: non-empty text plus one
/// `functionCall` part per tool call, or a single empty text part.
fn model_message(msg: &Value, content: Option<&Value>) -> Value {
    let mut parts: Vec<Value> = vec![];
    if let Some(text) = content.and_then(|c| c.as_str())
        && !text.is_empty()
    {
        parts.push(json!({"text": text}));
    }
    if let Some(tool_calls) = msg.get("tool_calls").and_then(|t| t.as_array()) {
        for tc in tool_calls {
            parts.push(json!({
                "functionCall": {
                    "name": tc.pointer("/function/name"),
                    "args": function_call_args(tc)
                }
            }));
        }
    }
    if parts.is_empty() {
        parts.push(json!({"text": ""}));
    }
    json!({"role": "model", "parts": parts})
}

/// Parse a tool call's JSON `arguments` string into `functionCall.args`,
/// falling back to `{}` (with a warning when the string is not valid JSON).
fn function_call_args(tc: &Value) -> Value {
    match tc.pointer("/function/arguments").and_then(|a| a.as_str()) {
        Some(args_str) => serde_json::from_str(args_str).unwrap_or_else(|error| {
            tracing::warn!(
                %error,
                "Gemini tool_calls[].function.arguments was not valid JSON; using an empty \
                 object"
            );
            json!({})
        }),
        None => json!({}),
    }
}

/// `generationConfig` from sampling, stop, logprobs, response-format,
/// modality and reasoning-effort fields.
fn generation_config(body: &Value) -> Value {
    let mut gen_config = json!({});
    if let Some(max_tokens) = body.get("max_completion_tokens").or_else(|| body.get("max_tokens")) {
        gen_config["maxOutputTokens"] = max_tokens.clone();
    }
    if let Some(temp) = body.get("temperature") {
        gen_config["temperature"] = temp.clone();
    }
    if let Some(top_p) = body.get("top_p") {
        gen_config["topP"] = top_p.clone();
    }
    if let Some(stop) = body.get("stop") {
        let sequences = if let Some(s) = stop.as_str() {
            vec![json!(s)]
        } else {
            stop.as_array().cloned().unwrap_or_default()
        };
        gen_config["stopSequences"] = json!(sequences);
    }

    // ~keep Gemini's GenerationConfig has a direct equivalent: `responseLogprobs` (bool) turns
    // ~keep logprobs on, `logprobs` (int) is the top-N count — the same pair as OpenAI's
    // ~keep `logprobs`/`top_logprobs`, just under swapped field names.
    if let Some(logprobs) = body.get("logprobs").and_then(|v| v.as_bool()) {
        gen_config["responseLogprobs"] = json!(logprobs);
    }
    if let Some(top_logprobs) = body.get("top_logprobs") {
        gen_config["logprobs"] = top_logprobs.clone();
    }

    if let Some(rf) = body.get("response_format") {
        let rf_type = rf.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match rf_type {
            "json_object" => {
                gen_config["responseMimeType"] = json!("application/json");
            }
            "json_schema" => {
                gen_config["responseMimeType"] = json!("application/json");
                if let Some(schema) = rf.get("json_schema").and_then(|s| s.get("schema")) {
                    gen_config["responseSchema"] = schema.clone();
                }
            }
            _ => {}
        }
    }

    if let Some(modalities) = body.get("modalities").and_then(|m| m.as_array()) {
        let gemini_modalities: Vec<Value> = modalities
            .iter()
            .filter_map(|m| m.as_str())
            .map(|m| json!(m.to_uppercase()))
            .collect();
        if !gemini_modalities.is_empty() {
            gen_config["responseModalities"] = json!(gemini_modalities);
        }
    }

    // ~keep `reasoning_effort` was previously dropped entirely (#52): map it to Gemini's
    // `thinkingConfig.thinkingBudget`, and request `includeThoughts` so the corresponding
    // thought parts actually come back (routed to `reasoning_content` in the response).
    if let Some(effort) = body.get("reasoning_effort").and_then(|e| e.as_str()) {
        let thinking_budget = reasoning_effort_to_thinking_budget(effort);
        gen_config["thinkingConfig"] = json!({
            "thinkingBudget": thinking_budget,
            "includeThoughts": true
        });
    }
    gen_config
}

/// Gemini `functionDeclarations` from OpenAI `tools`, when present.
fn function_declarations(body: &Value) -> Option<Value> {
    body.get("tools").and_then(|t| t.as_array()).map(|arr| {
        let declarations: Vec<Value> = arr
            .iter()
            .map(|t| {
                let name = t.pointer("/function/name").cloned().unwrap_or(json!("unknown"));
                let description = t.pointer("/function/description").cloned().unwrap_or(json!(""));
                let parameters = t
                    .pointer("/function/parameters")
                    .cloned()
                    .unwrap_or_else(|| json!({"type": "object"}));
                json!({
                    "name": name,
                    "description": description,
                    "parameters": parameters
                })
            })
            .collect();
        json!([{"functionDeclarations": declarations}])
    })
}

/// `safetySettings` and `cachedContent` from `extra_body`, adding the
/// search-retrieval grounding tool to `tools_value` when requested.
fn extra_body_settings(
    extra_body: Option<&Map<String, Value>>,
    tools_value: &mut Option<Value>,
) -> (Option<Value>, Option<Value>) {
    let mut safety_settings: Option<Value> = None;
    let mut cached_content: Option<Value> = None;

    if let Some(eb) = extra_body {
        if let Some(ss) = eb.get("safety_settings") {
            safety_settings = Some(ss.clone());
        }

        if eb.contains_key("grounding_config") || eb.contains_key("google_search_retrieval") {
            push_grounding_tool(tools_value);
        }

        if let Some(cc) = eb.get("cached_content") {
            cached_content = Some(cc.clone());
        }
    }

    (safety_settings, cached_content)
}

/// Append the `google_search_retrieval` tool, creating the tools array if absent.
fn push_grounding_tool(tools_value: &mut Option<Value>) {
    let grounding_tool = json!({"google_search_retrieval": {}});
    match tools_value {
        Some(existing) => {
            if let Some(arr) = existing.as_array_mut() {
                arr.push(grounding_tool);
            }
        }
        None => {
            *tools_value = Some(json!([grounding_tool]));
        }
    }
}

/// Warn about requested fields with no Gemini equivalent, which the rebuild drops.
fn warn_on_dropped_params(body: &Value) {
    // ~keep `audio` (voice/format) and `web_search_options` have no Gemini equivalent that can
    // ~keep be mapped without guessing: Gemini's speech voices and search-grounding wiring use
    // ~keep different vocabularies/shapes than OpenAI's. Warn rather than silently drop, since a
    // ~keep caller who asked for a specific voice or web-grounded answers gets neither without
    // ~keep any signal. `modalities: ["audio"]` itself still maps to `responseModalities` above.
    if body.get("audio").is_some() {
        tracing::warn!(
            "chat request set `audio`, which has no Gemini equivalent and was dropped; voice and \
             format cannot be configured for this provider"
        );
    }
    if body.get("web_search_options").is_some() {
        tracing::warn!(
            "chat request set `web_search_options`, which has no Gemini equivalent and was \
             dropped; configure grounding via extra_body.grounding_config or \
             extra_body.google_search_retrieval instead"
        );
    }
}
