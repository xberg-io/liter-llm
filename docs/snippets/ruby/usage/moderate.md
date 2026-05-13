<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

result = client.moderate_async(
  LiterLlm::ModerationRequest.new(
    model: 'openai/omni-moderation-latest',
    input: 'This is a test message.'
  )
)

first = result.results[0]
puts "Flagged: #{first.flagged}"
```
