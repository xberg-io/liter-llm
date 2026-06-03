ExUnit.start()

# Spawn mock-server binary and set MOCK_SERVER_URL for all tests.
#
# Two execution modes:
# 1. External mode (`alef test-apps run` parent): MOCK_SERVER_URL is already set.
#    Use it as-is together with any MOCK_SERVERS / MOCK_SERVER_<FIXTURE_ID> vars
#    that the parent exported. Do NOT spawn our own server.
# 2. Standalone mode (direct `mix test` / `task elixir:smoke`): Build the
#    mock-server binary if it is missing, then spawn it, capture its URL, and
#    let it run for the duration of the test suite.
mock_server_bin = Path.expand("../../rust/target/release/mock-server", __DIR__)
fixtures_dir = Path.expand("../../../fixtures", __DIR__)

unless System.get_env("MOCK_SERVER_URL") do
  unless File.exists?(mock_server_bin) do
    # Build the mock-server from the e2e/rust/ crate that alef generated.
    manifest = Path.expand("../../rust/Cargo.toml", __DIR__)
    unless File.exists?(manifest) do
      raise "mock-server Cargo.toml not found at #{manifest}"
    end
    {_output, 0} =
      System.cmd("cargo", ["build", "--release", "--manifest-path", manifest, "--bin", "mock-server"],
        stderr_to_stdout: true)
    unless File.exists?(mock_server_bin) do
      raise "mock-server binary still missing after build: #{mock_server_bin}"
    end
  end

  port = Port.open({:spawn_executable, mock_server_bin}, [
    :binary,
    # Use a large line buffer (default 1024 truncates `MOCK_SERVERS={...}` lines for
    # fixture sets with many host-root routes, splitting them into `:noeol` chunks
    # that the prefix-match clauses below would never see).
    {:line, 65_536},
    args: [fixtures_dir]
  ])
  # Read startup lines: MOCK_SERVER_URL= then MOCK_SERVERS= (always emitted, possibly `{}`).
  # The standalone mock-server prints noisy stderr lines BEFORE the stdout sentinels;
  # selective receive ignores anything that doesn't match the two prefix patterns.
  # Each iteration only halts after the MOCK_SERVERS= line is processed.
  {url, _} =
    Enum.reduce_while(1..16, {nil, port}, fn _, {url_acc, p} ->
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
      after
        30_000 ->
          raise "mock-server startup timeout"
      end
    end)

  if url != nil do
    System.put_env("MOCK_SERVER_URL", url)
  end
end
