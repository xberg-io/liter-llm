generated_marker = ["auto-generated", " by ", "alef"] |> Enum.join("")

generated_files =
  Path.wildcard("{lib,test}/**/*.{ex,exs}")
  |> Enum.filter(fn path ->
    case File.read(path) do
      {:ok, content} -> String.contains?(content, generated_marker)
      _ -> false
    end
  end)

inputs =
  ["{mix,.formatter}.exs"] ++
    (Path.wildcard("{config,lib,test}/**/*.{ex,exs}") -- generated_files)

[
  import_deps: [:rustler],
  inputs: inputs,
  line_length: 120
]
