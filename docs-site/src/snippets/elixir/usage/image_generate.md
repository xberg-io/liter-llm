<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    model: "openai/dall-e-3",
    prompt: "A sunset over mountains",
    n: 1,
    size: "1024x1024"
  })

{:ok, result} = LiterLlm.defaultclient_image_generate_async(client, request)
IO.puts(Enum.at(result.data, 0).url)
```
