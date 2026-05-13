<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("COHERE_API_KEY"))

request =
  Jason.encode!(%{
    model: "cohere/rerank-v3.5",
    query: "What is the capital of France?",
    documents: [
      "Paris is the capital of France.",
      "Berlin is the capital of Germany.",
      "London is the capital of England."
    ]
  })

{:ok, result} = LiterLlm.defaultclient_rerank_async(client, request)

for r <- result.results do
  IO.puts("Index: #{r.index}, Score: #{Float.round(r.relevance_score, 4)}")
end
```
