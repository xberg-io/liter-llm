use std::borrow::Cow;

use serde_json::Value;

use crate::error::{LiterLlmError, Result};
use crate::provider::{Provider, unix_timestamp_secs};
use crate::types::{
    ChatCompletionChunk, FinishReason, StreamChoice, StreamDelta, StreamFunctionCall, StreamToolCall, Usage,
};

/// Cohere provider (Command model family).
///
/// Differences from the OpenAI-compatible baseline:
/// - Chat endpoint is `/chat` instead of `/chat/completions`.
/// - Rerank endpoint is `/rerank` instead of the default path.
/// - Embed endpoint is `/embed` (v2), with a native body (`texts`, `input_type`,
///   `embedding_types`) and a native reply (`embeddings.float`).
/// - List models is `GET /v1/models`, which has no v2 form, with a `models` reply.
/// - `stream_options` is an OpenAI-specific field and must be stripped; `stream` is kept (Cohere v2 requires it).
/// - The non-streaming `/chat` response has **no `choices` wrapper**: `id`, `finish_reason`,
///   `message`, and `usage` are top-level fields (verified against Cohere's v2 API reference).
///   `finish_reason` uses Cohere-specific names (`COMPLETE`, `MAX_TOKENS`, `TOOL_CALL`).
///   `message.content` is an array of typed blocks (`text`, `thinking`), not a flat string.
///   `usage` is reported as `{billed_units: {...}, tokens: {...}}`, not OpenAI's flat
///   `prompt_tokens` / `completion_tokens`. `transform_response` rebuilds the whole body
///   into the OpenAI `choices` shape from these fields (#53).
pub struct CohereProvider;

impl Provider for CohereProvider {
    fn name(&self) -> &str {
        "cohere"
    }

    fn base_url(&self) -> &str {
        // ~keep `api.cohere.ai` is a working alias for the same API (both hosts route to the
        // ~keep same backend and return 401 rather than a DNS/redirect failure when probed
        // ~keep unauthenticated), but `api.cohere.com` is what Cohere's own API reference uses
        // ~keep for the v2 `/chat` endpoint, so it is the canonical choice here. schemas/
        // ~keep providers.json's `cohere`/`cohere_chat` entries must agree with this — do not
        // ~keep "fix" this back to `.ai`.
        "https://api.cohere.com/v2"
    }

    fn env_var(&self) -> Option<&str> {
        Some("COHERE_API_KEY")
    }

    fn auth_header<'a>(&'a self, api_key: &'a str) -> Option<(Cow<'static, str>, Cow<'a, str>)> {
        Some((Cow::Borrowed("Authorization"), Cow::Owned(format!("Bearer {api_key}"))))
    }

    fn matches_model(&self, model: &str) -> bool {
        model.starts_with("command-r")
            || model.starts_with("command-")
            || model.starts_with(EMBED_MODEL_PREFIX)
            || model.starts_with("cohere/")
    }

    fn strip_model_prefix<'m>(&self, model: &'m str) -> &'m str {
        model.strip_prefix("cohere/").unwrap_or(model)
    }

    /// Cohere uses `/chat` instead of `/chat/completions`.
    fn chat_completions_path(&self) -> &str {
        "/chat"
    }

    /// Cohere embeds at `/embed` on the v2 base.
    fn embeddings_path(&self) -> &str {
        "/embed"
    }

    /// Cohere lists models at `/models`. The path exists on v1 only: [`Self::build_url`]
    /// moves this one path off the v2 base and asks for the largest page.
    fn models_path(&self) -> &str {
        "/models"
    }

    /// Cohere uses `/rerank` at the v2 base.
    fn rerank_path(&self) -> &str {
        "/rerank"
    }

    /// Build the URL for an endpoint. Every path is relative to the v2 base, except the model
    /// list, which Cohere serves at `/v1/models` and not under `/v2`. The model list asks for
    /// `page_size=1000`, the largest page Cohere allows (the default is 20).
    fn build_url(&self, endpoint_path: &str, _model: &str) -> String {
        if endpoint_path == self.models_path() {
            return format!("{MODELS_BASE_URL}{endpoint_path}?page_size={MODELS_PAGE_SIZE}");
        }
        format!("{}{endpoint_path}", self.base_url())
    }

    /// Strip transport-level parameters that Cohere does not accept in the body, and
    /// rename `top_p` to Cohere's own nucleus-sampling field name.
    ///
    /// Note: Cohere v2 requires `stream` in the body, so only `stream_options`
    /// (an OpenAI-specific field) is removed.
    ///
    /// `temperature` is NOT range-checked here, unlike Anthropic and Bedrock. Cohere's
    /// API reference documents `temperature` as "a non-negative float" with no stated
    /// maximum, so there is no upper bound to enforce — the "0-1" figure that appears
    /// in Cohere's conceptual guides is guidance, not a schema constraint, and
    /// rejecting on it would fail requests Cohere accepts.
    ///
    /// `top_p` IS range-checked and renamed. Cohere's v2 `/chat` endpoint has no
    /// `top_p` field at all; its nucleus-sampling parameter is named `p` (default
    /// `0.75`, min `0.01`, max `0.99`, verified against Cohere's API reference) — so
    /// forwarding `top_p` verbatim meant the value was silently dropped, never
    /// reaching Cohere as a recognised field at any value the caller chose. The range
    /// check runs against the caller-facing `top_p` name *before* the rename, so a
    /// rejected request's error names the field the caller actually set (`top_p`)
    /// rather than Cohere's internal name (`p`), which the caller may never have
    /// written. Note the minimum is `0.01`, not `0.0`: `top_p: 0.0` is legal for
    /// OpenAI (and documented as legal by this crate's own `ChatCompletionRequest`)
    /// but must be rejected here rather than silently coerced to `0.01`.
    fn transform_request(&self, body: &mut Value) -> Result<()> {
        super::validate_sampling_param_range(body, "top_p", "Cohere", 0.01, 0.99)?;

        if let Some(obj) = body.as_object_mut() {
            obj.remove("stream_options");
            if let Some(top_p) = obj.remove("top_p") {
                obj.insert("p".to_owned(), top_p);
            }
        }
        Ok(())
    }

    /// Rebuild an embedding request into Cohere's embed body: see [`transform_embed_request`].
    ///
    /// The client calls this for the embeddings endpoint only, so no other request body
    /// (moderation, speech, chat) is ever taken for an embedding body.
    fn transform_embedding_request(&self, body: &mut Value) -> Result<()> {
        transform_embed_request(body)
    }

    /// Rebuild an embed reply like [`Self::transform_response`], then name the model.
    ///
    /// A Cohere embed reply carries no `model`, so the rebuilt list takes the model of the
    /// request. No other provider gets this step.
    fn transform_embedding_response(&self, body: &mut Value, request_model: &str) -> Result<()> {
        self.transform_response(body)?;
        if let Some(obj) = body.as_object_mut() {
            obj.insert("model".into(), Value::String(request_model.to_owned()));
        }
        Ok(())
    }

    /// Parse a Cohere v2 streaming SSE event into a `ChatCompletionChunk`.
    ///
    /// Cohere v2 streaming events use a `type` field to distinguish event kinds
    /// (verified against Cohere's v2 API reference, `docs.cohere.com/reference/chat-stream`
    /// and `docs.cohere.com/v2/docs/streaming`):
    /// - `message-start`: beginning of stream; id is top-level, role at `delta.message.role`
    /// - `content-delta`: text content token, extract from `delta.message.content.text`
    /// - `tool-call-start`: start of a tool call; id/name at `delta.message.tool_calls.{id,function.name}`
    /// - `tool-call-delta`: partial tool call arguments at `delta.message.tool_calls.function.arguments`
    /// - `tool-call-end`: end of a tool call (skipped)
    /// - `message-end`: end of stream; finish reason at `delta.finish_reason`, usage at
    ///   `delta.usage.billed_units.{input,output}_tokens`
    ///
    /// ~keep The previous implementation matched Cohere's legacy v1 NDJSON event names
    /// (`stream-start`/`stream-end`) and flat field paths (`delta.text`, `delta.id`,
    /// `delta.function.name`) against this v2 SSE endpoint. Every real v2 event silently
    /// missed: `content-delta` always yielded empty text, tool-call id/name/arguments were
    /// always empty, and `finish_reason`/`usage` never surfaced because the event-type
    /// strings never matched — streaming opened without error but delivered nothing.
    fn parse_stream_event(&self, event_data: &str) -> Result<Option<ChatCompletionChunk>> {
        let v: Value = serde_json::from_str(event_data).map_err(|e| LiterLlmError::Streaming {
            message: format!("failed to parse Cohere SSE event: {e}"),
        })?;

        let event_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        // ~keep An error event is either `{"error": {...}}` or a flat
        // `{"type": "error", "message": ...}`. Both go through the shared classifier,
        // so the error kind matches the same body in an HTTP reply.
        let wrapped = (v.get("error").is_none() && event_type == "error").then(|| serde_json::json!({ "error": &v }));
        let error_event = wrapped.as_ref().unwrap_or(&v);
        if let Some(err) =
            crate::provider::stream_error_event(error_event, crate::provider::status_code_field(error_event))
        {
            return Err(err);
        }

        // Cohere documents no error event. A failed generation ends with a `message-end`
        // event whose `delta.error` holds the text and whose finish reason is `ERROR`.
        if event_type == "message-end" {
            let error_text = v.pointer("/delta/error").and_then(Value::as_str);
            let errored = v.pointer("/delta/finish_reason").and_then(Value::as_str) == Some("ERROR");
            if error_text.is_some() || errored {
                return Err(LiterLlmError::Streaming {
                    message: error_text
                        .unwrap_or("Cohere ended the stream with finish reason ERROR")
                        .to_owned(),
                });
            }
        }

        Ok(match event_type {
            "message-start" => Some(cohere_message_start_chunk(&v)),
            "content-delta" => Some(cohere_content_delta_chunk(&v)),
            "tool-call-start" => Some(cohere_tool_call_start_chunk(&v)),
            "tool-call-delta" => Some(cohere_tool_call_delta_chunk(&v)),
            "message-end" => Some(cohere_message_end_chunk(&v)),
            _ => None,
        })
    }

    /// Normalize a Cohere v2 `/chat` response to OpenAI chat completion format.
    ///
    /// Cohere's response has no `choices` wrapper: `finish_reason` and `message`
    /// are top-level fields, `message.content` is an array of typed blocks
    /// (`text`, `thinking`, ...) rather than a flat string, `message.tool_calls`
    /// already matches the OpenAI `{id, type, function: {name, arguments}}` shape
    /// verbatim, and usage is reported under `usage.billed_units.{input,output}_tokens`
    /// (not OpenAI's flat `prompt_tokens` / `completion_tokens`). This rebuilds the
    /// whole body into the shape `ChatCompletionResponse` expects (#53): the
    /// previous implementation assumed the response was already `choices`-wrapped,
    /// which meant every real non-streaming Cohere response failed
    /// deserialization with a missing `choices` field.
    ///
    /// This step runs on the reply of every endpoint except rerank, so it rebuilds a reply
    /// only when the reply is the one it knows: an embed reply (`embeddings`) becomes an
    /// embeddings list, a model list (`models`) becomes an OpenAI model list, and a chat
    /// reply (`message` or `finish_reason`) becomes `choices`. Any other reply is left
    /// as it is, so a reply that is not a chat reply keeps all its fields.
    fn transform_response(&self, body: &mut Value) -> Result<()> {
        use serde_json::json;

        if body.get("embeddings").is_some() {
            return transform_embed_response(body);
        }
        if body.get("models").is_some() {
            return transform_models_response(body);
        }
        if body.get("message").is_none() && body.get("finish_reason").is_none() {
            return Ok(());
        }

        let id = body.get("id").cloned().unwrap_or_else(|| Value::String(String::new()));

        let finish_reason = normalize_cohere_response_finish_reason(body);

        let content_blocks = body.pointer("/message/content").and_then(Value::as_array).cloned();

        let text: String = content_blocks
            .as_ref()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        // ~keep Cohere "thinking" blocks are internal reasoning, not visible content;
        // route them to `reasoning_content` the same way Anthropic/Gemini thinking is handled.
        let reasoning_text: Option<String> = content_blocks.as_ref().and_then(|blocks| {
            let joined = blocks
                .iter()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("thinking"))
                .filter_map(|b| b.get("thinking").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("");
            if joined.is_empty() { None } else { Some(joined) }
        });

        let tool_calls = body.pointer("/message/tool_calls").and_then(Value::as_array).cloned();
        let has_tool_calls = tool_calls.as_ref().is_some_and(|tc| !tc.is_empty());

        let message_content = if text.is_empty() {
            Value::Null
        } else {
            Value::String(text)
        };

        let mut message = json!({"role": "assistant", "content": message_content});
        if let (Some(tc), true) = (tool_calls, has_tool_calls) {
            message["tool_calls"] = Value::Array(tc);
        }
        if let Some(reasoning) = reasoning_text {
            message["reasoning_content"] = Value::String(reasoning);
        }

        let input_tokens = body
            .pointer("/usage/billed_units/input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let output_tokens = body
            .pointer("/usage/billed_units/output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);

        *body = json!({
            "id": id,
            "object": "chat.completion",
            "created": unix_timestamp_secs(),
            "model": "",
            "choices": [{
                "index": 0,
                "message": message,
                "finish_reason": finish_reason
            }],
            "usage": {
                "prompt_tokens": input_tokens,
                "completion_tokens": output_tokens,
                "total_tokens": input_tokens + output_tokens
            }
        });

        Ok(())
    }
}

/// The model name prefix of Cohere embed models, such as `embed-english-v3.0`.
const EMBED_MODEL_PREFIX: &str = "embed-";

/// Base URL of the Cohere list-models endpoint, which has no v2 form.
const MODELS_BASE_URL: &str = "https://api.cohere.com/v1";

/// The largest `page_size` the Cohere list-models endpoint accepts. The default is 20.
const MODELS_PAGE_SIZE: u32 = 1000;

/// `input_type` sent with every embed request.
///
/// Cohere's v3 and later embed models require this field. `search_document` is the
/// indexing-side value, the same default as the Bedrock Cohere embed adapter. A caller
/// changes it through `extra_body`, whose keys the client merges over the rebuilt body.
const EMBED_DEFAULT_INPUT_TYPE: &str = "search_document";

/// Rebuild an OpenAI-style embedding body into Cohere's `POST /v2/embed` body.
///
/// Text input becomes `texts`. Multimodal input becomes one entry of `inputs`, with `text`
/// and `image_url` content parts (an `image_base64` part sends its data URL as `image_url`).
/// `dimensions` becomes `output_dimension`. The body always asks for `embedding_types:
/// ["float"]`, so the reply holds float vectors whatever `encoding_format` says. `user` has
/// no Cohere field and is dropped.
fn transform_embed_request(body: &mut Value) -> Result<()> {
    use serde_json::json;

    let model = body.get("model").cloned().unwrap_or(Value::Null);
    let input = body.get("input").cloned().unwrap_or(Value::Null);

    let mut out = json!({
        "model": model,
        "input_type": EMBED_DEFAULT_INPUT_TYPE,
        "embedding_types": ["float"],
    });

    match input {
        Value::Array(items) if items.is_empty() => {
            return Err(LiterLlmError::BadRequest {
                message: "Cohere embeddings need at least one input".into(),
                status: 400,
            });
        }
        Value::String(text) => out["texts"] = json!([text]),
        Value::Array(items) if items.iter().all(Value::is_string) => out["texts"] = Value::Array(items),
        Value::Array(parts) if !parts.iter().all(Value::is_object) => {
            return Err(LiterLlmError::BadRequest {
                message: "Cohere embeddings take a list of strings or a list of content parts; \
                          a list that mixes them, or that holds numbers or token arrays, is not supported"
                    .into(),
                status: 400,
            });
        }
        Value::Array(parts) => {
            let content = parts.iter().map(embed_content_part).collect::<Result<Vec<Value>>>()?;
            out["inputs"] = json!([{ "content": content }]);
        }
        _ => {
            return Err(LiterLlmError::BadRequest {
                message: "Cohere embeddings take a string, a list of strings or a list of content parts as input"
                    .into(),
                status: 400,
            });
        }
    }

    if let Some(dimensions) = body.get("dimensions").filter(|d| !d.is_null()) {
        out["output_dimension"] = dimensions.clone();
    }
    // The client merges `extra_body` into the body after this step, so it must survive the rebuild.
    if let Some(extra_body) = body.get("extra_body").filter(|e| !e.is_null()) {
        out["extra_body"] = extra_body.clone();
    }

    *body = out;
    Ok(())
}

/// Convert one multimodal embedding part to a Cohere `inputs[].content[]` entry.
fn embed_content_part(part: &Value) -> Result<Value> {
    use serde_json::json;

    match part.get("type").and_then(Value::as_str) {
        Some("text") => Ok(json!({"type": "text", "text": part.get("text").cloned().unwrap_or(Value::Null)})),
        Some("image_url") => {
            let url = part.pointer("/image_url/url").cloned().unwrap_or(Value::Null);
            Ok(json!({"type": "image_url", "image_url": {"url": url}}))
        }
        Some("image_base64") => {
            let url = part.get("image_base64").cloned().unwrap_or(Value::Null);
            Ok(json!({"type": "image_url", "image_url": {"url": url}}))
        }
        _ => Err(LiterLlmError::BadRequest {
            message: "unsupported embedding content part for Cohere: use `text`, `image_url` or `image_base64`".into(),
            status: 400,
        }),
    }
}

/// Rebuild a Cohere embed reply (`embeddings.float`) into an OpenAI embeddings list.
///
/// Usage comes from `meta.billed_units`: `input_tokens` plus `image_tokens`. A reply without `embeddings.float` is
/// an error: the request asks for float vectors, so a reply without them is not a usable
/// reply and must not become an empty list.
fn transform_embed_response(body: &mut Value) -> Result<()> {
    use serde_json::json;

    let vectors = body
        .pointer("/embeddings/float")
        .and_then(Value::as_array)
        .ok_or_else(|| LiterLlmError::ServerError {
            message: "Cohere embed reply has no `embeddings.float` list".into(),
            status: 502,
        })?;

    let data: Vec<Value> = vectors
        .iter()
        .enumerate()
        .map(|(index, embedding)| json!({"object": "embedding", "embedding": embedding, "index": index}))
        .collect();

    let billed = |field: &str| {
        body.pointer(&format!("/meta/billed_units/{field}"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let input_tokens = billed("input_tokens") + billed("image_tokens");

    *body = json!({
        "object": "list",
        "data": data,
        "model": "",
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": 0,
            "total_tokens": input_tokens
        }
    });
    Ok(())
}

/// Rebuild a Cohere model list (`models[].name`) into an OpenAI model list.
///
/// The request asks for the largest page ([`MODELS_PAGE_SIZE`]). A reply that still carries a
/// `next_page_token` is an error: the list would be cut without notice.
fn transform_models_response(body: &mut Value) -> Result<()> {
    use serde_json::json;

    if body
        .get("next_page_token")
        .and_then(Value::as_str)
        .is_some_and(|token| !token.is_empty())
    {
        return Err(LiterLlmError::ServerError {
            message: format!("Cohere has more than {MODELS_PAGE_SIZE} models, so the model list is cut at one page"),
            status: 502,
        });
    }

    let models = body
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| LiterLlmError::ServerError {
            message: "Cohere model list has no `models` array".into(),
            status: 502,
        })?;

    let data = models
        .iter()
        .map(|model| {
            let name = model
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| LiterLlmError::ServerError {
                    message: "Cohere model list holds a model with no `name`".into(),
                    status: 502,
                })?;
            Ok(json!({"id": name, "object": "model", "created": 0, "owned_by": "cohere"}))
        })
        .collect::<Result<Vec<Value>>>()?;

    *body = json!({"object": "list", "data": data});
    Ok(())
}

fn cohere_message_start_chunk(event: &Value) -> ChatCompletionChunk {
    let id = event.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
    let role = event
        .pointer("/delta/message/role")
        .and_then(Value::as_str)
        .unwrap_or("assistant")
        .to_owned();
    let delta = StreamDelta {
        role: Some(role),
        ..Default::default()
    };
    cohere_stream_chunk(id, delta, None, None)
}

fn cohere_content_delta_chunk(event: &Value) -> ChatCompletionChunk {
    let text = event
        .pointer("/delta/message/content/text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let delta = StreamDelta {
        content: Some(text),
        ..Default::default()
    };
    cohere_stream_chunk(String::new(), delta, None, None)
}

fn cohere_tool_call_start_chunk(event: &Value) -> ChatCompletionChunk {
    let index = event.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
    let tool_id = event
        .pointer("/delta/message/tool_calls/id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let tool_name = event
        .pointer("/delta/message/tool_calls/function/name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let delta = StreamDelta {
        tool_calls: Some(vec![StreamToolCall {
            index,
            id: Some(tool_id),
            call_type: Some(crate::types::ToolType::Function),
            function: Some(StreamFunctionCall {
                name: Some(tool_name),
                arguments: None,
            }),
        }]),
        ..Default::default()
    };
    cohere_stream_chunk(String::new(), delta, None, None)
}

fn cohere_tool_call_delta_chunk(event: &Value) -> ChatCompletionChunk {
    let index = event.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
    let arguments = event
        .pointer("/delta/message/tool_calls/function/arguments")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let delta = StreamDelta {
        tool_calls: Some(vec![StreamToolCall {
            index,
            id: None,
            call_type: None,
            function: Some(StreamFunctionCall {
                name: None,
                arguments: Some(arguments),
            }),
        }]),
        ..Default::default()
    };
    cohere_stream_chunk(String::new(), delta, None, None)
}

fn cohere_message_end_chunk(event: &Value) -> ChatCompletionChunk {
    let finish_reason = event
        .pointer("/delta/finish_reason")
        .and_then(Value::as_str)
        .map(map_cohere_finish_reason);
    cohere_stream_chunk(
        String::new(),
        StreamDelta::default(),
        finish_reason,
        extract_cohere_stream_usage(event),
    )
}

fn normalize_cohere_response_finish_reason(body: &Value) -> &str {
    match body.get("finish_reason").and_then(Value::as_str).unwrap_or("COMPLETE") {
        "COMPLETE" => "stop",
        "MAX_TOKENS" => "length",
        "TOOL_CALL" => "tool_calls",
        other => other,
    }
}

fn cohere_stream_chunk(
    id: String,
    delta: StreamDelta,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id,
        object: "chat.completion.chunk".to_owned(),
        created: unix_timestamp_secs(),
        model: String::new(),
        choices: vec![StreamChoice {
            index: 0,
            delta,
            finish_reason,
        }],
        usage,
        system_fingerprint: None,
        service_tier: None,
    }
}

/// Map Cohere finish reason strings to OpenAI-compatible `FinishReason`.
fn map_cohere_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "COMPLETE" => FinishReason::Stop,
        "MAX_TOKENS" => FinishReason::Length,
        "TOOL_CALL" => FinishReason::ToolCalls,
        _ => FinishReason::Other,
    }
}

/// Extract usage from a Cohere `message-end` event.
///
/// Cohere v2 reports usage under `delta.usage.billed_units.{input_tokens, output_tokens}`.
fn extract_cohere_stream_usage(v: &Value) -> Option<crate::types::Usage> {
    let billed = v.pointer("/delta/usage/billed_units")?;
    let input = billed.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0);
    let output = billed.get("output_tokens").and_then(|t| t.as_u64()).unwrap_or(0);

    Some(crate::types::Usage {
        prompt_tokens: input,
        completion_tokens: output,
        total_tokens: input + output,
        prompt_tokens_details: None,
        completion_tokens_details: None,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Revert line: remove the error-event check from `parse_stream_event` to make this
    /// test fail (the event is then skipped and the stream ends with no error).
    #[test]
    fn parse_stream_event_error_event_has_the_http_error_kind() {
        let cases = [
            (
                r#"{"type":"error","error":{"message":"slow down","type":"rate_limit_error"}}"#,
                429,
            ),
            (r#"{"error":{"message":"bad key","status_code":401}}"#, 401),
            (
                r#"{"type":"error","message":"too many","status_code":429,"retry_after":4}"#,
                429,
            ),
            (r#"{"type":"error","message":"boom","status_code":500}"#, 500),
        ];
        for (event, status) in cases {
            let err = CohereProvider.parse_stream_event(event).unwrap_err();
            let http = LiterLlmError::from_status(status, r#"{"error":{"message":"x"}}"#, None);
            assert_eq!(
                std::mem::discriminant(&err),
                std::mem::discriminant(&http),
                "{event}: stream gave {err:?}, HTTP {status} gives {http:?}"
            );
            assert_eq!(err.status_code(), status, "{event}");
        }
        let err = CohereProvider
            .parse_stream_event(r#"{"type":"error","message":"too many","status_code":429,"retry_after":4}"#)
            .unwrap_err();
        assert!(err.to_string().contains("too many"), "got: {err}");
        assert_eq!(err.retry_after(), Some(std::time::Duration::from_secs(4)));
    }

    /// Revert line: drop the top-level fallback or the string parse from `status_code_field`
    /// to make this test fail.
    #[test]
    fn parse_stream_event_error_event_status_edge_inputs() {
        let cases = [
            // A string status reads as a number.
            (r#"{"type":"error","message":"slow","status_code":"429"}"#, 429),
            (r#"{"error":{"message":"slow","status_code":"503"}}"#, 503),
            // A top-level status beside an `error` object counts.
            (r#"{"error":{"message":"slow"},"status_code":429}"#, 429),
            // The error object wins over the top level.
            (r#"{"error":{"message":"x","status_code":401},"status_code":429}"#, 401),
            // No status at all gives the default 400.
            (r#"{"type":"error","message":"odd"}"#, 400),
        ];
        for (event, status) in cases {
            let err = CohereProvider.parse_stream_event(event).unwrap_err();
            assert_eq!(err.status_code(), status, "{event}: {err:?}");
        }
    }

    /// Revert line: remove the `message-end` error check from `parse_stream_event`
    /// to make this test fail (the event is then a normal end chunk).
    #[test]
    fn parse_stream_event_message_end_error_is_an_error() {
        let err = CohereProvider
            .parse_stream_event(r#"{"type":"message-end","delta":{"error":"model failed","finish_reason":"ERROR"}}"#)
            .unwrap_err();
        assert!(matches!(err, LiterLlmError::Streaming { .. }), "got: {err:?}");
        assert!(err.to_string().contains("model failed"), "got: {err}");
        let err = CohereProvider
            .parse_stream_event(r#"{"type":"message-end","delta":{"finish_reason":"ERROR"}}"#)
            .unwrap_err();
        assert!(err.to_string().contains("ERROR"), "got: {err}");
        let ok = CohereProvider
            .parse_stream_event(r#"{"type":"message-end","delta":{"finish_reason":"COMPLETE"}}"#)
            .unwrap();
        assert!(ok.is_some());
    }

    #[test]
    fn test_cohere_name_and_base_url() {
        let provider = CohereProvider;
        assert_eq!(provider.name(), "cohere");
        assert_eq!(provider.base_url(), "https://api.cohere.com/v2");
    }

    /// Regression test: schemas/providers.json's `cohere_chat` entry hardcoded
    /// `https://api.cohere.ai/v2` while this file hardcoded `https://api.cohere.com/v2` — both
    /// hosts work (Cohere serves the same v2 API from either), but a silent disagreement
    /// between the registry and the code means the registry can no longer be trusted as
    /// documentation. Both now agree on `.com`; this test would have failed before that fix.
    #[test]
    fn base_url_agrees_with_the_providers_json_registry() {
        let configs = crate::provider::all_providers().expect("embedded providers.json must parse");
        let entry = configs
            .iter()
            .find(|c| c.name == "cohere_chat")
            .expect("providers.json must have a `cohere_chat` entry");

        assert_eq!(
            entry.base_url.as_deref(),
            Some(CohereProvider.base_url()),
            "schemas/providers.json's `cohere_chat` base_url must match CohereProvider::base_url()"
        );
    }

    #[test]
    fn test_cohere_auth_header() {
        let provider = CohereProvider;
        let (name, value) = provider.auth_header("test-key").expect("should return auth header");
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer test-key");
    }

    #[test]
    fn test_cohere_matches_model() {
        let provider = CohereProvider;
        assert!(provider.matches_model("command-r-plus"));
        assert!(provider.matches_model("command-r"));
        assert!(provider.matches_model("command-light"));
        assert!(provider.matches_model("cohere/command-r-plus"));
        assert!(!provider.matches_model("gpt-4"));
        assert!(!provider.matches_model("claude-3"));
    }

    #[test]
    fn test_cohere_strip_prefix() {
        let provider = CohereProvider;
        assert_eq!(provider.strip_model_prefix("cohere/command-r"), "command-r");
        assert_eq!(provider.strip_model_prefix("command-r"), "command-r");
    }

    #[test]
    fn test_cohere_endpoints() {
        let provider = CohereProvider;
        assert_eq!(provider.chat_completions_path(), "/chat");
        assert_eq!(provider.rerank_path(), "/rerank");
    }

    #[test]
    fn test_cohere_transform_request_preserves_stream_strips_options() {
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true,
            "stream_options": {"include_usage": true}
        });
        provider.transform_request(&mut body).expect("transform should succeed");
        assert_eq!(body["stream"], true);
        assert!(body.get("stream_options").is_none());
        assert_eq!(body["model"], "command-r-plus");
    }

    // ~keep Cohere's v2 `/chat` endpoint has no `top_p` field; its nucleus-sampling
    // parameter is named `p` (min 0.01, max 0.99), verified against
    // docs.cohere.com/reference/chat. Forwarding `top_p` verbatim meant the value was
    // silently dropped, never reaching Cohere as a recognised field at any value.

    #[test]
    fn transform_request_renames_top_p_to_cohere_p() {
        // Delete the `obj.insert("p".to_owned(), top_p);` line (or the whole `if let
        // Some(top_p) = ...` rename block) in `CohereProvider::transform_request` to
        // make this fail: `p` would then be absent from the transformed body.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "top_p": 0.5
        });
        provider.transform_request(&mut body).expect("transform should succeed");
        assert_eq!(body["p"], 0.5);
    }

    #[test]
    fn transform_request_removes_top_p_field_name() {
        // Delete `obj.remove("top_p")` (replacing it with a non-removing read) in
        // `CohereProvider::transform_request` to make this fail: `top_p` would still
        // be present in the transformed body alongside `p`.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "top_p": 0.5
        });
        provider.transform_request(&mut body).expect("transform should succeed");
        assert!(body.get("top_p").is_none());
    }

    #[test]
    fn transform_request_rejects_top_p_zero_point_zero() {
        // Delete the `super::validate_sampling_param_range(body, "top_p", "Cohere",
        // 0.01, 0.99)?;` line in `CohereProvider::transform_request` to make this
        // fail: 0.0 is legal for OpenAI (and this crate's own `ChatCompletionRequest`
        // docs) but is below Cohere's documented minimum of 0.01 and must be rejected,
        // not silently coerced.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "top_p": 0.0
        });
        let err = provider
            .transform_request(&mut body)
            .expect_err("0.0 is below Cohere's minimum p of 0.01");
        assert_eq!(err.status_code(), 400);
        assert_eq!(
            err.to_string(),
            "bad request: top_p=0 is outside Cohere's supported range [0.01, 0.99]; lower \
             the requested value or omit `top_p` to use the provider default"
        );
    }

    #[test]
    fn transform_request_accepts_top_p_zero_point_nine_nine() {
        // Change the `max` argument passed to `validate_sampling_param_range` from
        // `0.99` to something smaller (e.g. `0.9`) in `CohereProvider::transform_request`
        // to make this fail: 0.99 is Cohere's documented maximum and must be accepted.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}],
            "top_p": 0.99
        });
        provider
            .transform_request(&mut body)
            .expect("0.99 is within Cohere's range");
        assert_eq!(body["p"], 0.99);
        assert!(body.get("top_p").is_none());
    }

    #[test]
    fn transform_request_without_top_p_is_untouched() {
        // Change the rename block to unconditionally insert a `p` field (dropping the
        // `if let Some(top_p) = obj.remove("top_p")` guard) in
        // `CohereProvider::transform_request` to make this fail: a body with no
        // `top_p` would gain a spurious `p` field.
        let provider = CohereProvider;
        let mut body = json!({
            "model": "command-r-plus",
            "messages": [{"role": "user", "content": "hello"}]
        });
        provider.transform_request(&mut body).expect("transform should succeed");
        assert!(body.get("p").is_none());
        assert!(body.get("top_p").is_none());
        assert_eq!(body["model"], "command-r-plus");
    }

    /// Build a Cohere v2 `/chat` response body in the real shape (no `choices`
    /// wrapper), per Cohere's own API reference — see #53.
    fn real_cohere_response(finish_reason: &str, content: serde_json::Value) -> serde_json::Value {
        json!({
            "id": "resp-abc123",
            "finish_reason": finish_reason,
            "message": {
                "role": "assistant",
                "content": content
            },
            "usage": {
                "billed_units": {"input_tokens": 10, "output_tokens": 20},
                "tokens": {"input_tokens": 12, "output_tokens": 22}
            }
        })
    }

    #[test]
    fn test_cohere_transform_response_wraps_top_level_fields_into_choices() {
        // ~keep Regression test for #53: Cohere's real response has no `choices` array —
        // `finish_reason` and `message` are top-level. The old implementation assumed a
        // `choices`-wrapped input and was therefore a no-op on real responses, leaving
        // the body without the `choices` field `ChatCompletionResponse` requires.
        let provider = CohereProvider;
        let mut body = real_cohere_response("COMPLETE", json!([{"type": "text", "text": "hi"}]));

        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        assert_eq!(body["id"], "resp-abc123");
        assert_eq!(body["object"], "chat.completion");
        assert!(body["created"].as_u64().is_some());
        let choices = body["choices"].as_array().expect("choices array must be present");
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0]["index"], 0);
        assert_eq!(choices[0]["finish_reason"], "stop");
        assert_eq!(choices[0]["message"]["role"], "assistant");
        assert_eq!(choices[0]["message"]["content"], "hi");
    }

    #[test]
    fn test_cohere_transform_response_finish_reasons() {
        let provider = CohereProvider;
        for (raw, expected) in [
            ("COMPLETE", "stop"),
            ("MAX_TOKENS", "length"),
            ("TOOL_CALL", "tool_calls"),
        ] {
            let mut body = real_cohere_response(raw, json!([{"type": "text", "text": "hi"}]));
            provider
                .transform_response(&mut body)
                .expect("transform should succeed");
            assert_eq!(
                body["choices"][0]["finish_reason"], expected,
                "mapping {raw} -> {expected}"
            );
        }
    }

    #[test]
    fn test_cohere_transform_response_concatenates_text_blocks() {
        let provider = CohereProvider;
        let mut body = real_cohere_response(
            "COMPLETE",
            json!([{"type": "text", "text": "Hello, "}, {"type": "text", "text": "world!"}]),
        );
        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        assert_eq!(body["choices"][0]["message"]["content"], "Hello, world!");
    }

    #[test]
    fn test_cohere_transform_response_thinking_block_routes_to_reasoning_content() {
        let provider = CohereProvider;
        let mut body = real_cohere_response(
            "COMPLETE",
            json!([{"type": "thinking", "thinking": "step 1..."}, {"type": "text", "text": "answer"}]),
        );
        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        assert_eq!(body["choices"][0]["message"]["content"], "answer");
        assert_eq!(body["choices"][0]["message"]["reasoning_content"], "step 1...");
    }

    #[test]
    fn test_cohere_transform_response_no_text_block_is_null_content() {
        let provider = CohereProvider;
        let mut body = real_cohere_response("TOOL_CALL", json!([]));
        body["message"]["tool_calls"] = json!([{
            "id": "call_1",
            "type": "function",
            "function": {"name": "get_weather", "arguments": "{\"city\":\"Berlin\"}"}
        }]);

        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        assert!(body["choices"][0]["message"]["content"].is_null());
        let tool_calls = body["choices"][0]["message"]["tool_calls"]
            .as_array()
            .expect("tool_calls array");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["id"], "call_1");
        assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
        assert_eq!(tool_calls[0]["function"]["arguments"], "{\"city\":\"Berlin\"}");
    }

    #[test]
    fn test_cohere_transform_response_output_deserializes_as_chat_completion_response() {
        // ~keep Regression test for #53: the whole point of `transform_response` is that
        // its output must deserialize as `ChatCompletionResponse`. Before the fix this
        // failed with "missing field `choices`" on every real (non-`choices`-wrapped)
        // Cohere response.
        let provider = CohereProvider;
        let mut body = real_cohere_response("COMPLETE", json!([{"type": "text", "text": "hi"}]));

        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        let parsed: crate::types::ChatCompletionResponse =
            serde_json::from_value(body).expect("transform_response output must deserialize as ChatCompletionResponse");
        assert_eq!(
            parsed.choices[0]
                .message
                .content
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("hi")
        );
    }

    #[test]
    fn test_cohere_transform_response_usage_uses_billed_units() {
        let provider = CohereProvider;
        let mut body = real_cohere_response("COMPLETE", json!([{"type": "text", "text": "hi"}]));

        provider
            .transform_response(&mut body)
            .expect("transform should succeed");

        let usage = &body["usage"];
        assert_eq!(usage["prompt_tokens"], 10);
        assert_eq!(usage["completion_tokens"], 20);
        assert_eq!(usage["total_tokens"], 30);
    }

    // ~keep Regression coverage for the streaming fix: the previous event shapes below
    // (`stream-start`/`stream-end` with flat `delta.text` / `delta.id` / `delta.function.*`
    // paths) encoded Cohere's legacy v1 NDJSON format, not the v2 SSE format this provider
    // actually targets (base_url is `.../v2`). Every one of those tests passed against a
    // shape the real v2 endpoint never sends, so they asserted the *broken* production
    // behaviour (content-delta always yielding empty text, tool-call fields always empty,
    // finish_reason/usage never surfacing) as if it were correct. Shapes below are quoted
    // verbatim from Cohere's v2 API reference (`docs.cohere.com/reference/chat-stream`).

    #[test]
    fn test_parse_stream_event_message_start() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"role":"assistant"}},"id":"gen-123","type":"message-start"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.id, "gen-123");
        assert_eq!(chunk.object, "chat.completion.chunk");
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.role.as_deref(), Some("assistant"));
        assert!(chunk.choices[0].delta.content.is_none());
        assert!(chunk.choices[0].finish_reason.is_none());
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn test_parse_stream_event_content_delta() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"content":{"text":"Hello"}}},"index":0,"type":"content-delta"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
        assert!(chunk.choices[0].delta.role.is_none());
        assert!(chunk.choices[0].delta.tool_calls.is_none());
    }

    #[test]
    fn test_parse_stream_event_content_delta_whitespace() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"content":{"text":" world"}}},"index":0,"type":"content-delta"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some(" world"));
    }

    #[test]
    fn test_parse_stream_event_tool_call_start() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"tool_calls":{"function":{"arguments":"","name":"get_weather"},"id":"tc-001","type":"function"}}},"index":0,"type":"tool-call-start"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        let tool_calls = chunk.choices[0]
            .delta
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].index, 0);
        assert_eq!(tool_calls[0].id.as_deref(), Some("tc-001"));
        let func = tool_calls[0].function.as_ref().expect("should have function");
        assert_eq!(func.name.as_deref(), Some("get_weather"));
        assert!(func.arguments.is_none());
    }

    #[test]
    fn test_parse_stream_event_tool_call_delta() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"tool_calls":{"function":{"arguments":"{\"ci"}}}},"index":0,"type":"tool-call-delta"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        let tool_calls = chunk.choices[0]
            .delta
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].index, 0);
        assert!(tool_calls[0].id.is_none());
        let func = tool_calls[0].function.as_ref().expect("should have function");
        assert!(func.name.is_none());
        assert_eq!(func.arguments.as_deref(), Some("{\"ci"));
    }

    #[test]
    fn test_parse_stream_event_tool_call_end_returns_none() {
        let provider = CohereProvider;
        let event = r#"{"index":0,"type":"tool-call-end"}"#;
        let result = provider.parse_stream_event(event).expect("should parse");

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_stream_event_message_end_complete() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"COMPLETE","usage":{"billed_units":{"input_tokens":10,"output_tokens":5}}},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Stop));
        let usage = chunk.usage.as_ref().expect("should have usage");
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn test_parse_stream_event_message_end_max_tokens() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"MAX_TOKENS","usage":{"billed_units":{"input_tokens":20,"output_tokens":100}}},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Length));
        let usage = chunk.usage.as_ref().expect("should have usage");
        assert_eq!(usage.prompt_tokens, 20);
        assert_eq!(usage.completion_tokens, 100);
        assert_eq!(usage.total_tokens, 120);
    }

    #[test]
    fn test_parse_stream_event_message_end_tool_call() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"TOOL_CALL","usage":{"billed_units":{"input_tokens":15,"output_tokens":8}}},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::ToolCalls));
    }

    #[test]
    fn test_parse_stream_event_message_end_no_usage() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"COMPLETE"},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Stop));
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn test_parse_stream_event_unknown_type_returns_none() {
        let provider = CohereProvider;
        let event = r#"{"type":"debug","message":"some debug info"}"#;
        let result = provider.parse_stream_event(event).expect("should parse");

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_stream_event_invalid_json_returns_err() {
        let provider = CohereProvider;
        let result = provider.parse_stream_event("not valid json");

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_stream_event_tool_call_start_index_1() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{"tool_calls":{"function":{"arguments":"","name":"search"},"id":"tc-002","type":"function"}}},"index":1,"type":"tool-call-start"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        let tool_calls = chunk.choices[0]
            .delta
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls[0].index, 1);
        assert_eq!(tool_calls[0].id.as_deref(), Some("tc-002"));
    }

    #[test]
    fn test_parse_stream_event_message_end_unknown_finish_reason() {
        let provider = CohereProvider;
        let event = r#"{"delta":{"finish_reason":"STOP_SEQUENCE"},"type":"message-end"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].finish_reason, Some(FinishReason::Other));
    }

    #[test]
    fn test_parse_stream_event_message_start_missing_role_defaults_to_assistant() {
        // ~keep Cohere's docs always show role="assistant" on message-start, but the parser
        // should not panic/error if a future event omits it — default rather than fail.
        let provider = CohereProvider;
        let event = r#"{"delta":{"message":{}},"id":"gen-456","type":"message-start"}"#;
        let chunk = provider
            .parse_stream_event(event)
            .expect("should parse")
            .expect("should return Some");

        assert_eq!(chunk.choices[0].delta.role.as_deref(), Some("assistant"));
    }

    // Embed and model-list tests. The shapes follow Cohere's public API reference:
    // `POST /v2/embed` (`texts`, `inputs`, `images`, `input_type`, `embedding_types`,
    // `output_dimension`, reply `embeddings.<type>`) and `GET /v1/models` (reply
    // `models[].name`, `next_page_token`).

    /// Serialize a typed request the way `prepare_embedding_request` does, then run the
    /// provider step for the embeddings endpoint.
    fn prepared_embed_body(req: &crate::types::EmbeddingRequest) -> Value {
        let mut body = serde_json::to_value(req).expect("request serializes");
        body["model"] = json!(CohereProvider.strip_model_prefix(&req.model));
        CohereProvider
            .transform_embedding_request(&mut body)
            .expect("embed request transforms");
        body
    }

    fn embed_request(model: &str, input: crate::types::EmbeddingInput) -> crate::types::EmbeddingRequest {
        crate::types::EmbeddingRequest {
            model: model.into(),
            input,
            ..Default::default()
        }
    }

    #[test]
    fn test_cohere_embeddings_path_is_embed() {
        assert_eq!(CohereProvider.embeddings_path(), "/embed");
        assert_eq!(
            CohereProvider.build_url(CohereProvider.embeddings_path(), "embed-english-v3.0"),
            "https://api.cohere.com/v2/embed"
        );
    }

    #[test]
    fn test_cohere_models_url_is_v1_models() {
        assert_eq!(
            CohereProvider.build_url(CohereProvider.models_path(), ""),
            "https://api.cohere.com/v1/models?page_size=1000"
        );
    }

    #[test]
    fn test_cohere_other_urls_stay_on_the_v2_base() {
        assert_eq!(
            CohereProvider.build_url(CohereProvider.chat_completions_path(), "command-r"),
            "https://api.cohere.com/v2/chat"
        );
        assert_eq!(
            CohereProvider.build_url(CohereProvider.rerank_path(), "rerank-v3.5"),
            "https://api.cohere.com/v2/rerank"
        );
    }

    #[test]
    fn test_cohere_embed_request_single_text() {
        let req = embed_request(
            "cohere/embed-english-v3.0",
            crate::types::EmbeddingInput::Single("alpha".into()),
        );
        assert_eq!(
            prepared_embed_body(&req),
            json!({
                "model": "embed-english-v3.0",
                "texts": ["alpha"],
                "input_type": "search_document",
                "embedding_types": ["float"],
            })
        );
    }

    #[test]
    fn test_cohere_embed_request_many_texts_keep_their_order() {
        let req = embed_request(
            "embed-english-v3.0",
            crate::types::EmbeddingInput::Multiple(vec!["one".into(), "two".into(), "three".into()]),
        );
        let body = prepared_embed_body(&req);
        assert_eq!(body["texts"], json!(["one", "two", "three"]));
        assert!(body.get("input").is_none(), "the OpenAI `input` key must not be sent");
    }

    #[test]
    fn test_cohere_embed_request_maps_dimensions_and_drops_user_and_encoding_format() {
        let mut req = embed_request("embed-v4.0", crate::types::EmbeddingInput::Single("alpha".into()));
        req.dimensions = Some(512);
        req.user = Some("someone".into());
        req.encoding_format = Some(crate::types::EmbeddingFormat::Base64);
        let body = prepared_embed_body(&req);
        assert_eq!(body["output_dimension"], 512);
        assert!(body.get("dimensions").is_none());
        assert!(body.get("user").is_none());
        assert!(body.get("encoding_format").is_none());
        assert_eq!(body["embedding_types"], json!(["float"]));
    }

    #[test]
    fn test_cohere_embed_request_multimodal_input_becomes_one_inputs_entry() {
        use crate::types::EmbeddingContentPart as Part;
        let req = embed_request(
            "embed-v4.0",
            crate::types::EmbeddingInput::Multimodal(vec![
                Part::text("a red bicycle"),
                Part::image_url("https://example.com/bicycle.png"),
                Part::image_base64("data:image/png;base64,aW1hZ2U="),
            ]),
        );
        let body = prepared_embed_body(&req);
        assert_eq!(
            body["inputs"],
            json!([{"content": [
                {"type": "text", "text": "a red bicycle"},
                {"type": "image_url", "image_url": {"url": "https://example.com/bicycle.png"}},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,aW1hZ2U="}},
            ]}])
        );
        assert!(body.get("texts").is_none());
        assert_eq!(body["input_type"], "search_document");
    }

    #[test]
    fn test_cohere_embed_request_rejects_an_unknown_content_part() {
        let mut body = json!({"model": "embed-v4.0", "input": [{"type": "audio", "audio": "x"}]});
        let err = CohereProvider
            .transform_embedding_request(&mut body)
            .expect_err("an audio part has no Cohere embed form");
        assert_eq!(err.status_code(), 400);
    }

    #[test]
    fn test_cohere_embed_request_rejects_a_mixed_list_and_a_token_array() {
        for input in [
            json!(["a", 5]),
            json!([[1, 2, 3]]),
            json!(["a", {"type": "text", "text": "b"}]),
        ] {
            let mut body = json!({"model": "embed-v4.0", "input": input});
            let err = CohereProvider
                .transform_embedding_request(&mut body)
                .expect_err("a mixed list or a token array has no Cohere embed form");
            assert_eq!(err.status_code(), 400);
            assert!(
                err.to_string().contains("mixes them"),
                "the message should name the input problem, got: {err}"
            );
        }
    }

    #[test]
    fn test_cohere_embed_reply_takes_the_request_model() {
        let mut body = json!({"embeddings": {"float": [[0.5]]}});
        CohereProvider
            .transform_embedding_response(&mut body, "embed-v4.0")
            .expect("a float reply should rebuild");
        assert_eq!(body["model"], "embed-v4.0");
    }

    #[test]
    fn test_cohere_embed_request_keeps_extra_body_for_the_client_merge() {
        let mut body = json!({"model": "embed-v4.0", "input": "a", "extra_body": {"input_type": "search_query"}});
        CohereProvider
            .transform_embedding_request(&mut body)
            .expect("a string input should rebuild");
        assert_eq!(body["extra_body"], json!({"input_type": "search_query"}));
    }

    #[test]
    fn test_cohere_embed_request_rejects_a_number_as_input() {
        let mut body = json!({"model": "embed-v4.0", "input": 5});
        let err = CohereProvider
            .transform_embedding_request(&mut body)
            .expect_err("a number is not an embedding input");
        assert_eq!(err.status_code(), 400);
    }

    #[test]
    fn test_cohere_embed_request_rejects_an_empty_input_list() {
        let mut body = json!({"model": "embed-v4.0", "input": []});
        let err = CohereProvider
            .transform_embedding_request(&mut body)
            .expect_err("an empty list has nothing to embed");
        assert_eq!(err.status_code(), 400);
    }

    #[test]
    fn test_cohere_embed_request_with_null_dimensions_sends_no_output_dimension() {
        let mut body = json!({"model": "embed-v4.0", "input": "alpha", "dimensions": null});
        CohereProvider
            .transform_embedding_request(&mut body)
            .expect("embed request transforms");
        assert!(body.get("output_dimension").is_none());
    }

    #[test]
    fn test_cohere_non_embed_bodies_are_not_rebuilt_as_embed_bodies() {
        // Moderation and speech bodies carry `input` and no `messages`. Only the embeddings
        // endpoint rebuilds a body, so these two keep every field they had.
        let moderation = json!({"model": "omni-moderation", "input": "some text"});
        let speech = json!({"model": "tts-1", "input": "hello", "voice": "alloy"});
        for original in [moderation, speech] {
            let mut body = original.clone();
            CohereProvider.transform_request(&mut body).expect("request transforms");
            assert_eq!(body, original);
        }
    }

    #[test]
    fn test_cohere_chat_request_is_not_taken_for_an_embed_request() {
        let mut body = json!({
            "model": "command-r",
            "messages": [{"role": "user", "content": "hi"}],
            "top_p": 0.5,
            "stream_options": {"include_usage": true},
        });
        CohereProvider
            .transform_request(&mut body)
            .expect("chat request transforms");
        assert_eq!(body["messages"][0]["content"], "hi");
        assert_eq!(body["p"], 0.5);
        assert!(body.get("texts").is_none());
        assert!(body.get("input_type").is_none());
    }

    fn cohere_embed_reply() -> Value {
        json!({
            "id": "emb-1",
            "embeddings": {"float": [[0.1, 0.2], [0.3, 0.4]]},
            "texts": ["alpha", "beta"],
            "response_type": "embeddings_by_type",
            "meta": {"api_version": {"version": "2"}, "billed_units": {"input_tokens": 7, "image_tokens": 3}}
        })
    }

    #[test]
    fn test_cohere_embed_reply_becomes_an_embeddings_list() {
        let mut body = cohere_embed_reply();
        CohereProvider
            .transform_response(&mut body)
            .expect("embed reply transforms");
        let parsed: crate::types::EmbeddingResponse =
            serde_json::from_value(body).expect("the result must deserialize as an embedding response");
        assert_eq!(parsed.data.len(), 2);
        assert_eq!(parsed.data[0].index, 0);
        assert_eq!(parsed.data[1].index, 1);
        assert_eq!(parsed.data[1].embedding, vec![0.3_f32, 0.4_f32]);
        let usage = parsed.usage.expect("usage is read from billed units");
        assert_eq!(usage.prompt_tokens, 10, "text and image tokens are both billed input");
        assert_eq!(usage.total_tokens, 10);
    }

    #[test]
    fn test_cohere_embed_reply_without_float_vectors_is_an_error() {
        let mut body = json!({"id": "emb-1", "embeddings": {"int8": [[1, 2]]}});
        let err = CohereProvider
            .transform_response(&mut body)
            .expect_err("a reply without embeddings.float is not usable");
        assert_eq!(err.status_code(), 502);
    }

    #[test]
    fn test_cohere_models_reply_becomes_a_models_list() {
        let mut body = json!({
            "models": [
                {"name": "command-r", "endpoints": ["chat"], "context_length": 128000},
                {"name": "embed-english-v3.0", "endpoints": ["embed"]}
            ],
            "next_page_token": ""
        });
        CohereProvider
            .transform_response(&mut body)
            .expect("models reply transforms");
        let parsed: crate::types::ModelsListResponse =
            serde_json::from_value(body).expect("the result must deserialize as a models list");
        let ids: Vec<&str> = parsed.data.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["command-r", "embed-english-v3.0"]);
        assert_eq!(parsed.object, "list");
        assert!(parsed.data.iter().all(|m| m.owned_by == "cohere"));
    }

    #[test]
    fn test_cohere_models_reply_with_more_pages_is_an_error() {
        let mut body = json!({"models": [{"name": "command-r"}], "next_page_token": "page-2"});
        let err = CohereProvider
            .transform_response(&mut body)
            .expect_err("a list cut at one page must not look complete");
        assert_eq!(err.status_code(), 502);
        assert!(err.to_string().contains("1000"), "unexpected error: {err}");
    }

    #[test]
    fn test_cohere_models_reply_where_models_is_not_an_array_is_an_error() {
        let mut body = json!({"models": "none"});
        let err = CohereProvider
            .transform_response(&mut body)
            .expect_err("models must be an array");
        assert_eq!(err.status_code(), 502);
    }

    #[test]
    fn test_cohere_models_reply_with_a_nameless_model_is_an_error() {
        let mut body = json!({"models": [{"endpoints": ["chat"]}]});
        let err = CohereProvider
            .transform_response(&mut body)
            .expect_err("a model without a name cannot be listed");
        assert_eq!(err.status_code(), 502);
    }

    #[test]
    fn test_cohere_transform_response_keeps_a_reply_that_is_not_a_chat_reply() {
        // A moderation, search or OCR reply is not a chat reply. The chat rebuild used to
        // replace every reply with an empty chat completion and drop all of its fields.
        let original = json!({
            "id": "r1",
            "results": [{"index": 2, "relevance_score": 0.9}],
            "meta": {"billed_units": {"search_units": 1}}
        });
        let mut body = original.clone();
        CohereProvider
            .transform_response(&mut body)
            .expect("transform should succeed");
        assert_eq!(body, original);
    }

    #[test]
    fn test_cohere_embed_model_names_route_to_the_native_provider() {
        let provider = CohereProvider;
        assert!(provider.matches_model("embed-english-v3.0"));
        assert!(provider.matches_model("cohere/embed-multilingual-v3.0"));
        for model in ["embed-english-v3.0", "cohere/embed-english-v3.0"] {
            let routed = crate::provider::detect_builtin_provider(model).expect("a provider is found");
            assert_eq!(routed.name(), "cohere", "{model}");
            assert_eq!(routed.embeddings_path(), "/embed", "{model}");
        }
    }
}
