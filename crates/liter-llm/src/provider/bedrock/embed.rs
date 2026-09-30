//! Bedrock `InvokeModel` embedding request / response translation (Titan, Cohere).

use crate::error::Result;

/// `input_type` sent for Bedrock Cohere embed models.
///
/// Cohere's v3 embed models require this field, but OpenAI's `EmbeddingRequest` has
/// no equivalent concept to map from, so a default must be chosen. `search_document`
/// is the indexing-side value — correct for the dominant use of an embeddings
/// endpoint. A caller embedding *queries* wants `search_query` and will get subtly
/// mismatched vectors; that needs a provider-options passthrough to fix properly,
/// which this constant deliberately does not fake. ~keep
const COHERE_EMBED_DEFAULT_INPUT_TYPE: &str = "search_document";

/// Convert an OpenAI-style embedding request to a Bedrock model-family `InvokeModel` body.
///
/// Bedrock has no unified embeddings API — each model family defines its own wire
/// shape, and there is nothing else in `transform_request`'s signature to key off
/// of, so this dispatches on `body["model"]`. That field is populated by
/// `Client::prepare_request` (`crates/liter-llm/src/client/mod.rs`), which inserts
/// the already-prefix-stripped model ID into the body immediately before calling
/// `transform_request` — so it is reliably present here.
///
/// Supported families:
/// - Amazon Titan (`amazon.titan-embed-*`): `{"inputText": "..."}`.
/// - Cohere Embed (`cohere.embed-*`): `{"texts": [...], "input_type": "search_document"}`.
///
/// Any other model prefix is rejected with a clear error rather than guessed at.
///
/// ~keep Titan's `invoke` endpoint accepts exactly one string per call (batch Titan
/// ~keep embedding is a separate async `CreateModelInvocationJob` API, not this
/// ~keep synchronous path). A batched OpenAI `input` with more than one element is
/// ~keep therefore rejected with a `BadRequest` rather than truncated, since one
/// ~keep vector for an N-element batch would silently misalign `data[]` with `input[]`.
pub(super) fn transform_bedrock_embed_request(body: &mut serde_json::Value) -> Result<()> {
    use crate::error::LiterLlmError;
    use serde_json::json;

    let model = body.get("model").and_then(|m| m.as_str()).unwrap_or("").to_owned();

    let input = body.get("input").cloned().unwrap_or_default();
    let texts: Vec<String> = match &input {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) if arr.iter().all(serde_json::Value::is_string) => {
            arr.iter().filter_map(|v| v.as_str().map(ToOwned::to_owned)).collect()
        }
        _ => {
            return Err(LiterLlmError::BadRequest {
                message: "Bedrock embedding adapters support text input only; use a multimodal-compatible custom provider for image embeddings".into(),
                status: 400,
            });
        }
    };

    if model.starts_with("amazon.titan-embed") {
        // ~keep Reject rather than truncate. OpenAI's contract is that `data[]` parallels
        // ~keep `input[]`, so returning one vector for an N-element batch hands the caller
        // ~keep silently misaligned embeddings — the kind of defect that corrupts a vector
        // ~keep store and only surfaces much later as bad retrieval.
        if texts.len() > 1 {
            return Err(LiterLlmError::BadRequest {
                message: format!(
                    "Bedrock Titan embedding model '{model}' accepts a single input per call, but \
                     {} were supplied; issue one request per input (batch Titan embedding is a \
                     separate asynchronous job API)",
                    texts.len()
                ),
                status: 400,
            });
        }
        let text = texts.first().cloned().unwrap_or_default();
        let mut new_body = json!({"inputText": text});
        if let Some(dimensions) = body.get("dimensions") {
            new_body["dimensions"] = dimensions.clone();
        }
        *body = new_body;
        return Ok(());
    }

    if model.starts_with("cohere.embed") {
        *body = json!({
            "texts": texts,
            "input_type": COHERE_EMBED_DEFAULT_INPUT_TYPE
        });
        return Ok(());
    }

    Err(LiterLlmError::BadRequest {
        message: format!(
            "unsupported Bedrock embedding model '{model}': liter-llm currently supports \
             amazon.titan-embed-* and cohere.embed-* embedding models"
        ),
        status: 400,
    })
}

/// Normalize a Bedrock `InvokeModel` embedding response to OpenAI's embeddings list format.
///
/// Dispatched by response shape rather than model ID: `transform_response` has no
/// model parameter, and an `InvokeModel` response body carries no model field
/// either (same limitation noted on [`BedrockProvider::transform_response`]).
///
/// - Titan (`{"embedding": [...], "inputTextTokenCount": N}`) -> single embedding,
///   with `inputTextTokenCount` threaded through as `prompt_tokens`.
/// - Cohere (`{"embeddings": [[...], ...]}`) -> one embedding per input text.
///   ~keep Bedrock's Cohere embed response carries no token-usage field, so
///   ~keep usage is reported as zero rather than guessed at.
pub(super) fn transform_bedrock_embed_response(body: &mut serde_json::Value) -> Result<()> {
    use serde_json::json;

    if let Some(embedding) = body.get("embedding").cloned() {
        let prompt_tokens = body.get("inputTextTokenCount").and_then(|v| v.as_u64()).unwrap_or(0);
        *body = json!({
            "object": "list",
            "data": [{"object": "embedding", "embedding": embedding, "index": 0}],
            "model": "",
            "usage": {
                "prompt_tokens": prompt_tokens,
                "completion_tokens": 0,
                "total_tokens": prompt_tokens
            }
        });
        return Ok(());
    }

    if let Some(embeddings) = body.get("embeddings").and_then(|e| e.as_array()).cloned() {
        let data: Vec<serde_json::Value> = embeddings
            .into_iter()
            .enumerate()
            .map(|(index, embedding)| json!({"object": "embedding", "embedding": embedding, "index": index}))
            .collect();
        *body = json!({
            "object": "list",
            "data": data,
            "model": "",
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        });
        return Ok(());
    }

    Ok(())
}
