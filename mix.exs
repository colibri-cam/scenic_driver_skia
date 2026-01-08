defmodule ScenicDriverSkia.MixProject do
  use Mix.Project

  def project do
    [
      app: :scenic_driver_skia,
      version: "0.1.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      rustler_crates: rustler_crates(),
      compilers: Mix.compilers()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.37"}
    ]
  end

  defp rustler_crates do
    [
      scenic_driver_skia: [
        path: "native/scenic_driver_skia",
        mode: rust_mode(),
        cargo: System.get_env("CARGO", "cargo")
      ]
    ]
  end

  defp rust_mode do
    if Mix.env() == :prod, do: :release, else: :debug
  end
end
