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
//! # Fail-closed vs fail-open
//!
//! By default [`CelGuardrail`] is **fail-closed**: if the CEL expression cannot
//! be evaluated at runtime (eval error, non-bool result), the guardrail returns
//! [`GuardrailDecision::Block`] with code `4001`. This is the only secure
//! default — an attacker who can craft a request that triggers an eval error
//! must not be able to bypass ALL guardrails as a result.
//!
//! To opt in to fail-open behaviour (e.g., in development environments where
//! guardrails are advisory), chain [`CelGuardrail::with_fail_open`]:
//!
//! ```rust,ignore
//! let guardrail = CelGuardrail::new(...)
//!     .expect("valid CEL")
//!     .with_fail_open(true); // SECURITY WARNING: only use in non-production
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

// ── Error code for guardrail evaluation failures ──────────────────────────────

/// Numeric error code returned when a CEL expression cannot be evaluated and
/// the guardrail is configured to fail-closed (the default).
const CEL_EVAL_ERROR_CODE: u32 = 4001;

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
///
/// # Fail-closed behaviour (default, secure)
///
/// When the expression cannot be evaluated at runtime (eval error or non-bool
/// result), [`CelGuardrail`] returns [`GuardrailDecision::Block`] with code
/// `4001` and logs an `error`-level tracing event.
///
/// **SECURITY WARNING**: Call [`CelGuardrail::with_fail_open`] only in
/// non-production environments. Fail-open means a crafted request that triggers
/// an eval error bypasses ALL CEL guardrails on that instance.
pub struct CelGuardrail {
    guardrail_name: &'static str,
    program: Program,
    on_true: CelAction,
    stages: &'static [GuardrailStage],
    /// When `true`, runtime evaluation errors return `Allow` instead of `Block`.
    /// Defaults to `false` (fail-closed). Use [`with_fail_open`] to opt in.
    fail_open: bool,
}

impl CelGuardrail {
    /// Create a new [`CelGuardrail`].
    ///
    /// `expression` is parsed and compiled at construction time.
    ///
    /// The guardrail defaults to **fail-closed**: eval errors and non-bool
    /// results return [`GuardrailDecision::Block`] with code 4001. Use
    /// [`with_fail_open`] to override.
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
            fail_open: false,
        })
    }

    /// Set the fail-open mode for this guardrail.
    ///
    /// When `fail_open` is `true`, runtime evaluation errors and non-bool
    /// results return [`GuardrailDecision::Allow`] instead of blocking.
    ///
    /// **SECURITY WARNING**: Only use `fail_open(true)` in non-production
    /// environments where guardrails are advisory. In production, the default
    /// fail-closed behaviour prevents eval errors from becoming a bypass vector.
    #[must_use]
    pub fn with_fail_open(mut self, fail_open: bool) -> Self {
        self.fail_open = fail_open;
        self
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

                // Expression evaluated to false — allow the request.
                Ok(Value::Bool(false)) => GuardrailDecision::Allow,

                // Expression returned a non-bool value. This is a guardrail
                // authoring error. Fail-closed by default to prevent bypass.
                Ok(non_bool) => {
                    // Log the full internal detail server-side only.
                    // The opaque reason returned to the caller must NOT include
                    // `non_bool` — CEL result internals can reflect
                    // user-controlled expression fragments and must not be
                    // surfaced to the API caller.
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        guardrail = self.guardrail_name,
                        stage = ?stage,
                        result = ?non_bool,
                        "CEL expression returned non-bool value; \
                         defaulting to fail-closed (Block/4001) — \
                         set fail_open=true to suppress"
                    );
                    #[cfg(not(feature = "tracing"))]
                    {
                        let _ = stage;
                        let _ = non_bool;
                    }

                    if self.fail_open {
                        GuardrailDecision::Allow
                    } else {
                        GuardrailDecision::Block {
                            // Opaque reason — no internal detail.
                            reason: "policy evaluation error".to_owned(),
                            code: CEL_EVAL_ERROR_CODE,
                        }
                    }
                }

                // CEL runtime error. Fail-closed by default: an attacker who
                // can trigger eval errors must not be able to bypass guardrails.
                Err(e) => {
                    // Log the full error server-side only.
                    // The opaque reason returned to the caller must NOT include
                    // `e` — CEL runtime errors can reflect user-controlled
                    // expression internals and must not be surfaced to the
                    // API caller via the `reason` field.
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        guardrail = self.guardrail_name,
                        stage = ?stage,
                        error = %e,
                        "CEL expression evaluation error; \
                         defaulting to fail-closed (Block/4001) — \
                         set fail_open=true to suppress"
                    );
                    #[cfg(not(feature = "tracing"))]
                    {
                        let _ = stage;
                        let _ = e;
                    }

                    if self.fail_open {
                        GuardrailDecision::Allow
                    } else {
                        GuardrailDecision::Block {
                            // Opaque reason — no internal detail.
                            // The full error is logged above (server-side only).
                            reason: "policy evaluation error".to_owned(),
                            code: CEL_EVAL_ERROR_CODE,
                        }
                    }
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

    // ── Existing tests (unchanged behaviour) ──────────────────────────────────

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

    // ── New security tests ────────────────────────────────────────────────────

    /// A CEL expression that references an undeclared variable triggers an
    /// `ExecutionError::UndeclaredReference` at eval time. Default (fail-closed)
    /// must return Block/4001.
    #[tokio::test]
    async fn cel_guardrail_eval_error_defaults_to_block() {
        let meta = HashMap::new();
        let req = serde_json::json!({ "model": "gpt-4o" });

        let guardrail = CelGuardrail::new(
            "undeclared-var-guardrail",
            // `undeclared_var` is never bound in the activation — this is a
            // reliable `ExecutionError::UndeclaredReference` at eval time.
            "undeclared_var == true",
            CelAction::Block {
                code: 1500,
                reason: "blocked by policy".into(),
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
                assert_eq!(code, CEL_EVAL_ERROR_CODE, "eval error must use code 4001");
                // The reason returned to callers must be opaque — no internal
                // detail (e.g. CEL error message, expression internals) may be
                // included.  Full error detail is logged server-side only.
                assert_eq!(
                    reason, "policy evaluation error",
                    "eval error reason must be opaque; got: {reason}"
                );
                assert!(
                    !reason.contains("guardrail evaluation error"),
                    "old verbose reason must not appear in caller response; got: {reason}"
                );
            }
            other => panic!("expected Block(4001) on eval error (fail-closed default), got {other:?}"),
        }
    }

    /// Same eval-error scenario, but with `with_fail_open(true)`: must return Allow.
    #[tokio::test]
    async fn cel_guardrail_eval_error_with_fail_open_returns_allow() {
        let meta = HashMap::new();
        let req = serde_json::json!({ "model": "gpt-4o" });

        let guardrail = CelGuardrail::new(
            "undeclared-var-fail-open",
            "undeclared_var == true",
            CelAction::Block {
                code: 1500,
                reason: "blocked by policy".into(),
            },
            INPUT_STAGES,
        )
        .expect("valid CEL expression")
        .with_fail_open(true);

        let ctx = GuardrailContext {
            request: &req,
            response: None,
            chunk: None,
            metadata: &meta,
        };

        let decision = guardrail.check(GuardrailStage::Input, &ctx).await;
        assert!(
            decision.is_allow(),
            "with_fail_open(true) must return Allow on eval error, got {decision:?}"
        );
    }

    /// An expression that returns a string (non-bool) must Block by default.
    /// With fail_open=true it must Allow.
    #[tokio::test]
    async fn cel_guardrail_non_bool_result_blocks_by_default() {
        let meta = HashMap::new();
        let req = serde_json::json!({ "model": "gpt-4o" });

        // `request.model` is a string, not a bool — non-bool result path.
        let guardrail_default = CelGuardrail::new(
            "non-bool-default",
            "request.model",
            CelAction::Block {
                code: 1500,
                reason: "blocked by policy".into(),
            },
            INPUT_STAGES,
        )
        .expect("valid CEL expression");

        let guardrail_fail_open = CelGuardrail::new(
            "non-bool-fail-open",
            "request.model",
            CelAction::Block {
                code: 1500,
                reason: "blocked by policy".into(),
            },
            INPUT_STAGES,
        )
        .expect("valid CEL expression")
        .with_fail_open(true);

        let ctx = GuardrailContext {
            request: &req,
            response: None,
            chunk: None,
            metadata: &meta,
        };

        // Default: fail-closed → Block/4001
        let decision_default = guardrail_default.check(GuardrailStage::Input, &ctx).await;
        match &decision_default {
            GuardrailDecision::Block { code, .. } => {
                assert_eq!(*code, CEL_EVAL_ERROR_CODE, "non-bool must use code 4001");
            }
            other => panic!("expected Block(4001) for non-bool result (fail-closed default), got {other:?}"),
        }

        // With fail_open=true → Allow
        let decision_open = guardrail_fail_open.check(GuardrailStage::Input, &ctx).await;
        assert!(
            decision_open.is_allow(),
            "fail_open=true must return Allow for non-bool result, got {decision_open:?}"
        );
    }

    /// Malformed CEL expression must fail at construction time (existing behaviour preserved).
    #[tokio::test]
    async fn cel_guardrail_compile_error_still_blocks_construction() {
        let result = CelGuardrail::new(
            "bad-syntax",
            "request.model ==",
            CelAction::Block {
                code: 1399,
                reason: "test".into(),
            },
            INPUT_STAGES,
        );
        assert!(
            result.is_err(),
            "malformed CEL expression must fail at construction, not at eval time"
        );
    }

    // ── Security: opaque error reasons ───────────────────────────────────────

    /// Induce a CEL eval error via an undeclared variable reference and assert
    /// that the reason returned to the caller is the opaque sentinel string,
    /// NOT the internal `format!("guardrail evaluation error: {e}")` form.
    ///
    /// This prevents CEL runtime errors (which can reflect user-controlled
    /// expression internals) from reaching the API caller.
    #[tokio::test]
    async fn cel_error_reason_not_leaked_to_caller() {
        let meta = HashMap::new();
        let req = serde_json::json!({ "model": "gpt-4o" });

        // An undeclared variable reference triggers `ExecutionError::UndeclaredReference`
        // at eval time.  The error message produced by `cel_interpreter` contains
        // the variable name — that must NOT reach the caller.
        let guardrail = CelGuardrail::new(
            "leaky-error-guardrail",
            "undeclared_internal_var == true",
            CelAction::Block {
                code: 1500,
                reason: "should not appear — error path taken".into(),
            },
            INPUT_STAGES,
        )
        .expect("syntactically valid CEL");

        let ctx = GuardrailContext {
            request: &req,
            response: None,
            chunk: None,
            metadata: &meta,
        };

        let decision = guardrail.check(GuardrailStage::Input, &ctx).await;
        match decision {
            GuardrailDecision::Block { code, reason } => {
                assert_eq!(code, CEL_EVAL_ERROR_CODE, "eval error must use code 4001");

                // The caller-facing reason must be the opaque sentinel.
                assert_eq!(
                    reason, "policy evaluation error",
                    "reason must be opaque sentinel, not internal error detail; got: {reason:?}"
                );

                // Regression guard: the old verbose form must NOT appear.
                assert!(
                    !reason.contains("guardrail evaluation error"),
                    "old verbose reason must not leak to caller; got: {reason:?}"
                );

                // The internal variable name must NOT appear in the reason.
                assert!(
                    !reason.contains("undeclared_internal_var"),
                    "internal CEL variable name must not leak to caller; got: {reason:?}"
                );
            }
            other => panic!("expected Block(4001) from eval error, got {other:?}"),
        }
    }

    /// Same test for the non-bool result path: a CEL expression that returns
    /// a string must produce the opaque reason, not the actual CEL result value.
    #[tokio::test]
    async fn cel_non_bool_reason_not_leaked_to_caller() {
        let meta = HashMap::new();
        let req = serde_json::json!({ "model": "gpt-4o" });

        // `request.model` returns the string "gpt-4o" — a non-bool result.
        // The old code included the model name in the reason via `format!`.
        let guardrail = CelGuardrail::new(
            "non-bool-leak-check",
            "request.model",
            CelAction::Block {
                code: 1500,
                reason: "should not appear".into(),
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
                assert_eq!(code, CEL_EVAL_ERROR_CODE);
                assert_eq!(
                    reason, "policy evaluation error",
                    "non-bool reason must be opaque; got: {reason:?}"
                );
                // The actual model name must NOT appear in the reason.
                assert!(
                    !reason.contains("gpt-4o"),
                    "CEL result value must not leak to caller; got: {reason:?}"
                );
            }
            other => panic!("expected Block(4001), got {other:?}"),
        }
    }
}

