defmodule ScenicDriverSkia.MixProject do
  use Mix.Project

  def project do
    [
      app: :scenic_driver_skia,
      version: "0.1.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      rustler_opts: configure_rustler_cross_compile(System.get_env("NERVES_SDK_SYSROOT"))
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.37"},
      {:scenic, git: "https://github.com/ScenicFramework/scenic.git", override: true},
      {:scenic_clock, "~> 0.11.0"}
    ]
  end

  @nerves_rust_target_triple_mapping %{
    "armv6-nerves-linux-gnueabihf" => "arm-unknown-linux-gnueabihf",
    "armv7-nerves-linux-gnueabihf" => "armv7-unknown-linux-gnueabihf",
    "aarch64-nerves-linux-gnu" => "aarch64-unknown-linux-gnu",
    "x86_64-nerves-linux-musl" => "x86_64-unknown-linux-musl"
  }

  defp configure_rustler_cross_compile(nil), do: []

  defp configure_rustler_cross_compile(_sysroot) do
    cc = System.get_env("CC")

    if cc do
      target_triple =
        cc
        |> Path.basename()
        |> String.split("-")
        |> Enum.drop(-1)
        |> Enum.join("-")
        |> then(&Map.get(@nerves_rust_target_triple_mapping, &1))

      upcase_target_triple =
        target_triple
        |> String.upcase()
        |> String.replace("-", "_")

      [
        target: target_triple,
        env: [
          {"CARGO_TARGET_#{upcase_target_triple}_LINKER", cc}
        ]
      ]
    else
      []
    end
  end
end
