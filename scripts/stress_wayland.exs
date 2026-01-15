defmodule ScenicDriverSkia.StressWayland do
  defmodule StressScene do
    use Scenic.Scene

    import Scenic.Clock.Components
    import Scenic.Primitives

    @viewport_size {3840, 2180}
    @cell_width 240
    @cell_height 200
    @cols div(elem(@viewport_size, 0), @cell_width)
    @rows div(elem(@viewport_size, 1), @cell_height)
    @clock_radius 50

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> render_grid()

      scene = Scenic.Scene.push_graph(scene, graph)
      {:ok, scene}
    end

    defp render_grid(graph) do
      Enum.reduce(0..(@rows - 1), graph, fn row, row_graph ->
        Enum.reduce(0..(@cols - 1), row_graph, fn col, col_graph ->
          x = col * @cell_width
          y = row * @cell_height
          center_x = x + @cell_width / 2
          center_y = y + @cell_height / 2

          col_graph
          |> analog_clock(
            radius: @clock_radius,
            seconds: true,
            translate: {center_x, center_y - 20},
            theme: :light
          )
          |> digital_clock(
            format: :hours_12,
            seconds: true,
            translate: {center_x - 50, y + @cell_height - 40},
            font: :roboto_mono,
            font_size: 16,
            fill: :white
          )
        end)
      end)
    end
  end

  def run do
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {2560, 1440},
        default_scene: StressScene,
        drivers: [
          [module: ScenicDriverSkia.Driver, name: :skia_driver, backend: :wayland, debug: true]
        ]
      )

    Process.sleep(:infinity)
  end
end

ScenicDriverSkia.StressWayland.run()
