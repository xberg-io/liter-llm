```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    model: "openai/text-embedding-3-small",
    input: ["The quick brown fox jumps over the lazy dog"]
  })

{:ok, result} = LiterLlm.defaultclient_embed_async(client, request)
embedding = Enum.at(result.data, 0).embedding
IO.puts("Dimensions: #{length(embedding)}")
IO.puts("First 5 values: #{inspect(Enum.take(embedding, 5))}")
```
