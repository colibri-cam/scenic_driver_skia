defmodule Scenic.Driver.Skia.TestSupport.ViewPort do
  alias Scenic.ViewPort

  defmodule DefaultScene do
    use Scenic.Scene

    def init(scene, _args, _opts) do
      {:ok, scene}
    end
  end

  def start(opts \\ []) do
    ensure_viewport_supervisor()
    ensure_renderer_stopped()

    scene = Keyword.get(opts, :scene, DefaultScene)

    drivers =
      Keyword.get(opts, :drivers, [
        [module: Scenic.Driver.Skia, name: :skia_driver, backend: :raster]
      ])

    size = Keyword.get(opts, :size, {200, 120})

    {:ok, %ViewPort{} = vp} =
      ViewPort.start(
        size: size,
        default_scene: scene,
        drivers: drivers
      )

    vp
  end

  defp ensure_viewport_supervisor do
    case DynamicSupervisor.start_link(name: :scenic_viewports, strategy: :one_for_one) do
      {:ok, pid} ->
        Process.unlink(pid)
        :ok

      {:error, {:already_started, _pid}} ->
        :ok
    end
  end

  defp ensure_renderer_stopped do
    case Scenic.Driver.Skia.Native.stop() do
      :ok -> :ok
      {:ok, _} -> :ok
      {:error, "renderer not running"} -> :ok
      {:error, _reason} -> :ok
      _ -> :ok
    end
  end
end
