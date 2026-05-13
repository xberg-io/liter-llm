<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'base64'
require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

result = client.create_file_async(
  LiterLlm::CreateFileRequest.new(
    file: Base64.strict_encode64(File.binread('data.jsonl')),
    filename: 'data.jsonl',
    purpose: 'batch'
  )
)

puts "File ID: #{result.id}"
puts "Size: #{result.bytes} bytes"
```
