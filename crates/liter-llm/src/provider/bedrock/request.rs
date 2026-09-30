//! OpenAI chat request -> Bedrock Converse request translation.

use serde_json::{Value, json};

use super::{convert_content_to_bedrock_blocks, reasoning_effort_to_budget_tokens};
use crate::error::Result;

/// Convert an OpenAI-style chat request to Bedrock Converse API format.
///
/// See [`super::BedrockProvider`]'s `transform_request` for the differences
/// handled. The body is rebuilt wholesale, so any field not mapped here is
/// dropped.
pub(super) fn transform_converse_request(body: &mut Value) -> Result<()> {
    // ~keep The Bedrock Converse API's InferenceConfiguration documents both `temperature`
    // ~keep and `topP` as 0-1, narrower than this crate's OpenAI-shaped 0.0-2.0 doc for
    // ~keep `temperature`, regardless of the underlying foundation model (Anthropic, Titan,
    // ~keep Llama, ...) since Converse enforces this at the gateway, not the model. See
    // ~keep https://docs.aws.amazon.com/bedrock/latest/APIReference/API_runtime_InferenceConfiguration.html.
    // ~keep Reject out-of-range values locally rather than forwarding them for a 400.
    crate::provider::validate_sampling_param_range(body, "temperature", "Bedrock", 0.0, 1.0)?;
    crate::provider::validate_sampling_param_range(body, "top_p", "Bedrock", 0.0, 1.0)?;

    let messages = body
        .as_object_mut()
        .and_then(|o| o.remove("messages"))
        .and_then(|v| match v {
            serde_json::Value::Array(arr) => Some(arr),
            _ => None,
        })
        .unwrap_or_default();

    let (mut system_parts, converse_messages) = convert_messages(&messages);
    let inference_config = inference_config(body);
    let tool_config = tool_config(body);
    let additional_model_fields = additional_model_fields(body);
    append_response_format_instruction(body, &mut system_parts);
    let guardrail_config = body.get("extra_body").and_then(|eb| eb.get("guardrailConfig")).cloned();

    let mut new_body = json!({
        "messages": converse_messages,
    });
    if !system_parts.is_empty() {
        new_body["system"] = json!(system_parts);
    }
    if let Some(obj) = inference_config.as_object()
        && !obj.is_empty()
    {
        new_body["inferenceConfig"] = inference_config;
    }
    if let Some(tc) = tool_config {
        new_body["toolConfig"] = tc;
    }
    if let Some(amf) = additional_model_fields {
        new_body["additionalModelRequestFields"] = amf;
    }
    if let Some(gc) = guardrail_config {
        new_body["guardrailConfig"] = gc;
    }

    apply_service_tier_and_metadata(body, &mut new_body);
    warn_on_dropped_params(body);

    *body = new_body;
    Ok(())
}

/// Split OpenAI messages into Converse `system` blocks and `messages`,
/// converting each role's content; unknown roles are dropped.
fn convert_messages(messages: &[Value]) -> (Vec<Value>, Vec<Value>) {
    let mut system_parts = vec![];
    let mut converse_messages = vec![];

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        let content = msg.get("content");

        match role {
            "system" | "developer" => system_parts.extend(system_text_parts(content)),
            "user" => {
                let parts = convert_content_to_bedrock_blocks(content);
                converse_messages.push(json!({"role": "user", "content": parts}));
            }
            "assistant" => converse_messages.push(assistant_message(msg, content)),
            "tool" => converse_messages.push(tool_result_message(msg, content)),
            _ => {}
        }
    }

    (system_parts, converse_messages)
}

/// Text blocks from a system/developer message: a plain string, or the `text`
/// of each array part that has one.
fn system_text_parts(content: Option<&Value>) -> Vec<Value> {
    if let Some(text) = content.and_then(|c| c.as_str()) {
        vec![json!({"text": text})]
    } else if let Some(array) = content.and_then(|c| c.as_array()) {
        array
            .iter()
            .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
            .map(|text| json!({"text": text}))
            .collect()
    } else {
        vec![]
    }
}

/// An assistant turn: non-empty text plus one `toolUse` block per tool call,
/// or a single empty text block when there is neither.
fn assistant_message(msg: &Value, content: Option<&Value>) -> Value {
    let mut parts = vec![];
    if let Some(text) = content.and_then(|c| c.as_str())
        && !text.is_empty()
    {
        parts.push(json!({"text": text}));
    }
    if let Some(tool_calls) = msg.get("tool_calls").and_then(|t| t.as_array()) {
        for tc in tool_calls {
            parts.push(json!({
                "toolUse": {
                    "toolUseId": tc.get("id"),
                    "name": tc.pointer("/function/name"),
                    "input": tool_call_input(tc)
                }
            }));
        }
    }
    if parts.is_empty() {
        parts.push(json!({"text": ""}));
    }
    json!({"role": "assistant", "content": parts})
}

/// Parse a tool call's JSON `arguments` string into the `toolUse.input` object,
/// falling back to `{}` (with a warning when the string is not valid JSON).
fn tool_call_input(tc: &Value) -> Value {
    match tc.pointer("/function/arguments").and_then(|a| a.as_str()) {
        Some(args_str) => serde_json::from_str(args_str).unwrap_or_else(|error| {
            tracing::warn!(
                %error,
                "Bedrock tool_calls[].function.arguments was not valid JSON; using an \
                 empty object"
            );
            json!({})
        }),
        None => json!({}),
    }
}

/// A `tool` message: a user turn carrying one `toolResult` block.
fn tool_result_message(msg: &Value, content: Option<&Value>) -> Value {
    let tool_call_id = msg.get("tool_call_id").and_then(|t| t.as_str()).unwrap_or("");
    // toolResult.content accepts the same text/image/document block shapes as a
    // user turn's content, so a `ToolMessage::content` of `UserContent::Parts`
    // reaches Bedrock natively via the shared block conversion. ~keep
    let result_content = convert_content_to_bedrock_blocks(content);
    let is_error = msg.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
    let status = if is_error { "error" } else { "success" };
    json!({
        "role": "user",
        "content": [{
            "toolResult": {
                "toolUseId": tool_call_id,
                "content": result_content,
                "status": status
            }
        }]
    })
}

/// `inferenceConfig` from `max_tokens` / `max_completion_tokens`,
/// `temperature`, `top_p` and `stop`.
fn inference_config(body: &Value) -> Value {
    let mut inference_config = json!({});
    if let Some(max_tokens) = body.get("max_tokens").or_else(|| body.get("max_completion_tokens")) {
        inference_config["maxTokens"] = max_tokens.clone();
    }
    if let Some(temp) = body.get("temperature") {
        inference_config["temperature"] = temp.clone();
    }
    if let Some(top_p) = body.get("top_p") {
        inference_config["topP"] = top_p.clone();
    }
    if let Some(stop) = body.get("stop") {
        let sequences = if let Some(s) = stop.as_str() {
            vec![json!(s)]
        } else {
            stop.as_array().cloned().unwrap_or_default()
        };
        inference_config["stopSequences"] = json!(sequences);
    }
    inference_config
}

/// `toolConfig` from OpenAI `tools`, when present.
fn tool_config(body: &Value) -> Option<Value> {
    body.get("tools").and_then(|tools| {
        tools.as_array().map(|arr| {
            let bedrock_tools: Vec<serde_json::Value> = arr
                .iter()
                .map(|t| {
                    let parameters = t
                        .pointer("/function/parameters")
                        .cloned()
                        .unwrap_or_else(|| json!({"type": "object"}));
                    json!({
                        "toolSpec": {
                            "name": t.pointer("/function/name"),
                            "description": t.pointer("/function/description"),
                            "inputSchema": {"json": parameters}
                        }
                    })
                })
                .collect();
            json!({"tools": bedrock_tools})
        })
    })
}

/// `additionalModelRequestFields` enabling extended thinking for `reasoning_effort`.
fn additional_model_fields(body: &Value) -> Option<Value> {
    let mut additional_model_fields: Option<serde_json::Value> = None;
    if let Some(effort) = body.get("reasoning_effort").and_then(|e| e.as_str()) {
        let budget_tokens = reasoning_effort_to_budget_tokens(effort);
        additional_model_fields = Some(json!({
            "thinking": {
                "type": "enabled",
                "budget_tokens": budget_tokens
            }
        }));
    }
    additional_model_fields
}

/// Emulate `response_format` with a JSON instruction appended to `system`.
fn append_response_format_instruction(body: &Value, system_parts: &mut Vec<Value>) {
    if let Some(response_format) = body.get("response_format") {
        let rf_type = response_format.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match rf_type {
            "json_schema" => {
                let schema = response_format.get("json_schema").and_then(|js| js.get("schema"));
                let schema_str = schema
                    .map(|s| serde_json::to_string_pretty(s).unwrap_or_default())
                    .unwrap_or_default();
                let instruction = if schema_str.is_empty() {
                    "You MUST respond with valid JSON only. No other text.".to_owned()
                } else {
                    format!(
                        "You MUST respond with valid JSON only that conforms to this schema:\n```json\n{schema_str}\n```\nNo other text outside the JSON."
                    )
                };
                system_parts.push(json!({"text": instruction}));
            }
            "json_object" => {
                system_parts.push(json!({"text": "You MUST respond with valid JSON only. No other text."}));
            }
            _ => {}
        }
    }
}

/// Map `service_tier` (except `"auto"`) and `metadata` onto their Converse equivalents.
fn apply_service_tier_and_metadata(body: &Value, new_body: &mut Value) {
    // ~keep Bedrock's Converse API has real equivalents for these two OpenAI fields:
    // ~keep `serviceTier: {type}` accepts "priority"|"default"|"flex"|"reserved", so every
    // ~keep OpenAI value except "auto" maps directly; "auto" has no Bedrock counterpart and
    // ~keep omitting `serviceTier` already gives provider-default behaviour, which is the
    // ~keep same effect. `requestMetadata` is a string-to-string map used for CloudTrail/
    // ~keep CloudWatch filtering, the same shape as our `metadata` tag map.
    if let Some(tier) = body.get("service_tier").and_then(|v| v.as_str())
        && tier != "auto"
    {
        new_body["serviceTier"] = json!({"type": tier});
    }
    if let Some(metadata) = body.get("metadata").and_then(|v| v.as_object())
        && !metadata.is_empty()
    {
        new_body["requestMetadata"] = json!(metadata);
    }
}

/// Warn about requested fields the wholesale rebuild dropped and whose absence
/// changes the response the caller gets.
fn warn_on_dropped_params(body: &Value) {
    // ~keep `logprobs`/`top_logprobs`, `audio` and `web_search_options` have no Bedrock
    // ~keep Converse API equivalent (InferenceConfiguration has no logprobs field; Claude on
    // ~keep Bedrock has no audio-output or built-in web-search-tool configuration). They are
    // ~keep already dropped by the wholesale rebuild above; warn so a caller who asked for
    // ~keep any of them can tell the request silently ignored that part of the ask.
    let logprobs_requested = body.get("logprobs").and_then(|v| v.as_bool()).unwrap_or(false);
    if logprobs_requested || body.get("top_logprobs").is_some() {
        tracing::warn!(
            "chat request set logprobs/top_logprobs, which Bedrock's Converse API does not \
             support; the fields were dropped and the response will not include log \
             probabilities"
        );
    }
    if body.get("audio").is_some() {
        tracing::warn!(
            "chat request set `audio`, which was dropped: Bedrock's Converse API has no \
             audio output support"
        );
    }
    if body.get("web_search_options").is_some() {
        tracing::warn!(
            "chat request set `web_search_options`, which was dropped: Bedrock's Converse \
             API has no built-in web-search tool configuration equivalent"
        );
    }
}
