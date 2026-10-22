//! Bedrock `Rerank` (agent runtime) and `ListFoundationModels` (control plane) translation.

use serde_json::{Value, json};

use crate::error::{LiterLlmError, Result};

/// Whether a request body is a rerank request: a `query` and `documents` and no chat input.
pub(super) fn is_rerank_request(body: &Value) -> bool {
    body.get("query").is_some() && body.get("documents").is_some() && body.get("messages").is_none()
}

fn document_text(document: &Value) -> Option<&str> {
    document
        .as_str()
        .or_else(|| document.get("text").and_then(Value::as_str))
}

/// The AWS partition of a region: `aws-cn` for `cn-` regions, `aws-us-gov` for `us-gov-` regions
/// and `aws-eusc` for `eusc-` regions.
fn partition_for_region(region: &str) -> &'static str {
    if region.starts_with("eusc-") {
        "aws-eusc"
    } else if region.starts_with("cn-") {
        "aws-cn"
    } else if region.starts_with("us-gov-") {
        "aws-us-gov"
    } else {
        "aws"
    }
}

/// The cross-region inference profile prefixes of a model id.
const INFERENCE_PROFILE_PREFIXES: [&str; 3] = ["us.", "eu.", "apac."];

/// The ARN that names the reranking model in a `Rerank` call.
///
/// A model that is already an ARN is used as given. A model id names a foundation model in the
/// region of the client. An inference profile id has no ARN without an account id, so it is
/// refused: pass the inference profile ARN as the model.
fn model_arn(model: &str, region: &str) -> Result<String> {
    let model = model.trim();
    if model.is_empty() {
        return Err(LiterLlmError::BadRequest {
            message: "Bedrock rerank needs a model".into(),
            status: 400,
        });
    }
    if model.starts_with("arn:") {
        return Ok(model.to_owned());
    }
    if INFERENCE_PROFILE_PREFIXES
        .iter()
        .any(|prefix| model.starts_with(prefix))
    {
        return Err(LiterLlmError::BadRequest {
            message: format!(
                "Bedrock rerank takes a foundation model id or an ARN, and `{model}` is an inference profile id; pass the ARN of the inference profile"
            ),
            status: 400,
        });
    }
    Ok(format!(
        "arn:{}:bedrock:{region}::foundation-model/{model}",
        partition_for_region(region)
    ))
}

/// Transform an OpenAI-shaped rerank request to the Bedrock `Rerank` format.
///
/// The model is named by a foundation model ARN in the region of the client, or by the ARN given.
pub(super) fn transform_rerank_request(body: &mut Value, region: &str) -> Result<()> {
    let model = body.get("model").and_then(Value::as_str).unwrap_or_default();
    let query = body.get("query").and_then(Value::as_str).unwrap_or_default();
    let documents = body
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let model_arn = model_arn(model, region)?;

    let sources = documents
        .iter()
        .map(|document| {
            let text = document_text(document).ok_or_else(|| LiterLlmError::BadRequest {
                message: "Bedrock rerank supports text documents only".into(),
                status: 400,
            })?;
            Ok(json!({
                "type": "INLINE",
                "inlineDocumentSource": {"type": "TEXT", "textDocument": {"text": text}}
            }))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut configuration = json!({
        "modelConfiguration": {"modelArn": model_arn}
    });
    if let Some(top_n) = body.get("top_n").and_then(Value::as_u64) {
        configuration["numberOfResults"] = json!(top_n);
    }

    *body = json!({
        "queries": [{"type": "TEXT", "textQuery": {"text": query}}],
        "sources": sources,
        "rerankingConfiguration": {
            "type": "BEDROCK_RERANKING_MODEL",
            "bedrockRerankingConfiguration": configuration
        }
    });
    Ok(())
}

/// Transform a Bedrock `Rerank` reply to the canonical rerank reply.
///
/// `relevanceScore` becomes `relevance_score`, and a returned `document` of type `TEXT` becomes
/// `{"text": ...}`.
pub(super) fn transform_rerank_response(body: &mut Value) {
    let Some(results) = body.get_mut("results").and_then(Value::as_array_mut) else {
        return;
    };
    for result in results {
        let Some(result) = result.as_object_mut() else {
            continue;
        };
        if let Some(score) = result.remove("relevanceScore") {
            result.insert("relevance_score".into(), score);
        }
        let text = result
            .remove("document")
            .and_then(|document| document.pointer("/textDocument/text").cloned());
        if let Some(text) = text {
            result.insert("document".into(), json!({"text": text}));
        }
    }
}

/// Transform a `ListFoundationModels` reply (`modelSummaries`) to the OpenAI model list.
pub(super) fn transform_foundation_models(summaries: &[Value]) -> Value {
    let data: Vec<Value> = summaries
        .iter()
        .map(|summary| {
            let id = summary.get("modelId").and_then(Value::as_str).unwrap_or("");
            let owner = summary.get("providerName").and_then(Value::as_str).unwrap_or("bedrock");
            json!({"id": id, "object": "model", "created": 0, "owned_by": owner})
        })
        .collect();
    json!({"object": "list", "data": data})
}
