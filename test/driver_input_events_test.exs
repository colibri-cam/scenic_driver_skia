defmodule Scenic.Driver.Skia.InputEventsTest do
  use ExUnit.Case, async: false

  alias Scenic.Driver.Skia.Native
  alias Scenic.Driver.Skia.TestSupport.ViewPort, as: ViewPortHelper
  alias Scenic.ViewPort
  alias ExImageInfo

  test "drains input events while raster backend is running" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    case Native.start("raster", nil, "Scenic Window", false) do
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

  test "raster output matches viewport size" do
    assert {:ok, _} = Application.ensure_all_started(:scenic_driver_skia)

    output_path =
      Path.join(
        System.tmp_dir!(),
        "scenic_driver_skia_raster_#{System.unique_integer([:positive])}.png"
      )

    viewport_size = {321, 123}

    vp = ViewPortHelper.start(size: viewport_size)

    on_exit(fn ->
      if Process.alive?(vp.pid) do
        _ = ViewPort.stop(vp)
      end

      _ = Native.stop()
      _ = File.rm(output_path)
    end)

    case Native.set_raster_output(output_path) do
      :ok -> :ok
      {:ok, _} -> :ok
      other -> flunk("set_raster_output returned #{inspect(other)}")
    end

    wait_for_file!(output_path, 40)

    assert {_, width, height, _} = ExImageInfo.info(File.read!(output_path), :png)
    assert {width, height} == viewport_size
  end

  defp wait_for_file!(path, attempts_remaining) do
    cond do
      File.exists?(path) ->
        :ok

      attempts_remaining > 0 ->
        Process.sleep(50)
        wait_for_file!(path, attempts_remaining - 1)

      true ->
        flunk("timed out waiting for raster output at #{path}")
    end
  end
end
