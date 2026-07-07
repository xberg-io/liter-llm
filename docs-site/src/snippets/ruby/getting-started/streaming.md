```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

client.chat_stream(
  LiterLlm::ChatCompletionRequest.new(
    model: 'openai/gpt-4o-mini',
    messages: [{ 'role' => 'user', 'content' => 'Count from 1 to 5.' }],
    stream: true
  )
) do |chunk|
  delta = chunk.choices && chunk.choices[0] && chunk.choices[0].delta
  print delta.content if delta && delta.content
end
puts
```
