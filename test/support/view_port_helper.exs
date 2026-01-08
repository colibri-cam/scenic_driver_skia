defmodule ScenicDriverSkia.TestSupport.ViewPort do
  alias Scenic.ViewPort

  defmodule DefaultScene do
    use Scenic.Scene

    def init(scene, _args, _opts) do
      {:ok, scene}
    end
  end

  def start(opts \\ []) do
    ensure_viewport_supervisor()

    scene = Keyword.get(opts, :scene, DefaultScene)
    drivers = Keyword.get(opts, :drivers, [[module: ScenicDriverSkia.Driver, name: :skia_driver]])
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
      {:ok, _pid} -> :ok
      {:error, {:already_started, _pid}} -> :ok
    end
  end
end
