# Agent Guidelines

This repository contains a Rust project with an Elixir wrapper. Please follow these conventions when modifying files in this repo:

- Run `cargo fmt` before committing changes.
- Run `cargo check` to ensure the Rust code compiles.
- Run `mix format` and `mix test` to validate Elixir changes.
- Add or update tests with each incremental step and keep them passing.
- Document test commands you run in your final summary, noting any failures or external blockers.
- Source `~/.bashrc` to activate `mise` before running `mix` tasks if needed.
- Prefer small, focused modules. Keep rendering logic backend-agnostic and isolate backend/windowing concerns in their own modules.
- Avoid adding unnecessary dependencies; prefer the standard library where practical.

These instructions apply to all files in this repository.

Backends:
- `SCENIC_BACKEND=wayland` (default) renders through a Wayland window.
- `SCENIC_BACKEND=drm` renders directly on Linux DRM hardware (tested with AMD GPUs).
  - Override the DRM device path with `SCENIC_DRM_CARD` (defaults to `/dev/dri/card0`).
- `SCENIC_BACKEND=raster` renders to an offscreen surface (container-friendly).
  - Set the driver option `raster_output: "path/to.png"` to write a PNG.
