# Agent Guidelines

This repository contains a Rust project with an Elixir wrapper. Please follow these conventions when modifying files in this repo:

- Run `cargo fmt` before committing changes.
- Run `cargo check` to ensure the Rust code compiles.
- Run `cargo clippy -- -D warnings` to lint Rust changes.
- The Rust `Cargo.toml` lives in `native/scenic_driver_skia`.
- Run `mix format` and `mix test` to validate Elixir changes.
- Add or update tests with each incremental step and keep them passing.
- Document test commands you run in your final summary, noting any failures or external blockers.
- Prefer small, focused modules. Keep rendering logic backend-agnostic and isolate backend/windowing concerns in their own modules.
- Avoid adding unnecessary dependencies; prefer the standard library where practical.
- Reference Scenic source code at `/workspace/scenic` and local driver implementations at `/workspace/scenic_driver_local` when needed.
- Always address warnings (compiler, runtime, or test) rather than ignoring them.
- Keep `scripts/demo_wayland.exs` updated so every implemented script opcode is visible in the demo.
- For every implemented drawing opcode, add a raster test that validates:
  - A translated primitive renders at the expected location.
  - Pixels just outside the stroke are background.
  - Multiple pixels along each stroke edge match the stroke color.
  - At least one pixel inside the shape matches the fill color (when applicable).
- See `GUIDES.md` for architecture and driver data-flow notes.
- See `ASSETS.md` for asset pipeline and font alias details.

These instructions apply to all files in this repository.

Backends:
- `backend: :wayland` renders through a Wayland window.
- `backend: :drm` renders directly on Linux DRM hardware (tested with AMD GPUs).
  - Configure DRM with `drm: [card: "/dev/dri/card0", hw_cursor: true, input_log: false]`.
- `backend: :raster` renders to an offscreen surface (container-friendly).
  - Fetch the latest RGB frame via `Scenic.Driver.Skia.Native.get_raster_frame(renderer)`.
