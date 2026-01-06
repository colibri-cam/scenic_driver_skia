defmodule ScenicDriverSkia do
  @moduledoc """
  Scenic driver wrapper that delegates rendering to a Rust NIF implemented with Rustler.
  """

  alias ScenicDriverSkia.Native

  @doc """
  Start the renderer with the provided backend. Accepts `:wayland` or `:drm`.
  """
  @spec start() :: :ok | {:error, term()}
  def start, do: start(:wayland)

  @spec start(:wayland | :drm | String.t()) :: :ok | {:error, term()}
  def start(backend) when is_atom(backend) or is_binary(backend) do
    backend
    |> normalize_backend()
    |> Native.start()
    |> normalize_start_result()
  end

  defp normalize_start_result(:ok), do: :ok
  defp normalize_start_result({:ok, _}), do: :ok
  defp normalize_start_result({:error, _} = error), do: error
  defp normalize_start_result(other), do: {:error, {:unexpected_result, other}}

  defp normalize_backend(backend) do
    backend
    |> to_string()
    |> String.downcase()
    |> case do
      "kms" -> "drm"
      other -> other
    end
  end
end
