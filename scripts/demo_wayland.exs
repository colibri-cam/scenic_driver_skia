defmodule ScenicDriverSkia.DemoWayland do
  defmodule DemoScene do
    use Scenic.Scene
    import Scenic.Primitives
    alias Scenic.Script

    def init(scene, _args, _opts) do
      scene = Scenic.Scene.push_script(scene, build_rrectv_script(), "rrectv_demo")
      scene = Scenic.Scene.push_script(scene, build_path_shape_script(), "path_shape_demo")
      scene = Scenic.Scene.push_script(scene, build_clip_path_script(), "clip_path_demo")
      scene = Scenic.Scene.assign(scene, join_miter_limit: 1)
      scene = schedule_join_tick(scene)
      graph = build_graph(scene.assigns.join_miter_limit)
      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end

    def handle_info(:join_tick, scene) do
      limit =
        case scene.assigns.join_miter_limit do
          1 -> 20
          _ -> 1
        end

      scene =
        scene
        |> Scenic.Scene.assign(join_miter_limit: limit)
        |> Scenic.Scene.push_graph(build_graph(limit))

      {:noreply, schedule_join_tick(scene)}
    end

    defp build_graph(join_miter_limit) do
      x1 = 60
      x2 = 470
      x3 = 880
      x4 = 1290
      x5 = 1700
      y1 = 60
      y2 = 320
      y3 = 580
      y4 = 800
      label_offset = 120
      sprite_cmds = [
        {{0, 0}, {120, 80}, {0, 0}, {120, 80}},
        {{200, 80}, {120, 80}, {60, 30}, {120, 80}, 0.6}
      ]

      path_commands = [
        :begin,
        {:move_to, 0, 0},
        {:line_to, 200, 0},
        {:arc_to, 200, 60, 140, 60, 24},
        {:line_to, 140, 120},
        {:bezier_to, 140, 150, 60, 150, 60, 120},
        {:line_to, 0, 120},
        {:quadratic_to, 0, 60, 40, 60},
        :close_path
      ]

      Scenic.Graph.build(font_size: 20)
      |> rect({200, 120}, fill: :blue, stroke: {3, :white}, translate: {x1, y1})
      |> text("rect", fill: :white, translate: {x1, y1 + label_offset})
      |> rounded_rectangle({200, 120, 20},
        fill: :purple,
        stroke: {3, :white},
        translate: {x2, y1}
      )
      |> text("rrect", fill: :white, translate: {x2, y1 + label_offset})
      |> script("rrectv_demo", translate: {x3, y1})
      |> text("rrectv", fill: :white, translate: {x3, y1 + label_offset})
      |> rect({200, 120},
        fill: {:radial, {100, 60, 0, 80, :red, :blue}},
        translate: {x5, y1}
      )
      |> text("radial gradient", fill: :white, translate: {x5, y1 + label_offset})
      |> script("path_shape_demo", translate: {x5, y2})
      |> text("script path ops", fill: :white, translate: {x5, y2 + label_offset})
      |> sprites({:stock, sprite_cmds}, translate: {x5, y2 + 140})
      |> text("sprites", fill: :white, translate: {x5, y2 + 230})
      |> line({{0, 0}, {200, 0}}, stroke: {10, :white}, cap: :butt, translate: {x1, y2})
      |> text("cap: butt", fill: :white, translate: {x1, y2 + 30})
      |> line({{0, 0}, {200, 0}}, stroke: {10, :white}, cap: :round, translate: {x1, y2 + 50})
      |> text("cap: round", fill: :white, translate: {x1, y2 + 80})
      |> line({{0, 0}, {200, 0}}, stroke: {10, :white}, cap: :square, translate: {x1, y2 + 100})
      |> text("cap: square", fill: :white, translate: {x1, y2 + 130})
      |> circle(55, fill: :green, stroke: {3, :white}, translate: {x2 + 100, y2 + 60})
      |> text("circle", fill: :white, translate: {x2, y2 + label_offset})
      |> ellipse({70, 45}, fill: :orange, stroke: {3, :white}, translate: {x3 + 100, y2 + 60})
      |> text("ellipse", fill: :white, translate: {x3, y2 + label_offset})
      |> triangle({{0, 120}, {100, 0}, {200, 120}},
        fill: :pink,
        stroke: {3, :white},
        translate: {x4, y2}
      )
      |> text("triangle", fill: :white, translate: {x4, y2 + label_offset})
      |> quad({{0, 0}, {215, 0}, {185, 120}, {0, 140}},
        fill: :olive,
        stroke: {3, :white},
        join: :round,
        miter_limit: 2,
        translate: {x4, y1}
      )
      |> text("quad", fill: :white, translate: {x4, y1 + label_offset})
      |> arc({70, 1.6}, stroke: {6, :white}, translate: {x1 + 100, y3 + 60})
      |> text("arc", fill: :white, translate: {x1, y3 + label_offset})
      |> sector({70, 1.2}, fill: :teal, stroke: {3, :white}, translate: {x2 + 100, y3 + 60})
      |> text("sector", fill: :white, translate: {x2, y3 + label_offset})
      |> script("clip_path_demo", translate: {x2, y3 + 140})
      |> text("clip path", fill: :white, translate: {x2, y3 + 230})
      |> path(path_commands,
        fill: :maroon,
        stroke: {3, :white},
        translate: {x4, y3}
      )
      |> text("path", fill: :white, translate: {x4, y3 + label_offset})
      |> rect({200, 120},
        fill: {:image, :stock},
        translate: {x5, y3}
      )
      |> text("image", fill: :white, translate: {x5, y3 + label_offset})
      |> line({{0, 0}, {200, 0}},
        stroke: {12, {:image, :stock}},
        translate: {x5, y3 + 150}
      )
      |> text("image stroke", fill: :white, translate: {x5, y3 + 180})
      |> rect({200, 120},
        fill: :red,
        scissor: {120, 60},
        translate: {x1, y4}
      )
      |> text("scissor", fill: :white, translate: {x1, y4 + label_offset})
      |> line({{0, 0}, {200, 0}},
        stroke: {12, {:linear, {0, 0, 200, 0, :red, :blue}}},
        translate: {x1, y4 + 150}
      )
      |> text("linear stroke", fill: :white, translate: {x1, y4 + 180})
      |> rect({200, 120},
        fill: {:color_rgba, {255, 0, 0, 128}},
        translate: {x3, y4}
      )
      |> text("alpha 0.5", fill: :white, translate: {x3, y4 + label_offset})
      |> rect({200, 80},
        fill: {:linear, {0, 0, 200, 0, :red, :blue}},
        translate: {x3, y4 + 150}
      )
      |> text("linear gradient", fill: :white, translate: {x3, y4 + 240})
      |> rect({200, 120},
        fill: {:stream, "demo_stream"},
        translate: {x5, y4}
      )
      |> text("stream", fill: :white, translate: {x5, y4 + label_offset})
      |> line({{0, 0}, {200, 0}},
        stroke: {12, {:stream, "demo_stream"}},
        translate: {x5, y4 + 150}
      )
      |> text("stream stroke", fill: :white, translate: {x5, y4 + 180})
      |> path(
        [
          :begin,
          {:move_to, 0, 160},
          {:line_to, 100, 0},
          {:line_to, 200, 160}
        ],
        stroke: {24, :white},
        join: :miter,
        miter_limit: join_miter_limit,
        translate: {x2, y4}
      )
      |> line({{0, 160}, {100, 0}}, stroke: {2, :red}, translate: {x2, y4})
      |> line({{100, 0}, {200, 160}}, stroke: {2, :blue}, translate: {x2, y4})
      |> text("join: miter (limit #{join_miter_limit})",
        fill: :white,
        translate: {x2, y4 + 120}
      )
      |> path(
        [
          :begin,
          {:move_to, 0, 80},
          {:line_to, 100, 0},
          {:line_to, 200, 80}
        ],
        stroke: {10, :white},
        join: :bevel,
        translate: {x4, y4}
      )
      |> text("join: bevel", fill: :white, translate: {x4, y4 + 100})
      |> path(
        [
          :begin,
          {:move_to, 0, 80},
          {:line_to, 100, 0},
          {:line_to, 200, 80}
        ],
        stroke: {10, :white},
        join: :round,
        translate: {x4, y4 + 140}
      )
      |> text("join: round", fill: :white, translate: {x4, y4 + 240})
      |> text("text", fill: :yellow, font_size: 30, translate: {x3, y3 + 70})
      |> text("text", fill: :white, translate: {x3, y3 + label_offset})
    end

    defp schedule_join_tick(scene) do
      Process.send_after(self(), :join_tick, 1_000)

      scene
    end

    defp build_rrectv_script do
      Script.start()
      |> Script.fill_color(:navy)
      |> Script.stroke_color(:white)
      |> Script.stroke_width(3)
      |> Script.draw_variable_rounded_rectangle(200, 120, 36, 18, 54, 9, :fill_stroke)
      |> Script.finish()
    end

    defp build_clip_path_script do
      Script.start()
      |> Script.fill_color(:cyan)
      |> Script.stroke_color(:white)
      |> Script.stroke_width(2)
      |> Script.push_state()
      |> Script.translate(20, 20)
      |> Script.begin_path()
      |> Script.circle(25)
      |> clip_path(:intersect)
      |> Script.draw_rectangle(80, 60, :fill_stroke)
      |> Script.pop_state()
      |> Script.finish()
    end

    defp build_path_shape_script do
      Script.start()
      |> Script.fill_color(:purple)
      |> Script.stroke_color(:white)
      |> Script.stroke_width(2)
      |> Script.push_state()
      |> Script.translate(10, 10)
      |> Script.begin_path()
      |> Script.triangle(0, 30, 20, 0, 40, 30)
      |> Script.fill_path()
      |> Script.stroke_path()
      |> Script.pop_state()
      |> Script.push_state()
      |> Script.translate(60, 10)
      |> Script.begin_path()
      |> Script.quad(0, 0, 40, 0, 30, 30, 0, 40)
      |> Script.fill_path()
      |> Script.stroke_path()
      |> Script.pop_state()
      |> Script.push_state()
      |> Script.translate(110, 10)
      |> Script.begin_path()
      |> Script.rectangle(40, 30)
      |> Script.fill_path()
      |> Script.stroke_path()
      |> Script.pop_state()
      |> Script.push_state()
      |> Script.translate(160, 10)
      |> Script.begin_path()
      |> Script.rounded_rectangle(40, 30, 8)
      |> Script.fill_path()
      |> Script.stroke_path()
      |> Script.pop_state()
      |> Script.push_state()
      |> Script.translate(20, 65)
      |> Script.begin_path()
      |> Script.sector(20, 1.4)
      |> Script.fill_path()
      |> Script.stroke_path()
      |> Script.pop_state()
      |> Script.push_state()
      |> Script.translate(70, 65)
      |> Script.begin_path()
      |> Script.circle(16)
      |> Script.fill_path()
      |> Script.stroke_path()
      |> Script.pop_state()
      |> Script.push_state()
      |> Script.translate(120, 65)
      |> Script.begin_path()
      |> Script.ellipse(18, 12)
      |> Script.fill_path()
      |> Script.stroke_path()
      |> Script.pop_state()
      |> Script.push_state()
      |> Script.translate(170, 65)
      |> Script.begin_path()
      |> Script.arc(0, 0, 18, 0.0, 1.8, 1)
      |> Script.stroke_path()
      |> Script.pop_state()
      |> Script.finish()
    end

    defp clip_path(ops, mode) do
      [{:clip_path, mode} | ops]
    end
  end

  def run do
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)
    {:ok, stream_pid} = Scenic.Assets.Stream.start_link(nil)
    Process.unlink(stream_pid)
    :ok = Scenic.Assets.Stream.put("demo_stream", build_stream_bitmap(:green))
    :timer.send_interval(750, self(), :stream_tick)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {1920, 1080},
        default_scene: DemoScene,
        drivers: [
          [
            module: Scenic.Driver.Skia,
            name: :skia_driver,
            backend: :wayland,
            debug: false,
            window: [resizeable: false, title: "Scenic Wayland"]
          ]
        ]
      )

    stream_loop()
  end

  defp stream_loop do
    receive do
      :stream_tick ->
        color =
          case Process.get(:stream_color, :green) do
            :green ->
              Process.put(:stream_color, :magenta)
              :magenta

            _ ->
              Process.put(:stream_color, :green)
              :green
          end

        :ok = Scenic.Assets.Stream.put("demo_stream", build_stream_bitmap(color))
        stream_loop()
    end
  end

  defp build_stream_bitmap(color) do
    Scenic.Assets.Stream.Bitmap.build(:rgb, 16, 16, clear: color, commit: true)
  end
end

ScenicDriverSkia.DemoWayland.run()
