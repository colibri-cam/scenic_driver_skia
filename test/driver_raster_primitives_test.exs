defmodule Scenic.Driver.Skia.RasterPrimitivesTest do
  use ExUnit.Case, async: true

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

  defmodule StrokeRectScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({20, 20},
          stroke: {2, :white},
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

  defmodule StrokeRRectScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rounded_rectangle({20, 20, 8},
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

  defmodule StrokeRRectVScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> script("rrectv_stroke_demo", translate: {10, 10})

      script =
        Scenic.Script.start()
        |> Scenic.Script.stroke_color(:white)
        |> Scenic.Script.stroke_width(2)
        |> Scenic.Script.draw_variable_rounded_rectangle(20, 20, 2, 6, 10, 4, :stroke)
        |> Scenic.Script.finish()

      scene = Scenic.Scene.push_script(scene, script, "rrectv_stroke_demo")
      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule ClipPathScene do
    use Scenic.Scene
    import Scenic.Primitives
    alias Scenic.Script

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> script("clip_path_demo")

      script =
        Script.start()
        |> Script.fill_color(:white)
        |> Script.push_state()
        |> Script.translate(20, 20)
        |> Script.begin_path()
        |> Script.circle(10)
        |> clip_path()
        |> Script.draw_rectangle(30, 30, :fill)
        |> Script.pop_state()
        |> Script.finish()

      scene = Scenic.Scene.push_script(scene, script, "clip_path_demo")
      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end

    defp clip_path(ops) do
      [{:clip_path, :intersect} | ops]
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

  defmodule StrokeCircleScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> circle(8,
          stroke: {2, :white},
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

  defmodule StrokeTriangleScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> triangle({{0, 0}, {20, 0}, {0, 20}},
          stroke: {2, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule EllipseScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> ellipse({8, 6},
          fill: :red,
          stroke: {2, :white},
          translate: {20, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule StrokeEllipseScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> ellipse({8, 6},
          stroke: {2, :white},
          translate: {20, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule ArcScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> arc({12, :math.pi() * 2.0},
          stroke: {2, :white},
          translate: {20, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule StrokeArcScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> arc({12, :math.pi() * 2.0},
          stroke: {2, :white},
          translate: {20, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule SectorScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> sector({12, :math.pi() / 2},
          fill: :red,
          stroke: {2, :white},
          translate: {20, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule StrokeSectorScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> sector({12, :math.pi() / 2},
          stroke: {2, :white},
          translate: {20, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule QuadScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> quad({{0, 0}, {20, 0}, {24, 20}, {0, 20}},
          fill: :red,
          stroke: {2, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule PathScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      commands = [
        :begin,
        {:move_to, 0, 0},
        {:line_to, 20, 0},
        {:line_to, 20, 20},
        {:line_to, 0, 20},
        :close_path
      ]

      graph =
        Scenic.Graph.build()
        |> path(commands,
          fill: :red,
          stroke: {2, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule PathArcScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      commands = [
        :begin,
        {:move_to, 0, 0},
        {:arc_to, 20, 0, 20, 20, 10}
      ]

      graph =
        Scenic.Graph.build()
        |> path(commands,
          stroke: {2, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule PathBezierScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      commands = [
        :begin,
        {:move_to, 0, 20},
        {:bezier_to, 0, 0, 20, 0, 20, 20}
      ]

      graph =
        Scenic.Graph.build()
        |> path(commands,
          stroke: {2, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule PathQuadraticScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      commands = [
        :begin,
        {:move_to, 0, 20},
        {:quadratic_to, 10, 0, 20, 20}
      ]

      graph =
        Scenic.Graph.build()
        |> path(commands,
          stroke: {2, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule ScissorScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({40, 40},
          fill: :red,
          translate: {10, 10},
          scissor: {20, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule AlphaRectScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({20, 20},
          fill: {:color_rgba, {255, 0, 0, 128}},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule LinearGradientScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({20, 20},
          fill: {:linear, {0, 0, 20, 0, :red, :blue}},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule RadialGradientScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({20, 20},
          fill: {:radial, {10, 10, 0, 10, :red, :blue}},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule LinearStrokeScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> line({{0, 0}, {20, 0}},
          stroke: {6, {:linear, {0, 0, 20, 0, :red, :blue}}},
          translate: {10, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule ImageFillScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({20, 20},
          fill: {:image, :test_red},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule StrokeImageScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> line({{0, 0}, {20, 0}},
          stroke: {6, {:image, :test_red}},
          translate: {10, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule StreamFillScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, stream_id, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({20, 20},
          fill: {:stream, stream_id},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule StreamStrokeScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, stream_id, _opts) do
      graph =
        Scenic.Graph.build()
        |> line({{0, 0}, {20, 0}},
          stroke: {6, {:stream, stream_id}},
          translate: {10, 20}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule SpritesScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> sprites(
          {:test_red,
           [
             {{0, 0}, {2, 2}, {10, 10}, {20, 20}}
           ]}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule CapCompareScene do
    use Scenic.Scene
    import Scenic.Primitives
    alias Scenic.Script

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> script("cap_compare_script")

      script =
        Script.start()
        |> Script.stroke_color(:white)
        |> Script.stroke_width(10)
        |> Script.push_state()
        |> Script.cap(:butt)
        |> Script.translate(10, 20)
        |> Script.draw_line(0, 0, 20, 0, :stroke)
        |> Script.pop_state()
        |> Script.push_state()
        |> Script.cap(:square)
        |> Script.translate(10, 40)
        |> Script.draw_line(0, 0, 20, 0, :stroke)
        |> Script.pop_state()
        |> Script.finish()

      scene = Scenic.Scene.push_script(scene, script, "cap_compare_script")
      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  defmodule StrokeQuadScene do
    use Scenic.Scene
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> quad({{0, 0}, {20, 0}, {24, 20}, {0, 20}},
          stroke: {2, :white},
          translate: {10, 10}
        )

      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end
  end

  test "draw_rect fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: RectScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
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

  test "clip_path limits drawing to the path" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: ClipPathScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 20) == {255, 255, 255} and
          pixel_at(data, w, 35, 35) == {0, 0, 0}
      end)

    # Center of the clipped circle should be white.
    assert pixel_at(frame, width, 20, 20) == {255, 255, 255}
    # Inside the rectangle but outside the circle should remain background.
    assert pixel_at(frame, width, 35, 35) == {0, 0, 0}
  end

  test "draw_rect stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeRectScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 10) == {255, 255, 255} and
          pixel_at(data, w, 20, 20) == {0, 0, 0}
      end)

    # Background just outside the translated rect bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    # Stroke samples on each edge.
    assert pixel_at(frame, width, 20, 10) == {255, 255, 255}
    assert pixel_at(frame, width, 10, 20) == {255, 255, 255}
    assert pixel_at(frame, width, 30, 20) == {255, 255, 255}
    assert pixel_at(frame, width, 20, 30) == {255, 255, 255}
    # Interior stays background without fill.
    assert pixel_at(frame, width, 20, 20) == {0, 0, 0}
  end

  test "draw_rrect fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: RRectScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
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

  test "draw_rrect stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeRRectScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 10) == {255, 255, 255} and
          pixel_at(data, w, 20, 20) == {0, 0, 0}
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
    # Interior stays background without fill.
    assert pixel_at(frame, width, 20, 20) == {0, 0, 0}
  end

  test "draw_rrectv fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: RRectVScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
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

  test "draw_rrectv stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeRRectVScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 10) != {0, 0, 0} and
          pixel_at(data, w, 20, 20) == {0, 0, 0}
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
    # Interior stays background without fill.
    assert pixel_at(frame, width, 20, 20) == {0, 0, 0}
  end

  test "draw_line renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: LineScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
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
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
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

  test "draw_circle stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeCircleScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 28, 20) != {0, 0, 0} and
          pixel_at(data, w, 20, 20) == {0, 0, 0}
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
    # Interior stays background without fill.
    assert pixel_at(frame, width, 20, 20) == {0, 0, 0}
  end

  test "draw_triangle fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: TriangleScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
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

  test "draw_triangle stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeTriangleScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 14, 10) == {255, 255, 255} and
          pixel_at(data, w, 13, 13) == {0, 0, 0}
      end)

    # Background just outside the translated triangle bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    # Stroke samples along the two legs and the hypotenuse.
    assert pixel_at(frame, width, 14, 10) == {255, 255, 255}
    assert pixel_at(frame, width, 10, 14) == {255, 255, 255}
    assert pixel_at(frame, width, 22, 18) != {0, 0, 0}
    # Interior stays background without fill.
    assert pixel_at(frame, width, 13, 13) == {0, 0, 0}
  end

  test "draw_ellipse fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: EllipseScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_pixel?(pixel_at(data, w, 20, 20)) and
          pixel_at(data, w, 28, 20) != {0, 0, 0}
      end)

    # Background just outside the translated ellipse bounds.
    assert pixel_at(frame, width, 10, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 12) == {0, 0, 0}
    assert pixel_at(frame, width, 30, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 28) == {0, 0, 0}
    # Stroke samples on the left, right, top, and bottom edges.
    assert pixel_at(frame, width, 12, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 28, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 14) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 26) != {0, 0, 0}
    # Fill sample inside the ellipse.
    assert red_pixel?(pixel_at(frame, width, 20, 20))
  end

  test "draw_ellipse stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeEllipseScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 28, 20) != {0, 0, 0} and
          pixel_at(data, w, 20, 20) == {0, 0, 0}
      end)

    # Background just outside the translated ellipse bounds.
    assert pixel_at(frame, width, 10, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 12) == {0, 0, 0}
    assert pixel_at(frame, width, 30, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 28) == {0, 0, 0}
    # Stroke samples on the left, right, top, and bottom edges.
    assert pixel_at(frame, width, 12, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 28, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 14) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 26) != {0, 0, 0}
    # Interior stays background without fill.
    assert pixel_at(frame, width, 20, 20) == {0, 0, 0}
  end

  test "draw_arc renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: ArcScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 32, 20) != {0, 0, 0} and
          pixel_at(data, w, 20, 20) == {0, 0, 0}
      end)

    # Background just outside the translated arc bounds.
    assert pixel_at(frame, width, 6, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 6) == {0, 0, 0}
    assert pixel_at(frame, width, 34, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 34) == {0, 0, 0}
    # Stroke samples along the arc, leaving the center unfilled.
    assert pixel_at(frame, width, 32, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 8) != {0, 0, 0}
    assert pixel_at(frame, width, 8, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 32) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 20) == {0, 0, 0}
  end

  test "draw_arc stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeArcScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 32, 20) != {0, 0, 0} and
          pixel_at(data, w, 20, 20) == {0, 0, 0}
      end)

    # Background just outside the translated arc bounds.
    assert pixel_at(frame, width, 6, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 6) == {0, 0, 0}
    assert pixel_at(frame, width, 34, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 34) == {0, 0, 0}
    # Stroke samples along the arc, leaving the center unfilled.
    assert pixel_at(frame, width, 32, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 8) != {0, 0, 0}
    assert pixel_at(frame, width, 8, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 32) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 20) == {0, 0, 0}
  end

  test "draw_sector fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: SectorScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_pixel?(pixel_at(data, w, 24, 24)) and
          pixel_at(data, w, 32, 20) != {0, 0, 0}
      end)

    # Background just outside the translated sector bounds.
    assert pixel_at(frame, width, 6, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 6) == {0, 0, 0}
    assert pixel_at(frame, width, 34, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 34) == {0, 0, 0}
    # Background in quadrants outside the sector sweep.
    assert pixel_at(frame, width, 24, 16) == {0, 0, 0}
    assert pixel_at(frame, width, 16, 24) == {0, 0, 0}
    # Stroke samples on the arc and along the radial edges.
    assert pixel_at(frame, width, 32, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 32) != {0, 0, 0}
    assert pixel_at(frame, width, 26, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 26) != {0, 0, 0}
    # Fill sample inside the sector.
    assert red_pixel?(pixel_at(frame, width, 24, 24))
  end

  test "draw_sector stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeSectorScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 32, 20) != {0, 0, 0} and
          pixel_at(data, w, 24, 24) == {0, 0, 0}
      end)

    # Background just outside the translated sector bounds.
    assert pixel_at(frame, width, 6, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 6) == {0, 0, 0}
    assert pixel_at(frame, width, 34, 20) == {0, 0, 0}
    assert pixel_at(frame, width, 20, 34) == {0, 0, 0}
    # Background in quadrants outside the sector sweep.
    assert pixel_at(frame, width, 24, 16) == {0, 0, 0}
    assert pixel_at(frame, width, 16, 24) == {0, 0, 0}
    # Stroke samples on the arc and along the radial edges.
    assert pixel_at(frame, width, 32, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 32) != {0, 0, 0}
    assert pixel_at(frame, width, 26, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 20, 26) != {0, 0, 0}
    # Interior stays background without fill.
    assert pixel_at(frame, width, 24, 24) == {0, 0, 0}
  end

  test "draw_quad fills expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: QuadScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_pixel?(pixel_at(data, w, 18, 20))
      end)

    # Background just outside the translated quad bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    # Background inside the bounding box but outside the quad fill.
    assert pixel_at(frame, width, 33, 12) == {0, 0, 0}
    # Stroke samples along the edges.
    assert pixel_at(frame, width, 14, 10) == {255, 255, 255}
    assert pixel_at(frame, width, 10, 14) == {255, 255, 255}
    assert pixel_at(frame, width, 32, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 22, 30) != {0, 0, 0}
    # Fill sample inside the quad.
    assert red_pixel?(pixel_at(frame, width, 18, 20))
  end

  test "draw_quad stroke only renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeQuadScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 14, 10) == {255, 255, 255} and
          pixel_at(data, w, 18, 20) == {0, 0, 0}
      end)

    # Background just outside the translated quad bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    # Stroke samples along the edges.
    assert pixel_at(frame, width, 14, 10) == {255, 255, 255}
    assert pixel_at(frame, width, 10, 14) == {255, 255, 255}
    assert pixel_at(frame, width, 32, 20) != {0, 0, 0}
    assert pixel_at(frame, width, 22, 30) != {0, 0, 0}
    # Interior stays background without fill.
    assert pixel_at(frame, width, 18, 20) == {0, 0, 0}
  end

  test "draw_path fills and strokes expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: PathScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 20, 20) == {255, 0, 0} and
          pixel_at(data, w, 20, 10) == {255, 255, 255}
      end)

    # Background just outside the translated path bounds.
    assert pixel_at(frame, width, 7, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 7) == {0, 0, 0}
    assert pixel_at(frame, width, 33, 10) == {0, 0, 0}
    assert pixel_at(frame, width, 10, 33) == {0, 0, 0}
    # Stroke samples on each edge.
    assert pixel_at(frame, width, 20, 10) == {255, 255, 255}
    assert pixel_at(frame, width, 10, 20) == {255, 255, 255}
    assert pixel_at(frame, width, 30, 20) == {255, 255, 255}
    assert pixel_at(frame, width, 20, 30) == {255, 255, 255}
    # Fill sample inside the path.
    assert pixel_at(frame, width, 20, 20) == {255, 0, 0}
  end

  test "draw_path arc_to renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: PathArcScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        any_non_background?(data, w, 20..40, 10..30)
      end)

    # Stroke sample somewhere in the translated arc region.
    assert any_non_background?(frame, width, 20..40, 10..30)
  end

  test "draw_path bezier_to renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: PathBezierScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        any_non_background?(data, w, 20..40, 20..40)
      end)

    # Stroke sample somewhere in the translated curve region.
    assert any_non_background?(frame, width, 20..40, 20..40)
  end

  test "draw_path quadratic_to renders expected pixels" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: PathQuadraticScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        any_non_background?(data, w, 20..40, 20..40)
      end)

    # Stroke sample somewhere in the translated curve region.
    assert any_non_background?(frame, width, 20..40, 20..40)
  end

  test "scissor clips drawing to expected bounds" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: ScissorScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        pixel_at(data, w, 15, 15) == {255, 0, 0}
      end)

    # Fill inside scissor bounds.
    assert pixel_at(frame, width, 15, 15) == {255, 0, 0}
    # Outside scissor but inside original rect stays background.
    assert pixel_at(frame, width, 35, 15) == {0, 0, 0}
    assert pixel_at(frame, width, 15, 35) == {0, 0, 0}
  end

  test "alpha fill blends with background" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: AlphaRectScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_channel_in_range?(pixel_at(data, w, 20, 20), 120, 140)
      end)

    # Half-alpha red should blend to mid red on black.
    assert red_channel_in_range?(pixel_at(frame, width, 20, 20), 120, 140)
    assert pixel_at(frame, width, 20, 20) |> green_blue_zero?()
  end

  test "linear gradient fill transitions across the rect" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: LinearGradientScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_dominant?(pixel_at(data, w, 12, 20)) and
          blue_dominant?(pixel_at(data, w, 28, 20))
      end)

    # Left side is closer to red.
    assert red_dominant?(pixel_at(frame, width, 12, 20))
    # Right side is closer to blue.
    assert blue_dominant?(pixel_at(frame, width, 28, 20))
  end

  test "radial gradient fill transitions across the rect" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: RadialGradientScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_dominant?(pixel_at(data, w, 20, 20)) and
          blue_dominant?(pixel_at(data, w, 28, 20))
      end)

    # Center stays closer to red.
    assert red_dominant?(pixel_at(frame, width, 20, 20))
    # Edge of the rect shifts towards blue.
    assert blue_dominant?(pixel_at(frame, width, 28, 20))
  end

  test "linear gradient stroke transitions across the line" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: LinearStrokeScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_dominant?(pixel_at(data, w, 12, 20)) and
          blue_dominant?(pixel_at(data, w, 28, 20))
      end)

    # Left side is closer to red.
    assert red_dominant?(pixel_at(frame, width, 12, 20))
    # Right side is closer to blue.
    assert blue_dominant?(pixel_at(frame, width, 28, 20))
  end

  test "image fill repeats across the rect" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: ImageFillScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_pixel?(pixel_at(data, w, 15, 15))
      end)

    # Image fill should render the red pixel.
    assert red_pixel?(pixel_at(frame, width, 15, 15))
  end

  test "image stroke renders image color" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: StrokeImageScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_pixel?(pixel_at(data, w, 15, 20))
      end)

    # Stroke image should render the red pixel along the line.
    assert red_pixel?(pixel_at(frame, width, 15, 20))
  end

  test "stream fill renders provided bitmap" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    :ok = ensure_stream_started()
    stream_id = "raster_stream_#{System.unique_integer([:positive])}"
    bitmap = Scenic.Assets.Stream.Bitmap.build(:rgb, 2, 2, clear: :green, commit: true)
    :ok = Scenic.Assets.Stream.put(stream_id, bitmap)
    assert {:ok, _} = Scenic.Assets.Stream.fetch(stream_id)

    vp = ViewPortHelper.start(size: {64, 64}, scene: {StreamFillScene, stream_id})
    renderer = ViewPortHelper.renderer(vp)
    {Scenic.Assets.Stream.Bitmap, {w, h, format}, bin} = bitmap

    :ok =
      normalize_nif_result(
        Native.put_stream_texture(renderer, stream_id, Atom.to_string(format), w, h, bin)
      )

    on_exit(fn ->
      Scenic.Assets.Stream.delete(stream_id)

      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        any_non_background?(data, w, 12..28, 12..28)
      end)

    # Stream fill should render within the rect bounds.
    assert any_non_background?(frame, width, 12..28, 12..28)
  end

  test "stream stroke renders provided bitmap" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    :ok = ensure_stream_started()
    stream_id = "raster_stream_stroke_#{System.unique_integer([:positive])}"
    bitmap = Scenic.Assets.Stream.Bitmap.build(:rgb, 2, 2, clear: :green, commit: true)
    :ok = Scenic.Assets.Stream.put(stream_id, bitmap)
    assert {:ok, _} = Scenic.Assets.Stream.fetch(stream_id)

    vp = ViewPortHelper.start(size: {64, 64}, scene: {StreamStrokeScene, stream_id})
    renderer = ViewPortHelper.renderer(vp)
    {Scenic.Assets.Stream.Bitmap, {w, h, format}, bin} = bitmap

    :ok =
      normalize_nif_result(
        Native.put_stream_texture(renderer, stream_id, Atom.to_string(format), w, h, bin)
      )

    on_exit(fn ->
      Scenic.Assets.Stream.delete(stream_id)

      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        any_non_background?(data, w, 12..28, 18..22)
      end)

    # Stream stroke should render along the line.
    assert any_non_background?(frame, width, 12..28, 18..22)
  end

  test "sprites draw the source image section" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: SpritesScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        red_pixel?(pixel_at(data, w, 15, 15))
      end)

    # Sprites should render the red source pixel in the target region.
    assert red_pixel?(pixel_at(frame, width, 15, 15))
  end

  test "cap square extends farther than butt" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    vp = ViewPortHelper.start(size: {64, 64}, scene: CapCompareScene)
    renderer = ViewPortHelper.renderer(vp)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop(renderer)
    end)

    {width, _height, frame} =
      wait_for_frame!(renderer, 40, fn {w, _h, data} ->
        any_non_background?(data, w, 10..30, 16..24) and
          any_non_background?(data, w, 10..30, 36..44)
      end)

    butt_max_x = max_non_background_x(frame, width, 10..40, 16..24)
    square_max_x = max_non_background_x(frame, width, 10..40, 36..44)

    # Square caps should not be shorter than butt caps (AA can blur endpoints).
    assert square_max_x >= butt_max_x
  end

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

  defp any_non_background?(frame, width, x_range, y_range) do
    Enum.any?(x_range, fn x ->
      Enum.any?(y_range, fn y ->
        pixel_at(frame, width, x, y) != {0, 0, 0}
      end)
    end)
  end

  defp max_non_background_x(frame, width, x_range, y_range) do
    Enum.reduce(x_range, -1, fn x, acc ->
      if Enum.any?(y_range, fn y -> pixel_at(frame, width, x, y) != {0, 0, 0} end) do
        max(acc, x)
      else
        acc
      end
    end)
  end

  defp red_pixel?({r, g, b}) do
    r > 200 and g < 80 and b < 80
  end

  defp ensure_stream_started do
    case Scenic.Assets.Stream.start_link(nil) do
      {:ok, pid} ->
        Process.unlink(pid)
        :ok

      {:error, {:already_started, _pid}} ->
        :ok
    end
  end

  defp normalize_nif_result(:ok), do: :ok
  defp normalize_nif_result({:ok, _}), do: :ok
  defp normalize_nif_result(other), do: other

  defp red_channel_in_range?({r, _g, _b}, min, max), do: r >= min and r <= max

  defp green_blue_zero?({_r, g, b}), do: g == 0 and b == 0

  defp red_dominant?({r, g, b}), do: r > b and r > g
  defp blue_dominant?({r, g, b}), do: b > r and b > g
end
