//! Collect the thinking blocks of a streamed assistant turn.

use super::{ChatCompletionChunk, ThinkingBlock};

/// Collects the [`ThinkingBlock`]s of one streamed assistant turn from its chunks, so they
/// can be set as [`AssistantMessage::thinking_blocks`](super::AssistantMessage::thinking_blocks)
/// of the turn when it is sent back.
///
/// It merges the fragments of each block (by `index`), and records how much text and how
/// many tool calls the stream had produced when the block started, as a reply without
/// streaming does.
///
/// Rust only: binding users merge the fragments as [`ThinkingBlock`] describes.
#[derive(Debug, Clone, Default)]
#[cfg_attr(alef, alef(skip))]
pub struct ThinkingBlockAccumulator {
    blocks: Vec<ThinkingBlock>,
    text_len: usize,
    tool_calls: usize,
}

impl ThinkingBlockAccumulator {
    /// An empty accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Take in the next chunk of the stream (only its first choice).
    pub fn push(&mut self, chunk: &ChatCompletionChunk) {
        let Some(choice) = chunk.choices.first() else {
            return;
        };
        let delta = &choice.delta;
        for fragment in delta.thinking_blocks.iter().flatten() {
            self.merge(fragment);
        }
        self.text_len += delta.content.as_deref().map_or(0, str::len);
        self.tool_calls += delta
            .tool_calls
            .iter()
            .flatten()
            .filter(|call| call.id.is_some())
            .count();
    }

    /// The thinking blocks of the turn, in stream order. A block whose opening fragment
    /// never arrived gets its type from what it holds; a block with nothing in it is dropped.
    #[must_use]
    pub fn finish(self) -> Vec<ThinkingBlock> {
        self.blocks
            .into_iter()
            .filter_map(|mut block| {
                if block.block_type.is_none() {
                    block.block_type = if block.data.is_some() {
                        Some("redacted_thinking".to_owned())
                    } else if block.thinking.is_some() || block.signature.is_some() {
                        Some("thinking".to_owned())
                    } else {
                        return None;
                    };
                }
                Some(block)
            })
            .collect()
    }

    fn merge(&mut self, fragment: &ThinkingBlock) {
        let existing = fragment
            .index
            .and_then(|index| self.blocks.iter_mut().find(|block| block.index == Some(index)));
        let Some(block) = existing else {
            self.blocks.push(ThinkingBlock {
                text_offset: u32::try_from(self.text_len).ok(),
                tool_call_offset: u32::try_from(self.tool_calls).ok(),
                ..fragment.clone()
            });
            return;
        };
        if block.block_type.is_none() {
            block.block_type.clone_from(&fragment.block_type);
        }
        for (merged, part) in [
            (&mut block.thinking, &fragment.thinking),
            (&mut block.signature, &fragment.signature),
            (&mut block.data, &fragment.data),
        ] {
            if let Some(part) = part {
                merged.get_or_insert_with(String::new).push_str(part);
            }
        }
    }
}
