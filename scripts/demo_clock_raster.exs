defmodule ScenicDriverSkia.DemoClockRaster do
  defmodule DemoScene do
    use Scenic.Scene

    import Scenic.Clock.Components
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      graph =
        Scenic.Graph.build()
        |> analog_clock(
          radius: 60,
          seconds: true,
          translate: {100, 100},
          theme: :light
        )
        |> digital_clock(
          format: :hours_12,
          seconds: true,
          translate: {220, 120},
          font: :roboto_mono,
          font_size: 18,
          fill: :white
        )
        |> rect({420, 200}, stroke: {2, :white})

      scene = Scenic.Scene.push_graph(scene, graph)
      {:ok, scene}
    end
  end

  def run do
    output = Path.expand("priv/raster_clock.png")

    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {480, 240},
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

ScenicDriverSkia.DemoClockRaster.run()
