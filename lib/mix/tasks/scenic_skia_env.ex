defmodule Mix.Tasks.ScenicSkia.Env do
  @shortdoc "Prints environment variables needed for Scenic Driver Skia"
  @moduledoc """
  Prints the environment variables needed for Scenic Driver Skia runtime.

  ## Usage

      mix scenic_skia.env

  This will print shell export statements that can be added to your
  startup script or vm.args.eex.

  ## For Nerves vm.args.eex

  Add these lines to `rel/vm.args.eex`:

      -env LD_LIBRARY_PATH <%= Scenic.Driver.Skia.Runtime.lib_path() %>
      -env GBM_BACKENDS_PATH <%= Scenic.Driver.Skia.Runtime.lib_path() %>/gbm
      -env XKB_CONFIG_ROOT <%= Scenic.Driver.Skia.Runtime.xkb_path() %>
      -env LIBINPUT_QUIRKS_DIR <%= Scenic.Driver.Skia.Runtime.libinput_quirks_path() %>
  """

  use Mix.Task

  @impl Mix.Task
  def run(_args) do
    Mix.shell().info("# Scenic Driver Skia Runtime Environment")
    Mix.shell().info("# Add to your shell startup or vm.args.eex")
    Mix.shell().info("")
    Mix.shell().info(Scenic.Driver.Skia.Runtime.shell_exports())
    Mix.shell().info("")
    Mix.shell().info("# For Nerves vm.args.eex, use:")

    Mix.shell().info(~S"""
    -env LD_LIBRARY_PATH <%= Scenic.Driver.Skia.Runtime.lib_path() %>
    -env GBM_BACKENDS_PATH <%= Scenic.Driver.Skia.Runtime.lib_path() %>/gbm
    -env XKB_CONFIG_ROOT <%= Scenic.Driver.Skia.Runtime.xkb_path() %>
    -env LIBINPUT_QUIRKS_DIR <%= Scenic.Driver.Skia.Runtime.libinput_quirks_path() %>
    """)
  end
end
