defmodule Scenic.Driver.Skia.DriverLoggingTest do
  use ExUnit.Case, async: false

  import ExUnit.CaptureLog
  import Scenic.Primitives

  alias Scenic.Graph
  alias Scenic.ViewPort
  alias Scenic.Driver.Skia.TestSupport.ViewPort, as: ViewPortHelper

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
        Process.sleep(200)
      end)

    assert_receive {:viewport, vp}

    assert log_start =~ "Scenic.Driver.Skia init"
    assert log_start =~ "clear_color"

    _log_update =
      capture_log(fn ->
        {:ok, _} = ViewPort.put_graph(vp, :test_graph, simple_graph())
        Process.sleep(200)
      end)

    _log_delete =
      capture_log(fn ->
        :ok = ViewPort.del_graph(vp, :test_graph)
        Process.sleep(200)
      end)

    _log_reset =
      capture_log(fn ->
        :ok = ViewPort.set_root(vp, AlternateScene)
        Process.sleep(200)
      end)

    :ok = ViewPort.stop(vp)
  end

  defp simple_graph do
    Graph.build()
    |> rect({10, 10}, fill: :red)
  end
end
