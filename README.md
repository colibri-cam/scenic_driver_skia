# Scenic.Driver.Skia

WIP: This driver is under active development and is not production-ready.

Disclaimer: Most of the code in this repository is AI generated.

This is a Scenic driver that renders through Skia. It includes a Rust core (Rustler) and an
Elixir wrapper used by Scenic applications.

## Installation

This project is currently developed from source. Add it as a path dependency:

```elixir
def deps do
  [
    {:scenic_driver_skia, path: "../scenic_driver_skia"}
  ]
end
```

The repository expects local `scenic` and `scenic_clock` paths (see `mix.exs`), so keep
those repos alongside this one or adjust the paths for your environment.

## Configuration

Configure the driver on a `ViewPort`:

```elixir
[
  drivers: [
    [
      module: ScenicDriverSkia.Driver,
      name: :skia_driver,
      backend: :wayland,
      debug: false,
      raster_output: "tmp/output.png",
      window: [title: "Scenic Window", resizeable: false]
    ]
  ]
]
```

Options are validated with `NimbleOptions`. See `ScenicDriverSkia.Driver` for the full
schema and defaults.

## Backends

The driver can target different rendering backends:

- `SCENIC_BACKEND=wayland` (default) renders through a Wayland window.
- `SCENIC_BACKEND=drm` renders directly on Linux DRM hardware.
  - Override the DRM device path with `SCENIC_DRM_CARD` (defaults to `/dev/dri/card0`).
- `SCENIC_BACKEND=raster` renders to an offscreen surface.
  - Set `raster_output: "path/to.png"` to write a PNG.

You can also set the `backend` option in the driver config to `:wayland`, `:drm`, or
`:raster`.

## Prerequisites

- Elixir 1.15+ and a Rust toolchain (for Rustler).
- Scenic and Scenic Clock checked out locally (see `mix.exs`).
- System libraries required by the Skia backend for your platform.

## Documentation

Project notes live in `GUIDES.md` and `ASSETS.md`. No published Hex docs yet.
