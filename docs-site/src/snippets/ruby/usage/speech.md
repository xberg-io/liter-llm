<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

audio_bytes = client.speech_async(
  LiterLlm::CreateSpeechRequest.new(
    model: 'openai/tts-1',
    input: 'Hello, world!',
    voice: 'alloy'
  )
)

File.binwrite('output.mp3', audio_bytes.pack('C*'))
puts "Wrote #{audio_bytes.length} bytes to output.mp3"
```
