require "./spec_helper"

describe LiterLlm do
  describe "smoke" do
    pending "Basic chat completion via the Anthropic provider (claude-3-5-sonnet-20241022) with system and user messages"
    pending "Chat completion via Azure OpenAI with the azure/ prefix for provider routing — verifies the prefix is stripped before dispatching and the response is normalised to the standard OpenAI chat completion shape"
    pending "Embedding request via Azure OpenAI using the azure/ provider prefix — response follows the standard OpenAI embeddings shape that Azure returns unchanged"
    pending "Basic chat completion with a single user message"
    pending "Basic embedding request for a single input string"
    pending "List available models from the API"
    pending "Basic chat completion via the AWS Bedrock provider using the bedrock/ prefix for routing — verifies the prefix is stripped before dispatching and the Converse API response is normalised to the standard OpenAI chat completion shape"
    pending "Basic chat completion via the GitHub Copilot provider (gpt-4o) with a single user message"
    it "Chat completion against local Ollama with qwen2:0.5b model" do
            __client = LiterLlm.create_client("test-key", "", 0_u64, 0_u32, "")
      __result = __client.chat(LiterLlm::ChatCompletionRequest.from_json("{}"))
      # TODO: unsupported assertion `not_error`
      # TODO: unsupported assertion `not_error`
      __result.choices.to_s.should_not be_empty
    end
    it "List models from local Ollama instance" do
            __client = LiterLlm.create_client("test-key", "", 0_u64, 0_u32, "")
      __result = __client.list_models()
      # TODO: unsupported assertion `not_error`
      # TODO: unsupported assertion `not_error`
      __result.try(&.data).to_s.size.should be >=(1)
    end
    pending "Test in-memory caching by sending identical requests twice"
    pending "Basic chat completion against real Anthropic API"
    pending "Basic chat completion against real Google Gemini API"
    pending "Basic chat completion against real OpenAI API"
    pending "Embeddings request against real OpenAI API"
    pending "List models against real OpenAI API"
    pending "Test provider routing by sending requests to OpenAI and Anthropic"
    pending "Basic chat completion via the Google Vertex AI provider using the vertex_ai/ prefix for routing — verifies the prefix is stripped before dispatching and the Gemini response is normalised to the standard OpenAI chat completion shape"
    pending "Embedding request via Google Vertex AI using the vertex_ai/ provider prefix and the text-embedding-005 model — response follows the standard OpenAI embeddings shape"
  end
end
