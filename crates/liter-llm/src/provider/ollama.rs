use std::borrow::Cow;

use serde_json::{Map, Value, json};

use crate::error::{LiterLlmError, Result};
use crate::provider::{Provider, StreamFormat, unix_timestamp_secs};
use crate::types::ChatCompletionChunk;

const DEFAULT_BASE_URL: &str = "http://localhost:11434";
const PREFIXES: [&str; 2] = ["ollama/", "ollama_chat/"];

/// Request keys that belong at the top level of an Ollama request. Every other key a caller
/// puts in `extra_body` is a model setting and goes into `options`. ~keep
const TOP_LEVEL_KEYS: &[&str] = &["keep_alive", "think", "format", "logprobs", "top_logprobs"];

/// Request keys that belong at the top level of an `/api/embed` request. Every other key a caller
/// puts in `extra_body` is a model setting and goes into `options`. ~keep
const EMBED_TOP_LEVEL_KEYS: &[&str] = &["model", "input", "truncate", "keep_alive", "dimensions"];

/// Ollama's native API (`/api/chat`, `/api/embed`, `/api/tags`).
///
/// Model settings such as `num_ctx`, `top_k` and `repeat_penalty` go in `options`, where
/// Ollama reads them. Ollama ignores them on its OpenAI-compatible endpoint. A caller who
/// wants that endpoint uses the `openai/` prefix with the Ollama `base_url`.
/// Select this provider with the `ollama/` model prefix or its alias `ollama_chat/`, or with a
/// client hint of `ollama` or `ollama_chat`. A hint routes every bare model here whatever the
/// `base_url` is; a model with another provider's prefix, such as `openai/m` or the name of a
/// registered custom provider, goes to the OpenAI-compatible endpoint at that `base_url`.
pub(crate) struct OllamaProvider {
    base_url: String,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_owned())
    }
}

impl OllamaProvider {
    /// The server address. One trailing `/v1`, the OpenAI-compatible root, comes off before
    /// `/api/...` is appended, so a `base_url` written for the compatible endpoint still works.
    pub(crate) fn with_base_url(base_url: String) -> Self {
        let root = base_url.trim_end_matches('/');
        let root = root.strip_suffix("/v1").unwrap_or(root);
        Self {
            base_url: root.to_owned(),
        }
    }

    /// Whether `model` names this provider.
    pub(crate) fn owns(model: &str) -> bool {
        PREFIXES.iter().any(|prefix| model.starts_with(prefix))
    }

    /// Whether a client built with `model_hint` talks to Ollama: the hint is a model with an
    /// Ollama prefix, or exactly `ollama` or `ollama_chat`.
    pub(crate) fn names_hint(model_hint: &str) -> bool {
        Self::owns(model_hint) || PREFIXES.iter().any(|prefix| prefix.trim_end_matches('/') == model_hint)
    }
}

impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}"))))
    }

    fn matches_model(&self, model: &str) -> bool {
        Self::owns(model)
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        PREFIXES
            .iter()
            .find_map(|prefix| model.strip_prefix(prefix))
            .unwrap_or(model)
    }

    fn chat_completions_path(&self) -> &str {
        "/api/chat"
    }

    fn embeddings_path(&self) -> &str {
        "/api/embed"
    }

    fn models_path(&self) -> &str {
        "/api/tags"
    }

    fn stream_format(&self) -> StreamFormat {
        StreamFormat::Ndjson
    }

    /// The last line of a chat stream carries `done: true` and `done_reason`.
    fn stream_has_final_event(&self) -> bool {
        true
    }

    /// Each stream line carries complete tool calls, numbered from 0.
    fn stream_tool_call_indexes_restart(&self) -> bool {
        true
    }

    fn transform_request(&self, body: &mut Value) -> Result<()> {
        let Some(obj) = body.as_object_mut() else {
            return Ok(());
        };
        if obj.contains_key("messages") {
            chat_request(obj)
        } else {
            embed_request(obj)
        }
    }

    fn transform_response(&self, body: &mut Value) -> Result<()> {
        if let Some(message) = body.get("error").and_then(Value::as_str) {
            return Err(LiterLlmError::ServerError {
                message: message.to_owned(),
                status: 500,
            });
        }
        if body.get("models").is_some() {
            *body = models_response(body);
        } else if body.get("embeddings").is_some() {
            *body = embed_response(body);
        } else if body.get("message").is_some() {
            *body = chat_response(body);
        }
        Ok(())
    }

    fn transform_error(&self, error: LiterLlmError) -> LiterLlmError {
        error.map_message(plain_error_text)
    }

    fn parse_stream_event(&self, event_data: &str) -> Result<Option<ChatCompletionChunk>> {
        let line: Value = serde_json::from_str(event_data).map_err(|e| LiterLlmError::Streaming {
            message: format!("failed to parse Ollama stream line: {e}"),
        })?;
        if let Some(error) = line.get("error") {
            let message = error.as_str().map_or_else(|| error.to_string(), str::to_owned);
            return Err(LiterLlmError::ServerError { message, status: 500 });
        }
        let done = line.get("done").and_then(Value::as_bool).unwrap_or(false);
        let message = line.get("message").unwrap_or(&Value::Null);
        let content = message.get("content").and_then(Value::as_str).unwrap_or_default();
        let thinking = message.get("thinking").and_then(Value::as_str).unwrap_or_default();
        let tool_calls = native_tool_calls(message);
        if !done && content.is_empty() && thinking.is_empty() && tool_calls.is_empty() {
            return Ok(None);
        }

        let mut delta = Map::new();
        delta.insert("role".into(), json!("assistant"));
        if !content.is_empty() {
            delta.insert("content".into(), json!(content));
        }
        if !thinking.is_empty() {
            delta.insert("reasoning_content".into(), json!(thinking));
        }
        if !tool_calls.is_empty() {
            let calls: Vec<Value> = tool_calls
                .iter()
                .enumerate()
                .map(|(index, call)| {
                    json!({
                        "index": index,
                        "id": format!("call_{index}"),
                        "type": "function",
                        "function": {"name": call.0, "arguments": call.1}
                    })
                })
                .collect();
            delta.insert("tool_calls".into(), Value::Array(calls));
        }
        let finish = if done {
            json!(finish_reason(&line, false))
        } else {
            Value::Null
        };
        let mut chunk = json!({
            "id": "chatcmpl-ollama",
            "object": "chat.completion.chunk",
            "created": unix_timestamp_secs(),
            "model": line.get("model").and_then(Value::as_str).unwrap_or_default(),
            "choices": [{
                "index": 0,
                "delta": Value::Object(delta),
                "finish_reason": finish
            }]
        });
        if done && let Some(usage) = usage(&line) {
            chunk["usage"] = usage;
        }
        serde_json::from_value(chunk)
            .map(Some)
            .map_err(|e| LiterLlmError::Streaming {
                message: format!("failed to read Ollama stream line: {e}"),
            })
    }
}

/// Ollama reports `{"error": "<text>"}`: keep the text. Any other message stays as it is.
fn plain_error_text(message: String) -> String {
    serde_json::from_str::<Value>(&message)
        .ok()
        .and_then(|body| body.get("error")?.as_str().map(str::to_owned))
        .unwrap_or(message)
}

/// Continue the tool call numbering across stream lines, and end a stream that made tool calls
/// with `tool_calls`. Ollama reports `done_reason: "stop"` for those streams, and numbers the
/// calls of each line from 0.
pub(crate) fn number_stream_tool_calls(chunk: &mut ChatCompletionChunk, seen: &mut u32) {
    use crate::types::FinishReason;
    for choice in &mut chunk.choices {
        if let Some(calls) = choice.delta.tool_calls.as_mut() {
            let count = u32::try_from(calls.len()).unwrap_or(u32::MAX);
            for call in calls {
                call.index += *seen;
                call.id = Some(format!("call_{}", call.index));
            }
            *seen += count;
        }
        if *seen > 0 && choice.finish_reason == Some(FinishReason::Stop) {
            choice.finish_reason = Some(FinishReason::ToolCalls);
        }
    }
}

fn bad_request(message: impl Into<String>) -> LiterLlmError {
    LiterLlmError::BadRequest {
        message: message.into(),
        status: 400,
    }
}

/// Rewrite an OpenAI chat request into Ollama's `/api/chat` shape.
fn chat_request(obj: &mut Map<String, Value>) -> Result<()> {
    warn_on_dropped_params(obj);
    let extra_body = obj.remove("extra_body");
    let mut options = Map::new();
    for (from, to) in [
        ("temperature", "temperature"),
        ("top_p", "top_p"),
        ("seed", "seed"),
        ("presence_penalty", "presence_penalty"),
        ("frequency_penalty", "frequency_penalty"),
        ("max_tokens", "num_predict"),
        // ~keep max_completion_tokens is listed after max_tokens, so it wins when both are set.
        ("max_completion_tokens", "num_predict"),
    ] {
        if let Some(value) = obj.remove(from) {
            options.insert(to.into(), value);
        }
    }
    if let Some(stop) = obj.remove("stop") {
        options.insert("stop".into(), if stop.is_array() { stop } else { json!([stop]) });
    }

    let mut native = Map::new();
    for key in ["model", "stream", "tools"] {
        if let Some(value) = obj.remove(key) {
            native.insert(key.into(), value);
        }
    }
    if let Some(Value::Array(messages)) = obj.remove("messages") {
        native.insert("messages".into(), Value::Array(native_messages(messages)?));
    }
    if let Some(format) = obj.remove("response_format").and_then(native_format) {
        native.insert("format".into(), format);
    }
    // ~keep `reasoning_effort` becomes `think`. An `extra_body` `think` is applied later and wins.
    if let Some(think) = obj
        .remove("reasoning_effort")
        .as_ref()
        .and_then(Value::as_str)
        .and_then(native_think)
    {
        native.insert("think".into(), think);
    }
    let streaming = native.get("stream") == Some(&Value::Bool(true));
    let wants_logprobs = obj.get("logprobs") == Some(&Value::Bool(true));
    if streaming && wants_logprobs {
        tracing::warn!(
            "chat request set `logprobs` on a stream, which the stream chunk type cannot carry; \
             the field was dropped"
        );
    } else if wants_logprobs {
        native.insert("logprobs".into(), json!(true));
        if let Some(top) = obj.remove("top_logprobs") {
            native.insert("top_logprobs".into(), top);
        }
    }

    match extra_body {
        Some(Value::Object(extra)) => {
            // ~keep A null value sets nothing, so it is not sent.
            for (key, value) in extra {
                match value {
                    Value::Null => {}
                    Value::Object(given) if key == "options" => {
                        options.extend(given.into_iter().filter(|(_, v)| !v.is_null()));
                    }
                    value if TOP_LEVEL_KEYS.contains(&key.as_str()) => {
                        native.insert(key, value);
                    }
                    value => {
                        options.insert(key, value);
                    }
                }
            }
        }
        Some(other) => {
            native.insert("extra_body".into(), other);
        }
        None => {}
    }
    if !options.is_empty() {
        native.insert("options".into(), Value::Object(options));
    }
    *obj = native;
    Ok(())
}

/// The Ollama `think` value for an OpenAI `reasoning_effort`. Ollama knows `low`, `medium` and
/// `high`, so `minimal` becomes `low` and `max` becomes `high`; `none` turns thinking off.
fn native_think(effort: &str) -> Option<Value> {
    match effort {
        "minimal" | "low" => Some(json!("low")),
        "medium" => Some(json!("medium")),
        "high" | "max" => Some(json!("high")),
        "none" => Some(json!(false)),
        _ => None,
    }
}

/// Fields Ollama's `/api/chat` has no equivalent for. Like the Anthropic provider, drop them,
/// and warn when the caller asked for something that changes the reply. ~keep
fn warn_on_dropped_params(obj: &Map<String, Value>) {
    for key in ["metadata", "audio", "web_search_options"] {
        if obj.contains_key(key) {
            tracing::warn!(
                "chat request set `{key}`, which Ollama's chat API has no equivalent for; the field was dropped"
            );
        }
    }
    let wants_non_text = obj
        .get("modalities")
        .and_then(Value::as_array)
        .is_some_and(|modalities| modalities.iter().any(|m| m.as_str() != Some("text")));
    if wants_non_text {
        tracing::warn!(
            "chat request set `modalities` to include a non-text modality, which Ollama cannot produce; the field was dropped"
        );
    }
    if obj
        .get("tool_choice")
        .is_some_and(|choice| !matches!(choice.as_str(), Some("auto")))
    {
        tracing::warn!(
            "chat request set `tool_choice`, which Ollama's chat API has no equivalent for; the field was dropped"
        );
    }
}

fn native_format(response_format: Value) -> Option<Value> {
    match response_format.get("type").and_then(Value::as_str)? {
        "json_object" => Some(json!("json")),
        "json_schema" => response_format.get("json_schema")?.get("schema").cloned(),
        _ => None,
    }
}

fn native_messages(messages: Vec<Value>) -> Result<Vec<Value>> {
    let mut tool_names: Vec<(String, String)> = Vec::new();
    let mut out = Vec::with_capacity(messages.len());
    for message in messages {
        let Value::Object(mut message) = message else {
            return Err(bad_request("each message must be a JSON object"));
        };
        let role = match message.get("role").and_then(Value::as_str) {
            Some("developer") => "system".to_owned(),
            Some(role) => role.to_owned(),
            None => return Err(bad_request("each message needs a role")),
        };
        let mut native = Map::new();
        native.insert("role".into(), json!(role));

        let (text, images) = native_content(message.remove("content"))?;
        native.insert("content".into(), json!(text));
        if !images.is_empty() {
            native.insert("images".into(), json!(images));
        }

        if let Some(Value::Array(calls)) = message.remove("tool_calls") {
            let mut native_calls = Vec::with_capacity(calls.len());
            for call in calls {
                let function = call.get("function").unwrap_or(&Value::Null);
                let name = function.get("name").and_then(Value::as_str).unwrap_or_default();
                let arguments = match function.get("arguments") {
                    Some(Value::String(raw)) if raw.trim().is_empty() => json!({}),
                    Some(Value::String(raw)) => serde_json::from_str::<Value>(raw).map_err(|e| {
                        bad_request(format!("tool call arguments for `{name}` are not valid JSON: {e}"))
                    })?,
                    Some(other) => other.clone(),
                    None => json!({}),
                };
                if let Some(id) = call.get("id").and_then(Value::as_str) {
                    tool_names.push((id.to_owned(), name.to_owned()));
                }
                native_calls.push(json!({"function": {"name": name, "arguments": arguments}}));
            }
            native.insert("tool_calls".into(), Value::Array(native_calls));
        }
        if role == "tool"
            && let Some(id) = message.get("tool_call_id").and_then(Value::as_str)
            && let Some((_, name)) = tool_names.iter().find(|(known, _)| known == id)
        {
            native.insert("tool_name".into(), json!(name));
        }
        out.push(Value::Object(native));
    }
    Ok(out)
}

/// Split OpenAI message content into the text and the base64 images Ollama expects.
fn native_content(content: Option<Value>) -> Result<(String, Vec<String>)> {
    match content {
        None | Some(Value::Null) => Ok((String::new(), Vec::new())),
        Some(Value::String(text)) => Ok((text, Vec::new())),
        Some(Value::Array(parts)) => {
            let mut text = String::new();
            let mut images = Vec::new();
            for part in parts {
                match part.get("type").and_then(Value::as_str) {
                    Some("text") => text.push_str(part.get("text").and_then(Value::as_str).unwrap_or_default()),
                    Some("image_url") => {
                        let url = part
                            .get("image_url")
                            .and_then(|image| image.get("url"))
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        let Some((header, data)) = url.split_once(',').filter(|(h, _)| h.starts_with("data:")) else {
                            return Err(bad_request(
                                "ollama cannot fetch an image from a URL; send the image as a data URL",
                            ));
                        };
                        if !header.ends_with(";base64") {
                            return Err(bad_request("ollama needs base64 image data URLs"));
                        }
                        images.push(data.to_owned());
                    }
                    other => {
                        return Err(bad_request(format!(
                            "ollama does not support the message content type {}",
                            other.unwrap_or("(missing)")
                        )));
                    }
                }
            }
            Ok((text, images))
        }
        Some(_) => Err(bad_request("message content must be a string or a list of parts")),
    }
}

fn embed_request(obj: &mut Map<String, Value>) -> Result<()> {
    if obj.get("encoding_format").and_then(Value::as_str) == Some("base64") {
        tracing::warn!(
            "embedding request set `encoding_format` to base64, which Ollama cannot return; the vectors are floats"
        );
    }
    let mut native = Map::new();
    for key in ["model", "dimensions"] {
        if let Some(value) = obj.remove(key) {
            native.insert(key.into(), value);
        }
    }
    match obj.remove("input") {
        Some(input @ (Value::String(_) | Value::Array(_))) => {
            if input
                .as_array()
                .is_some_and(|items| items.iter().any(|item| !item.is_string()))
            {
                return Err(bad_request("ollama embeds text input only"));
            }
            native.insert("input".into(), input);
        }
        _ => return Err(bad_request("embedding input must be a string or a list of strings")),
    }
    embed_extra_body(obj.remove("extra_body"), &mut native);
    *obj = native;
    Ok(())
}

/// Apply the caller's `extra_body` to a native embed request, as chat does.
///
/// The keys `/api/embed` takes at the top level (`model`, `input`, `truncate`, `keep_alive`,
/// `dimensions`) replace the request's own value. Every other key is a model setting such as
/// `num_ctx` and goes into `options`. An explicit `options` object is merged into those keys,
/// and its keys win on a conflict. A null value sets nothing, so it is not sent. A value that is
/// not an object stays on the body as `extra_body`, where the client drops it with a warning.
fn embed_extra_body(extra_body: Option<Value>, native: &mut Map<String, Value>) {
    let extra = match extra_body {
        Some(Value::Object(extra)) => extra,
        Some(Value::Null) | None => return,
        Some(other) => {
            native.insert("extra_body".into(), other);
            return;
        }
    };
    let mut options = Map::new();
    let mut explicit = Map::new();
    for (key, value) in extra {
        match value {
            Value::Null => {}
            Value::Object(given) if key == "options" => {
                explicit.extend(given.into_iter().filter(|(_, v)| !v.is_null()));
            }
            value if EMBED_TOP_LEVEL_KEYS.contains(&key.as_str()) => {
                native.insert(key, value);
            }
            value => {
                options.insert(key, value);
            }
        }
    }
    options.extend(explicit);
    if !options.is_empty() {
        native.insert("options".into(), Value::Object(options));
    }
}

/// The tool calls of a native message as `(name, arguments as JSON text)`.
fn native_tool_calls(message: &Value) -> Vec<(String, String)> {
    message
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .map(|call| {
                    let function = call.get("function").unwrap_or(&Value::Null);
                    let name = function.get("name").and_then(Value::as_str).unwrap_or_default();
                    let arguments = match function.get("arguments") {
                        Some(Value::String(raw)) => raw.clone(),
                        Some(other) => other.to_string(),
                        None => "{}".to_owned(),
                    };
                    (name.to_owned(), arguments)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Ollama's `done_reason` as an OpenAI finish reason. A reply with tool calls finishes with
/// `tool_calls`, which Ollama does not report.
fn finish_reason(reply: &Value, has_tool_calls: bool) -> &str {
    if has_tool_calls {
        return "tool_calls";
    }
    match reply.get("done_reason").and_then(Value::as_str) {
        Some("length") => "length",
        _ => "stop",
    }
}

/// Token counts from `prompt_eval_count` and `eval_count`, when Ollama sent either.
fn usage(reply: &Value) -> Option<Value> {
    let prompt = reply.get("prompt_eval_count").and_then(Value::as_u64);
    let completion = reply.get("eval_count").and_then(Value::as_u64);
    if prompt.is_none() && completion.is_none() {
        return None;
    }
    let (prompt, completion) = (prompt.unwrap_or(0), completion.unwrap_or(0));
    Some(json!({
        "prompt_tokens": prompt,
        "completion_tokens": completion,
        "total_tokens": prompt + completion
    }))
}

fn chat_response(reply: &Value) -> Value {
    let message = reply.get("message").unwrap_or(&Value::Null);
    let content = message.get("content").and_then(Value::as_str).unwrap_or_default();
    let tool_calls = native_tool_calls(message);

    let mut out_message = Map::new();
    out_message.insert("role".into(), json!("assistant"));
    out_message.insert("content".into(), json!(content));
    if let Some(thinking) = message
        .get("thinking")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
    {
        out_message.insert("reasoning_content".into(), json!(thinking));
    }
    if !tool_calls.is_empty() {
        let calls: Vec<Value> = tool_calls
            .iter()
            .enumerate()
            .map(|(index, (name, arguments))| {
                json!({
                    "id": format!("call_{index}"),
                    "type": "function",
                    "function": {"name": name, "arguments": arguments}
                })
            })
            .collect();
        out_message.insert("tool_calls".into(), Value::Array(calls));
    }

    let mut out = json!({
        "id": "chatcmpl-ollama",
        "object": "chat.completion",
        "created": unix_timestamp_secs(),
        "model": reply.get("model").and_then(Value::as_str).unwrap_or_default(),
        "choices": [{
            "index": 0,
            "message": Value::Object(out_message),
            "finish_reason": finish_reason(reply, !tool_calls.is_empty())
        }]
    });
    if let Some(usage) = usage(reply) {
        out["usage"] = usage;
    }
    if let Some(logprobs) = reply
        .get("logprobs")
        .filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))
    {
        out["choices"][0]["logprobs"] = json!({"content": logprobs});
    }
    out
}

fn models_response(reply: &Value) -> Value {
    let data: Vec<Value> = reply
        .get("models")
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter_map(|model| {
                    let id = model.get("name").or_else(|| model.get("model"))?.as_str()?;
                    let created = model
                        .get("modified_at")
                        .and_then(Value::as_str)
                        .and_then(unix_seconds)
                        .unwrap_or(0);
                    Some(json!({"id": id, "object": "model", "created": created, "owned_by": "ollama"}))
                })
                .collect()
        })
        .unwrap_or_default();
    json!({"object": "list", "data": data})
}

/// Seconds since the Unix epoch for an RFC 3339 time such as `2026-01-01T00:00:00.5-07:00`.
fn unix_seconds(time: &str) -> Option<u64> {
    let (date, rest) = time.split_once('T')?;
    let mut ymd = date.splitn(3, '-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (ymd.next()??, ymd.next()??, ymd.next()??);
    let clock_end = rest.find(['Z', '+', '-']).unwrap_or(rest.len());
    let (clock, zone) = rest.split_at(clock_end);
    let mut hms = clock.splitn(3, ':');
    let hour = hms.next()?.parse::<i64>().ok()?;
    let minute = hms.next()?.parse::<i64>().ok()?;
    let second = hms.next()?.split('.').next()?.parse::<i64>().ok()?;
    let offset = match zone.chars().next()? {
        'Z' => 0,
        sign @ ('+' | '-') => {
            let (h, m) = zone[1..].split_once(':')?;
            let seconds = h.parse::<i64>().ok()? * 3600 + m.parse::<i64>().ok()? * 60;
            if sign == '+' { seconds } else { -seconds }
        }
        _ => return None,
    };
    // Days since 1970-01-01 for a proleptic Gregorian date.
    let shifted_year = if month <= 2 { year - 1 } else { year };
    let era = shifted_year.div_euclid(400);
    let year_of_era = shifted_year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    u64::try_from(days * 86_400 + hour * 3600 + minute * 60 + second - offset).ok()
}

fn embed_response(reply: &Value) -> Value {
    let data: Vec<Value> = reply
        .get("embeddings")
        .and_then(Value::as_array)
        .map(|vectors| {
            vectors
                .iter()
                .enumerate()
                .map(|(index, vector)| json!({"object": "embedding", "index": index, "embedding": vector}))
                .collect()
        })
        .unwrap_or_default();
    let prompt = reply.get("prompt_eval_count").and_then(Value::as_u64).unwrap_or(0);
    json!({
        "object": "list",
        "data": data,
        "model": reply.get("model").and_then(Value::as_str).unwrap_or_default(),
        "usage": {"prompt_tokens": prompt, "completion_tokens": 0, "total_tokens": prompt}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_string_becomes_a_list() {
        let mut body = json!({"model": "m", "messages": [], "stop": "END"});
        OllamaProvider::default().transform_request(&mut body).unwrap();
        assert_eq!(body["options"]["stop"], json!(["END"]));
    }

    #[test]
    fn max_completion_tokens_beats_max_tokens() {
        let mut body = json!({"model": "m", "messages": [], "max_tokens": 5, "max_completion_tokens": 9});
        OllamaProvider::default().transform_request(&mut body).unwrap();
        assert_eq!(body["options"]["num_predict"], 9);
    }

    #[test]
    fn extra_body_think_stays_top_level() {
        let mut body = json!({"model": "m", "messages": [], "extra_body": {"think": false, "num_ctx": 8}});
        OllamaProvider::default().transform_request(&mut body).unwrap();
        assert_eq!(body["think"], false);
        assert_eq!(body["options"], json!({"num_ctx": 8}));
        assert!(body.get("extra_body").is_none());
    }

    #[test]
    fn image_url_is_refused() {
        let mut body = json!({"model": "m", "messages": [{"role": "user", "content": [
            {"type": "image_url", "image_url": {"url": "https://example.com/a.png"}}]}]});
        let error = OllamaProvider::default().transform_request(&mut body).unwrap_err();
        assert!(error.to_string().contains("data URL"), "{error}");
    }

    #[test]
    fn invalid_tool_arguments_are_refused() {
        let mut body = json!({"model": "m", "messages": [{"role": "assistant", "content": "", "tool_calls": [
            {"id": "c", "type": "function", "function": {"name": "f", "arguments": "{oops"}}]}]});
        assert!(OllamaProvider::default().transform_request(&mut body).is_err());
    }

    #[test]
    fn json_schema_format_is_passed_as_the_schema() {
        let mut body = json!({"model": "m", "messages": [], "response_format":
            {"type": "json_schema", "json_schema": {"name": "n", "schema": {"type": "object"}}}});
        OllamaProvider::default().transform_request(&mut body).unwrap();
        assert_eq!(body["format"], json!({"type": "object"}));
    }

    #[test]
    fn plain_error_reply_is_an_error() {
        let mut body = json!({"error": "boom"});
        assert!(OllamaProvider::default().transform_response(&mut body).is_err());
    }

    #[test]
    fn reply_without_counts_has_no_usage() {
        let mut body = json!({"model": "m", "message": {"role": "assistant", "content": "x"}, "done": true});
        OllamaProvider::default().transform_response(&mut body).unwrap();
        assert!(body.get("usage").is_none());
        assert_eq!(body["choices"][0]["finish_reason"], "stop");
    }

    #[test]
    fn base_url_loses_a_trailing_slash() {
        let provider = OllamaProvider::with_base_url("http://h:1/".into());
        assert_eq!(provider.build_url("/api/chat", "m"), "http://h:1/api/chat");
    }

    #[test]
    fn base_url_loses_one_trailing_v1() {
        for base_url in ["http://h:1/v1", "http://h:1/v1/"] {
            let provider = OllamaProvider::with_base_url(base_url.into());
            assert_eq!(
                provider.build_url("/api/chat", "m"),
                "http://h:1/api/chat",
                "{base_url}"
            );
        }
        let provider = OllamaProvider::with_base_url("http://h:1/v1/v1".into());
        assert_eq!(provider.build_url("/api/chat", "m"), "http://h:1/v1/api/chat");
        let provider = OllamaProvider::with_base_url("http://h:1/v10".into());
        assert_eq!(provider.build_url("/api/chat", "m"), "http://h:1/v10/api/chat");
        for base_url in ["http://h:1/dev1", "http://h:1/dev1/"] {
            let provider = OllamaProvider::with_base_url(base_url.into());
            assert_eq!(
                provider.build_url("/api/chat", "m"),
                "http://h:1/dev1/api/chat",
                "{base_url}: only a whole `/v1` segment comes off"
            );
        }
    }

    #[test]
    fn embed_extra_body_follows_the_chat_rule() {
        let cases = [
            (
                "top level keys",
                json!({"truncate": false, "keep_alive": "5m"}),
                json!({"model": "m", "input": "a", "truncate": false, "keep_alive": "5m"}),
            ),
            (
                "a bare setting goes to options",
                json!({"num_ctx": 8192}),
                json!({"model": "m", "input": "a", "options": {"num_ctx": 8192}}),
            ),
            (
                "explicit options win on a conflict",
                json!({"num_ctx": 1, "num_thread": 2, "options": {"num_ctx": 8192}}),
                json!({"model": "m", "input": "a", "options": {"num_ctx": 8192, "num_thread": 2}}),
            ),
            (
                "null members are not sent",
                json!({"keep_alive": null, "num_ctx": null, "options": {"top_k": null}}),
                json!({"model": "m", "input": "a"}),
            ),
            (
                "a non-object stays for the client to drop",
                json!("x"),
                json!({"model": "m", "input": "a", "extra_body": "x"}),
            ),
        ];
        for (name, extra, expected) in cases {
            let mut body = json!({"model": "m", "input": "a", "extra_body": extra});
            OllamaProvider::default().transform_request(&mut body).unwrap();
            assert_eq!(body, expected, "{name}");
        }
    }

    #[test]
    fn multimodal_embed_input_is_refused() {
        let mut body = json!({"model": "m", "input": [{"type": "text", "text": "a"}]});
        assert!(OllamaProvider::default().transform_request(&mut body).is_err());
    }

    #[test]
    fn embed_input_of_another_shape_is_refused() {
        for input in [json!(7), json!({"text": "a"}), json!(null)] {
            let mut body = json!({"model": "m", "input": input});
            let error = OllamaProvider::default().transform_request(&mut body).unwrap_err();
            assert!(error.to_string().contains("string or a list of strings"), "{error}");
        }
        let mut body = json!({"model": "m"});
        let error = OllamaProvider::default().transform_request(&mut body).unwrap_err();
        assert!(error.to_string().contains("string or a list of strings"), "{error}");
    }

    #[test]
    fn unknown_content_part_type_is_refused() {
        for part in [
            json!({"type": "input_audio", "input_audio": {"data": "AA==", "format": "wav"}}),
            json!({"text": "no type"}),
        ] {
            let mut body = json!({"model": "m", "messages": [{"role": "user", "content": [part]}]});
            let error = OllamaProvider::default().transform_request(&mut body).unwrap_err();
            assert!(error.to_string().contains("content type"), "{error}");
        }
        let mut body = json!({"model": "m", "messages": [{"role": "user", "content": [
            {"type": "input_audio", "input_audio": {"data": "AA==", "format": "wav"}}]}]});
        let error = OllamaProvider::default().transform_request(&mut body).unwrap_err();
        assert!(error.to_string().contains("input_audio"), "{error}");
    }

    #[test]
    fn content_that_is_neither_text_nor_parts_is_refused() {
        for content in [json!(7), json!({"text": "a"}), json!(true)] {
            let mut body = json!({"model": "m", "messages": [{"role": "user", "content": content}]});
            let error = OllamaProvider::default().transform_request(&mut body).unwrap_err();
            assert!(error.to_string().contains("string or a list of parts"), "{error}");
        }
    }

    #[test]
    fn developer_role_becomes_system() {
        let mut body = json!({"model": "m", "messages": [
            {"role": "developer", "content": "Be brief"}, {"role": "user", "content": "Hi"}]});
        OllamaProvider::default().transform_request(&mut body).unwrap();
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "Be brief");
        assert_eq!(body["messages"][1]["role"], "user");
    }

    #[test]
    fn extra_body_that_is_not_an_object_is_sent_as_extra_body() {
        for extra in [json!([1, 2]), json!("x"), json!(3)] {
            let mut body = json!({"model": "m", "messages": [], "extra_body": extra.clone()});
            OllamaProvider::default().transform_request(&mut body).unwrap();
            assert_eq!(body["extra_body"], extra);
            assert!(body.get("options").is_none(), "{body}");
        }
    }

    #[test]
    fn stream_request_drops_logprobs() {
        let mut body = json!({"model": "m", "messages": [], "stream": true, "logprobs": true, "top_logprobs": 2});
        OllamaProvider::default().transform_request(&mut body).unwrap();
        assert_eq!(body["stream"], true);
        assert!(body.get("logprobs").is_none(), "{body}");
        assert!(body.get("top_logprobs").is_none(), "{body}");

        let mut body = json!({"model": "m", "messages": [], "stream": false, "logprobs": true, "top_logprobs": 2});
        OllamaProvider::default().transform_request(&mut body).unwrap();
        assert_eq!(body["logprobs"], true);
        assert_eq!(body["top_logprobs"], 2);
    }
}
