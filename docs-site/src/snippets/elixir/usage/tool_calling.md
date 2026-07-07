```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

tools = [
  %{
    type: "function",
    function: %{
      name: "get_weather",
      description: "Get the current weather for a location",
      parameters: %{
        type: "object",
        properties: %{location: %{type: "string", description: "City name"}},
        required: ["location"]
      }
    }
  }
]

request =
  Jason.encode!(%{
    model: "openai/gpt-4o-mini",
    messages: [%{role: "user", content: "What is the weather in Berlin?"}],
    tools: tools,
    tool_choice: "auto"
  })

{:ok, result} = LiterLlm.defaultclient_chat_async(client, request)

for call <- Enum.at(result.choices, 0).message.tool_calls || [] do
  IO.puts("Tool: #{call.function.name}, Args: #{call.function.arguments}")
end
```
