defmodule Scenic.Driver.Skia.DriverLifecycleTest do
  use ExUnit.Case, async: false

  import Scenic.Primitives

  alias Scenic.Graph
  alias Scenic.ViewPort
  alias Scenic.Driver.Skia.TestSupport.ViewPort, as: ViewPortHelper

  test "applies script updates and stops cleanly" do
    vp = ViewPortHelper.start()
    monitor = Process.monitor(vp.pid)

    {:ok, _} = ViewPort.put_graph(vp, :graph_a, graph_a())
    refute_receive {:DOWN, ^monitor, :process, _pid, _reason}, 200

    {:ok, _} = ViewPort.put_graph(vp, :graph_a, graph_b())
    refute_receive {:DOWN, ^monitor, :process, _pid, _reason}, 200

    :ok = ViewPort.del_graph(vp, :graph_a)
    refute_receive {:DOWN, ^monitor, :process, _pid, _reason}, 200

    :ok = ViewPort.stop(vp)
    assert_receive {:DOWN, ^monitor, :process, _pid, _reason}, 500
  end

  defp graph_a do
    Graph.build()
    |> rect({10, 10}, fill: :red)
  end

  defp graph_b do
    Graph.build()
    |> rect({20, 15}, fill: :blue, translate: {5, 5})
  end
end
