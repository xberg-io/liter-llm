```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

full_text = +''
client.chat_stream(
  LiterLlm::ChatCompletionRequest.new(
    model: 'openai/gpt-4o-mini',
    messages: [{ 'role' => 'user', 'content' => 'Explain quantum computing briefly' }],
    stream: true
  )
) do |chunk|
  delta = chunk.choices && chunk.choices[0] && chunk.choices[0].delta
  if delta && delta.content
    full_text << delta.content
    print delta.content
  end
end
puts
puts "Full response length: #{full_text.length} characters"
```
