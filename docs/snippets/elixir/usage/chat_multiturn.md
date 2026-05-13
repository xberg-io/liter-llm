```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

messages = [
  %{role: "system", content: "You are a helpful assistant."},
  %{role: "user", content: "What is the capital of France?"}
]

{:ok, result} =
  LiterLlm.defaultclient_chat_async(
    client,
    Jason.encode!(%{model: "openai/gpt-4o-mini", messages: messages})
  )

answer = Enum.at(result.choices, 0).message.content
IO.puts("Assistant: #{answer}")

messages =
  messages ++
    [
      %{role: "assistant", content: answer},
      %{role: "user", content: "What about Germany?"}
    ]

{:ok, result} =
  LiterLlm.defaultclient_chat_async(
    client,
    Jason.encode!(%{model: "openai/gpt-4o-mini", messages: messages})
  )

IO.puts("Assistant: #{Enum.at(result.choices, 0).message.content}")

if result.usage do
  IO.puts("Tokens: #{result.usage.prompt_tokens} in, #{result.usage.completion_tokens} out")
end
```
