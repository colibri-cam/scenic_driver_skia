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

    scene = Keyword.get(opts, :scene, DefaultScene)

    drivers =
      Keyword.get(opts, :drivers, [
        [module: Scenic.Driver.Skia, backend: :raster]
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

  def driver_pid(%ViewPort{pid: pid}) do
    wait_for_driver(pid, 40)
  end

  def renderer(%ViewPort{} = vp) do
    vp
    |> driver_pid()
    |> Scenic.Driver.Skia.renderer_handle()
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

  defp wait_for_driver(pid, attempts_remaining) when attempts_remaining > 0 do
    %{driver_pids: driver_pids} = :sys.get_state(pid)

    case driver_pids do
      [driver_pid | _] ->
        driver_pid

      [] ->
        Process.sleep(50)
        wait_for_driver(pid, attempts_remaining - 1)
    end
  end

  defp wait_for_driver(pid, _attempts_remaining) do
    raise "failed to find driver pid for viewport #{inspect(pid)}"
  end
end
