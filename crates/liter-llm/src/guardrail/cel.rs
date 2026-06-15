//! CEL (Common Expression Language) policy engine for guardrails.
//!
//! Provides [`CelGuardrail`], which evaluates a CEL expression against the
//! [`GuardrailContext`] and applies a configurable action when the expression
//! evaluates to `true`.
//!
//! # Feature gate
//!
//! This module is only compiled when the `guardrail-cel` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! liter-llm = { features = ["guardrail-cel"] }
//! ```
//!
//! # CEL variables available in expressions
//!
//! | Variable | Type | Description |
//! |---|---|---|
//! | `request` | map | Full JSON request object |
//! | `response` | map | Full JSON response object (Output stage only; empty map otherwise) |
//! | `chunk` | string | Raw streaming chunk text (OutputChunk stage only; empty string otherwise) |
//! | `metadata` | map | Per-call tags (user_id, tenant_id, route, …) |
//!
//! # Example
//!
//! ```rust,ignore
//! use liter_llm::guardrail::cel::{CelAction, CelGuardrail};
//! use liter_llm::guardrail::GuardrailStage;
//!
//! // Block requests from non-premium tenants targeting gpt-4o.
//! let guardrail = CelGuardrail::new(
//!     "gpt4o-premium-only",
//!     r#"request.model == "gpt-4o" && metadata.tier != "premium""#,
//!     CelAction::Block { code: 1300, reason: "gpt-4o requires premium tier".into() },
//!     &[GuardrailStage::Input],
//! ).expect("invalid CEL expression");
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cel_interpreter::objects::{Key, Map, Value};
use cel_interpreter::{Context, ParseError, Program};

use super::{Guardrail, GuardrailContext, GuardrailDecision, GuardrailStage};

// ── Action when CEL expression evaluates to true ──────────────────────────────

/// The action taken when a [`CelGuardrail`]'s expression evaluates to `true`.
#[derive(Debug, Clone)]
pub enum CelAction {
    /// Block the request/response with the given code and reason.
    Block {
        /// Numeric error code (≥ 1000).
        code: u32,
        /// Human-readable reason.
        reason: String,
    },
    /// Replace the payload with a static JSON value (e.g., for redaction).
    Mutate {
        /// The replacement payload.
        new_payload: serde_json::Value,
    },
}

// ── CelGuardrail ──────────────────────────────────────────────────────────────

/// Evaluates a CEL expression against the [`GuardrailContext`] and applies
/// `on_true` when the expression evaluates to `true`.
///
/// The CEL expression is compiled at construction time so runtime evaluation
/// is fast. Construction returns an error if the expression is syntactically
/// invalid.
pub struct CelGuardrail {
    guardrail_name: &'static str,
    program: Program,
    on_true: CelAction,
    stages: &'static [GuardrailStage],
}

impl CelGuardrail {
    /// Create a new [`CelGuardrail`].
    ///
    /// `expression` is parsed and compiled at construction time.
    ///
    /// # Errors
    ///
    /// Returns an error if `expression` is not a valid CEL expression.
    pub fn new(
        name: &'static str,
        expression: &str,
        on_true: CelAction,
        stages: &'static [GuardrailStage],
    ) -> Result<Self, ParseError> {
        let program = Program::compile(expression)?;
        Ok(Self {
            guardrail_name: name,
            program,
            on_true,
            stages,
        })
    }
}

impl Guardrail for CelGuardrail {
    fn name(&self) -> &'static str {
        self.guardrail_name
    }

    fn supported_stages(&self) -> &'static [GuardrailStage] {
        self.stages
    }

    fn check<'a>(
        &'a self,
        stage: GuardrailStage,
        ctx: &'a GuardrailContext<'a>,
    ) -> Pin<Box<dyn Future<Output = GuardrailDecision> + Send + 'a>> {
        Box::pin(async move {
            // Suppress unused-variable lint when the tracing feature is off.
            let _ = stage;
            let mut cel_ctx = Context::default();

            // Bind `request` — always present.
            cel_ctx.add_variable_from_value("request", json_value_to_cel(ctx.request));

            // Bind `response` — empty map when not at Output stage.
            let response_val = ctx.response.map(json_value_to_cel).unwrap_or_else(|| {
                Value::Map(Map {
                    map: Arc::new(HashMap::new()),
                })
            });
            cel_ctx.add_variable_from_value("response", response_val);

            // Bind `chunk` — empty string when not at OutputChunk stage.
            let chunk_str = ctx.chunk.unwrap_or("").to_string();
            cel_ctx.add_variable_from_value("chunk", Value::String(Arc::new(chunk_str)));

            // Bind `metadata` as a CEL map.
            cel_ctx.add_variable_from_value("metadata", metadata_to_cel(ctx.metadata));

            // Evaluate the expression. Treat eval errors as Allow (fail-open)
            // to avoid blocking legitimate requests on CEL engine errors.
            match self.program.execute(&cel_ctx) {
                Ok(Value::Bool(true)) => match &self.on_true {
                    CelAction::Block { code, reason } => GuardrailDecision::Block {
                        reason: reason.clone(),
                        code: *code,
                    },
                    CelAction::Mutate { new_payload } => GuardrailDecision::Mutate {
                        new_payload: new_payload.clone(),
                    },
                },
                Ok(_) => GuardrailDecision::Allow,
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        guardrail = self.guardrail_name,
                        stage = ?stage,
                        "CEL expression evaluation error (fail-open): {e}"
                    );
                    #[cfg(not(feature = "tracing"))]
                    let _ = e;
                    GuardrailDecision::Allow
                }
            }
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Convert a [`serde_json::Value`] to a CEL [`Value`].
///
/// Complex nested objects become CEL maps; arrays become CEL lists.
fn json_value_to_cel(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(Arc::new(s.clone())),
        serde_json::Value::Array(arr) => {
            let items: Vec<Value> = arr.iter().map(json_value_to_cel).collect();
            Value::List(Arc::new(items))
        }
        serde_json::Value::Object(obj) => {
            let mut map: HashMap<Key, Value> = HashMap::new();
            for (key, val) in obj {
                map.insert(Key::String(Arc::new(key.clone())), json_value_to_cel(val));
            }
            Value::Map(Map { map: Arc::new(map) })
        }
    }
}

/// Convert a `HashMap<String, String>` metadata map to a CEL [`Value::Map`].
fn metadata_to_cel(metadata: &HashMap<String, String>) -> Value {
    let mut map: HashMap<Key, Value> = HashMap::new();
    for (key, val) in metadata {
        map.insert(Key::String(Arc::new(key.clone())), Value::String(Arc::new(val.clone())));
    }
    Value::Map(Map { map: Arc::new(map) })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::guardrail::{GuardrailContext, GuardrailStage};

    static INPUT_STAGES: &[GuardrailStage] = &[GuardrailStage::Input];

    fn meta_with(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[tokio::test]
    async fn cel_guardrail_blocks_when_expression_is_true() {
        let meta = meta_with(&[("tier", "free")]);
        let req = serde_json::json!({ "model": "gpt-4o" });

        let guardrail = CelGuardrail::new(
            "gpt4o-premium-only",
            r#"request.model == "gpt-4o" && metadata.tier != "premium""#,
            CelAction::Block {
                code: 1300,
                reason: "gpt-4o requires premium tier".into(),
            },
            INPUT_STAGES,
        )
        .expect("valid CEL expression");

        let ctx = GuardrailContext {
            request: &req,
            response: None,
            chunk: None,
            metadata: &meta,
        };

        let decision = guardrail.check(GuardrailStage::Input, &ctx).await;
        match decision {
            GuardrailDecision::Block { code, reason } => {
                assert_eq!(code, 1300);
                assert!(reason.contains("premium"), "reason should mention premium tier");
            }
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn cel_guardrail_allows_when_expression_is_false() {
        let meta = meta_with(&[("tier", "premium")]);
        let req = serde_json::json!({ "model": "gpt-4o" });

        let guardrail = CelGuardrail::new(
            "gpt4o-premium-only",
            r#"request.model == "gpt-4o" && metadata.tier != "premium""#,
            CelAction::Block {
                code: 1300,
                reason: "gpt-4o requires premium tier".into(),
            },
            INPUT_STAGES,
        )
        .expect("valid CEL expression");

        let ctx = GuardrailContext {
            request: &req,
            response: None,
            chunk: None,
            metadata: &meta,
        };

        let decision = guardrail.check(GuardrailStage::Input, &ctx).await;
        assert!(decision.is_allow(), "premium tier should be allowed");
    }

    #[tokio::test]
    async fn cel_guardrail_allows_when_model_does_not_match() {
        let meta = meta_with(&[("tier", "free")]);
        let req = serde_json::json!({ "model": "gpt-3.5-turbo" });

        let guardrail = CelGuardrail::new(
            "gpt4o-premium-only",
            r#"request.model == "gpt-4o" && metadata.tier != "premium""#,
            CelAction::Block {
                code: 1300,
                reason: "gpt-4o requires premium tier".into(),
            },
            INPUT_STAGES,
        )
        .expect("valid CEL expression");

        let ctx = GuardrailContext {
            request: &req,
            response: None,
            chunk: None,
            metadata: &meta,
        };

        let decision = guardrail.check(GuardrailStage::Input, &ctx).await;
        assert!(
            decision.is_allow(),
            "non-gpt-4o model should be allowed regardless of tier"
        );
    }

    #[tokio::test]
    async fn cel_guardrail_returns_error_for_invalid_expression() {
        let result = CelGuardrail::new(
            "broken",
            "this is not valid !!! CEL $$$",
            CelAction::Block {
                code: 1399,
                reason: "test".into(),
            },
            INPUT_STAGES,
        );
        assert!(result.is_err(), "invalid CEL should fail at construction");
    }

    #[tokio::test]
    async fn cel_guardrail_simple_boolean_true_expression_blocks() {
        let meta = HashMap::new();
        let req = serde_json::json!({});

        let guardrail = CelGuardrail::new(
            "always-block",
            "true",
            CelAction::Block {
                code: 1399,
                reason: "always blocked".into(),
            },
            INPUT_STAGES,
        )
        .expect("valid CEL");

        let ctx = GuardrailContext {
            request: &req,
            response: None,
            chunk: None,
            metadata: &meta,
        };
        let decision = guardrail.check(GuardrailStage::Input, &ctx).await;
        assert!(decision.is_block());
    }

    #[tokio::test]
    async fn cel_guardrail_simple_boolean_false_expression_allows() {
        let meta = HashMap::new();
        let req = serde_json::json!({});

        let guardrail = CelGuardrail::new(
            "never-block",
            "false",
            CelAction::Block {
                code: 1399,
                reason: "never blocked".into(),
            },
            INPUT_STAGES,
        )
        .expect("valid CEL");

        let ctx = GuardrailContext {
            request: &req,
            response: None,
            chunk: None,
            metadata: &meta,
        };
        let decision = guardrail.check(GuardrailStage::Input, &ctx).await;
        assert!(decision.is_allow());
    }
}
