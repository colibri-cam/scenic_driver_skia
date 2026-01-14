defmodule ScenicDriverSkia.DemoWayland do
  defmodule DemoScene do
    use Scenic.Scene
    import Scenic.Primitives
    alias Scenic.Script

    def init(scene, _args, _opts) do
      scene = Scenic.Scene.push_script(scene, build_rrectv_script(), "rrectv_demo")
      graph = build_graph()
      {:ok, Scenic.Scene.push_graph(scene, graph)}
    end

    defp build_graph do
      x1 = 80
      x2 = 700
      x3 = 1320
      x4 = 1940
      y1 = 80
      y2 = 460
      y3 = 840
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
      |> line({{0, 0}, {200, 120}}, stroke: {4, :white}, translate: {x1, y2})
      |> text("line", fill: :white, translate: {x1, y2 + label_offset})
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
      |> text("text", fill: :yellow, font_size: 30, translate: {x3, y3 + 70})
      |> text("text", fill: :white, translate: {x3, y3 + label_offset})
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
