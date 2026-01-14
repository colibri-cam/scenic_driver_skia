defmodule Scenic.Driver.Skia do
  @moduledoc """
  Scenic driver that renders through Skia and exposes a small wrapper API.

  Configure it on a ViewPort:

      drivers: [
        [
          module: Scenic.Driver.Skia,
          name: :skia_driver,
          backend: :raster
        ]
      ]
  """

  use Scenic.Driver
  require Logger
  import Bitwise, only: [|||: 2]
  alias Scenic.Driver

  alias Scenic.Driver.Skia.Native
  alias Scenic.{Script, ViewPort}

  @window_schema [
    title: [type: :string, default: "Scenic Window"],
    resizeable: [type: :boolean, default: false]
  ]

  @opts_schema [
    backend: [type: {:or, [:atom, :string]}, default: :wayland],
    debug: [type: :boolean, default: false],
    raster_output: [type: :string],
    window: [type: :keyword_list, keys: @window_schema, default: []]
  ]

  @input_mask_key 0x01
  @input_mask_codepoint 0x02
  @input_mask_cursor_pos 0x04
  @input_mask_cursor_button 0x08
  @input_mask_cursor_scroll 0x10
  @input_mask_viewport 0x20
  @input_mask_all 0x3F
  @impl Scenic.Driver
  def validate_opts(opts) do
    with {:ok, opts} <- NimbleOptions.validate(opts, @opts_schema) do
      {:ok, Keyword.put(opts, :backend, normalize_backend(opts[:backend]))}
    end
  end

  @doc """
  Scenic callback invoked when a driver is started as part of a ViewPort.

  Use `start/0` or `start/1` only for manual renderer startup outside the ViewPort lifecycle.
  """
  @impl Scenic.Driver
  def init(driver, opts) do
    Logger.info("Scenic.Driver.Skia init: #{inspect(opts)}")

    viewport_size = normalize_viewport_size(driver.viewport.size)

    case Native.start(opts[:backend], viewport_size) do
      :ok ->
        maybe_set_raster_output(opts)
        maybe_set_input_target(self())
        {:ok, assign(driver, opts: opts, update_count: 0, input_mask: 0)}

      {:ok, _} ->
        maybe_set_raster_output(opts)
        maybe_set_input_target(self())
        {:ok, assign(driver, opts: opts, update_count: 0, input_mask: 0)}

      {:error, reason} ->
        {:stop, reason}

      other ->
        {:stop, {:unexpected_start_result, other}}
    end
  end

  @impl Scenic.Driver
  def reset_scene(driver) do
    Logger.debug("Scenic.Driver.Skia reset_scene")
    _ = Native.reset_scene()
    {:ok, driver}
  end

  @impl Scenic.Driver
  def request_input(input, driver) do
    mask = input_mask_from_request(input)

    case Native.set_input_mask(mask) do
      :ok -> :ok
      {:ok, _} -> :ok
      {:error, reason} -> Logger.warning("set_input_mask failed: #{inspect(reason)}")
      other -> Logger.warning("set_input_mask returned #{inspect(other)}")
    end

    if mask == 0 do
      maybe_set_input_target(nil)
    else
      maybe_set_input_target(self())
    end

    {:ok, assign(driver, :input_mask, mask)}
  end

  @impl GenServer
  def handle_info(:input_ready, driver) do
    events =
      case Native.drain_input_events() do
        {:ok, list} when is_list(list) ->
          list

        list when is_list(list) ->
          list

        {:error, reason} ->
          Logger.warning("drain_input_events failed: #{inspect(reason)}")
          []

        other ->
          Logger.warning("drain_input_events returned #{inspect(other)}")
          []
      end

    driver =
      Enum.reduce(events, driver, fn event, acc ->
        Driver.send_input(acc, event)
      end)

    {:noreply, driver}
  end

  @impl Scenic.Driver
  def update_scene(script_ids, %{viewport: vp} = driver) do
    Logger.debug("Scenic.Driver.Skia update_scene: #{inspect(script_ids)}")

    updates =
      Enum.reduce(script_ids, [], fn id, acc ->
        case ViewPort.get_script(vp, id) do
          {:ok, script} ->
            binary =
              script
              |> Script.serialize()
              |> IO.iodata_to_binary()

            [{to_string(id), binary} | acc]

          _ ->
            acc
        end
      end)

    case updates do
      [] ->
        :ok

      _ ->
        Native.submit_scripts(updates)
        |> case do
          :ok -> :ok
          {:ok, _} -> :ok
          {:error, reason} -> Logger.warning("submit_scripts failed: #{inspect(reason)}")
          other -> Logger.warning("submit_scripts returned #{inspect(other)}")
        end
    end

    driver = maybe_log_script_count(driver)
    {:ok, driver}
  end

  defp normalize_viewport_size(nil), do: nil

  defp normalize_viewport_size({width, height}) do
    {round(width), round(height)}
  end

  @impl Scenic.Driver
  def del_scripts(script_ids, driver) do
    Logger.debug("Scenic.Driver.Skia del_scripts: #{inspect(script_ids)}")
    Enum.each(script_ids, &Native.del_script(to_string(&1)))
    {:ok, driver}
  end

  @impl Scenic.Driver
  def clear_color(color, driver) do
    Logger.debug("Scenic.Driver.Skia clear_color: #{inspect(color)}")
    {:color_rgba, {r, g, b, a}} = Scenic.Color.to_rgba(color)
    _ = Native.set_clear_color({r, g, b, a})
    {:ok, driver}
  end

  @impl GenServer
  def terminate(_reason, _driver) do
    _ = Native.stop()
    :ok
  end

  @doc """
  Start the renderer manually with the provided backend.

  This bypasses the Scenic ViewPort lifecycle and is intended for demos/tests.
  Accepts `:wayland` or `:drm`.
  """
  @spec start() :: :ok | {:error, term()}
  def start, do: start(:wayland)

  @doc """
  Start the renderer manually with the provided backend.

  This bypasses the Scenic ViewPort lifecycle and is intended for demos/tests.
  Accepts `:wayland` or `:drm`.
  """
  @spec start(:wayland | :drm | String.t()) :: :ok | {:error, term()}
  def start(backend) when is_atom(backend) or is_binary(backend) do
    backend
    |> normalize_backend()
    |> Native.start(nil)
    |> normalize_start_result()
  end

  @doc """
  Stop the renderer if it is running.
  """
  @spec stop() :: :ok | {:error, term()}
  def stop do
    Native.stop()
  end

  @doc """
  Update the text rendered by the driver.
  """
  @spec set_text(String.t()) :: :ok | {:error, term()}
  def set_text(text) when is_binary(text) do
    Native.set_text(text)
  end

  defp maybe_set_raster_output(opts) do
    case opts[:raster_output] do
      nil ->
        :ok

      path ->
        case Native.set_raster_output(path) do
          :ok -> :ok
          {:ok, _} -> :ok
          {:error, reason} -> Logger.warning("set_raster_output failed: #{inspect(reason)}")
          other -> Logger.warning("set_raster_output returned #{inspect(other)}")
        end
    end
  end

  defp normalize_backend(backend) do
    backend
    |> to_string()
    |> String.downcase()
  end

  defp input_mask_from_request(:all), do: @input_mask_all

  defp input_mask_from_request(request) when is_list(request) do
    Enum.reduce(request, 0, fn
      :key, mask -> mask ||| @input_mask_key
      :codepoint, mask -> mask ||| @input_mask_codepoint
      :cursor_pos, mask -> mask ||| @input_mask_cursor_pos
      :cursor_button, mask -> mask ||| @input_mask_cursor_button
      :cursor_scroll, mask -> mask ||| @input_mask_cursor_scroll
      :viewport, mask -> mask ||| @input_mask_viewport
      _, mask -> mask
    end)
  end

  defp input_mask_from_request(_), do: 0

  defp normalize_start_result(:ok), do: :ok
  defp normalize_start_result({:ok, _}), do: :ok
  defp normalize_start_result({:error, _} = error), do: error
  defp normalize_start_result(other), do: {:error, {:unexpected_result, other}}

  defp maybe_set_input_target(pid) do
    case Native.set_input_target(pid) do
      :ok -> :ok
      {:ok, _} -> :ok
      {:error, reason} -> Logger.warning("set_input_target failed: #{inspect(reason)}")
      other -> Logger.warning("set_input_target returned #{inspect(other)}")
    end
  end

  defp maybe_log_script_count(%{assigns: %{opts: opts, update_count: count}} = driver) do
    count = count + 1
    driver = assign(driver, :update_count, count)

    if opts[:debug] && rem(count, 60) == 0 do
      case Native.script_count() do
        {:ok, total} ->
          Logger.info("Scenic.Driver.Skia cached scripts: #{total}")

        total when is_integer(total) ->
          Logger.info("Scenic.Driver.Skia cached scripts: #{total}")

        {:error, reason} ->
          Logger.warning("script_count failed: #{inspect(reason)}")

        other ->
          Logger.warning("script_count returned #{inspect(other)}")
      end
    end

    driver
  end
end
