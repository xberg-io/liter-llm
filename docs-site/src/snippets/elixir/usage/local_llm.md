<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client("")

request =
  Jason.encode!(%{
    "model" => "ollama/qwen2:0.5b",
    "messages" => [
      %{
        "role" => "user",
        "content" => "Hello!"
      }
    ]
  })

{:ok, response} = LiterLlm.defaultclient_chat(client, request, "http://localhost:11434/v1")
IO.puts(Enum.at(response.choices, 0).message.content)
```
