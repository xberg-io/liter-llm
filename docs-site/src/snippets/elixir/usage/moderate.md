<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    model: "openai/omni-moderation-latest",
    input: "This is a test message."
  })

{:ok, result} = LiterLlm.defaultclient_moderate_async(client, request)
first = Enum.at(result.results, 0)
IO.puts("Flagged: #{first.flagged}")
```
