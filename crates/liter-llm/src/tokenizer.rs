//! Token counting via HuggingFace tokenizers.
//!
//! Feature-gated behind `tokenizer`. Provides lazy-cached tokenizer
//! loading and token counting for text and chat completion requests.
//!
//! # Architecture
//!
//! Tokenizers are loaded from HuggingFace Hub on first use and cached in a
//! process-global `LazyLock<RwLock<HashMap>>`. A two-phase locking strategy
//! minimises contention: the fast path takes a read lock to look up a cached
//! tokenizer; only on a cache miss is the write lock acquired (with a
//! double-check to handle concurrent misses).
//!
//! # Model Mapping
//!
//! Model name prefixes are mapped to HuggingFace tokenizer repository IDs via
//! [`resolve_tokenizer_id`]. Unknown models fall back to the GPT-4o tokenizer,
//! which gives a reasonable approximation for most modern LLMs.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use tokenizers::Tokenizer;

use crate::error::{LiterLlmError, Result};
use crate::types::{AssistantContent, AssistantPart, ChatCompletionRequest, ContentPart, Message, UserContent};

/// Process-global tokenizer cache keyed by HuggingFace tokenizer ID.
static TOKENIZER_CACHE: LazyLock<RwLock<HashMap<String, Arc<Tokenizer>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Map a model name to the HuggingFace tokenizer repository ID best
/// approximating its vocabulary.
///
/// The mapping is based on model name prefixes. When no prefix matches,
/// the GPT-4o tokenizer is used as a reasonable default for modern LLMs.
fn resolve_tokenizer_id(model: &str) -> &'static str {
    if model.starts_with("gpt-4")
        || model.starts_with("gpt-3.5")
        || model.starts_with("chatgpt")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
    {
        "Xenova/gpt-4o"
    } else if model.starts_with("claude") || model.starts_with("anthropic") {
        "Xenova/claude-tokenizer"
    } else if model.starts_with("gemini") || model.starts_with("vertex_ai") {
        "google/gemma-2b"
    } else if model.starts_with("mistral") || model.starts_with("codestral") {
        "mistralai/Mistral-7B-v0.1"
    } else if model.starts_with("command") || model.starts_with("cohere") {
        "Cohere/command-r-plus-tokenizer"
    } else if model.starts_with("llama") || model.starts_with("meta-llama") {
        "meta-llama/Meta-Llama-3-8B"
    } else {
        // Default to GPT-4o tokenizer for unknown models — it produces a
        // reasonable approximation for most modern LLMs.
        "Xenova/gpt-4o"
    }
}

/// Retrieve a cached tokenizer or load one from HuggingFace Hub.
///
/// Uses two-phase locking: read lock (fast path), then write lock on miss
/// with a double-check to avoid redundant downloads under contention.
fn get_or_load_tokenizer(model: &str) -> Result<Arc<Tokenizer>> {
    let tokenizer_id = resolve_tokenizer_id(model);

    // Fast path: read lock
    {
        let cache = TOKENIZER_CACHE.read().map_err(|e| LiterLlmError::BadRequest {
            message: format!("tokenizer cache lock poisoned: {e}"),
            status: 400,
        })?;
        if let Some(tok) = cache.get(tokenizer_id) {
            return Ok(Arc::clone(tok));
        }
    }

    // Slow path: write lock, double-check, initialize
    let mut cache = TOKENIZER_CACHE.write().map_err(|e| LiterLlmError::BadRequest {
        message: format!("tokenizer cache lock poisoned: {e}"),
        status: 400,
    })?;
    if let Some(tok) = cache.get(tokenizer_id) {
        return Ok(Arc::clone(tok));
    }

    let tokenizer = Tokenizer::from_pretrained(tokenizer_id, None).map_err(|e| LiterLlmError::BadRequest {
        message: format!("failed to load tokenizer '{tokenizer_id}': {e}"),
        status: 400,
    })?;

    let arc = Arc::new(tokenizer);
    cache.insert(tokenizer_id.to_owned(), Arc::clone(&arc));
    Ok(arc)
}

/// Count tokens in a text string using the tokenizer for the given model.
///
/// The tokenizer is resolved from the model name prefix (e.g. `"gpt-4o"` maps
/// to the `Xenova/gpt-4o` HuggingFace tokenizer). Tokenizers are cached after
/// first load.
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] if the tokenizer cannot be loaded
/// (e.g. network failure on first use) or if tokenization itself fails.
pub fn count_tokens(model: &str, text: &str) -> Result<usize> {
    let tokenizer = get_or_load_tokenizer(model)?;
    let encoding = tokenizer.encode(text, false).map_err(|e| LiterLlmError::BadRequest {
        message: format!("tokenization failed: {e}"),
        status: 400,
    })?;
    Ok(encoding.get_ids().len())
}

/// Extract the text content from a single [`ContentPart`].
///
/// Returns `Some(&str)` for text parts, `None` for images/documents/audio.
fn content_part_text(part: &ContentPart) -> Option<&str> {
    match part {
        ContentPart::Text { text } => Some(text.as_str()),
        ContentPart::ImageUrl { .. } | ContentPart::Document { .. } | ContentPart::InputAudio { .. } => None,
    }
}

/// Extract the text content from a single [`AssistantPart`].
///
/// Returns `Some(&str)` for `Text` parts, `None` for refusals / output media.
fn assistant_part_text(part: &AssistantPart) -> Option<&str> {
    match part {
        AssistantPart::Text { text } => Some(text.as_str()),
        AssistantPart::Refusal { .. }
        | AssistantPart::OutputImage { .. }
        | AssistantPart::OutputAudio { .. } => None,
    }
}

/// Count tokens for a full [`ChatCompletionRequest`].
///
/// Sums tokens across all message text contents plus a per-message overhead
/// of ~4 tokens (for role, separators, and formatting metadata). Tool
/// definitions and multimodal content parts (images, audio, documents) are
/// not counted — only textual content contributes to the token total.
///
/// # Errors
///
/// Returns [`LiterLlmError::BadRequest`] if the tokenizer cannot be loaded or
/// if tokenization fails for any message.
pub fn count_request_tokens(model: &str, req: &ChatCompletionRequest) -> Result<usize> {
    let tokenizer = get_or_load_tokenizer(model)?;
    let mut total = 0usize;

    let encode = |t: &str| -> Result<usize> {
        let encoding = tokenizer.encode(t, false).map_err(|e| LiterLlmError::BadRequest {
            message: format!("tokenization failed: {e}"),
            status: 400,
        })?;
        Ok(encoding.get_ids().len())
    };

    for msg in &req.messages {
        match msg {
            Message::System(m) => match &m.content {
                UserContent::Text(t) => total += encode(t)?,
                UserContent::Parts(parts) => {
                    for part in parts {
                        if let Some(text) = content_part_text(part) {
                            total += encode(text)?;
                        }
                    }
                }
            },
            Message::User(m) => match &m.content {
                UserContent::Text(t) => total += encode(t)?,
                UserContent::Parts(parts) => {
                    for part in parts {
                        if let Some(text) = content_part_text(part) {
                            total += encode(text)?;
                        }
                    }
                }
            },
            Message::Assistant(m) => {
                match &m.content {
                    Some(AssistantContent::Text(t)) => total += encode(t)?,
                    Some(AssistantContent::Parts(parts)) => {
                        for part in parts {
                            if let Some(text) = assistant_part_text(part) {
                                total += encode(text)?;
                            }
                        }
                    }
                    None => {}
                }
                // Tool-call-only assistant messages also contribute their tool-call
                // arguments to the token budget regardless of content shape.
                if m.content.is_none()
                    && let Some(ref calls) = m.tool_calls
                {
                    for call in calls {
                        total += encode(call.function.arguments.as_str())?;
                    }
                }
            }
            Message::Tool(m) => total += encode(&m.content)?,
            Message::Developer(m) => total += encode(&m.content)?,
            Message::Function(m) => total += encode(&m.content)?,
        }
    }

    // Per-message overhead: ~4 tokens for role tag, separators, and formatting
    // metadata. This matches the OpenAI tokenization overhead estimate.
    total += req.messages.len() * 4;

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_tokenizer_id_openai() {
        assert_eq!(resolve_tokenizer_id("gpt-4o"), "Xenova/gpt-4o");
        assert_eq!(resolve_tokenizer_id("gpt-4-turbo"), "Xenova/gpt-4o");
        assert_eq!(resolve_tokenizer_id("gpt-3.5-turbo"), "Xenova/gpt-4o");
        assert_eq!(resolve_tokenizer_id("chatgpt-4o-latest"), "Xenova/gpt-4o");
        assert_eq!(resolve_tokenizer_id("o1-preview"), "Xenova/gpt-4o");
        assert_eq!(resolve_tokenizer_id("o3-mini"), "Xenova/gpt-4o");
    }

    #[test]
    fn test_resolve_tokenizer_id_anthropic() {
        assert_eq!(resolve_tokenizer_id("claude-3-opus"), "Xenova/claude-tokenizer");
        assert_eq!(resolve_tokenizer_id("anthropic/claude-3"), "Xenova/claude-tokenizer");
    }

    #[test]
    fn test_resolve_tokenizer_id_google() {
        assert_eq!(resolve_tokenizer_id("gemini-pro"), "google/gemma-2b");
        assert_eq!(resolve_tokenizer_id("vertex_ai/gemini-pro"), "google/gemma-2b");
    }

    #[test]
    fn test_resolve_tokenizer_id_mistral() {
        assert_eq!(resolve_tokenizer_id("mistral-large"), "mistralai/Mistral-7B-v0.1");
        assert_eq!(resolve_tokenizer_id("codestral-latest"), "mistralai/Mistral-7B-v0.1");
    }

    #[test]
    fn test_resolve_tokenizer_id_cohere() {
        assert_eq!(
            resolve_tokenizer_id("command-r-plus"),
            "Cohere/command-r-plus-tokenizer"
        );
    }

    #[test]
    fn test_resolve_tokenizer_id_llama() {
        assert_eq!(resolve_tokenizer_id("llama-3-70b"), "meta-llama/Meta-Llama-3-8B");
        assert_eq!(
            resolve_tokenizer_id("meta-llama/Meta-Llama-3-70B"),
            "meta-llama/Meta-Llama-3-8B"
        );
    }

    #[test]
    fn test_resolve_tokenizer_id_unknown_falls_back() {
        assert_eq!(resolve_tokenizer_id("some-unknown-model"), "Xenova/gpt-4o");
    }

    /// Integration test: requires network access to download tokenizers from
    /// HuggingFace Hub. Run with `cargo test --features tokenizer -- --ignored`.
    #[test]
    #[ignore]
    fn test_count_tokens_gpt4() {
        let count = count_tokens("gpt-4o", "Hello, world!").expect("tokenization should succeed");
        assert!(count > 0, "token count should be positive");
        assert!(count < 20, "token count for short text should be small");
    }

    /// Integration test: requires network access.
    #[test]
    #[ignore]
    fn test_count_request_tokens() {
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_owned(),
            messages: vec![
                Message::System(crate::types::SystemMessage {
                    content: UserContent::Text("You are a helpful assistant.".to_owned()),
                    name: None,
                }),
                Message::User(crate::types::UserMessage {
                    content: UserContent::Text("What is 2+2?".to_owned()),
                    name: None,
                }),
            ],
            ..Default::default()
        };
        let count = count_request_tokens("gpt-4o", &req).expect("tokenization should succeed");
        // 2 messages * 4 overhead + actual tokens
        assert!(count >= 8, "should include per-message overhead");
        assert!(count < 100, "short conversation should not be many tokens");
    }
}
