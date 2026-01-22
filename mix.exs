defmodule ScenicDriverSkia.MixProject do
  use Mix.Project

  def project do
    [
      app: :scenic_driver_skia,
      version: "0.1.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description:
        "A Scenic GUI driver that renders through Skia with Wayland, DRM, and Raster backends",
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
      {:scenic, path: "../scenic", override: true},
      {:scenic_clock, "~> 0.11.0"}
    ]
  end

  defp package do
    [
      files: ~w(
        lib
        native
        priv/lib
        priv/xkb
        priv/libinput-quirks
        nerves-clang-wrapper.sh
        nerves-clangxx-wrapper.sh
        mix.exs
        README.md
        LICENSE
      ),
      licenses: ["Apache-2.0"],
      links: %{
        "GitHub" => "https://github.com/ScenicFramework/scenic_driver_skia"
      }
    ]
  end

  @nerves_rust_target_triple_mapping %{
    "armv6-nerves-linux-gnueabihf" => "arm-unknown-linux-gnueabihf",
    "armv7-nerves-linux-gnueabihf" => "armv7-unknown-linux-gnueabihf",
    "aarch64-nerves-linux-gnu" => "aarch64-unknown-linux-gnu",
    "x86_64-nerves-linux-musl" => "x86_64-unknown-linux-musl"
  }

  defp configure_rustler_cross_compile(nil), do: []

  defp configure_rustler_cross_compile(sysroot) do
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

      # Get the toolchain prefix (e.g., "aarch64-nerves-linux-gnu-")
      toolchain_prefix =
        cc
        |> Path.basename()
        |> String.replace("gcc", "")

      # Get the toolchain directory
      toolchain_dir = Path.dirname(cc)

      ar = Path.join(toolchain_dir, "#{toolchain_prefix}ar")

      # Use clang for skia-bindings (which requires clang for cross-compilation)
      # Point it to the Nerves sysroot and toolchain
      clang_flags = "--target=#{target_triple} --sysroot=#{sysroot} -I#{sysroot}/usr/include"

      # Get path to wrapper scripts
      wrapper_dir = __DIR__
      clang_wrapper = Path.join(wrapper_dir, "nerves-clang-wrapper.sh")
      clangxx_wrapper = Path.join(wrapper_dir, "nerves-clangxx-wrapper.sh")

      # Path to bundled sysroot for linking (contains Mesa, wayland, etc.)
      arch =
        case target_triple do
          "aarch64-unknown-linux-gnu" -> "aarch64"
          "armv7-unknown-linux-gnueabihf" -> "armv7"
          "arm-unknown-linux-gnueabihf" -> "arm"
          "x86_64-unknown-linux-musl" -> "x86_64"
          _ -> nil
        end

      bundled_sysroot =
        if arch do
          Path.join([wrapper_dir, "builder", "sysroot", arch, "usr", "lib"])
        end

      # Add bundled sysroot to linker search path for libraries not in Nerves staging
      rustflags =
        if bundled_sysroot && File.dir?(bundled_sysroot) do
          "-L #{bundled_sysroot}"
        else
          ""
        end

      [
        target: target_triple,
        env: [
          {"CARGO_TARGET_#{upcase_target_triple}_LINKER", cc},
          {"CARGO_TARGET_#{upcase_target_triple}_RUSTFLAGS", rustflags},
          # Override CC/CXX to use clang wrappers (skia-bindings requires clang)
          {"CC", "#{clang_wrapper} #{clang_flags}"},
          {"CXX", "#{clangxx_wrapper} #{clang_flags}"},
          {"AR_#{target_triple}", ar},
          {"CFLAGS", clang_flags},
          {"CXXFLAGS", clang_flags},
          {"PKG_CONFIG_SYSROOT_DIR", sysroot},
          {"PKG_CONFIG_ALLOW_CROSS", "1"}
        ]
      ]
    else
      []
    end
  end
end
