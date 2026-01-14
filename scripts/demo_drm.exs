defmodule ScenicDriverSkia.DemoDrm do
  defmodule DemoScene do
    use Scenic.Scene
    import Scenic.Clock.Components
    import Scenic.Primitives

    require Logger

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

      graph = build_graph()

      scene =
        scene
        |> Scenic.Scene.assign(:rect_fill, :blue)
        |> Scenic.Scene.assign(:cursor_pos_text, "cursor_pos: none")
        |> Scenic.Scene.assign(:cursor_pos, {0, 0})
        |> Scenic.Scene.assign(:cursor_button_text, "cursor_button: none")
        |> Scenic.Scene.assign(:key_text, "key: none")
        |> Scenic.Scene.assign(:cursor_visible, true)
        |> Scenic.Scene.assign(:cursor_toggle_text, "cursor_visible: true (press 'c' to toggle)")
        |> Scenic.Scene.assign(:viewport_size_text, "viewport_size: none")
        |> Scenic.Scene.assign(:graph, graph)
        |> Scenic.Scene.push_graph(graph)

      {:ok, scene}
    end

    def handle_input({:cursor_pos, {x, y}}, _context, %{assigns: assigns} = scene) do
      graph =
        assigns.graph
        |> Scenic.Graph.modify(:cursor_dot, &Scenic.Primitives.circle(&1, 3, translate: {x, y}))
        |> Scenic.Graph.modify(
          :cursor_pos_text,
          &Scenic.Primitives.text(&1, format_cursor_pos(x, y))
        )

      scene =
        scene
        |> Scenic.Scene.assign(:cursor_pos_text, format_cursor_pos(x, y))
        |> Scenic.Scene.assign(:cursor_pos, {x, y})
        |> Scenic.Scene.assign(:graph, graph)
        |> Scenic.Scene.push_graph(graph)

      {:noreply, scene}
    end

    def handle_input({:cursor_button, {button, action, _mods, {x, y}}}, _context, scene) do
      text = format_cursor_button(button, action, x, y)

      graph =
        scene.assigns.graph
        |> Scenic.Graph.modify(:cursor_button_text, &Scenic.Primitives.text(&1, text))

      scene =
        scene
        |> Scenic.Scene.assign(:cursor_button_text, text)
        |> Scenic.Scene.assign(:graph, graph)
        |> Scenic.Scene.push_graph(graph)

      {:noreply, scene}
    end

    def handle_input({:key, {key, action, _mods}}, _context, scene) do
      text = format_key(key, action)

      graph =
        scene.assigns.graph
        |> Scenic.Graph.modify(:key_text, &Scenic.Primitives.text(&1, text))

      scene =
        scene
        |> Scenic.Scene.assign(:key_text, text)
        |> Scenic.Scene.assign(:graph, graph)
        |> Scenic.Scene.push_graph(graph)

      {:noreply, scene}
    end

    def handle_input({:codepoint, {codepoint, _mods}}, _context, scene) do
      {scene, graph} =
        if codepoint in ["c", "C"] do
          visible = !scene.assigns.cursor_visible
          toggle_cursor(visible)
          toggle_text = "cursor_visible: #{visible} (press 'c' to toggle)"

          graph =
            scene.assigns.graph
            |> Scenic.Graph.modify(:cursor_toggle_text, &Scenic.Primitives.text(&1, toggle_text))

          scene =
            scene
            |> Scenic.Scene.assign(:cursor_visible, visible)
            |> Scenic.Scene.assign(:cursor_toggle_text, toggle_text)

          {scene, graph}
        else
          {scene, scene.assigns.graph}
        end

      key_text = "codepoint: #{codepoint}"

      graph =
        graph
        |> Scenic.Graph.modify(:key_text, &Scenic.Primitives.text(&1, key_text))

      scene =
        scene
        |> Scenic.Scene.assign(:key_text, key_text)
        |> Scenic.Scene.assign(:graph, graph)
        |> Scenic.Scene.push_graph(graph)

      {:noreply, scene}
    end

    def handle_input({:viewport, {:reshape, _size}}, _context, scene) do
      {width, height} = current_viewport_size(scene)
      Logger.info("viewport reshape -> ViewPort size now #{width}x#{height}")
      text = "viewport_size: #{width}x#{height}"

      graph =
        scene.assigns.graph
        |> Scenic.Graph.modify(:viewport_size_text, &Scenic.Primitives.text(&1, text))

      scene =
        scene
        |> Scenic.Scene.assign(:viewport_size_text, text)
        |> Scenic.Scene.assign(:graph, graph)
        |> Scenic.Scene.push_graph(graph)

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

      graph =
        scene.assigns.graph
        |> Scenic.Graph.modify(
          :color_rect,
          &Scenic.Primitives.rect(&1, {200, 120}, fill: rect_fill)
        )

      scene =
        scene
        |> Scenic.Scene.assign(:rect_fill, rect_fill)
        |> Scenic.Scene.assign(:graph, graph)
        |> Scenic.Scene.push_graph(graph)

      {:noreply, scene}
    end

    defp build_graph do
      Scenic.Graph.build()
      |> rect({200, 120}, id: :color_rect, fill: :blue, translate: {50, 50})
      |> circle(3, id: :cursor_dot, fill: :white, translate: {0, 0})
      |> text("Skia DRM", fill: :yellow, translate: {60, 90})
      |> text("cursor_pos: none", id: :cursor_pos_text, fill: :white, translate: {60, 140})
      |> text("cursor_button: none", id: :cursor_button_text, fill: :white, translate: {60, 165})
      |> text("key: none", id: :key_text, fill: :white, translate: {60, 190})
      |> text("cursor_visible: true (press 'c' to toggle)",
        id: :cursor_toggle_text,
        fill: :white,
        translate: {60, 215}
      )
      |> text("viewport_size: none",
        id: :viewport_size_text,
        fill: :white,
        translate: {60, 265}
      )
      |> analog_clock(radius: 50, seconds: true, translate: {800, 160}, theme: :light)
      |> digital_clock(
        format: :hours_12,
        seconds: true,
        translate: {50, 240},
        font: :roboto_mono,
        font_size: 18,
        fill: :white
      )
    end

    defp current_viewport_size(scene) do
      {:ok, info} = Scenic.ViewPort.info(scene.viewport)
      info.size
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

    defp toggle_cursor(visible) do
      with pid when is_pid(pid) <- Process.whereis(:skia_driver),
           renderer <- Scenic.Driver.Skia.renderer_handle(pid) do
        if visible,
          do: Scenic.Driver.Skia.show_cursor(renderer),
          else: Scenic.Driver.Skia.hide_cursor(renderer)
      end
    end
  end

  def run do
    drm_card = parse_drm_card(System.argv())
    {:ok, _} = DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one)

    driver_opts =
      case drm_card do
        nil -> []
        path -> [drm: [card: path]]
      end

    {:ok, _vp} =
      Scenic.ViewPort.start(
        size: {2560, 1440},
        default_scene: DemoScene,
        drivers: [
          [module: Scenic.Driver.Skia, name: :skia_driver, backend: :drm] ++ driver_opts
        ]
      )

    Process.sleep(:infinity)
  end

  defp parse_drm_card(args) do
    case args do
      ["--device", path] -> path
      ["--device=" <> path] -> path
      _ -> nil
    end
  end
end

ScenicDriverSkia.DemoDrm.run()
