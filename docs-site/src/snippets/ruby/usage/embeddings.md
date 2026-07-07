```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

result = client.embed_async(
  LiterLlm::EmbeddingRequest.new(
    model: 'openai/text-embedding-3-small',
    input: ['The quick brown fox jumps over the lazy dog']
  )
)

embedding = result.data[0].embedding
puts "Dimensions: #{embedding.length}"
puts "First 5 values: #{embedding.first(5)}"
```
