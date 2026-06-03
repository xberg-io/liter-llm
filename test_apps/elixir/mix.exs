defmodule E2eElixir.MixProject do
  use Mix.Project

  def project do
    [
      app: :e2e_elixir,
      version: "0.1.0",
      elixir: "~> 1.14",
      deps: deps()
    ]
  end

  defp deps do
    [
      {:liter_llm, "1.4.0-rc.53"},
      {:rustler_precompiled, "~> 0.9"},
      {:rustler, "~> 0.38.0", runtime: false}
    ]
  end
end
