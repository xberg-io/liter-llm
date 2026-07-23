use base64::Engine as _;
use serde::{Deserialize, Serialize};

use super::common::Usage;
use crate::cost;

/// The format in which the embedding vectors are returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingFormat {
    /// 32-bit floating-point numbers (default).
    Float,
    /// Base64-encoded string representation of the floats.
    Base64,
}

/// Embedding request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingRequest {
    /// Model ID (e.g., `"text-embedding-3-small"`).
    pub model: String,
    /// Text or texts to embed.
    pub input: EmbeddingInput,
    /// Output format: float (native) or base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<EmbeddingFormat>,
    /// Requested embedding dimensions (if supported by the model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    /// User identifier for request tracking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Text or texts to embed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// Single text string.
    Single(String),
    /// Multiple text strings (batch embedding).
    Multiple(Vec<String>),
}

#[cfg_attr(alef, alef(skip))]
impl Default for EmbeddingInput {
    fn default() -> Self {
        Self::Single(String::new())
    }
}

/// Embedding response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Always `"list"` from OpenAI-compatible APIs.  Stored as a plain
    /// `String` so non-standard provider values do not break deserialization.
    pub object: String,
    /// List of embeddings.
    pub data: Vec<EmbeddingObject>,
    /// Model used to generate embeddings.
    pub model: String,
    /// Token usage (input tokens only; embeddings have zero output tokens).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

impl EmbeddingResponse {
    /// Estimate the cost of this embedding request based on embedded pricing data.
    ///
    /// Returns `None` if:
    /// - the `model` field is not present in the embedded pricing registry, or
    /// - the `usage` field is absent from the response.
    ///
    /// Embedding models only charge for input tokens; output cost is zero.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cost = response.estimated_cost();
    /// if let Some(usd) = cost {
    ///     println!("Embedding cost: ${usd:.8}");
    /// }
    /// ```
    #[cfg_attr(alef, alef(skip))]
    #[must_use]
    pub fn estimated_cost(&self) -> Option<f64> {
        let usage = self.usage.as_ref()?;
        cost::completion_cost(&self.model, usage.prompt_tokens, usage.completion_tokens)
    }
}

/// A single embedding vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingObject {
    /// Always `"embedding"` from OpenAI-compatible APIs.  Stored as a plain
    /// `String` so non-standard provider values do not break deserialization.
    pub object: String,
    /// The embedding vector.
    ///
    /// Providers may return this as a JSON float array or, when
    /// `encoding_format: "base64"` was requested, as a base64 string of
    /// little-endian `f32` bytes. Base64 responses are decoded on read; this
    /// field always serializes back out as a JSON float array.
    #[serde(deserialize_with = "deserialize_embedding")]
    pub embedding: Vec<f32>,
    /// Index in the batch (corresponds to input order).
    pub index: u32,
}

/// Deserialization helper for [`EmbeddingObject::embedding`].
///
/// Accepts either a JSON array of floats or a base64-encoded string of
/// little-endian `f32` bytes (the OpenAI-compatible `encoding_format:
/// "base64"` response shape).
///
/// Uses a [`Visitor`](serde::de::Visitor) rather than an untagged enum: an
/// untagged enum buffers the whole value into an intermediate before
/// re-deserializing, which measured ~3x slower on the base64 path. `visit_seq`
/// handles the float array and `visit_str` decodes the base64 string, both with
/// zero buffering.
fn deserialize_embedding<'de, D>(deserializer: D) -> Result<Vec<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct EmbeddingVisitor;

    impl<'de> serde::de::Visitor<'de> for EmbeddingVisitor {
        type Value = Vec<f32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a float array or a base64-encoded string of little-endian f32 bytes")
        }

        fn visit_str<E>(self, value: &str) -> Result<Vec<f32>, E>
        where
            E: serde::de::Error,
        {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(value)
                .map_err(|e| E::custom(format!("invalid base64 embedding: {e}")))?;
            if bytes.len() % 4 != 0 {
                return Err(E::custom(format!(
                    "base64 embedding length {} is not a multiple of 4",
                    bytes.len()
                )));
            }
            Ok(bytes
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect())
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Vec<f32>, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(value) = seq.next_element::<f32>()? {
                out.push(value);
            }
            Ok(out)
        }
    }

    deserializer.deserialize_any(EmbeddingVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedding_body(embedding_json: &str) -> String {
        format!(r#"{{"object":"embedding","index":0,"embedding":{embedding_json}}}"#)
    }

    #[test]
    fn base64_embedding_round_trips_bit_exact() {
        let src: [f32; 5] = [1.0, -2.5, 12.375, 0.0, f32::MIN_POSITIVE];
        let mut bytes = Vec::with_capacity(src.len() * 4);
        for v in src {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let body = embedding_body(&format!("{encoded:?}"));

        let obj: EmbeddingObject = serde_json::from_str(&body).expect("base64 embedding should deserialize");
        assert_eq!(obj.embedding, src, "decoded floats must match source bit-exactly");
    }

    #[test]
    fn openai_base64_anchor_decodes_to_one() {
        let body = embedding_body(r#""AACAPw==""#);
        let obj: EmbeddingObject = serde_json::from_str(&body).expect("anchor base64 embedding should deserialize");
        assert_eq!(obj.embedding, vec![1.0_f32]);
    }

    #[test]
    fn float_array_body_still_parses() {
        let body = embedding_body("[0.1,0.2,0.3]");
        let obj: EmbeddingObject = serde_json::from_str(&body).expect("float array embedding should deserialize");
        assert_eq!(obj.embedding, vec![0.1_f32, 0.2, 0.3]);
    }

    #[test]
    fn odd_length_base64_errors_with_multiple_of_four_message() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(b"abcdef");
        let body = embedding_body(&format!("{encoded:?}"));

        let err = serde_json::from_str::<EmbeddingObject>(&body).expect_err("6-byte base64 payload must error");
        assert!(
            err.to_string().contains("not a multiple of 4"),
            "expected multiple-of-4 message, got: {err}"
        );
    }

    #[test]
    fn invalid_base64_errors() {
        let body = embedding_body(r#""not valid base64!!!""#);
        let err = serde_json::from_str::<EmbeddingObject>(&body).expect_err("non-base64 string must error");
        assert!(
            err.to_string().contains("invalid base64 embedding"),
            "expected invalid base64 message, got: {err}"
        );
    }

    #[test]
    fn serialization_stays_a_json_array() {
        let body = embedding_body(r#""AACAPw==""#);
        let obj: EmbeddingObject = serde_json::from_str(&body).expect("anchor base64 embedding should deserialize");

        let serialized = serde_json::to_value(&obj).expect("serialize back to JSON");
        assert!(
            serialized["embedding"].is_array(),
            "expected embedding field to serialize as an array, got: {serialized}"
        );
    }
}
