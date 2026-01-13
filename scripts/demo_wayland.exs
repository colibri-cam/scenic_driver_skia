defmodule ScenicDriverSkia.DemoWayland do
  require Logger

  defmodule DemoScene do
    use Scenic.Scene
    import Scenic.Clock.Components
    import Scenic.Primitives

    def init(scene, _args, _opts) do
      :ok =
        Scenic.Scene.request_input(scene, [
          :key,
          :codepoint,
          :cursor_pos,
          :cursor_button,
          :cursor_scroll,
          :viewport
        ])

      Process.send_after(self(), :change_color, 3_000)

      graph =
        Scenic.Graph.build()
        |> rect({200, 120}, fill: :blue, translate: {50, 50})
        |> text("Skia Wayland", fill: :yellow, translate: {60, 90})
        |> analog_clock(radius: 50, seconds: true, translate: {300, 160}, theme: :light)
        |> digital_clock(
          format: :hours_12,
          seconds: true,
          translate: {50, 200},
          font: :roboto_mono,
          font_size: 18,
          fill: :white
        )

      scene =
        scene
        |> Scenic.Scene.push_graph(graph)
        |> Scenic.Scene.assign(:rect_bounds, {50.0, 50.0, 250.0, 170.0})
      {:ok, scene}
    end

    def handle_input(event, _context, scene) do
      maybe_send_rect_event(event, scene)
      {:noreply, scene}
    end

    def handle_info(:change_color, scene) do
      graph =
        Scenic.Graph.build()
        |> rect({200, 120}, fill: :red, translate: {50, 50})
        |> text("Skia Wayland", fill: :yellow, translate: {60, 90})
        |> analog_clock(radius: 50, seconds: true, translate: {300, 160}, theme: :light)
        |> digital_clock(
          format: :hours_12,
          seconds: true,
          translate: {50, 200},
          font: :roboto_mono,
          font_size: 18,
          fill: :white
        )

      scene =
        scene
        |> Scenic.Scene.push_graph(graph)
        |> Scenic.Scene.assign(:rect_bounds, {50.0, 50.0, 250.0, 170.0})
      {:noreply, scene}
    end

    def handle_event({:rect_click, pos}, from, scene) do
      Logger.info("demo_wayland handle_event rect_click: from=#{inspect(from)} pos=#{inspect(pos)}")
      {:halt, scene}
    end

    defp maybe_send_rect_event({:cursor_button, {_btn, action, _mods, {x, y}}}, scene)
         when action in [1] do
      {x0, y0, x1, y1} = scene.assigns.rect_bounds
      if x >= x0 and x <= x1 and y >= y0 and y <= y1 do
        Scenic.Scene.send_event(self(), {:rect_click, {x, y}})
      end
    end

    defp maybe_send_rect_event(_event, _scene), do: :ok
  end

  def run do
    Logger.configure(level: :info)
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {400, 300},
        default_scene: DemoScene,
        drivers: [
          [module: Scenic.Driver.Skia, name: :skia_driver, backend: :wayland, debug: false]
        ]
      )

    Process.sleep(:infinity)
  end
end

ScenicDriverSkia.DemoWayland.run()
