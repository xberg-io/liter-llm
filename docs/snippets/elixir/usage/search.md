<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("BRAVE_API_KEY"))

request =
  Jason.encode!(%{
    model: "brave/web-search",
    query: "What is Rust programming language?",
    max_results: 5
  })

{:ok, result} = LiterLlm.defaultclient_search_async(client, request)

for r <- result.results do
  IO.puts("#{r.title}: #{r.url}")
end
```
