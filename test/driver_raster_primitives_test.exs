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

  defmodule RRectScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rounded_rectangle({20, 20, 8},
          fill: :red,
          stroke: {2, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule RRectVScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> script("rrectv_demo", translate: {10, 10})

      script =
        Scenic.Script.start()
        |> Scenic.Script.fill_color(:red)
        |> Scenic.Script.stroke_color(:white)
        |> Scenic.Script.stroke_width(2)
        |> Scenic.Script.draw_variable_rounded_rectangle(20, 20, 2, 6, 10, 4, :fill_stroke)
        |> Scenic.Script.finish()

      scene = Scenic.Scene.push_script(scene, script, "rrectv_demo")
      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule LineScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> line({{0, 10}, {30, 10}},
          stroke: {2, :white},
          translate: {10, 40}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule CircleScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> circle(8,
          fill: :red,
          stroke: {1, :white},
          translate: {20, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule TriangleScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> triangle({{0, 0}, {20, 0}, {0, 20}},
          fill: :red,
          stroke: {2, :white},
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

  test "draw_rrect fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: RRectScene)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
    end)

    {width, _height, frame} =
      wait_for_frame!(40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 20) == {255, 0, 0}
      end)

    # Background just outside the translated rrect bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    # Rounded corners leave background at the outer corner pixels.
    assert pixel_at(frame, width, 10, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 30, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 30) == {0, 0, 0}
    assert pixel_at(frame, width, 30, 30) == {0, 0, 0}
    # Stroke samples on each edge (away from rounded corners).
    assert pixel_at(frame, width, 20, 10) == {255, 255, 255}
    assert pixel_at(frame, width, 10, 20) == {255, 255, 255}
    assert pixel_at(frame, width, 30, 20) == {255, 255, 255}
    assert pixel_at(frame, width, 20, 30) == {255, 255, 255}
    # Fill sample inside the rrect.
    assert pixel_at(frame, width, 20, 20) == {255, 0, 0}
  end

  test "draw_rrectv fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: RRectVScene)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
    end)

    {width, _height, frame} =
      wait_for_frame!(40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 20) == {255, 0, 0} and
          pixel_at(data, w, 20, 10) == {255, 255, 255}
      end)

    # Background just outside the translated rrect bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    # Corner radii differ; check a sharp (small radius) and rounded (large radius) corner.
    assert pixel_at(frame, width, 10, 10) != {0, 0, 0}
    assert pixel_at(frame, width, 30, 30) == {0, 0, 0}
    # Stroke samples on each edge (away from corners).
    assert pixel_at(frame, width, 20, 10) != {0, 0, 0}
    assert pixel_at(frame, width, 10, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 30, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 30) != {0, 0, 0}
    # Fill sample inside the rrectv.
    assert pixel_at(frame, width, 20, 20) == {255, 0, 0}
  end

  test "draw_line renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: LineScene)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
    end)

    {width, _height, frame} =
      wait_for_frame!(40, fn {w, _h, data} ->
        pixel_at(data, w, 25, 50) != {0, 0, 0} and
          pixel_at(data, w, 25, 35) == {0, 0, 0}
      end)

    # Background above and below the horizontal line.
    assert pixel_at(frame, width, 25, 35) == {0, 0, 0}
    assert pixel_at(frame, width, 25, 65) == {0, 0, 0}
    # Stroke samples along the line.
    assert pixel_at(frame, width, 15, 50) != {0, 0, 0}
    assert pixel_at(frame, width, 25, 50) != {0, 0, 0}
    assert pixel_at(frame, width, 35, 50) != {0, 0, 0}
    assert pixel_at(frame, width, 38, 50) != {0, 0, 0}
  end

  test "draw_circle fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: CircleScene)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
    end)

    {width, _height, frame} =
      wait_for_frame!(40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 20) == {255, 0, 0} and
          pixel_at(data, w, 28, 20) != {0, 0, 0}
      end)

    # Background just outside the translated circle bounds.
    assert pixel_at(frame, width, 10, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 30, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 30) == {0, 0, 0}
    # Stroke samples on each side of the circle.
    assert pixel_at(frame, width, 28, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 28) != {0, 0, 0}
    assert pixel_at(frame, width, 12, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 12) != {0, 0, 0}
    # Fill sample at the center.
    assert pixel_at(frame, width, 20, 20) == {255, 0, 0}
  end

  test "draw_triangle fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: TriangleScene)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
    end)

    {width, _height, frame} =
      wait_for_frame!(40, fn {w, _h, data} ->
        pixel_at(data, w, 15, 15) == {255, 0, 0} and
          pixel_at(data, w, 22, 18) != {0, 0, 0} and
          pixel_at(data, w, 25, 25) == {0, 0, 0}
      end)

    # Background just outside the translated triangle bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    # Background inside the bounding box but outside the triangle fill.
    assert pixel_at(frame, width, 25, 25) == {0, 0, 0}
    # Stroke samples along the two legs and the hypotenuse.
    assert pixel_at(frame, width, 14, 10) == {255, 255, 255}
    assert pixel_at(frame, width, 10, 14) == {255, 255, 255}
    assert pixel_at(frame, width, 22, 18) != {0, 0, 0}
    # Fill sample inside the triangle.
    assert pixel_at(frame, width, 15, 15) == {255, 0, 0}
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
