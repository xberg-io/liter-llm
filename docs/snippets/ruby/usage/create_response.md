<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

result = client.create_response_async(
  LiterLlm::CreateResponseRequest.new(
    model: 'openai/gpt-4o',
    input: 'Explain quantum computing in one sentence.'
  )
)

puts "Response ID: #{result.id}"
puts "Status: #{result.status}"
result.output.each { |item| puts item.content }
```
