```elixir
# LiterLlm.create_client(api_key, base_url \\ nil, timeout_secs \\ nil,
#                        max_retries \\ nil, model_hint \\ nil)
{:ok, client} =
  LiterLlm.create_client(
    System.get_env("OPENAI_API_KEY"),
    nil,        # base_url — override provider base URL
    60,         # timeout_secs
    3,          # max_retries
    "openai"    # model_hint — pre-resolve provider
  )

request =
  Jason.encode!(%{
    model: "openai/gpt-4o-mini",
    messages: [%{role: "user", content: "Hello!"}]
  })

{:ok, result} = LiterLlm.defaultclient_chat_async(client, request)
IO.puts(Enum.at(result.choices, 0).message.content)
```
