defmodule Scenic.Driver.Skia.Native do
  use Rustler, otp_app: :scenic_driver_skia, crate: "scenic_driver_skia"

  @doc false
  def start(_backend), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def stop, do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_text(_text), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def submit_script(_script), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def submit_script_with_id(_id, _script), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def submit_scripts(_scripts), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def del_script(_id), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_clear_color(_color), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def reset_scene, do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_raster_output(_path), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def script_count, do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_input_mask(_mask), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def drain_input_events, do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_input_target(_pid), do: :erlang.nif_error(:nif_not_loaded)
end
