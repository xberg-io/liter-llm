```crystal title="Crystal"
require "liter_llm"

client = LiterLlm.create_client("your-api-key", nil, nil, nil, nil)
response = client.chat(%({"messages":[{"content":"Hello","role":"user"}],"model":"gpt-4"}))
puts response
client.free
```