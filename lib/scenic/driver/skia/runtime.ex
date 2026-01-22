defmodule Scenic.Driver.Skia.Runtime do
  @moduledoc """
  Runtime setup for Scenic Driver Skia on embedded systems (Nerves).

  The NIF and bundled runtime libraries use RPATH to find each other automatically,
  so no `LD_LIBRARY_PATH` configuration is needed. This module sets up auxiliary
  environment variables for GBM, XKB keyboard layouts, and libinput quirks.

  ## Automatic Setup

  The driver automatically calls `setup/0` during initialization. For most use cases,
  no manual configuration is required.

  ## How It Works

  The cross-compiled NIF has RPATH set to `$ORIGIN/../lib/<arch>`, and all bundled
  .so files have RPATH set to `$ORIGIN`. This allows the dynamic linker to find
  all dependencies without environment variables.

  Directory structure:
  ```
  priv/
  ├── native/
  │   └── libscenic_driver_skia.so  # NIF with RPATH=$ORIGIN/../lib/aarch64
  ├── lib/
  │   └── aarch64/
  │       ├── libdrm.so.2           # All libs have RPATH=$ORIGIN
  │       ├── libEGL.so.1
  │       └── ...
  ├── xkb/                          # Keyboard layouts
  └── libinput-quirks/              # Device quirks
  ```

  ## Environment Variables (set by setup/0)

  These are set at runtime for libraries that need them:
  - `GBM_BACKENDS_PATH` - Path to GBM backend plugins
  - `XKB_CONFIG_ROOT` - Path to XKB keyboard configuration
  - `LIBINPUT_QUIRKS_DIR` - Path to libinput device quirks
  - `LIBGL_DRIVERS_PATH` - Path to DRI drivers
  """

  @doc """
  Returns the path to the priv directory for this application.
  """
  def priv_path do
    :code.priv_dir(:scenic_driver_skia)
    |> to_string()
  end

  @doc """
  Returns the path to the bundled runtime libraries for the current architecture.
  """
  def lib_path do
    arch = detect_arch()
    Path.join([priv_path(), "lib", arch])
  end

  @doc """
  Returns the path to the XKB keyboard configuration.
  """
  def xkb_path do
    Path.join(priv_path(), "xkb")
  end

  @doc """
  Returns the path to libinput quirks.
  """
  def libinput_quirks_path do
    Path.join(priv_path(), "libinput-quirks")
  end

  @doc """
  Detects the current system architecture.

  Returns one of: "aarch64", "armv7", "arm", "x86_64", or "unknown"
  """
  def detect_arch do
    case :erlang.system_info(:system_architecture) |> to_string() do
      "aarch64" <> _ ->
        "aarch64"

      "arm" <> _ ->
        # Distinguish between armv6 and armv7
        if String.contains?(to_string(:erlang.system_info(:system_architecture)), "v7") do
          "armv7"
        else
          "arm"
        end

      "x86_64" <> _ ->
        "x86_64"

      _ ->
        "unknown"
    end
  end

  @doc """
  Sets up auxiliary environment variables for the Skia driver.

  **Important**: This does NOT set `LD_LIBRARY_PATH` because that must be set
  before the BEAM starts. Use `env_vars/0` to get all required variables for
  your vm.args.eex or startup script.

  This function sets:
  - `GBM_BACKENDS_PATH` - Points to GBM plugins
  - `XKB_CONFIG_ROOT` - Points to keyboard config
  - `LIBINPUT_QUIRKS_DIR` - Points to input quirks
  - `LIBGL_DRIVERS_PATH` - Points to DRI drivers

  Returns `:ok` on success.
  """
  def setup do
    lib = lib_path()

    if File.dir?(lib) do
      # GBM backend plugins
      gbm_path = Path.join(lib, "gbm")

      if File.dir?(gbm_path) do
        System.put_env("GBM_BACKENDS_PATH", gbm_path)
      end

      # DRI drivers
      dri_path = Path.join(lib, "dri")

      if File.dir?(dri_path) do
        System.put_env("LIBGL_DRIVERS_PATH", dri_path)
      end

      # XKB keyboard configuration
      xkb = xkb_path()

      if File.dir?(xkb) do
        System.put_env("XKB_CONFIG_ROOT", xkb)
      end

      # libinput quirks
      quirks = libinput_quirks_path()

      if File.dir?(quirks) do
        System.put_env("LIBINPUT_QUIRKS_DIR", quirks)
      end

      :ok
    else
      # No bundled libraries for this architecture - might be running on host
      :ok
    end
  end

  @doc """
  Returns a map of all environment variables needed for the runtime.

  Use this to generate vm.args.eex or startup scripts:

      iex> Scenic.Driver.Skia.Runtime.env_vars()
      %{
        "LD_LIBRARY_PATH" => "/path/to/priv/lib/aarch64",
        "GBM_BACKENDS_PATH" => "/path/to/priv/lib/aarch64/gbm",
        ...
      }
  """
  def env_vars do
    lib = lib_path()

    %{
      "LD_LIBRARY_PATH" => lib,
      "GBM_BACKENDS_PATH" => Path.join(lib, "gbm"),
      "LIBGL_DRIVERS_PATH" => Path.join(lib, "dri"),
      "XKB_CONFIG_ROOT" => xkb_path(),
      "LIBINPUT_QUIRKS_DIR" => libinput_quirks_path()
    }
  end

  @doc """
  Returns a string suitable for shell export statements.

  Example output:
      export LD_LIBRARY_PATH="/path/to/lib/aarch64"
      export GBM_BACKENDS_PATH="/path/to/lib/aarch64/gbm"
      ...
  """
  def shell_exports do
    env_vars()
    |> Enum.map(fn {k, v} -> ~s[export #{k}="#{v}"] end)
    |> Enum.join("\n")
  end

  @doc """
  Checks if runtime libraries are available for the current architecture.
  """
  def libraries_available? do
    File.dir?(lib_path())
  end

  @doc """
  Lists the bundled libraries for the current architecture.
  """
  def list_libraries do
    lib = lib_path()

    if File.dir?(lib) do
      lib
      |> File.ls!()
      |> Enum.filter(&String.ends_with?(&1, ".so"))
      |> Enum.sort()
    else
      []
    end
  end
end
