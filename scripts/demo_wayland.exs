defmodule ScenicDriverSkia.DemoWayland do
  defmodule DemoScene do
    use Scenic.Scene
    import Scenic.Primitives
    alias Scenic.Script

    def init(scene, _args, _opts) do
      scene = Scenic.Scene.push_script(scene, build_rrectv_script(), "rrectv_demo")
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
      x1 = 80
      x2 = 700
      x3 = 1320
      x4 = 1940
      y1 = 80
      y2 = 460
      y3 = 840
      y4 = 1180
      label_offset = 160

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
      |> path(path_commands,
        fill: :maroon,
        stroke: {3, :white},
        translate: {x4, y3}
      )
      |> text("path", fill: :white, translate: {x4, y3 + label_offset})
      |> rect({200, 120},
        fill: :red,
        scissor: {120, 60},
        translate: {x1, y4}
      )
      |> text("scissor", fill: :white, translate: {x1, y4 + label_offset})
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
  end

  def run do
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {2560, 1440},
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

    Process.sleep(:infinity)
  end
end

ScenicDriverSkia.DemoWayland.run()
