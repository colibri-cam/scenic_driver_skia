defmodule ScenicDriverSkia.BenchScriptIngest do
  import Scenic.Primitives

  alias Scenic.Graph
  alias Scenic.Graph.Compiler
  alias Scenic.Script
  alias Scenic.Driver.Skia.Native

  def run(opts \\ []) do
    rects = Keyword.get(opts, :rects, [50, 200, 800])
    iterations = Keyword.get(opts, :iterations, 1_000)
    warmup = Keyword.get(opts, :warmup, 100)

    renderer =
      case Native.start("raster", nil, "Scenic Window", false, nil, true, false) do
        {:ok, renderer} -> renderer
        other -> raise "start returned #{inspect(other)}"
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
          |> Native.submit_script(renderer)
          |> case do
            :ok -> :ok
            {:ok, _} -> :ok
          end
        end)

        run_bench("submit_script (binary)", warmup, iterations, fn ->
          :ok = Native.submit_script(renderer, binary)
        end)

      end)
    after
      _ = Native.stop(renderer)
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
