<!-- snippet:compile-only -->

```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('MISTRAL_API_KEY'))

result = client.ocr_async(
  LiterLlm::OcrRequest.new(
    model: 'mistral/mistral-ocr-latest',
    document: { 'type' => 'document_url', 'url' => 'https://example.com/invoice.pdf' }
  )
)

result.pages.each do |page|
  preview = page.markdown[0, 100] || ''
  puts "Page #{page.index}: #{preview}..."
end
```
