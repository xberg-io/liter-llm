```ruby
# frozen_string_literal: true

require 'liter_llm'

client = LiterLlm.create_client(ENV.fetch('OPENAI_API_KEY'))

tools = [
  {
    'type' => 'function',
    'function' => {
      'name' => 'get_weather',
      'description' => 'Get the current weather for a location',
      'parameters' => {
        'type' => 'object',
        'properties' => { 'location' => { 'type' => 'string', 'description' => 'City name' } },
        'required' => ['location']
      }
    }
  }
]

result = client.chat_async(
  LiterLlm::ChatCompletionRequest.new(
    model: 'openai/gpt-4o-mini',
    messages: [{ 'role' => 'user', 'content' => 'What is the weather in Berlin?' }],
    tools: tools,
    tool_choice: 'auto'
  )
)

(result.choices[0].message.tool_calls || []).each do |call|
  puts "Tool: #{call.function.name}, Args: #{call.function.arguments}"
end
```
