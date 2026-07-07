<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('BRAVE_API_KEY'))

result = client.search_async(
  LiterLlm::SearchRequest.new(
    model: 'brave/web-search',
    query: 'What is Rust programming language?',
    max_results: 5
  )
)

result.results.each do |r|
  puts "#{r.title}: #{r.url}"
end
```
