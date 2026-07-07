<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

result = client.image_generate_async(
  LiterLlm::CreateImageRequest.new(
    model: 'openai/dall-e-3',
    prompt: 'A sunset over mountains',
    n: 1,
    size: '1024x1024'
  )
)

puts result.data[0].url
```
