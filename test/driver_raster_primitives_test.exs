defmodule Scenic.Driver.Skia.RasterPrimitivesTest do
  use ExUnit.Case, async: false

  alias Scenic.Driver.Skia.Native
  alias Scenic.Driver.Skia.TestSupport.ViewPort, as: ViewPortHelper
  alias Scenic.ViewPort

  defmodule RectScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({20, 20},
          fill: :red,
          stroke: {4, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  test "draw_rect fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: RectScene)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
    end)

    {width, _height, frame} =
      wait_for_frame!(40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 20) == {255, 0, 0} and
          pixel_at(data, w, 15, 10) == {255, 255, 255}
      end)

    # Background just outside the translated rect bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 7, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 33) == {0, 0, 0}
    # Stroke samples on each edge.
    assert pixel_at(frame, width, 15, 10) == {255, 255, 255}
    assert pixel_at(frame, width, 10, 15) == {255, 255, 255}
    assert pixel_at(frame, width, 30, 15) == {255, 255, 255}
    assert pixel_at(frame, width, 15, 30) == {255, 255, 255}
    # Fill sample inside the rect.
    assert pixel_at(frame, width, 20, 20) == {255, 0, 0}
  end

  defp wait_for_frame!(attempts_remaining, predicate) do
    case Native.get_raster_frame() do
      {:ok, {width, height, frame}} = ok ->
        if predicate.({width, height, frame}) do
          {width, height, frame}
        else
          retry_frame(ok, attempts_remaining, predicate)
        end

      other ->
        retry_frame(other, attempts_remaining, predicate)
    end
  end

  defp retry_frame(_last_result, attempts_remaining, predicate) when attempts_remaining > 0 do
    Process.sleep(50)
    wait_for_frame!(attempts_remaining - 1, predicate)
  end

  defp retry_frame(last_result, _attempts_remaining, _predicate) do
    flunk("timed out waiting for raster frame: #{inspect(last_result)}")
  end

  defp pixel_at(frame, width, x, y) do
    offset = (y * width + x) * 3

    case frame do
      <<_::binary-size(offset), r, g, b, _::binary>> -> {r, g, b}
      _ -> {0, 0, 0}
    end
  end
end
