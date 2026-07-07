<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    input_file_id: "file-abc123",
    endpoint: "/v1/chat/completions",
    completion_window: "24h"
  })

{:ok, result} = LiterLlm.defaultclient_create_batch_async(client, request)
IO.puts("Batch ID: #{result.id}")
IO.puts("Status: #{result.status}")
```
