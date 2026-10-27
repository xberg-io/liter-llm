//! Tool call numbers for providers that number tool calls by content block.

use crate::types::ChatCompletionChunk;

/// Gives the tool calls of one stream their OpenAI numbers.
///
/// Anthropic and Bedrock number the blocks of a reply from 0, and text and thinking blocks count
/// too. A stream parser reports a tool call with the number of its block. The first tool call of
/// an OpenAI stream has `index` 0, whatever blocks come before it.
///
/// This type keeps the blocks that started a tool call, in order. The position of a block in that
/// list is the number of its tool call.
#[derive(Debug, Default)]
pub(crate) struct ToolCallNumbering {
    blocks: Vec<u32>,
}

impl ToolCallNumbering {
    /// Replace the block number in each tool call of `chunk` with the tool call number.
    ///
    /// A call with an `id` starts a tool call. A call without one carries an argument delta,
    /// and gets the number of the block that started it.
    pub(crate) fn renumber(&mut self, chunk: &mut ChatCompletionChunk) {
        for choice in &mut chunk.choices {
            for call in choice.delta.tool_calls.iter_mut().flatten() {
                let block = call.index;
                let started = if call.id.is_some() {
                    None
                } else {
                    self.blocks.iter().rposition(|started| *started == block)
                };
                let position = started.unwrap_or_else(|| {
                    self.blocks.push(block);
                    self.blocks.len() - 1
                });
                call.index = u32::try_from(position).unwrap_or(u32::MAX);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(block: u32, id: Option<&str>) -> ChatCompletionChunk {
        serde_json::from_value(serde_json::json!({
            "id": "",
            "object": "chat.completion.chunk",
            "created": 0,
            "model": "",
            "choices": [{
                "index": 0,
                "delta": {"tool_calls": [{"index": block, "id": id, "function": {"arguments": "{}"}}]},
                "finish_reason": null
            }]
        }))
        .expect("chunk")
    }

    fn numbered(numbering: &mut ToolCallNumbering, block: u32, id: Option<&str>) -> u32 {
        let mut chunk = chunk(block, id);
        numbering.renumber(&mut chunk);
        chunk.choices[0].delta.tool_calls.as_ref().expect("tool calls")[0].index
    }

    #[test]
    fn a_delta_without_a_start_gets_the_next_number_and_keeps_it() {
        let mut numbering = ToolCallNumbering::default();
        assert_eq!(numbered(&mut numbering, 3, None), 0);
        assert_eq!(numbered(&mut numbering, 3, None), 0);
        assert_eq!(numbered(&mut numbering, 5, Some("b")), 1);
    }

    #[test]
    fn a_block_that_starts_again_gets_a_new_number() {
        let mut numbering = ToolCallNumbering::default();
        assert_eq!(numbered(&mut numbering, 1, Some("a")), 0);
        assert_eq!(numbered(&mut numbering, 1, Some("b")), 1);
        assert_eq!(numbered(&mut numbering, 1, None), 1);
    }

    #[test]
    fn a_chunk_without_tool_calls_is_unchanged() {
        let mut chunk = chunk(4, Some("a"));
        chunk.choices[0].delta.tool_calls = None;
        let before = chunk.clone();
        ToolCallNumbering::default().renumber(&mut chunk);
        assert_eq!(chunk, before);
    }
}
