//! OpenAI chat request -> Anthropic Messages API request translation.

use serde_json::{Value, json};

use super::{DEFAULT_MAX_TOKENS, is_hosted_tool_type, sanitize_tool_call_id};
use crate::error::{LiterLlmError, Result};

/// Transform an OpenAI-format request body into Anthropic Messages API format.
///
/// See [`super::AnthropicProvider`]'s `transform_request` for the full list of
/// differences handled. The steps run in the order below; each mutates `body`
/// in place.
pub(super) fn transform_request(body: &mut Value) -> Result<()> {
    // ~keep Anthropic's Messages API documents temperature as 0.0-1.0, narrower than this
    // ~keep crate's OpenAI-shaped 0.0-2.0 doc; reject out-of-range values locally rather than
    // ~keep forwarding a value Anthropic will 400 on. See
    // ~keep https://platform.claude.com/docs/en/api/messages ("Defaults to 1.0. Ranges from
    // ~keep 0.0 to 1.0."). `top_p` has no documented range in Anthropic's own reference and is
    // ~keep intentionally left unchecked.
    crate::provider::validate_sampling_param_range(body, "temperature", "Anthropic", 0.0, 1.0)?;

    let messages = take_messages(body)?;
    let (system_blocks, non_system_messages) = split_system_messages(messages);

    if !system_blocks.is_empty() {
        body["system"] = json!(system_blocks);
    }

    let converted_messages: Vec<Value> = non_system_messages
        .into_iter()
        .map(convert_message_to_anthropic)
        .collect();

    let merged_messages = merge_consecutive_same_role(converted_messages);

    body["messages"] = json!(merged_messages);

    apply_max_tokens(body);
    apply_stop_sequences(body);
    apply_tool_choice(body);
    convert_tools(body);
    apply_reasoning_effort(body);
    apply_response_format(body);
    warn_on_dropped_params(body);
    strip_unsupported_params(body);

    Ok(())
}

/// Take the `messages` array out of `body`, rejecting a missing or empty one.
fn take_messages(body: &mut Value) -> Result<Vec<Value>> {
    let messages = body
        .as_object_mut()
        .and_then(|o| o.remove("messages"))
        .and_then(|v| match v {
            Value::Array(arr) => Some(arr),
            _ => None,
        })
        .unwrap_or_default();

    if messages.is_empty() {
        return Err(LiterLlmError::BadRequest {
            message: "messages array must not be empty".to_owned(),
            status: 400,
        });
    }
    Ok(messages)
}

/// Split system/developer messages (as Anthropic `system` content blocks)
/// from every other message, preserving order within each group.
fn split_system_messages(messages: Vec<Value>) -> (Vec<Value>, Vec<Value>) {
    let mut system_blocks: Vec<Value> = Vec::new();
    let mut non_system_messages: Vec<Value> = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        match role {
            "system" | "developer" => match msg.get("content") {
                Some(Value::String(s)) if !s.is_empty() => {
                    let mut block = json!({"type": "text", "text": s});
                    if let Some(cc) = msg.get("cache_control") {
                        block["cache_control"] = cc.clone();
                    }
                    system_blocks.push(block);
                }
                Some(Value::Array(parts)) => {
                    for part in parts {
                        system_blocks.push(part.clone());
                    }
                }
                _ => {}
            },
            _ => non_system_messages.push(msg),
        }
    }

    (system_blocks, non_system_messages)
}

/// Default `max_tokens` from `max_completion_tokens` or [`DEFAULT_MAX_TOKENS`];
/// Anthropic requires the field.
fn apply_max_tokens(body: &mut Value) {
    if body.get("max_tokens").is_none() {
        if let Some(mct) = body.get("max_completion_tokens").cloned() {
            body["max_tokens"] = mct;
        } else {
            body["max_tokens"] = json!(DEFAULT_MAX_TOKENS);
        }
    }
    body.as_object_mut().map(|o| o.remove("max_completion_tokens"));
}

/// Rename `stop` to `stop_sequences`, normalised to an array.
fn apply_stop_sequences(body: &mut Value) {
    if let Some(stop) = body.as_object_mut().and_then(|o| o.remove("stop")) {
        let stop_sequences = match stop {
            Value::String(s) => json!([s]),
            arr @ Value::Array(_) => arr,
            _ => json!([]),
        };
        body["stop_sequences"] = stop_sequences;
    }
}

/// Map `tool_choice` to Anthropic semantics; `"none"` removes `tools` entirely.
fn apply_tool_choice(body: &mut Value) {
    if let Some(tool_choice) = body.as_object_mut().and_then(|o| o.remove("tool_choice")) {
        let anthropic_tool_choice = convert_tool_choice(&tool_choice);
        match anthropic_tool_choice {
            Some(tc) => {
                body["tool_choice"] = tc;
            }
            None => {
                body.as_object_mut().map(|o| o.remove("tools"));
            }
        }
    }
}

/// Convert OpenAI `function` tool wrappers to Anthropic `input_schema` tools,
/// passing hosted tool types through unchanged.
fn convert_tools(body: &mut Value) {
    if let Some(tools) = body.as_object_mut().and_then(|o| o.remove("tools"))
        && let Some(tools_array) = tools.as_array()
    {
        let anthropic_tools: Vec<Value> = tools_array
            .iter()
            .map(|tool| {
                let tool_type = tool.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if is_hosted_tool_type(tool_type) {
                    tool.clone()
                } else {
                    convert_tool_to_anthropic(tool)
                }
            })
            .collect();
        body["tools"] = json!(anthropic_tools);
    }
}

/// Map `reasoning_effort` (top-level or `extra_body`) to an extended-thinking
/// budget, raising `max_tokens` above the budget when needed.
fn apply_reasoning_effort(body: &mut Value) {
    let reasoning_effort = body
        .as_object_mut()
        .and_then(|o| o.remove("reasoning_effort"))
        .and_then(|v| v.as_str().map(String::from))
        .or_else(|| {
            body.pointer("/extra_body/reasoning_effort")
                .and_then(|v| v.as_str().map(String::from))
        });

    if let Some(effort) = reasoning_effort {
        // ~keep 1024 is Anthropic's documented minimum thinking budget_tokens, so
        // "minimal" and "low" both floor there — the duplicate value is intentional.
        let budget_tokens: u64 = match effort.as_str() {
            "minimal" => 1024,
            "low" => 1024,
            "medium" => 4096,
            "high" => 16384,
            "max" => 32768,
            _ => 4096,
        };
        body["thinking"] = json!({
            "type": "enabled",
            "budget_tokens": budget_tokens
        });

        let min_max_tokens = budget_tokens + 1;
        let current_max = body.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        if current_max < min_max_tokens {
            body["max_tokens"] = json!(min_max_tokens);
        }
    }
}

/// Emulate `response_format` with a JSON instruction prepended to `system`.
fn apply_response_format(body: &mut Value) {
    let Some(response_format) = body.as_object_mut().and_then(|o| o.remove("response_format")) else {
        return;
    };
    let rf_type = response_format.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let instruction = match rf_type {
        "json_object" => {
            json!({"type": "text", "text": "Respond with valid JSON only. Do not include any text outside the JSON object."})
        }
        "json_schema" => {
            let Some(schema_def) = response_format.get("json_schema") else {
                return;
            };
            let schema_name = schema_def.get("name").and_then(|n| n.as_str()).unwrap_or("output");
            let schema = schema_def.get("schema").cloned().unwrap_or(json!({}));
            let schema_str = serde_json::to_string_pretty(&schema).unwrap_or_default();
            let instruction_text = format!(
                "Respond with valid JSON matching the following schema named '{schema_name}':\n```json\n{schema_str}\n```\nDo not include any text outside the JSON object."
            );
            json!({"type": "text", "text": instruction_text})
        }
        _ => return,
    };
    prepend_system_instruction(body, instruction);
}

/// Insert `instruction` as the first `system` block, creating `system` when it
/// is absent or not an array.
fn prepend_system_instruction(body: &mut Value, instruction: Value) {
    if let Some(system) = body.get_mut("system").and_then(|s| s.as_array_mut()) {
        system.insert(0, instruction);
    } else {
        body["system"] = json!([instruction]);
    }
}

/// Warn about requested fields that [`strip_unsupported_params`] is about to
/// drop and whose absence changes the response the caller gets.
fn warn_on_dropped_params(body: &Value) {
    // ~keep Unlike vertex.rs/bedrock.rs, this function mutates `body` in place rather than
    // ~keep rebuilding it wholesale, so any field not explicitly named below is forwarded
    // ~keep to Anthropic verbatim. `metadata`, `store`, `prediction`, `audio`,
    // ~keep `web_search_options`, `logprobs`, `top_logprobs`, `modalities` and `seed` have
    // ~keep no Anthropic equivalent and must be stripped here or they leak onto the wire —
    // ~keep and Anthropic's Messages API validates the body strictly, rejecting *any*
    // ~keep unrecognized top-level key with a 400 "Extra inputs are not permitted" error,
    // ~keep so an unstripped field does not merely go unused, it fails every request that
    // ~keep sets it, even a value that would otherwise be a no-op (e.g. `modalities:
    // ~keep ["text"]`). For `metadata` specifically, Anthropic has its own `metadata:
    // ~keep {user_id}` field, so forwarding our arbitrary tag map would collide with a real
    // ~keep field, not just add an unrecognized one.
    let logprobs_requested = body.get("logprobs").and_then(Value::as_bool).unwrap_or(false);
    let top_logprobs_requested = body.get("top_logprobs").is_some();
    if logprobs_requested || top_logprobs_requested {
        tracing::warn!(
            "chat request set logprobs/top_logprobs, which Anthropic's Messages API does not \
             support; the fields were dropped and the response will not include log \
             probabilities"
        );
    }
    if body.get("metadata").is_some() {
        tracing::warn!(
            "chat request set `metadata`, which was dropped: Anthropic's own `metadata` \
             field only accepts `user_id` and is not compatible with arbitrary key/value tags"
        );
    }
    if body.get("audio").is_some() {
        tracing::warn!(
            "chat request set `audio`, which was dropped: Anthropic's Messages API has no \
             audio output support"
        );
    }
    if body.get("web_search_options").is_some() {
        tracing::warn!(
            "chat request set `web_search_options`, which was dropped: Anthropic has no \
             equivalent top-level field; use a `web_search_20250305` tool in `tools` instead"
        );
    }
    // ~keep `modalities: ["text"]` is a no-op for Anthropic (text is the only output it can
    // ~keep produce), so that case is dropped silently like `store`/`prediction`. A caller
    // ~keep asking for a modality Claude cannot produce (`audio`, `image`) gets text instead
    // ~keep with no indication otherwise, so that case warns like `audio` above.
    let modalities_want_non_text = body
        .get("modalities")
        .and_then(Value::as_array)
        .is_some_and(|modalities| modalities.iter().any(|m| m.as_str() != Some("text")));
    if modalities_want_non_text {
        tracing::warn!(
            "chat request set `modalities` to include a non-text modality, which Anthropic's \
             Messages API cannot produce; the field was dropped and the response will be \
             text only"
        );
    }
    if body.get("seed").is_some() {
        tracing::warn!(
            "chat request set `seed`, which Anthropic's Messages API has no equivalent for; \
             the field was dropped and output will not be reproducible via a fixed seed"
        );
    }
}

/// Remove fields Anthropic does not accept.
fn strip_unsupported_params(body: &mut Value) {
    // ~keep Keep `stream` in the body; Anthropic requires it for streaming responses.
    if let Some(obj) = body.as_object_mut() {
        for key in &[
            "n",
            "presence_penalty",
            "frequency_penalty",
            "logit_bias",
            "stream_options",
            "parallel_tool_calls",
            "service_tier",
            "user",
            "reasoning_effort",
            "extra_body",
            "logprobs",
            "top_logprobs",
            "store",
            "metadata",
            "prediction",
            "audio",
            "web_search_options",
            "modalities",
            "seed",
        ] {
            obj.remove(*key);
        }
    }
}

/// Convert an OpenAI `image_url` URL to an Anthropic image source block.
///
/// Handles two cases:
/// - Data URIs (`data:<media_type>;base64,<data>`) → base64 source.
/// - Plain URLs → url source.
fn convert_image_url_to_anthropic_source(url: &str) -> Value {
    if url.starts_with("data:")
        && let Some((header, data)) = url.split_once(',')
    {
        let media_type = header.trim_start_matches("data:").trim_end_matches(";base64");
        return json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media_type,
                "data": data
            }
        });
    }
    json!({
        "type": "image",
        "source": {"type": "url", "url": url}
    })
}

/// Merge consecutive messages with the same role by concatenating their content blocks.
/// Anthropic requires strictly alternating user/assistant roles.
fn merge_consecutive_same_role(messages: Vec<Value>) -> Vec<Value> {
    let mut merged: Vec<Value> = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");

        if let Some(last) = merged.last_mut()
            && last.get("role").and_then(|r| r.as_str()).unwrap_or("") == role
        {
            let incoming_content = content_as_blocks(msg.get("content"));

            if let Some(Value::Array(existing)) = last.get_mut("content") {
                existing.extend(incoming_content);
            } else {
                let mut combined = content_as_blocks(last.get("content"));
                combined.extend(incoming_content);
                last["content"] = json!(combined);
            }
            continue;
        }

        merged.push(msg);
    }

    merged
}

/// A message `content` value as a list of content blocks: arrays as-is, a
/// string or any other value as a single text block, absent as empty.
fn content_as_blocks(content: Option<&Value>) -> Vec<Value> {
    match content {
        Some(Value::Array(arr)) => arr.clone(),
        Some(Value::String(s)) => vec![json!({"type": "text", "text": s})],
        Some(other) => vec![json!({"type": "text", "text": other.to_string()})],
        None => vec![],
    }
}

/// Convert an OpenAI-format message JSON value to Anthropic Messages API format.
fn convert_message_to_anthropic(msg: Value) -> Value {
    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");

    match role {
        "user" => convert_user_message(&msg),
        "assistant" => convert_assistant_message(&msg),
        "tool" => convert_tool_message(&msg),
        "function" => convert_function_message(&msg),
        _ => msg,
    }
}

/// A `user` message: content parts become blocks, and a message-level
/// `cache_control` lands on the last block.
fn convert_user_message(msg: &Value) -> Value {
    let content = convert_user_content_to_anthropic(msg.get("content"));
    let mut user_msg = json!({"role": "user", "content": content});
    if let Some(cc) = msg.get("cache_control")
        && let Some(blocks) = user_msg.get_mut("content").and_then(|c| c.as_array_mut())
        && let Some(last) = blocks.last_mut()
    {
        last["cache_control"] = cc.clone();
    }
    user_msg
}

/// An `assistant` message: text and tool calls become `text` / `tool_use` blocks.
fn convert_assistant_message(msg: &Value) -> Value {
    let mut blocks: Vec<Value> = Vec::new();

    if let Some(text) = msg.get("content").and_then(|c| c.as_str())
        && !text.is_empty()
    {
        let mut block = json!({"type": "text", "text": text});
        if let Some(cc) = msg.get("cache_control") {
            block["cache_control"] = cc.clone();
        }
        blocks.push(block);
    }

    if let Some(tool_calls) = msg.get("tool_calls").and_then(|tc| tc.as_array()) {
        for tc in tool_calls {
            let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let name = tc.pointer("/function/name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments_str = tc
                .pointer("/function/arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let input: Value = serde_json::from_str(arguments_str).unwrap_or_else(|_| json!({}));
            blocks.push(json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": input
            }));
        }
    }

    if blocks.is_empty() {
        blocks.push(json!({"type": "text", "text": ""}));
    }

    json!({"role": "assistant", "content": blocks})
}

/// A `tool` message: a `user` turn carrying one `tool_result` block.
fn convert_tool_message(msg: &Value) -> Value {
    let raw_id = msg.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("");
    let tool_use_id = sanitize_tool_call_id(raw_id);

    let result_content = convert_tool_result_content_to_anthropic(msg.get("content"));

    let mut tool_result_block = json!({
        "type": "tool_result",
        "tool_use_id": tool_use_id,
        "content": result_content
    });

    if let Some(cc) = msg.get("cache_control") {
        tool_result_block["cache_control"] = cc.clone();
    }

    json!({
        "role": "user",
        "content": [tool_result_block]
    })
}

/// A legacy `function` message: a `user` turn carrying one `tool_result` block
/// keyed by the sanitized function name.
fn convert_function_message(msg: &Value) -> Value {
    let name = msg.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let sanitized_name = sanitize_tool_call_id(name);
    let content_text = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
    json!({
        "role": "user",
        "content": [{
            "type": "tool_result",
            "tool_use_id": sanitized_name,
            "content": [{"type": "text", "text": content_text}]
        }]
    })
}

/// Convert OpenAI user content (string or content-part array) to Anthropic content blocks.
fn convert_user_content_to_anthropic(content: Option<&Value>) -> Value {
    match content {
        None => json!([]),
        Some(Value::String(s)) => json!([{"type": "text", "text": s}]),
        Some(Value::Array(parts)) => {
            let blocks: Vec<Value> = parts
                .iter()
                .filter_map(|part| {
                    let part_type = part.get("type").and_then(|t| t.as_str())?;
                    match part_type {
                        "text" => {
                            let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            let mut block = json!({"type": "text", "text": text});
                            if let Some(cc) = part.get("cache_control") {
                                block["cache_control"] = cc.clone();
                            }
                            Some(block)
                        }
                        "image_url" => {
                            let url = part.pointer("/image_url/url").and_then(|u| u.as_str())?;
                            let mut block = convert_image_url_to_anthropic_source(url);
                            if let Some(cc) = part.get("cache_control") {
                                block["cache_control"] = cc.clone();
                            }
                            Some(block)
                        }
                        "document" => {
                            let data = part.pointer("/document/data").and_then(|d| d.as_str())?;
                            let media_type = part
                                .pointer("/document/media_type")
                                .and_then(|m| m.as_str())
                                .unwrap_or("application/pdf");
                            let mut block = json!({
                                "type": "document",
                                "source": {
                                    "type": "base64",
                                    "media_type": media_type,
                                    "data": data
                                }
                            });
                            if let Some(cc) = part.get("cache_control") {
                                block["cache_control"] = cc.clone();
                            }
                            Some(block)
                        }
                        _ => {
                            tracing::warn!(
                                part_type = part_type,
                                "unrecognized user content part type; falling back to text"
                            );
                            let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            if text.is_empty() {
                                None
                            } else {
                                Some(json!({"type": "text", "text": text}))
                            }
                        }
                    }
                })
                .collect();
            json!(blocks)
        }
        Some(other) => json!([{"type": "text", "text": other.to_string()}]),
    }
}

/// Convert a `ToolMessage::content` value (plain string or content-part array) to
/// Anthropic `tool_result` content blocks.
///
/// Reuses [`convert_user_content_to_anthropic`] for the text/image mapping —
/// Anthropic's `tool_result.content` accepts `TextBlockParam | ImageBlockParam`,
/// the same blocks a user turn emits for those two part types. Unlike a user
/// turn, Anthropic's API does not accept a `document` (or any other) block
/// inside `tool_result`, so any such part degrades to a text block instead of
/// being silently dropped. ~keep
fn convert_tool_result_content_to_anthropic(content: Option<&Value>) -> Value {
    let Value::Array(blocks) = convert_user_content_to_anthropic(content) else {
        return json!([{"type": "text", "text": ""}]);
    };

    let blocks: Vec<Value> = blocks
        .into_iter()
        .map(|block| match block.get("type").and_then(|t| t.as_str()) {
            Some("text") | Some("image") => block,
            other => {
                tracing::warn!(
                    block_type = other.unwrap_or("unknown"),
                    "anthropic tool_result: unsupported content block degraded to text"
                );
                json!({"type": "text", "text": "[unsupported content in tool result]"})
            }
        })
        .collect();

    json!(blocks)
}

/// Map an OpenAI `tool_choice` value to Anthropic format.
///
/// Returns `None` when the tool_choice means "none" (tools should be removed entirely).
fn convert_tool_choice(tool_choice: &Value) -> Option<Value> {
    match tool_choice {
        Value::String(s) => match s.as_str() {
            "none" => None,
            "required" => Some(json!({"type": "any"})),
            _ => Some(json!({"type": "auto"})),
        },
        Value::Object(_) => {
            let name = tool_choice.pointer("/function/name").and_then(|v| v.as_str());
            if let Some(name) = name {
                Some(json!({"type": "tool", "name": name}))
            } else {
                Some(json!({"type": "auto"}))
            }
        }
        _ => Some(json!({"type": "auto"})),
    }
}

/// Convert an OpenAI tool definition to Anthropic format.
///
/// OpenAI: `{"type": "function", "function": {"name": "X", "description": "Y", "parameters": Z}}`
/// Anthropic: `{"name": "X", "description": "Y", "input_schema": Z}`
///
/// Also normalises `input_schema.type` to `"object"` if absent or mistyped.
fn convert_tool_to_anthropic(tool: &Value) -> Value {
    let function = tool.get("function");
    let name = function.and_then(|f| f.get("name")).cloned().unwrap_or(json!(""));
    let description = function.and_then(|f| f.get("description")).cloned();
    let mut parameters = function
        .and_then(|f| f.get("parameters"))
        .cloned()
        .unwrap_or(json!({"type": "object", "properties": {}}));

    if parameters.get("type").and_then(|t| t.as_str()) != Some("object") {
        parameters["type"] = json!("object");
    }

    let mut tool_def = json!({
        "name": name,
        "input_schema": parameters
    });

    if let Some(desc) = description {
        tool_def["description"] = desc;
    }

    if let Some(cc) = tool.get("cache_control") {
        tool_def["cache_control"] = cc.clone();
    } else if let Some(cc) = function.and_then(|f| f.get("cache_control")) {
        tool_def["cache_control"] = cc.clone();
    }

    tool_def
}
