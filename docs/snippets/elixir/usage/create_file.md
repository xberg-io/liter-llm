<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    file: Base.encode64(File.read!("data.jsonl")),
    filename: "data.jsonl",
    purpose: "batch"
  })

{:ok, result} = LiterLlm.defaultclient_create_file_async(client, request)
IO.puts("File ID: #{result.id}")
IO.puts("Size: #{result.bytes} bytes")
```
