require "../src/liter_llm"

client = LiterLlm.create_client("your-api-key", nil, nil, nil, nil)

request = %({"messages":[{"content":"Say hello","role":"user"}],"model":"gpt-4","temperature":0})
result = client.chat(request)
puts result
client.free
