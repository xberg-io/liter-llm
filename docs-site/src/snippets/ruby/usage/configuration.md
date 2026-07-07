```ruby
# frozen_string_literal: true

require 'liter_llm'

# Positional args: (api_key, base_url=nil, timeout_secs=nil, max_retries=nil, model_hint=nil)
client = LiterLlm.create_client(
  ENV.fetch('OPENAI_API_KEY'),
  nil,        # base_url — override provider base URL
  60,         # timeout_secs
  3,          # max_retries
  'openai'    # model_hint — pre-resolve provider
)

result = client.chat_async(
  LiterLlm::ChatCompletionRequest.new(
    model: 'openai/gpt-4o-mini',
    messages: [{ 'role' => 'user', 'content' => 'Hello!' }]
  )
)
puts result.choices[0].message.content
```
