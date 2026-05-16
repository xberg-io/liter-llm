<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'liter_llm'

# No API key needed for local providers
client = LiterLlm.create_client("", "http://localhost:11434/v1")

request = LiterLlm::ChatCompletionRequest.new(
  model: "ollama/qwen2:0.5b",
  messages: [
    LiterLlm::Message::User.new(
      LiterLlm::UserMessage.new(
        content: LiterLlm::UserContent::Text.new("Hello!"),
        name: nil
      )
    )
  ]
)

response = client.chat_async(request)
puts response.choices[0].message.content
```
