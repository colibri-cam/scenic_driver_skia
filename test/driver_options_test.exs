defmodule Scenic.Driver.Skia.DriverOptionsTest do
  use ExUnit.Case, async: true

  alias Scenic.Driver.Skia

  test "validate_opts applies defaults and normalizes backend" do
    assert {:ok, opts} = Skia.validate_opts([])
    assert opts[:backend] == "wayland"
    assert opts[:debug] == false
    assert Keyword.get(opts[:window], :title) == "Scenic Window"
    assert Keyword.get(opts[:window], :resizeable) == false

    assert {:ok, opts} = Skia.validate_opts(backend: :drm, debug: true)

    assert opts[:backend] == "drm"
    assert opts[:debug] == true
  end

  test "validate_opts rejects invalid backend type" do
    assert {:error, %NimbleOptions.ValidationError{}} = Skia.validate_opts(backend: 123)
  end

  test "validate_opts rejects invalid window options" do
    assert {:error, %NimbleOptions.ValidationError{}} =
             Skia.validate_opts(window: [resizeable: "nope"])
  end
end
