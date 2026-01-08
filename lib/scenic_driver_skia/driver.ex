defmodule ScenicDriverSkia.Driver do
  @moduledoc """
  Minimal Scenic driver that logs viewport callbacks.

  This is scaffolding for the Skia-backed driver. Configure it on a ViewPort:

      drivers: [
        [
        module: ScenicDriverSkia.Driver,
          name: :skia_driver
        ]
      ]
  """

  use Scenic.Driver
  require Logger

  @window_schema [
    title: [type: :string, default: "Scenic Window"],
    resizeable: [type: :boolean, default: false]
  ]

  @opts_schema [
    backend: [type: {:or, [:atom, :string]}, default: :wayland],
    debug: [type: :boolean, default: false],
    window: [type: :keyword_list, keys: @window_schema, default: []]
  ]

  @impl Scenic.Driver
  def validate_opts(opts) do
    with {:ok, opts} <- NimbleOptions.validate(opts, @opts_schema) do
      {:ok, Keyword.put(opts, :backend, normalize_backend(opts[:backend]))}
    end
  end

  @impl Scenic.Driver
  def init(driver, opts) do
    Logger.info("ScenicDriverSkia.Driver init: #{inspect(opts)}")
    {:ok, assign(driver, :opts, opts)}
  end

  @impl Scenic.Driver
  def reset_scene(driver) do
    Logger.debug("ScenicDriverSkia.Driver reset_scene")
    {:ok, driver}
  end

  @impl Scenic.Driver
  def request_input(input, driver) do
    Logger.debug("ScenicDriverSkia.Driver request_input: #{inspect(input)}")
    {:ok, driver}
  end

  @impl Scenic.Driver
  def update_scene(script_ids, driver) do
    Logger.debug("ScenicDriverSkia.Driver update_scene: #{inspect(script_ids)}")
    {:ok, driver}
  end

  @impl Scenic.Driver
  def del_scripts(script_ids, driver) do
    Logger.debug("ScenicDriverSkia.Driver del_scripts: #{inspect(script_ids)}")
    {:ok, driver}
  end

  @impl Scenic.Driver
  def clear_color(color, driver) do
    Logger.debug("ScenicDriverSkia.Driver clear_color: #{inspect(color)}")
    {:ok, driver}
  end

  defp normalize_backend(backend) do
    backend
    |> to_string()
    |> String.downcase()
    |> case do
      "kms" -> "drm"
      other -> other
    end
  end
end
