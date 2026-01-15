defmodule ScenicDriverSkia.DemoRaster do
  defmodule DemoScene do
    use Scenic.Scene
    import Scenic.Clock.Components
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> rect({200, 120}, fill: :green, translate: {50, 50})
        |> text("Skia Raster", fill: :orange, translate: {60, 90})
        |> analog_clock(radius: 50, seconds: true, translate: {300, 160}, theme: :light)
        |> digital_clock(
          format: :hours_12,
          seconds: true,
          translate: {50, 200},
          font: :roboto_mono,
          font_size: 18,
          fill: :white
        )

      scene = Scenic.Scene.push_graph(scene, graph)
      {:ok, scene}
    end
  end

  def run do
    output = Path.expand("priv/raster_demo.png")

    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {400, 300},
        default_scene: DemoScene,
        drivers: [
          [
            module: ScenicDriverSkia.Driver,
            name: :skia_driver,
            backend: :raster,
            raster_output: output
          ]
        ]
      )

    IO.puts("Writing raster output to #{output}")
    Process.sleep(2000)
  end
end

ScenicDriverSkia.DemoRaster.run()
