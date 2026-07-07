```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

begin
  result = client.chat_async(
    LiterLlm::ChatCompletionRequest.new(
      model: 'openai/gpt-4o-mini',
      messages: [{ 'role' => 'user', 'content' => 'Hello' }]
    )
  )
  puts result.choices[0].message.content
rescue RuntimeError => e
  # The Ruby binding raises plain RuntimeError. The message is the Rust
  # error's Display string — branch on its prefix to identify the category.
  case e.message
  when /\Arate limited:/            then warn "rate limited: #{e.message}"
  when /\Aauthentication failed:/   then warn "auth failed: #{e.message}"
  when /\Acontext window exceeded:/ then warn "prompt too long: #{e.message}"
  when /\Aservice unavailable:/     then warn "provider unavailable: #{e.message}"
  else warn "llm error: #{e.message}"
  end
end
```
