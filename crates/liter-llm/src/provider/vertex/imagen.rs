//! Imagen `:predict` image generation and publisher model listing for Vertex AI.

use serde_json::{Value, json};

use crate::error::{LiterLlmError, Result};

/// Aspect ratios that the Imagen `aspectRatio` parameter accepts.
const IMAGEN_ASPECT_RATIOS: [(u64, u64); 5] = [(1, 1), (3, 4), (4, 3), (9, 16), (16, 9)];

/// Whether a request body is an image generation request: a `prompt` and no chat or embedding input.
pub(super) fn is_image_request(body: &Value) -> bool {
    body.get("prompt").is_some() && body.get("messages").is_none() && body.get("input").is_none()
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Map an OpenAI `size` such as `1024x1792` to an Imagen `aspectRatio` such as `9:16`.
fn aspect_ratio_for_size(size: &str) -> Result<String> {
    let parsed = size.split_once('x').and_then(|(width, height)| {
        let width = width.trim().parse::<u64>().ok().filter(|v| *v > 0)?;
        let height = height.trim().parse::<u64>().ok().filter(|v| *v > 0)?;
        Some((width, height))
    });
    let (width, height) = parsed.ok_or_else(|| LiterLlmError::BadRequest {
        message: format!("image size `{size}` is not of the form WIDTHxHEIGHT"),
        status: 400,
    })?;
    let divisor = gcd(width, height);
    let ratio = (width / divisor, height / divisor);
    if IMAGEN_ASPECT_RATIOS.contains(&ratio) {
        Ok(format!("{}:{}", ratio.0, ratio.1))
    } else {
        Err(LiterLlmError::BadRequest {
            message: format!(
                "Imagen sets the shape of an image with an aspect ratio, and size `{size}` is not 1:1, 3:4, 4:3, 9:16 or 16:9"
            ),
            status: 400,
        })
    }
}

/// Transform an OpenAI image generation request to the Imagen `:predict` format.
///
/// OpenAI: `{"model": "...", "prompt": "a cat", "n": 2, "size": "1024x1024"}`
/// Imagen: `{"instances": [{"prompt": "a cat"}], "parameters": {"sampleCount": 2, "aspectRatio": "1:1"}}`
pub(super) fn transform_imagen_request(body: &mut Value) -> Result<()> {
    let prompt = body.get("prompt").cloned().unwrap_or_default();

    if body.get("response_format").and_then(Value::as_str) == Some("url") {
        return Err(LiterLlmError::BadRequest {
            message: "Imagen returns image bytes, not URLs; omit response_format or set it to b64_json".into(),
            status: 400,
        });
    }

    let mut parameters = serde_json::Map::new();
    if let Some(n) = body.get("n").and_then(Value::as_u64) {
        parameters.insert("sampleCount".into(), json!(n));
    }
    if let Some(size) = body.get("size").and_then(Value::as_str) {
        parameters.insert("aspectRatio".into(), json!(aspect_ratio_for_size(size)?));
    }
    for field in ["quality", "style", "user"] {
        if body.get(field).is_some() {
            tracing::warn!("image request set `{field}`, which has no Imagen equivalent and was dropped");
        }
    }

    *body = json!({"instances": [{"prompt": prompt}], "parameters": parameters});
    Ok(())
}

/// Whether a reply is an Imagen `:predict` reply: a prediction carries image bytes, or the
/// safety filter removed it and left a `raiFilteredReason`.
pub(super) fn is_image_response(body: &Value) -> bool {
    body.get("predictions")
        .and_then(Value::as_array)
        .is_some_and(|predictions| {
            predictions
                .iter()
                .any(|p| p.get("bytesBase64Encoded").is_some() || p.get("raiFilteredReason").is_some())
        })
}

/// Transform an Imagen `:predict` reply to the OpenAI image response.
///
/// A prediction that the safety filter removed has no image bytes and is skipped, so a reply
/// that the filter removed in full has no images.
pub(super) fn transform_imagen_response(body: &Value) -> Value {
    let data: Vec<Value> = body
        .get("predictions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|prediction| {
            let bytes = prediction.get("bytesBase64Encoded")?;
            let mut image = json!({"b64_json": bytes});
            if let Some(prompt) = prediction.get("prompt") {
                image["revised_prompt"] = prompt.clone();
            }
            Some(image)
        })
        .collect();
    json!({"created": crate::provider::unix_timestamp_secs(), "data": data})
}

/// Transform a publisher model listing (`publisherModels`) to the OpenAI model list.
///
/// A publisher model name is `publishers/{publisher}/models/{model}`.
pub(super) fn transform_publisher_models(models: &[Value]) -> Value {
    let data: Vec<Value> = models
        .iter()
        .map(|model| {
            let name = model.get("name").and_then(Value::as_str).unwrap_or("");
            let (publisher, id) = match name
                .strip_prefix("publishers/")
                .and_then(|rest| rest.split_once("/models/"))
            {
                Some((publisher, id)) => (publisher, id),
                None => ("google", name),
            };
            json!({"id": id, "object": "model", "created": 0, "owned_by": publisher})
        })
        .collect();
    json!({"object": "list", "data": data})
}
