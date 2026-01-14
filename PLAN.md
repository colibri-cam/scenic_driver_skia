# Scenic Driver Skia Plan

## Goal
Implement a cache-based Scenic script renderer using Rust/Skia via Rustler, with shared backends (Wayland/DRM/raster) and enough primitives and IO to behave like Scenic's local driver for real apps.

## Current Status
Completed:
- Script cache in Rust keyed by script id; renderer replays cached ops per redraw.
- `draw_script` resolves cached sub-scripts with push/pop draw-state stack.
- Parser supports `fill_color`, `translate`, `draw_rect`, `draw_text`, line, circle, stroke, and text style ops.
- Driver submits scripts by id and deletes stale scripts.
- Raster backend validated with `raster_output`; Wayland/DRM share the same render state.
- Assets module with local fonts + aliases.
- Script ingestion tests in Rust.

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

## Next
1. **DRM input integration**
   - Choose evdev or reuse Scenic local input pipeline.
   - Translate events through the shared translation module.
2. **Cursor support**
   - Track cursor visibility and last position in driver assigns.
   - Implement show/hide cursor and cursor position updates for input events.
3. **Window resize flow (DRM, if needed)**
   - Confirm whether DRM needs explicit resize handling beyond initial modeset (hotplug).
4. **Viewport options wiring**
   - Wire `window` options (title, resizeable) to backend initialization.
   - Use viewport size for initial window sizing (fallback to defaults).
5. **Script opcode parity (rendering coverage)**
   - **Paths & geometry**: add path primitives (move/line/curve/close), rounded rects, ellipses, arcs.
   - **Paint features**: gradients, image patterns, alpha/opacity, blend modes, stroke caps/joins/dashes.
   - **Images/bitmaps**: implement image draw ops and texture/stream asset handling similar to Scenic local driver `put_texture` flow.
   - **Clipping**: implement clip rect/path and save/restore semantics to match Scenic script behavior.
6. **Asset pipeline completeness**
   - Implement streaming asset updates and caching for images/bitmaps from Scenic assets.
   - Validate font aliasing and fallback behavior with Scenic defaults in `ASSETS.md`.
7. **Backend polish & correctness**
   - Wayland: honor resizeable and other window options; ensure redraw scheduling on updates.
   - DRM: confirm atomic commit error paths, consider robust mode selection, and add device selection overrides as needed.
   - Raster: replace deprecated Skia image encode API, ensure deterministic output for tests.
8. **Performance**
   - Consider caching Skia `Picture`s per script id for static sub-graphs.
   - Reduce allocations in script parsing and replay; avoid repeated font loads.
   - Track script dependencies to only redraw affected sub-graphs when possible.
9. **Testing**
   - Add integration tests: render script to raster, validate output properties (dimensions, non-empty image).
   - Add regression test for `draw_script` recursion guard.
   - Add smoke tests for input events and window resize behavior.
