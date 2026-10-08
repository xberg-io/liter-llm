//! Contract tests validating that our Rust types produce JSON
//! conforming to the OpenAI API JSON Schema specifications.
//!
//! Each test:
//!   1. Constructs a canonical instance of a Rust response type.
//!   2. Serialises it to `serde_json::Value`.
//!   3. Validates the value against the corresponding `$defs` entry from the
//!      OpenAI JSON Schema files embedded at compile time.
//!
//! Cross-file `$ref` pointers (e.g. `common.json#/$defs/CompletionUsage`) are
//! resolved via a custom `Retrieve` implementation that serves the schema
//! files from the compile-time `include_str!` constants.  The root schema has
//! no `$id`, so the `jsonschema` crate assigns it the base URI
//! `json-schema:///`; relative refs like `"common.json"` therefore resolve to
//! `"json-schema:///common.json"`.

mod common;

use std::collections::HashMap;

use jsonschema::{Retrieve, Uri};
use serde_json::{Value, json};

const CHAT_COMPLETION_SCHEMA: &str = include_str!("../../../schemas/api/chat_completion.json");
const EMBEDDING_SCHEMA: &str = include_str!("../../../schemas/api/embedding.json");
const MODELS_SCHEMA: &str = include_str!("../../../schemas/api/models.json");
const ERRORS_SCHEMA: &str = include_str!("../../../schemas/api/errors.json");
const COMMON_SCHEMA: &str = include_str!("../../../schemas/api/common.json");

/// Serves statically known schema files so that `$ref` values such as
/// `"common.json#/$defs/CompletionUsage"` resolve without network access.
struct StaticRetriever {
    schemas: HashMap<&'static str, &'static str>,
}

impl StaticRetriever {
    fn new() -> Self {
        let mut schemas = HashMap::new();
        schemas.insert("json-schema:///common.json", COMMON_SCHEMA);
        schemas.insert("json-schema:///chat_completion.json", CHAT_COMPLETION_SCHEMA);
        schemas.insert("json-schema:///embedding.json", EMBEDDING_SCHEMA);
        schemas.insert("json-schema:///models.json", MODELS_SCHEMA);
        schemas.insert("json-schema:///errors.json", ERRORS_SCHEMA);
        Self { schemas }
    }
}

impl Retrieve for StaticRetriever {
    fn retrieve(&self, uri: &Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let key = uri.as_str();
        self.schemas
            .get(key)
            .map(|src| serde_json::from_str(src).expect("schema is valid JSON"))
            .ok_or_else(|| format!("Schema not found for URI: {key}").into())
    }
}

/// Build a validator for `def_name` from `primary_schema`.
///
/// Cross-file `$ref` pointers are resolved via `StaticRetriever`.  The
/// target definition is extracted and used as the root schema so that the
/// validator operates on the right constraints without an extra wrapping level.
fn build_validator(primary_schema: &str, def_name: &str) -> jsonschema::Validator {
    let primary: Value = serde_json::from_str(primary_schema).expect("primary schema is valid JSON");

    let def = primary["$defs"][def_name].clone();
    assert!(
        def.is_object(),
        "Schema definition '{def_name}' not found in primary schema"
    );

    let mut root = def;
    if let Some(defs) = primary["$defs"].as_object() {
        root["$defs"] = Value::Object(defs.clone());
    }
    root["$schema"] = json!("https://json-schema.org/draft/2020-12/schema");

    jsonschema::options()
        .with_retriever(StaticRetriever::new())
        .build(&root)
        .unwrap_or_else(|e| panic!("Failed to compile schema for '{def_name}': {e}"))
}

/// Validate `instance` against the compiled `validator`, panicking with a
/// descriptive message on failure.
fn assert_valid(validator: &jsonschema::Validator, instance: &Value, label: &str) {
    let errors: Vec<String> = validator.iter_errors(instance).map(|e| format!("  - {e}")).collect();
    assert!(
        errors.is_empty(),
        "JSON instance for '{label}' violates schema:\n{}",
        errors.join("\n")
    );
}

/// The `CreateChatCompletionResponse` definition requires:
///   choices[].finish_reason   — string enum (non-nullable at top level)
///   choices[].logprobs        — object | null  (required key)
///   choices[].message.role    — required key, const `"assistant"`
///   choices[].message.content — string | null (required key)
///   choices[].message.refusal — string | null (required key)
///
/// Regression pin: this test previously PATCHED the serialized JSON before
/// validating — unconditionally injecting `role: "assistant"` and
/// back-filling `refusal`/`content`/`logprobs` with `null` whenever the real
/// serialization omitted them — then asserted against the *patched* document
/// instead of the type's real wire output. That could never catch a schema
/// mismatch in `ChatCompletionResponse`/`Choice`/`AssistantMessage`, since the
/// patch papered over exactly the gaps a mismatch would produce. It now
/// validates the real, unpatched `serde_json::to_value` output directly.
///
/// ~keep EXPECTED TO FAIL, for two independent reasons found while removing
/// the patch: (1) `Choice` (crates/liter-llm/src/types/chat.rs) has no
/// `logprobs` field at all, so the required `choices[].logprobs` key is never
/// emitted; (2) `AssistantMessage` (crates/liter-llm/src/types/common.rs) has
/// no `role` field at all — there is nowhere in the type to carry the
/// required `"assistant"` const — and its `refusal` field is
/// `#[serde(skip_serializing_if = "Option::is_none")]`, so a `None` refusal
/// (the common case) omits the required `refusal` key rather than emitting
/// `null`. All three are keys the schema requires to always be present
/// (nullable where applicable, but present); this is genuine non-conformance
/// in the real types, not a bug in this test.
#[test]
fn chat_completion_response_matches_schema() {
    use liter_llm::{AssistantMessage, ChatCompletionResponse, Choice, FinishReason, Usage};

    let response = ChatCompletionResponse {
        id: "chatcmpl-abc123".into(),
        object: "chat.completion".into(),
        created: 1_700_000_000,
        model: "gpt-4".into(),
        choices: vec![Choice {
            index: 0,
            message: AssistantMessage {
                content: Some("Hello!".into()),
                name: None,
                tool_calls: None,
                refusal: None,
                function_call: None,
                reasoning_content: None,
            },
            finish_reason: Some(FinishReason::Stop),
            logprobs: None,
        }],
        usage: Some(Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }),
        system_fingerprint: Some("fp_abc123".into()),
        service_tier: None,
    };

    let json = serde_json::to_value(&response).unwrap();

    let validator = build_validator(CHAT_COMPLETION_SCHEMA, "CreateChatCompletionResponse");
    assert_valid(&validator, &json, "CreateChatCompletionResponse");
}

/// `CreateChatCompletionStreamResponse` choices require `delta`, `finish_reason`,
/// and `index`.
///
/// The OpenAI schema uses the OpenAPI extension `"nullable": true` on
/// `finish_reason`, which is not a JSON Schema 2020-12 keyword.  The strict
/// validator therefore only accepts the string enum values.  We test with a
/// terminal streaming chunk (`finish_reason: "stop"`) which is always a valid
/// document under both the schema and the OpenAPI interpretation.
#[test]
fn chat_completion_chunk_matches_schema() {
    use liter_llm::{ChatCompletionChunk, FinishReason, StreamChoice, StreamDelta};

    let chunk = ChatCompletionChunk {
        id: "chatcmpl-chunk123".into(),
        object: "chat.completion.chunk".into(),
        created: 1_700_000_000,
        model: "gpt-4".into(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: None,
                content: None,
                tool_calls: None,
                function_call: None,
                refusal: None,
                reasoning_content: None,
            },
            finish_reason: Some(FinishReason::Stop),
        }],
        usage: None,
        system_fingerprint: None,
        service_tier: None,
    };

    let json = serde_json::to_value(&chunk).unwrap();

    let validator = build_validator(CHAT_COMPLETION_SCHEMA, "CreateChatCompletionStreamResponse");
    assert_valid(&validator, &json, "CreateChatCompletionStreamResponse");
}

/// `CreateEmbeddingResponse` requires `object`, `model`, `data`, `usage`.
/// The embedded usage requires `prompt_tokens` and `total_tokens`.
/// Our `EmbeddingResponse` marks `usage` as `Option<Usage>` so we test the
/// populated path here.
///
/// Regression pin: this test previously validated a hand-written
/// `json!({...})` literal that never referenced `EmbeddingResponse`,
/// `EmbeddingObject`, or `Usage` — the real types could be deleted and the
/// test would still pass. It now serializes the actual types and validates
/// that output.
#[test]
fn embedding_response_matches_schema() {
    use liter_llm::{EmbeddingObject, EmbeddingResponse, Usage};

    let response = EmbeddingResponse {
        object: "list".into(),
        data: vec![EmbeddingObject {
            object: "embedding".into(),
            index: 0,
            embedding: vec![0.1, 0.2, 0.3],
        }],
        model: "text-embedding-3-small".into(),
        usage: Some(Usage {
            prompt_tokens: 8,
            completion_tokens: 0,
            total_tokens: 8,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }),
    };

    let json = serde_json::to_value(&response).unwrap();

    let validator = build_validator(EMBEDDING_SCHEMA, "CreateEmbeddingResponse");
    assert_valid(&validator, &json, "CreateEmbeddingResponse");
}

/// Validate a single `Embedding` object.
#[test]
fn embedding_object_matches_schema() {
    use liter_llm::EmbeddingObject;

    let obj = EmbeddingObject {
        object: "embedding".into(),
        index: 0,
        embedding: vec![0.1, 0.2, 0.3],
    };

    let json = serde_json::to_value(&obj).unwrap();

    let validator = build_validator(EMBEDDING_SCHEMA, "Embedding");
    assert_valid(&validator, &json, "Embedding");
}

#[test]
fn models_list_response_matches_schema() {
    use liter_llm::{ModelObject, ModelsListResponse};

    let response = ModelsListResponse {
        object: "list".into(),
        data: vec![ModelObject {
            id: "gpt-4".into(),
            object: "model".into(),
            created: 1_686_935_002,
            owned_by: "openai".into(),
        }],
    };

    let json = serde_json::to_value(&response).unwrap();

    let validator = build_validator(MODELS_SCHEMA, "ListModelsResponse");
    assert_valid(&validator, &json, "ListModelsResponse");
}

/// Validate a single `Model` object.
#[test]
fn model_object_matches_schema() {
    use liter_llm::ModelObject;

    let obj = ModelObject {
        id: "gpt-4".into(),
        object: "model".into(),
        created: 1_686_935_002,
        owned_by: "openai".into(),
    };

    let json = serde_json::to_value(&obj).unwrap();

    let validator = build_validator(MODELS_SCHEMA, "Model");
    assert_valid(&validator, &json, "Model");
}

/// Regression for #139: DeepSeek's `/v1/models` omits `created`, so a
/// `ModelObject` must deserialize with the field missing and default it to `0`.
#[test]
fn model_object_deserializes_when_created_is_missing() {
    use liter_llm::ModelObject;

    let body = r#"{"id":"deepseek-chat","object":"model","owned_by":"deepseek"}"#;
    let obj: ModelObject = serde_json::from_str(body).expect("must deserialize without `created`");

    assert_eq!(obj.id, "deepseek-chat");
    assert_eq!(obj.object, "model");
    assert_eq!(obj.owned_by, "deepseek");
    assert_eq!(obj.created, 0, "missing `created` must default to 0");
}

/// A model object where every non-`id` field is absent must still deserialize —
/// `object`, `created`, and `owned_by` are all defaulted.
#[test]
fn model_object_deserializes_with_only_id() {
    use liter_llm::ModelObject;

    let obj: ModelObject = serde_json::from_str(r#"{"id":"some-model"}"#).expect("id-only must deserialize");

    assert_eq!(obj.id, "some-model");
    assert_eq!(obj.object, "");
    assert_eq!(obj.created, 0);
    assert_eq!(obj.owned_by, "");
}

/// Regression for #139 at the list level: a DeepSeek-shaped `/v1/models`
/// response (elements missing `created`) must deserialize into
/// `ModelsListResponse` — this is the exact payload `list_models` parses.
#[test]
fn models_list_response_deserializes_deepseek_shape() {
    use liter_llm::ModelsListResponse;

    let body = r#"{
        "object": "list",
        "data": [
            {"id": "deepseek-chat", "object": "model", "owned_by": "deepseek"},
            {"id": "deepseek-reasoner", "object": "model", "owned_by": "deepseek"}
        ]
    }"#;
    let parsed: ModelsListResponse = serde_json::from_str(body).expect("DeepSeek list response must deserialize");

    assert_eq!(parsed.data.len(), 2);
    assert_eq!(parsed.data[0].id, "deepseek-chat");
    assert_eq!(parsed.data[0].created, 0);
}

// ~keep NOTE ON `error_response_matches_schema` / `error_object_with_code_matches_schema`
// below: both were flagged as vacuous — they build a `json!({..})` literal by
// hand and validate that literal, never constructing or serializing a real
// Rust type. Investigating what the "real type" would even be turned up a
// blocker: `crates/liter-llm/src/error.rs` defines `pub(crate) struct
// ErrorResponse { error: ApiError }` and `pub(crate) struct ApiError {
// message: String, code: Option<String> }` — both `pub(crate)`, and neither
// is re-exported from the crate root (`lib.rs` only re-exports
// `error::{LiterLlmError, Result}`). `tests/contract.rs` is a separate
// integration-test crate that only sees the crate's public API, so
// `ErrorResponse`/`ApiError` are not nameable here at all; there is no way to
// `use liter_llm::ErrorResponse` or `liter_llm::ApiError`. Nor does the crate
// expose any other public type shaped like the OpenAI wire error object (no
// `pub` struct anywhere in the crate carries a `param` field). These two
// tests are therefore left validating the same hand-written literal as
// before — genuinely still vacuous — because fixing them for real requires
// either exposing `ApiError`/`ErrorResponse` for testing or moving the check
// into a `#[cfg(test)]` unit test inside `src/error.rs` (which has
// crate-internal visibility), and both of those are edits outside
// `tests/contract.rs`, which is the only file this change is scoped to touch.
//
// This also surfaces a second, independent defect worth flagging: even if
// `ApiError` were made reachable, it only has `message` and `code` fields —
// it has no `type` or `param` field at all — while the `Error` schema
// requires `type`, `message`, `param`, and `code` to always be present as
// keys. Serializing the real `ApiError` today would fail this schema on two
// required keys (`type`, `param`), regardless of visibility.

/// Validate the `ErrorResponse` wrapper and the inner `Error` object.
///
/// See the block comment above: the real `pub(crate)` `ErrorResponse`/
/// `ApiError` types cannot be named from this integration-test crate, so this
/// still validates a hand-written literal rather than a serialized real type.
#[test]
fn error_response_matches_schema() {
    let instance = json!({
        "error": {
            "type": "invalid_request_error",
            "message": "You must provide a model parameter",
            "param": null,
            "code": null
        }
    });

    let validator = build_validator(ERRORS_SCHEMA, "ErrorResponse");
    assert_valid(&validator, &instance, "ErrorResponse");
}

/// See the block comment above `error_response_matches_schema`: the real
/// `pub(crate)` `ApiError` type cannot be named from this integration-test
/// crate, so this still validates a hand-written literal rather than a
/// serialized real type.
#[test]
fn error_object_with_code_matches_schema() {
    let instance = json!({
        "type": "invalid_request_error",
        "message": "The model does not exist",
        "param": "model",
        "code": "model_not_found"
    });

    let validator = build_validator(ERRORS_SCHEMA, "Error");
    assert_valid(&validator, &instance, "Error");
}
