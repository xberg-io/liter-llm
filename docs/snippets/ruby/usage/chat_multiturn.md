```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

messages = [
  { 'role' => 'system', 'content' => 'You are a helpful assistant.' },
  { 'role' => 'user', 'content' => 'What is the capital of France?' }
]

result = client.chat_async(
  LiterLlm::ChatCompletionRequest.new(model: 'openai/gpt-4o-mini', messages: messages)
)
answer = result.choices[0].message.content
puts "Assistant: #{answer}"

messages << { 'role' => 'assistant', 'content' => answer }
messages << { 'role' => 'user', 'content' => 'What about Germany?' }

result = client.chat_async(
  LiterLlm::ChatCompletionRequest.new(model: 'openai/gpt-4o-mini', messages: messages)
)
puts "Assistant: #{result.choices[0].message.content}"

usage = result.usage
puts "Tokens: #{usage.prompt_tokens} in, #{usage.completion_tokens} out" if usage
```
