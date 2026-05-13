<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'base64'
require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

result = client.transcribe_async(
  LiterLlm::CreateTranscriptionRequest.new(
    model: 'openai/whisper-1',
    file: Base64.strict_encode64(File.binread('audio.mp3'))
  )
)

puts result.text
```
