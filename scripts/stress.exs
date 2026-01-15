defmodule ScenicDriverSkia.Stress do
  require Logger

  defmodule StressScene do
    use Scenic.Scene

    import Scenic.Primitives

    @viewport_size {2560, 1440}
    @cell_size 80
    @cols div(elem(@viewport_size, 0), @cell_size)
    @rows div(elem(@viewport_size, 1), @cell_size)
    @frame_ms 4
    @palette [:cyan, :magenta, :yellow, :white, :orange, :green]

    def init(scene, _args, _opts) do
      start_ms = System.monotonic_time(:millisecond)
      Process.send_after(self(), :tick, @frame_ms)

      graph =
        Scenic.Graph.build()
        |> render_grid(0.0)

      scene =
        scene
        |> Scenic.Scene.assign(:start_ms, start_ms)
        |> Scenic.Scene.push_graph(graph)

      {:ok, scene}
    end

    def handle_info(:tick, scene) do
      now_ms = System.monotonic_time(:millisecond)
      start_ms = scene.assigns[:start_ms] || now_ms
      t = (now_ms - start_ms) / 1000.0

      graph =
        Scenic.Graph.build()
        |> render_grid(t)

      scene = Scenic.Scene.push_graph(scene, graph)
      Process.send_after(self(), :tick, @frame_ms)
      {:noreply, scene}
    end

    defp render_grid(graph, t) do
      Enum.reduce(0..(@rows - 1), graph, fn row, row_graph ->
        Enum.reduce(0..(@cols - 1), row_graph, fn col, col_graph ->
          idx = row * @cols + col
          base_x = col * @cell_size
          base_y = row * @cell_size
          center_x = base_x + @cell_size / 2
          center_y = base_y + @cell_size / 2
          wobble = :math.sin(t * 2.0 + idx * 0.15) * 8.0
          angle = t * 1.5 + idx * 0.05
          color = Enum.at(@palette, rem(idx, length(@palette)))

          col_graph
          |> rect(
            {@cell_size * 0.6, @cell_size * 0.6},
            fill: color,
            translate: {center_x + wobble, center_y + wobble},
            rotate: angle
          )
          |> circle(
            @cell_size * 0.25,
            stroke: {2, :white},
            translate: {center_x - wobble, center_y + wobble},
            rotate: -angle
          )
          |> rect(
            {@cell_size * 0.15, @cell_size * 0.9},
            stroke: {2, :blue},
            translate: {center_x - wobble, center_y - wobble},
            rotate: angle * 0.5
          )
        end)
      end)
    end
  end

  def run do
    {backend, device} = parse_args(System.argv())
    driver_opts = maybe_set_drm_card(device)
    Logger.configure(level: :info)
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {2560, 1440},
        default_scene: StressScene,
        drivers: [
          [module: Scenic.Driver.Skia, name: :skia_driver, backend: backend] ++ driver_opts
        ]
      )

    Process.sleep(:infinity)
  end

  defp parse_args(args) do
    {opts, _, _} =
      OptionParser.parse(args,
        strict: [backend: :string, device: :string],
        aliases: [b: :backend, d: :device]
      )

    backend =
      opts
      |> Keyword.get(:backend, "drm")
      |> String.downcase()
      |> case do
        "wayland" -> :wayland
        "raster" -> :raster
        _ -> :drm
      end

    {backend, Keyword.get(opts, :device)}
  end

  defp maybe_set_drm_card(nil), do: []
  defp maybe_set_drm_card(path), do: [drm: [card: path]]
end

ScenicDriverSkia.Stress.run()
