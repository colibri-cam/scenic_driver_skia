# Architecture Guide: Script Cache + Render Replay

## Overview
This driver follows Scenic’s local driver model: Scenic scripts are serialized on the Elixir side, sent to Rust, parsed once into a cached op list, and replayed on each redraw. The renderer resolves `draw_script` by looking up cached sub-scripts and replaying their ops with a small style/transform stack.

The primary goals are:
- Keep BEAM↔NIF traffic minimal and binary-only.
- Parse scripts once per update, not per frame.
- Replay cached ops quickly during redraw.

## Data Flow
1. Scenic ViewPort updates the script table.
2. Driver fetches scripts and serializes them with `Scenic.Script.serialize/1`.
3. Driver calls `Native.submit_script_with_id(id, binary)` for each script id.
4. Rust parses the binary into `Vec<ScriptOp>` and stores it in `RenderState.scripts`.
5. Redraw resolves the root script (`_root_`) and replays cached ops on the Skia canvas.

## Key Components
- `lib/scenic_driver_skia/driver.ex`
  - Fetches scripts from the ViewPort.
  - Serializes scripts to binary.
  - Calls `submit_script_with_id/2` and `del_script/1`.

- `native/scenic_driver_skia/src/lib.rs`
  - Parses Scenic script binaries into `ScriptOp` lists.
  - Stores scripts in `RenderState.scripts`.
  - Tracks the root script id (`_root_`).

- `native/scenic_driver_skia/src/renderer.rs`
  - Replays `ScriptOp` lists during `redraw`.
  - Resolves `DrawScript` recursively and prevents cycles.
  - Maintains draw state (fill/stroke/text) plus a canvas transform stack.
  - Handles `DrawText` with font, size, alignment, and baseline.

## Script Parsing
Currently supported ops in Rust:
- `push_state`, `pop_state`, `pop_push_state`
- `translate`, `rotate`, `scale`, `transform`
- `fill_color`, `stroke_color`, `stroke_width`
- `draw_rect`, `draw_rrect`, `draw_rrectv`, `draw_line`, `draw_triangle`, `draw_circle`, `draw_ellipse`, `draw_arc`, `draw_sector`
- `draw_text`, `font`, `font_size`, `text_align`, `text_base`
- `draw_script` (stored as `ScriptOp::DrawScript`)

Unknown ops return an error; add support by:
1. Extending the parser to emit a new `ScriptOp`.
2. Handling the new op in the renderer replay.

## Render Replay Model
The renderer does not parse per frame. It:
- Clears the canvas using `clear_color`.
- Looks up the `_root_` script id.
- Replays cached ops with a draw-state stack and canvas transforms.
- For `DrawScript`, it recursively replays the referenced script.

## Performance Notes
- The driver batches script submissions and only signals one redraw for updates.
- Enable the driver option `debug: true` to log cached script counts periodically.

## Backends
All backends share the same render state:
- Wayland (windowed, GL surface)
- DRM (direct framebuffer)
- Raster (offscreen surface)

The render state is cloned and sent to the backend on updates; the backend redraws from cached ops.

## Assets and Fonts
This driver uses the Scenic static assets pipeline with local sources. Fonts live in
`assets/fonts/` and are aliased to `:roboto` and `:roboto_mono` via the assets module.
The renderer loads font binaries from `priv/__scenic/assets/<hash>` when handling
`font` ops. See `ASSETS.md` for details.

## Extending the Architecture
Recommended next steps:
- Expand `ScriptOp` coverage (stroke, path ops, text, images).
- Add a per-script cache of Skia `Picture` for static scripts.
- Track per-script dependencies to avoid replaying unchanged sub-scripts.
