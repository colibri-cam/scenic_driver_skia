# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Scenic Driver Skia is a Scenic GUI driver that renders through Skia. It's an Elixir + Rust hybrid project using Rustler for NIF bindings. The driver supports multiple rendering backends: Wayland (windowed), DRM (direct framebuffer), and Raster (offscreen).

**Status:** WIP - Under active development, mostly AI-generated code.

## Build Commands

```bash
# Elixir
mix deps.get          # Install dependencies
mix compile           # Build (includes Rust via Rustler)
mix test              # Run all tests
mix format            # Format Elixir code

# Rust (from native/scenic_driver_skia/)
cargo fmt             # Format Rust code
cargo check           # Verify compilation
cargo clippy -- -D warnings  # Lint (required before commits)

# Demos
mix run scripts/demo_wayland.exs   # Visual primitive demo
mix run scripts/demo_drm.exs       # DRM + input demo
mix run scripts/demo_raster.exs    # Offscreen rendering
```

## Architecture

### Data Flow
1. Scenic ViewPort updates scripts
2. Elixir serializes scripts to binary via `Scenic.Script.serialize/1`
3. Binary sent to Rust via `Native.submit_scripts/2`
4. Rust parses binary into `Vec<ScriptOp>`, caches by script ID
5. On redraw: renderer clears canvas, looks up `_root_` script, replays cached ops
6. `DrawScript` ops recursively resolve referenced scripts (with cycle detection)

### Key Files
- `lib/scenic/driver/skia.ex` - Main driver (GenServer, Scenic.Driver impl)
- `lib/scenic/driver/skia/native.ex` - NIF bindings to Rust
- `native/scenic_driver_skia/src/lib.rs` - NIF entry points, script management
- `native/scenic_driver_skia/src/renderer.rs` - Script parsing and render replay
- `native/scenic_driver_skia/src/backend.rs` - Wayland backend (winit/glutin)
- `native/scenic_driver_skia/src/drm_backend.rs` - DRM direct rendering
- `native/scenic_driver_skia/src/raster_backend.rs` - Offscreen rendering

### Script Operations
The `ScriptOp` enum in `renderer.rs` defines all drawable operations:
- State: `PushState`, `PopState`, `PopPushState`
- Transform: `Translate`, `Rotate`, `Scale`, `Transform`
- Paint: `FillColor`, `StrokeColor`, `FillLinear`, `FillRadial`, `FillImage`, `FillStream`
- Drawing: `DrawRect`, `DrawCircle`, `DrawLine`, `DrawText`, path ops, etc.
- Clipping: `Scissor`, `ClipPath`

## Adding a New Drawing Operation

1. Add variant to `ScriptOp` enum in `renderer.rs`
2. Implement parsing in `parse_op()`
3. Implement rendering in `replay_op()`
4. Add demo scene in `scripts/demo_wayland.exs`
5. Add raster tests in `test/driver_raster_primitives_test.exs` validating:
   - Translated primitive renders at expected position
   - Stroke edge pixels match stroke color
   - Fill interior pixels are correct
   - Background pixels are unchanged

## Testing Requirements

- Run `mix test` for Elixir tests
- Raster tests use `ViewPortHelper` to capture frames and validate pixel colors
- Add/update tests with each change and keep them passing
- Always address warnings rather than ignoring them

## Dependencies

- Requires local Scenic at `/workspace/scenic` (path override in mix.exs)
- Tool versions pinned in `mise.toml`: Elixir 1.19.4, Erlang 28.3
- Rustler 0.37, skia-safe 0.91.1

## Additional Documentation

- `GUIDES.md` - Detailed architecture and data flow
- `ASSETS.md` - Asset pipeline and font aliases
- `AGENTS.md` - Development conventions
- `PLAN.md` - Development roadmap
