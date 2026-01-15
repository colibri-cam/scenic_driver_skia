defmodule Scenic.Driver.Skia.Native do
  @rustler_opts Mix.Project.config()[:rustler_opts]

  use Rustler,
      Keyword.merge(
        [
          otp_app: :scenic_driver_skia,
          crate: "scenic_driver_skia",
          mode: if(Mix.env() == :prod, do: :release, else: :debug)
        ],
        @rustler_opts
      )

  @doc false
  def start(
        _backend,
        _viewport_size,
        _window_title,
        _resizeable,
        _drm_card,
        _drm_hw_cursor,
        _drm_input_log
      ),
      do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def stop(_renderer), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_text(_renderer, _text), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def submit_script(_renderer, _script), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def submit_script_with_id(_renderer, _id, _script), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def submit_scripts(_renderer, _scripts), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def del_script(_renderer, _id), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def put_static_image(_renderer, _id, _data), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def put_font(_renderer, _id, _data), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def put_stream_texture(_renderer, _id, _format, _width, _height, _data),
    do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def del_stream_texture(_renderer, _id), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_clear_color(_renderer, _color), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def reset_scene(_renderer), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def get_raster_frame(_renderer), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def script_count(_renderer), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_input_mask(_renderer, _mask), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def drain_input_events(_renderer), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def set_input_target(_renderer, _pid), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def show_cursor(_renderer), do: :erlang.nif_error(:nif_not_loaded)

  @doc false
  def hide_cursor(_renderer), do: :erlang.nif_error(:nif_not_loaded)
end
