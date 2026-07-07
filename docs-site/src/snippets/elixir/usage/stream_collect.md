```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    model: "openai/gpt-4o-mini",
    messages: [%{role: "user", content: "Explain quantum computing briefly"}],
    stream: true
  })

{:ok, stream} = LiterLlm.defaultclient_chat_stream(client, request)

full_text =
  Enum.reduce(stream, "", fn chunk, acc ->
    delta = get_in(chunk, [:choices, Access.at(0), :delta, :content])
    if delta, do: (IO.write(delta); acc <> delta), else: acc
  end)

IO.puts("")
IO.puts("Full response length: #{String.length(full_text)} characters")
```
