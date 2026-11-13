//! Thinking blocks of an assistant turn, sent back where the reply had them (#474).

use serde_json::{Value, json};

use super::request::is_blank;
use crate::error::{LiterLlmError, Result};

/// A thinking block to send back, with where the reply had it.
pub(in crate::provider) struct ReplayedThinking {
    block: Value,
    /// Where the block goes among the turn's blocks: its `index`, or for a block without
    /// one, its position among the thinking blocks.
    order: usize,
    text_offset: Option<usize>,
    tool_call_offset: Option<usize>,
}

/// Validate `thinking_blocks` and build the Anthropic block of each, in `index` order.
/// Anthropic rejects a thinking block without its signature, so a malformed block fails
/// here with the field named instead of as a provider 400.
pub(in crate::provider) fn replayed_thinking_blocks(thinking_blocks: Option<&Value>) -> Result<Vec<ReplayedThinking>> {
    let bad_request = |message: String| LiterLlmError::BadRequest {
        message: format!("assistant message {message}"),
        status: 400,
    };
    let blocks = match thinking_blocks {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(blocks)) => blocks,
        Some(_) => return Err(bad_request("thinking_blocks must be an array".to_owned())),
    };
    let mut replayed = Vec::with_capacity(blocks.len());
    for (position, thinking_block) in blocks.iter().enumerate() {
        let field = |name: &str| {
            thinking_block
                .get(name)
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
        };
        let offset = |name: &str| {
            thinking_block
                .get(name)
                .and_then(Value::as_u64)
                .and_then(|v| usize::try_from(v).ok())
        };
        let block = match field("type") {
            Some("thinking") => {
                let signature = field("signature").ok_or_else(|| {
                    bad_request(format!(
                        "thinking_blocks[{position}].signature is required for a thinking block"
                    ))
                })?;
                let thinking = thinking_block.get("thinking").and_then(Value::as_str).unwrap_or("");
                json!({"type": "thinking", "thinking": thinking, "signature": signature})
            }
            Some("redacted_thinking") => {
                let data = field("data").ok_or_else(|| {
                    bad_request(format!(
                        "thinking_blocks[{position}].data is required for a redacted_thinking block"
                    ))
                })?;
                json!({"type": "redacted_thinking", "data": data})
            }
            _ => {
                return Err(bad_request(format!(
                    "thinking_blocks[{position}].type must be \"thinking\" or \"redacted_thinking\""
                )));
            }
        };
        replayed.push(ReplayedThinking {
            block,
            order: offset("index").unwrap_or(position),
            text_offset: offset("text_offset"),
            tool_call_offset: offset("tool_call_offset"),
        });
    }
    // ~keep Stable, so blocks with the same order keep the order they were given in.
    replayed.sort_by_key(|block| block.order);
    Ok(replayed)
}

/// The turn rebuilt from its text, tool calls and thinking blocks, each thinking block
/// between the text and tool calls that came before and after it in the reply. `None`
/// when a block has no offsets or they do not fit the text (it was edited).
pub(in crate::provider) fn positioned_blocks(
    text: &str,
    cache_control: Option<&Value>,
    tool_uses: &[Value],
    thinking: &[ReplayedThinking],
) -> Option<Vec<Value>> {
    if thinking.is_empty() {
        return None;
    }
    let mut blocks: Vec<Value> = Vec::new();
    let (mut text_at, mut tool_at) = (0usize, 0usize);
    for replayed in thinking {
        let text_offset = replayed.text_offset?.max(text_at);
        let tool_call_offset = replayed.tool_call_offset?.min(tool_uses.len()).max(tool_at);
        push_text_block(&mut blocks, text.get(text_at..text_offset)?);
        blocks.extend(tool_uses.get(tool_at..tool_call_offset)?.iter().cloned());
        blocks.push(replayed.block.clone());
        (text_at, tool_at) = (text_offset, tool_call_offset);
    }
    push_text_block(&mut blocks, text.get(text_at..)?);
    blocks.extend(tool_uses.get(tool_at..)?.iter().cloned());

    if let (Some(cc), Some(last_text)) = (cache_control, blocks.iter_mut().rev().find(|b| b["type"] == "text")) {
        last_text["cache_control"] = cc.clone();
    }
    Some(blocks)
}

/// Push a `text` block unless `text` is blank (Anthropic rejects blank text blocks).
fn push_text_block(blocks: &mut Vec<Value>, text: &str) {
    if !is_blank(text) {
        blocks.push(json!({"type": "text", "text": text}));
    }
}

/// Put each thinking block back at its `order`, after any block inserted before it, so
/// blocks that share an order keep theirs.
pub(in crate::provider) fn insert_by_index(blocks: &mut Vec<Value>, thinking: Vec<ReplayedThinking>) {
    let mut next = 0usize;
    for replayed in thinking {
        let at = replayed.order.max(next).min(blocks.len());
        blocks.insert(at, replayed.block);
        next = at + 1;
    }
}
