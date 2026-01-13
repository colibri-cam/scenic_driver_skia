defmodule Scenic.Driver.Skia.InputEventsTest do
  use ExUnit.Case, async: false

  alias Scenic.Driver.Skia.Native

  test "drains input events while raster backend is running" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    case Native.start("raster") do
      :ok -> :ok
      {:ok, _} -> :ok
      other -> flunk("start returned #{inspect(other)}")
    end

    on_exit(fn ->
      _ = Native.stop()
    end)

    case Native.set_input_mask(0x01) do
      :ok -> :ok
      {:ok, _} -> :ok
      other -> flunk("set_input_mask returned #{inspect(other)}")
    end

    case Native.drain_input_events() do
      [] -> :ok
      {:ok, []} -> :ok
      other -> flunk("drain_input_events returned #{inspect(other)}")
    end
  end
end
