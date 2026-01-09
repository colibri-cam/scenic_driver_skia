defmodule ScenicDriverSkia.DriverScriptSerializationTest do
  use ExUnit.Case, async: true

  import Scenic.Primitives

  alias Scenic.Graph
  alias Scenic.Graph.Compiler
  alias Scenic.Script

  test "serializes a translated, filled rectangle to expected bytes" do
    graph =
      Graph.build()
      |> rect({10, 12}, fill: :red, translate: {5, 6})

    {:ok, script} = Compiler.compile(graph)
    binary = Script.serialize(script) |> IO.iodata_to_binary()

    expected =
      <<
        0x00,
        0x40,
        0x00,
        0x00,
        0x00,
        0x53,
        0x00,
        0x00,
        5.0::float-32-big,
        6.0::float-32-big,
        0x00,
        0x60,
        0x00,
        0x00,
        255,
        0,
        0,
        255,
        0x00,
        0x04,
        0x00,
        0x01,
        10.0::float-32-big,
        12.0::float-32-big,
        0x00,
        0x41,
        0x00,
        0x00
      >>

    assert binary == expected
  end
end
