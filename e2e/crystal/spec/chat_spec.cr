require "./spec_helper"

describe LiterLlm do
  describe "chat" do
    pending "Chat request that includes a developer role message alongside user messages"
    pending "Chat request with max_tokens=1 terminates with length finish_reason"
    pending "Chat request with system message and user message"
    pending "Chat request with temperature=0 for deterministic responses"
    pending "Chat response stopped by content filter with finish_reason of content_filter and null content"
    pending "Chat response truncated due to max_tokens limit with finish_reason of length"
    pending "Multi-turn conversation with system, user, assistant, and follow-up user messages"
    pending "Chat request that results in parallel tool calls in the response"
    pending "Chat request with response_format json_object that returns valid JSON content"
    pending "Chat request with response_format json_schema that validates the output structure"
    pending "Chat request with seed parameter for deterministic output; response includes system_fingerprint"
    pending "Chat request with custom stop sequences that terminates generation at a stop token"
    pending "Chat request with tool_choice set to required forces the model to call a tool"
    pending "Chat request with tool_choice specifying a particular function to call"
  end
end
