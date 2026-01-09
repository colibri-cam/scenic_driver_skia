defmodule ScenicDriverSkia.BenchScriptIngest do
  import Scenic.Primitives

  alias Scenic.Graph
  alias Scenic.Graph.Compiler
  alias Scenic.Script
  alias ScenicDriverSkia.Native

  def run(opts \\ []) do
    rects = Keyword.get(opts, :rects, [50, 200, 800])
    iterations = Keyword.get(opts, :iterations, 1_000)
    warmup = Keyword.get(opts, :warmup, 100)

    stop_renderer()
    case Native.start("raster") do
      :ok -> :ok
      {:ok, _} -> :ok
    end

    try do
      Enum.each(rects, fn count ->
        graph = build_graph(count)
        {:ok, script} = Compiler.compile(graph)
        binary = script |> Script.serialize() |> IO.iodata_to_binary()

        IO.puts("\nrects: #{count}")

        run_bench("serialize only", warmup, iterations, fn ->
          script |> Script.serialize() |> IO.iodata_to_binary()
        end)

        run_bench("serialize + submit_script", warmup, iterations, fn ->
          script
          |> Script.serialize()
          |> IO.iodata_to_binary()
          |> Native.submit_script()
          |> case do
            :ok -> :ok
            {:ok, _} -> :ok
          end
        end)

        run_bench("submit_script (binary)", warmup, iterations, fn ->
          case Native.submit_script(binary) do
            :ok -> :ok
            {:ok, _} -> :ok
          end
        end)

      end)
    after
      _ = Native.stop()
    end
  end

  defp build_graph(rects) do
    Enum.reduce(0..(rects - 1), Graph.build(), fn idx, graph ->
      x = rem(idx, 20) * 12
      y = div(idx, 20) * 12
      graph |> rect({10, 10}, fill: :red, translate: {x, y})
    end)
  end

  defp run_bench(label, warmup, iterations, fun) do
    Enum.each(1..warmup, fn _ -> fun.() end)

    {total_us, _} =
      :timer.tc(fn ->
        Enum.each(1..iterations, fn _ -> fun.() end)
      end)

    per_op = total_us / iterations
    IO.puts("#{label}: #{Float.round(per_op, 2)} us/op (#{iterations} iters)")
  end

  defp stop_renderer do
    case Native.stop() do
      :ok -> :ok
      {:ok, _} -> :ok
      {:error, _reason} -> :ok
      _ -> :ok
    end
  end
end

args = System.argv()

rects =
  case args do
    [sizes] ->
      sizes
      |> String.split(",", trim: true)
      |> Enum.map(&String.to_integer/1)

    _ ->
      [50, 200, 800]
  end

ScenicDriverSkia.BenchScriptIngest.run(rects: rects)
