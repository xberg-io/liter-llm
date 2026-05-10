ExUnit.start()

# Spawn mock-server binary and set MOCK_SERVER_URL for all tests.
mock_server_bin = Path.expand("../../rust/target/release/mock-server", __DIR__)
fixtures_dir = Path.expand("../../../fixtures", __DIR__)

if File.exists?(mock_server_bin) do
  port = Port.open({:spawn_executable, mock_server_bin}, [
    :binary,
    :line,
    args: [fixtures_dir]
  ])
  # Read startup lines: MOCK_SERVER_URL= then optional MOCK_SERVERS=.
  {url, _} =
    Enum.reduce_while(1..8, {nil, port}, fn _, {url_acc, p} ->
      receive do
        {^p, {:data, {:eol, "MOCK_SERVER_URL=" <> u}}} ->
          {:cont, {u, p}}

        {^p, {:data, {:eol, "MOCK_SERVERS=" <> json_val}}} ->
          System.put_env("MOCK_SERVERS", json_val)
          case Jason.decode(json_val) do
            {:ok, servers} ->
              Enum.each(servers, fn {fid, furl} ->
                System.put_env("MOCK_SERVER_#{String.upcase(fid)}", furl)
              end)

            _ ->
              :ok
          end

          {:halt, {url_acc, p}}

        {^p, _other} when url_acc != nil ->
          {:halt, {url_acc, p}}
      after
        30_000 ->
          raise "mock-server startup timeout"
      end
    end)

  if url != nil do
    System.put_env("MOCK_SERVER_URL", url)
  end
end
