<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    model: "openai/gpt-4o",
    input: "Explain quantum computing in one sentence."
  })

{:ok, result} = LiterLlm.defaultclient_create_response_async(client, request)
IO.puts("Response ID: #{result.id}")
IO.puts("Status: #{result.status}")

for item <- result.output, do: IO.puts(item.content)
```
