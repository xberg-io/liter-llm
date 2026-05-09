ExUnit.start()

# Best-effort load of liter-llm/.env so smoke tests gated on
# OPENAI_API_KEY/ANTHROPIC_API_KEY/GEMINI_API_KEY pick them up automatically.
env_file = Path.expand("../../../.env", __DIR__)

if File.exists?(env_file) do
  env_file
  |> File.read!()
  |> String.split("\n", trim: true)
  |> Enum.each(fn line ->
    line = String.trim(line)

    unless line == "" or String.starts_with?(line, "#") do
      case String.split(line, "=", parts: 2) do
        [key, value] ->
          key = String.trim(key)
          value = value |> String.trim() |> String.trim("\"") |> String.trim("'")

          if System.get_env(key) in [nil, ""] do
            System.put_env(key, value)
          end

        _ ->
          :ok
      end
    end
  end)
end

# Spawn mock-server binary and set MOCK_SERVER_URL for all tests.
mock_server_bin = Path.expand("../../rust/target/release/mock-server", __DIR__)
fixtures_dir = Path.expand("../../../fixtures", __DIR__)

if File.exists?(mock_server_bin) do
  port = Port.open({:spawn_executable, mock_server_bin}, [
    :binary,
    :line,
    args: [fixtures_dir]
  ])
  receive do
    {^port, {:data, {:eol, "MOCK_SERVER_URL=" <> url}}} ->
      System.put_env("MOCK_SERVER_URL", url)
  after
    30_000 ->
      raise "mock-server startup timeout"
  end
end
