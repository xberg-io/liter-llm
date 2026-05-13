<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('COHERE_API_KEY'))

result = client.rerank_async(
  LiterLlm::RerankRequest.new(
    model: 'cohere/rerank-v3.5',
    query: 'What is the capital of France?',
    documents: [
      'Paris is the capital of France.',
      'Berlin is the capital of Germany.',
      'London is the capital of England.'
    ]
  )
)

result.results.each do |r|
  puts "Index: #{r.index}, Score: #{format('%.4f', r.relevance_score)}"
end
```
