# Scenic.Driver.Skia

WIP: This driver is under development and is not production-ready.

Disclaimer: Most of the code in this repository is AI generated.

This is a Scenic driver that renders through Skia. It includes a Rust core (Rustler) and an
Elixir wrapper used by Scenic applications.

## Installation

This project is currently developed from source. Add it as a git dependency:

```elixir
def deps do
  [
    {:scenic_driver_skia, git: "https://github.com/colibri-cam/scenic_driver_skia.git"}
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
      module: Scenic.Driver.Skia,
      name: :skia_driver,
      backend: :wayland,
      debug: false,
      window: [title: "Scenic Window", resizeable: false]
    ]
  ]
]
```

Options are validated with `NimbleOptions`. See `Scenic.Driver.Skia` for the full
schema and defaults.

## Backends

The driver can target different rendering backends:

- `backend: :wayland` renders through a Wayland window.
- `backend: :drm` renders directly on Linux DRM hardware.
  - Configure DRM with `drm: [card: "/dev/dri/card0", hw_cursor: true, input_log: false]`.
- `backend: :raster` renders to an offscreen surface.
  - Fetch the latest RGB frame via `Scenic.Driver.Skia.Native.get_raster_frame(renderer)`.

## Demos

- `mix run scripts/demo_wayland.exs` renders each supported primitive in a Wayland window.
- `mix run scripts/demo_drm.exs` renders and shows input events on DRM.

## What is missing
- running example scenic app on wayland seems to have problems with backspace on text
- still needs to be tested on nerves

## Prerequisites

- Elixir 1.15+ and a Rust toolchain (for Rustler).
- Scenic and Scenic Clock checked out locally (see `mix.exs`).
- System libraries required by the Skia backend for your platform.
- **ninja** build tool - required if pre-built Skia binaries are unavailable for your platform.

### Installing ninja

If the first build fails with `failed to run 'ninja', does it exist in PATH?`, install ninja:

```bash
# Arch Linux
sudo pacman -S ninja

# Debian/Ubuntu
sudo apt install ninja-build

# Fedora
sudo dnf install ninja-build

# macOS
brew install ninja
```

## Documentation

Project notes live in `GUIDES.md` and `ASSETS.md`. No published Hex docs yet.
