# Scenic Driver Skia Plan

## Goal
Implement a cache-based Scenic script renderer using Rust/Skia via Rustler, with shared backends (Wayland/DRM/raster) and enough primitives and IO to behave like Scenic's local driver for real apps.

## Current Status
Completed:
- Script cache in Rust keyed by script id; renderer replays cached ops per redraw.
- `draw_script` resolves cached sub-scripts with push/pop draw-state stack.
- Parser supports `fill_color`, `translate`, `draw_rect`, `draw_text`, line, circle, stroke, and text style ops.
- Driver submits scripts by id and deletes stale scripts.
- Raster backend validated with raw RGB frame capture; Wayland/DRM share the same render state.
- Assets module with local fonts + aliases.
- Script ingestion tests in Rust.
- Added script ops for `draw_ellipse`, `draw_arc`, `draw_sector`, `draw_rrectv`, and path primitives (begin/move/line/arc_to/bezier/quadratic/close + fill/stroke + scissor).
- Added stroke cap/join/miter_limit support for raster/Wayland/DRM.
- Added linear gradient fill/stroke paint support.
- Added radial gradient fill/stroke paint support.
- Added static image and stream paint support.
- Added Script path-shape ops (`triangle`, `quad`, `rect`, `rrect`, `sector`, `circle`, `ellipse`, `arc`) and `draw_sprites` support.
- Added clip-path support for driver scripts with raster coverage.
- Added raster coverage for sprites, stroke image/stream paint, and draw_script recursion guard.
- Added demo coverage for image/stream stroke paints in Wayland.
- Documented stream image update flow in `GUIDES.md`.
- Validated font aliasing details in `ASSETS.md`.
- Stream asset updates refresh textures and trigger redraws.

## Done
1. **Input capability plumbing**
   - Map `request_input/2` options to a driver-side input mask.
   - Store requested inputs in driver state and pass to backends.
2. **Wayland input integration**
   - Translate winit events into Scenic inputs (key, codepoint, cursor_pos, cursor_button, cursor_scroll, viewport enter/leave).
3. **Event translation layer**
   - Backend-agnostic Rust module for key/button/modifier mapping.
4. **Elixir input dispatch**
   - Push-based `:input_ready` notifications and event draining into `send_input/2`.
5. **Input tests**
   - Translation unit tests and input event drain integration test.
6. **Wayland resize flow**
   - Resize GL surface and renderer on window resize.
   - Emit viewport reshape input on size changes.
7. **DRM initial mode selection**
   - Pick closest connector mode to viewport size.
   - Emit viewport reshape input if mode differs from requested size.
8. **Viewport window options (Wayland)**
   - Use `window` options for initial title/resizeable configuration.
9. **DRM input integration**
   - Evdev-based input pipeline with Scenic input translation.
10. **Cursor support**
   - Track cursor visibility and position in Rust.
   - Hardware cursor plane support with software fallback.
11. **Viewport options wiring**
   - Use viewport size for initial window sizing (fallback to defaults).
   - Configure DRM settings via driver options (card path, hw cursor, input logging).
12. **DRM hotplug polling**
   - Periodically rescan connectors and reinitialize on mode/connector changes.

## Next
1. **Script opcode parity (rendering coverage)**
   - **Raster coverage**: expand per-primitive raster tests for upcoming paint, image, and clipping features.
2. **Asset pipeline completeness**
   - Verify stream updates on Wayland/DRM in addition to raster.
7. **Backend polish & correctness**
   - Wayland: honor resizeable and other window options; ensure redraw scheduling on updates.
   - DRM: confirm atomic commit error paths, consider robust mode selection, and add device selection overrides as needed.
   - Raster: ensure deterministic output for tests.
8. **Performance**
   - Consider caching Skia `Picture`s per script id for static sub-graphs.
   - Reduce allocations in script parsing and replay; avoid repeated font loads.
   - Track script dependencies to only redraw affected sub-graphs when possible.
9. **Testing**
   - Add integration tests: render script to raster, validate output properties (dimensions, non-empty image).
   - Add smoke tests for input events and window resize behavior.
