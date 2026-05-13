```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    model: "openai/gpt-4o-mini",
    messages: [%{role: "user", content: "Count from 1 to 5."}],
    stream: true
  })

{:ok, stream} = LiterLlm.defaultclient_chat_stream(client, request)

Enum.each(stream, fn chunk ->
  content = get_in(chunk, [:choices, Access.at(0), :delta, :content])
  if content, do: IO.write(content)
end)

IO.puts("")
```
