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
  alias Scenic.Assets.Stream
  alias Scenic.{Script, ViewPort}

  @window_schema [
    title: [type: :string, default: "Scenic Window"],
    resizeable: [type: :boolean, default: false]
  ]

  @drm_schema [
    card: [type: :string],
    hw_cursor: [type: :boolean, default: true],
    input_log: [type: :boolean, default: false]
  ]

  @opts_schema [
    backend: [type: {:or, [:atom, :string]}, default: :wayland],
    debug: [type: :boolean, default: false],
    window: [type: :keyword_list, keys: @window_schema, default: []],
    drm: [type: :keyword_list, keys: @drm_schema, default: []]
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
    window_opts = Keyword.get(opts, :window, [])
    window_title = Keyword.get(window_opts, :title, "Scenic Window")
    window_resizeable = Keyword.get(window_opts, :resizeable, false)
    drm_opts = Keyword.get(opts, :drm, [])
    drm_card = Keyword.get(drm_opts, :card)
    drm_hw_cursor = Keyword.get(drm_opts, :hw_cursor, true)
    drm_input_log = Keyword.get(drm_opts, :input_log, false)

    case Native.start(
           opts[:backend],
           viewport_size,
           window_title,
           window_resizeable,
           drm_card,
           drm_hw_cursor,
           drm_input_log
         ) do
      {:ok, renderer} ->
        maybe_set_input_target(renderer, self())

        {:ok,
         assign(driver,
           opts: opts,
           update_count: 0,
           input_mask: 0,
           renderer: renderer,
           media: %{images: [], streams: []}
         )}

      {:error, reason} ->
        {:stop, reason}

      other ->
        {:stop, {:unexpected_start_result, other}}
    end
  end

  @impl Scenic.Driver
  def reset_scene(driver) do
    Logger.debug("Scenic.Driver.Skia reset_scene")
    _ = Native.reset_scene(driver.assigns.renderer)
    {:ok, driver}
  end

  @impl Scenic.Driver
  def request_input(input, driver) do
    mask = input_mask_from_request(input)

    case Native.set_input_mask(driver.assigns.renderer, mask) do
      :ok -> :ok
      {:ok, _} -> :ok
      {:error, reason} -> Logger.warning("set_input_mask failed: #{inspect(reason)}")
      other -> Logger.warning("set_input_mask returned #{inspect(other)}")
    end

    if mask == 0 do
      maybe_set_input_target(driver.assigns.renderer, nil)
    else
      maybe_set_input_target(driver.assigns.renderer, self())
    end

    {:ok, assign(driver, :input_mask, mask)}
  end

  @impl GenServer
  def handle_info(:input_ready, driver) do
    events =
      case Native.drain_input_events(driver.assigns.renderer) do
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

  @impl GenServer
  def handle_info({{Stream, :put}, _type, id}, driver) do
    driver = put_stream_asset(id, driver)
    {:noreply, driver}
  end

  @impl GenServer
  def handle_info({{Stream, :delete}, _type, id}, driver) do
    _ = Native.del_stream_texture(driver.assigns.renderer, id)
    driver = drop_stream(id, driver)
    {:noreply, driver}
  end

  @impl GenServer
  def handle_call(:renderer_handle, _from, driver) do
    {:reply, driver.assigns.renderer, driver}
  end

  @impl Scenic.Driver
  def update_scene(script_ids, %{viewport: vp} = driver) do
    Logger.debug("Scenic.Driver.Skia update_scene: #{inspect(script_ids)}")

    {updates, driver} =
      Enum.reduce(script_ids, {[], driver}, fn id, {acc, driver} ->
        case ViewPort.get_script(vp, id) do
          {:ok, script} ->
            driver = ensure_media(script, driver)

            binary = serialize_script(script)

            {[{to_string(id), binary} | acc], driver}

          _ ->
            {acc, driver}
        end
      end)

    case updates do
      [] ->
        :ok

      _ ->
        Native.submit_scripts(driver.assigns.renderer, updates)
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

  defp serialize_script(script) do
    script
    |> Script.serialize(&serialize_op/1)
    |> IO.iodata_to_binary()
  end

  defp serialize_op({:clip_path, mode}) do
    encode_clip_path(mode)
  end

  defp serialize_op(other), do: other

  defp encode_clip_path(:intersect), do: <<0x0045::16-big, 0x00::16-big>>
  defp encode_clip_path(:difference), do: <<0x0045::16-big, 0x01::16-big>>

  defp encode_clip_path(mode) do
    raise ArgumentError, "invalid clip_path mode: #{inspect(mode)}"
  end

  @impl Scenic.Driver
  def del_scripts(script_ids, driver) do
    Logger.debug("Scenic.Driver.Skia del_scripts: #{inspect(script_ids)}")
    Enum.each(script_ids, &Native.del_script(driver.assigns.renderer, to_string(&1)))
    {:ok, driver}
  end

  @impl Scenic.Driver
  def clear_color(color, driver) do
    Logger.debug("Scenic.Driver.Skia clear_color: #{inspect(color)}")
    {:color_rgba, {r, g, b, a}} = Scenic.Color.to_rgba(color)
    _ = Native.set_clear_color(driver.assigns.renderer, {r, g, b, a})
    {:ok, driver}
  end

  defp ensure_media(script, driver) do
    media = Script.media(script)

    driver
    |> ensure_images(Map.get(media, :images, []))
    |> ensure_streams(Map.get(media, :streams, []))
  end

  defp ensure_images(driver, []), do: driver

  defp ensure_images(%{assigns: %{renderer: renderer, media: media}} = driver, ids) do
    images = Map.get(media, :images, [])

    images =
      Enum.reduce(ids, images, fn id, images ->
        with false <- Enum.member?(images, id),
             {:ok, bin} <- read_asset_binary(id) do
          _ = Native.put_static_image(renderer, id, bin)
          [id | images]
        else
          _ -> images
        end
      end)

    assign(driver, :media, Map.put(media, :images, images))
  end

  defp ensure_streams(driver, []), do: driver

  defp ensure_streams(%{assigns: %{media: media}} = driver, ids) do
    streams = Map.get(media, :streams, [])

    streams =
      Enum.reduce(ids, streams, fn id, streams ->
        with false <- Enum.member?(streams, id),
             :ok <- Stream.subscribe(id) do
          _ = put_stream_asset(id, driver)
          [id | streams]
        else
          _ -> streams
        end
      end)

    assign(driver, :media, Map.put(media, :streams, streams))
  end

  defp put_stream_asset(id, %{assigns: %{renderer: renderer}} = driver) do
    case Stream.fetch(id) do
      {:ok, {Stream.Image, {w, h, _format}, bin}} ->
        _ = put_stream_texture(renderer, id, "file", w, h, bin)

      {:ok, {Stream.Bitmap, {w, h, format}, bin}} ->
        _ = put_stream_texture(renderer, id, Atom.to_string(format), w, h, bin)

      _ ->
        :ok
    end

    driver
  end

  defp put_stream_texture(renderer, id, format, width, height, bin) do
    case Native.put_stream_texture(renderer, id, format, width, height, bin) do
      :ok -> :ok
      {:ok, _} -> :ok
      {:error, reason} -> Logger.warning("put_stream_texture failed: #{inspect(reason)}")
      other -> Logger.warning("put_stream_texture returned #{inspect(other)}")
    end
  end

  defp drop_stream(id, %{assigns: %{media: media}} = driver) do
    streams = List.delete(Map.get(media, :streams, []), id)
    assign(driver, :media, Map.put(media, :streams, streams))
  end

  defp read_asset_binary(id) do
    app_priv = :code.priv_dir(:scenic_driver_skia) |> to_string()
    path = Path.join([app_priv, "__scenic", "assets", id])
    File.read(path)
  end

  @impl GenServer
  def terminate(_reason, driver) do
    _ = Native.stop(driver.assigns.renderer)
    :ok
  end

  @doc """
  Start the renderer manually with the provided backend.

  This bypasses the Scenic ViewPort lifecycle and is intended for demos/tests.
  Accepts `:wayland` or `:drm` and returns a renderer handle.
  """
  @spec start() :: {:ok, term()} | {:error, term()}
  def start, do: start(:wayland)

  @doc """
  Start the renderer manually with the provided backend.

  This bypasses the Scenic ViewPort lifecycle and is intended for demos/tests.
  Accepts `:wayland` or `:drm` and returns a renderer handle.
  """
  @spec start(:wayland | :drm | String.t()) :: {:ok, term()} | {:error, term()}
  def start(backend) when is_atom(backend) or is_binary(backend) do
    backend
    |> normalize_backend()
    |> Native.start(nil, "Scenic Window", false, nil, true, false)
  end

  @doc """
  Stop the renderer if it is running.

  Accepts a renderer handle returned by `start/0` or `start/1`.
  """
  @spec stop(term()) :: :ok | {:error, term()}
  def stop(renderer) do
    Native.stop(renderer)
  end

  @doc """
  Show the cursor when using the DRM backend.

  Accepts a renderer handle returned by `start/0` or `start/1`.
  """
  @spec show_cursor(term()) :: :ok | {:error, term()}
  def show_cursor(renderer) do
    Native.show_cursor(renderer)
    |> normalize_start_result()
  end

  @doc """
  Hide the cursor when using the DRM backend.

  Accepts a renderer handle returned by `start/0` or `start/1`.
  """
  @spec hide_cursor(term()) :: :ok | {:error, term()}
  def hide_cursor(renderer) do
    Native.hide_cursor(renderer)
    |> normalize_start_result()
  end

  @doc """
  Update the text rendered by the driver.

  Accepts a renderer handle returned by `start/0` or `start/1`.
  """
  @spec set_text(term(), String.t()) :: :ok | {:error, term()}
  def set_text(renderer, text) when is_binary(text) do
    Native.set_text(renderer, text)
  end

  @doc false
  @spec renderer_handle(GenServer.server()) :: term()
  def renderer_handle(driver_pid) do
    GenServer.call(driver_pid, :renderer_handle)
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

  defp maybe_set_input_target(renderer, pid) do
    case Native.set_input_target(renderer, pid) do
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
      case Native.script_count(driver.assigns.renderer) do
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
