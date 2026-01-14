defmodule Scenic.Driver.Skia.InputEventsTest do
  use ExUnit.Case, async: true

  alias Scenic.Driver.Skia.Native
  alias Scenic.Driver.Skia.TestSupport.ViewPort, as: ViewPortHelper
  alias Scenic.ViewPort

  defmodule RasterScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({140, 80}, fill: :red, translate: {20, 20})

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule PixelScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({20, 20}, fill: :red, translate: {0, 0})

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  test "drains input events while raster backend is running" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)
    ensure_renderer_stopped()

    renderer =
      case Native.start("raster", nil, "Scenic Window", false, nil, true, false) do
        {:ok, renderer} -> renderer
        other -> flunk("start returned #{inspect(other)}")
      end

    on_exit(fn ->
      _ = Native.stop(renderer)
    end)

    case Native.set_input_mask(renderer, 0x01) do
      :ok -> :ok
      {:ok, _} -> :ok
      other -> flunk("set_input_mask returned #{inspect(other)}")
    end

    case Native.drain_input_events(renderer) do
      [] -> :ok
      {:ok, []} -> :ok
      other -> flunk("drain_input_events returned #{inspect(other)}")
    end
  end

  test "raster output matches viewport size" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    viewport_size = {321, 123}

    vp = ViewPortHelper.start(size: viewport_size)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, height, frame} = wait_for_frame!(renderer, 40)
    assert {width, height} == viewport_size
    assert byte_size(frame) == width * height * 3
  end

  test "raster output contains drawn content" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {200, 120}, scene: RasterScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {_width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {_w, _h, data} ->
        Enum.any?(:binary.bin_to_list(data), &(&1 > 0))
      end)

    assert Enum.any?(:binary.bin_to_list(frame), &(&1 > 0))
  end

  test "raster output returns expected pixel colors" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: PixelScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 5, 5) == {255, 0, 0}
      end)

    assert pixel_at(frame, width, 5, 5) == {255, 0, 0}
    assert pixel_at(frame, width, 30, 30) == {0, 0, 0}
  end

  test "cursor visibility toggles are accepted while renderer is running" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)
    ensure_renderer_stopped()

    renderer =
      case Native.start("raster", nil, "Scenic Window", false, nil, true, false) do
        {:ok, renderer} -> renderer
        other -> flunk("start returned #{inspect(other)}")
      end

    on_exit(fn ->
      _ = Native.stop(renderer)
    end)

    assert :ok = Scenic.Driver.Skia.hide_cursor(renderer)
    assert :ok = Scenic.Driver.Skia.show_cursor(renderer)
  end

  defp wait_for_frame!(renderer, attempts_remaining),
    do: wait_for_frame!(renderer, attempts_remaining, fn _ -> true end)

  defp wait_for_frame!(renderer, attempts_remaining, predicate) do
    case Native.get_raster_frame(renderer) do
      {:ok, {width, height, frame}} = ok ->
        if predicate.({width, height, frame}) do
          {width, height, frame}
        else
          retry_frame(renderer, ok, attempts_remaining, predicate)
        end

      other ->
        retry_frame(renderer, other, attempts_remaining, predicate)
    end
  end

  defp retry_frame(renderer, _last_result, attempts_remaining, predicate)
       when attempts_remaining > 0 do
    Process.sleep(50)
    wait_for_frame!(renderer, attempts_remaining - 1, predicate)
  end

  defp retry_frame(_renderer, last_result, _attempts_remaining, _predicate) do
    flunk("timed out waiting for raster frame: #{inspect(last_result)}")
  end

  defp pixel_at(frame, width, x, y) do
    offset = (y * width + x) * 3

    case frame do
      <<_::binary-size(offset), r, g, b, _::binary>> -> {r, g, b}
      _ -> {0, 0, 0}
    end
  end

  defp ensure_renderer_stopped, do: :ok
end
