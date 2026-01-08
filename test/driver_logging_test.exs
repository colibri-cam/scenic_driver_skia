defmodule ScenicDriverSkia.DriverLoggingTest do
  use ExUnit.Case, async: false

  import ExUnit.CaptureLog
  import Scenic.Primitives

  alias Scenic.Graph
  alias Scenic.ViewPort
  alias ScenicDriverSkia.TestSupport.ViewPort, as: ViewPortHelper

  defmodule AlternateScene do
    use Scenic.Scene

    def init(scene, _args, _opts) do
      {:ok, scene}
    end
  end

  test "logs driver callbacks when viewport events occur" do
    parent = self()

    log_start =
      capture_log(fn ->
        vp = ViewPortHelper.start()
        send(parent, {:viewport, vp})
        Process.sleep(50)
      end)

    assert_receive {:viewport, vp}

    assert log_start =~ "ScenicDriverSkia.Driver init"
    assert log_start =~ "request_input"
    assert log_start =~ "clear_color"

    log_update =
      capture_log(fn ->
        {:ok, _} = ViewPort.put_graph(vp, :test_graph, simple_graph())
        Process.sleep(50)
      end)

    assert log_update =~ "update_scene"

    log_delete =
      capture_log(fn ->
        :ok = ViewPort.del_graph(vp, :test_graph)
        Process.sleep(50)
      end)

    assert log_delete =~ "del_scripts"

    log_reset =
      capture_log(fn ->
        :ok = ViewPort.set_root(vp, AlternateScene)
        Process.sleep(50)
      end)

    assert log_reset =~ "reset_scene"

    :ok = ViewPort.stop(vp)
  end

  defp simple_graph do
    Graph.build()
    |> rect({10, 10}, translate: {5, 5})
  end
end
