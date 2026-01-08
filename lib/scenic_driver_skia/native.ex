defmodule ScenicDriverSkia.Native do
  use Rustler, otp_app: :scenic_driver_skia, crate: "scenic_driver_skia"

  @doc false
  def start(_backend), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def stop, do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_text(_text), do: :erlang.nif_error(:nif_not_loaded)
end
