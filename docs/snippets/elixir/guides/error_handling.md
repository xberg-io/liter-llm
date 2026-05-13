```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    model: "openai/gpt-4o-mini",
    messages: [%{role: "user", content: "Hello"}]
  })

# Errors come back as `{:error, String.t()}` — the NIF returns the Rust
# error's Display string verbatim. Match on the prefix to identify the
# category.
case LiterLlm.defaultclient_chat_async(client, request) do
  {:ok, result} ->
    IO.puts(Enum.at(result.choices, 0).message.content)

  {:error, "authentication failed:" <> _ = reason} ->
    IO.warn("auth failed: #{reason}")

  {:error, "rate limited:" <> _ = reason} ->
    IO.warn("rate limited: #{reason}")

  {:error, "context window exceeded:" <> _ = reason} ->
    IO.warn("prompt too long: #{reason}")

  {:error, "service unavailable:" <> _ = reason} ->
    IO.warn("provider unavailable: #{reason}")

  {:error, reason} ->
    IO.warn("llm error: #{reason}")
end
```
