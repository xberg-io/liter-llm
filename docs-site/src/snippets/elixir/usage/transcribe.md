<!-- snippet:compile-only -->

```elixir
{:ok, client} = LiterLlm.create_client(System.get_env("OPENAI_API_KEY"))

request =
  Jason.encode!(%{
    model: "openai/whisper-1",
    file: Base.encode64(File.read!("audio.mp3"))
  })

{:ok, result} = LiterLlm.defaultclient_transcribe_async(client, request)
IO.puts(result.text)
```
