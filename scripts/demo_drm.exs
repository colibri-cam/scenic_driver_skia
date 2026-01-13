defmodule ScenicDriverSkia.DemoDrm do
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

      scene =
        scene
        |> Scenic.Scene.assign(:rect_fill, :blue)
        |> Scenic.Scene.assign(:cursor_pos_text, "cursor_pos: none")
        |> Scenic.Scene.assign(:cursor_button_text, "cursor_button: none")
        |> Scenic.Scene.assign(:key_text, "key: none")

      scene = Scenic.Scene.push_graph(scene, build_graph(scene))
      {:ok, scene}
    end

    def handle_input({:cursor_pos, {x, y}}, _context, scene) do
      scene =
        scene
        |> Scenic.Scene.assign(:cursor_pos_text, format_cursor_pos(x, y))
        |> Scenic.Scene.push_graph(build_graph(scene))

      {:noreply, scene}
    end

    def handle_input({:cursor_button, {button, action, _mods, {x, y}}}, _context, scene) do
      scene =
        scene
        |> Scenic.Scene.assign(:cursor_button_text, format_cursor_button(button, action, x, y))
        |> Scenic.Scene.push_graph(build_graph(scene))

      {:noreply, scene}
    end

    def handle_input({:key, {key, action, _mods}}, _context, scene) do
      scene =
        scene
        |> Scenic.Scene.assign(:key_text, format_key(key, action))
        |> Scenic.Scene.push_graph(build_graph(scene))

      {:noreply, scene}
    end

    def handle_input({:codepoint, {codepoint, _mods}}, _context, scene) do
      scene =
        scene
        |> Scenic.Scene.assign(:key_text, "codepoint: #{codepoint}")
        |> Scenic.Scene.push_graph(build_graph(scene))

      {:noreply, scene}
    end

    def handle_input(_event, _context, scene) do
      {:noreply, scene}
    end

    def handle_info(:change_color, scene) do
      rect_fill =
        case scene.assigns.rect_fill do
          :blue -> :red
          _ -> :blue
        end

      scene =
        scene
        |> Scenic.Scene.assign(:rect_fill, rect_fill)
        |> Scenic.Scene.push_graph(build_graph(scene))

      {:noreply, scene}
    end

    defp build_graph(scene) do
      Scenic.Graph.build()
      |> rect({200, 120}, fill: scene.assigns.rect_fill, translate: {50, 50})
      |> text("Skia DRM", fill: :yellow, translate: {60, 90})
      |> text(scene.assigns.cursor_pos_text, fill: :white, translate: {60, 140})
      |> text(scene.assigns.cursor_button_text, fill: :white, translate: {60, 165})
      |> text(scene.assigns.key_text, fill: :white, translate: {60, 190})
      |> analog_clock(radius: 50, seconds: true, translate: {800, 160}, theme: :light)
      |> digital_clock(
        format: :hours_12,
        seconds: true,
        translate: {50, 215},
        font: :roboto_mono,
        font_size: 18,
        fill: :white
      )
    end

    defp format_cursor_pos(x, y) do
      "cursor_pos: #{Float.round(x, 1)}, #{Float.round(y, 1)}"
    end

    defp format_cursor_button(button, action, x, y) do
      "cursor_button: #{button} #{action} @ #{Float.round(x, 1)}, #{Float.round(y, 1)}"
    end

    defp format_key(key, action) do
      "key: #{key} #{action}"
    end
  end

  def run do
    maybe_set_drm_card(System.argv())
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {400, 300},
        default_scene: DemoScene,
        drivers: [
          [module: Scenic.Driver.Skia, name: :skia_driver, backend: :drm]
        ]
      )

    Process.sleep(:infinity)
  end

  defp maybe_set_drm_card(args) do
    case args do
      ["--device", path] -> System.put_env("SCENIC_DRM_CARD", path)
      ["--device=" <> path] -> System.put_env("SCENIC_DRM_CARD", path)
      _ -> :ok
    end
  end
end

ScenicDriverSkia.DemoDrm.run()
