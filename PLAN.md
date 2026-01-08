# Scenic Driver Skia Plan

## Goal
Implement the shortest path to display a Scenic rectangle on screen using a Rust/Skia renderer via Rustler, with a container-friendly raster backend for validation.

## Incremental Steps
1. **Script ingestion (Elixir)**
   - In `update_scene/2`, fetch scripts from the ViewPort and serialize them.
   - Send serialized scripts to the NIF (`submit_script/1`).
   - Add tests that validate the serialized output for a simple rectangle.

2. **Minimal script parser (Rust)**
   - Parse `fill_color`, `translate`, and `draw_rect` ops from the Scenic script binary.
   - Skip `draw_script` ops so root graphs don’t fail parsing.
   - Store render state (clear color, fill, translate, rect) and emit a rectangle draw call.
   - Add Rust unit tests for opcode parsing and edge cases.

3. **Skia draw path**
   - Use the parsed ops to draw a filled rectangle on the Skia canvas.
   - Trigger redraw on script updates (Wayland, DRM, and raster).
   - Add an integration test that starts a ViewPort with a rect graph and asserts no errors (log-based).

4. **Thin lifecycle wiring**
   - Ensure driver start/stop aligns with renderer lifecycle.
   - Keep backend choice (Wayland/DRM) stable while rendering the rectangle.
   - Add tests that ensure start/stop still work with script updates.

5. **Next minimal rendering features**
   - Add scale/rotate/transform op support.
   - Add stroke support (width + color).
   - Expand script op coverage for text/image to unblock basic UI.
   - Replace deprecated Skia image encode API in the raster backend.

## Done So Far
- Driver module ingests scripts and forwards them to the Rust NIF.
- Rust parser handles `fill_color`, `translate`, and `draw_rect`; skips `draw_script`.
- Raster backend added for container testing with a `raster_output` option.
- Option validation implemented and tested.
- Demo scripts for Wayland and raster backends added.
