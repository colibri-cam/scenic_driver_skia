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

## Prioritized Plan
1. **Close functional gaps vs Scenic driver local (highest priority)**
   - **A. Input capability plumbing**
     - Map `request_input/2` options to a driver-side input mask.
     - Store requested inputs in driver state and pass to backends.
   - **B. Input event source integration**
     - Wayland: translate winit events into Scenic inputs (key, codepoint, cursor_pos, cursor_button, cursor_scroll, viewport enter/leave).
     - DRM: choose evdev or reuse Scenic local input pipeline; implement matching translation.
     - Raster: no input; treat requests as no-ops with clear logging.
   - **C. Event translation layer**
     - Create a backend-agnostic Rust module that maps raw events to Scenic input events.
     - Keep the translation logic independent of windowing backends.
   - **D. Elixir input dispatch**
     - Add a Rustler NIF to push input events into the driver.
     - In `lib/scenic/driver/skia.ex`, gate events by requested inputs and call `send_input/2`.
   - **E. Cursor support**
     - Track cursor visibility and last position in driver assigns.
     - Implement show/hide cursor and cursor position updates for input events.
   - **F. Window resize flow**
     - Wayland: on resize, update the Skia surface, store new size, and request redraw.
     - Notify viewport when size changes as required by Scenic.
   - **G. Viewport options wiring**
     - Wire `window` options (title, resizeable) to backend initialization.
     - Use viewport size for initial window sizing (fallback to defaults).

2. **Script opcode parity (rendering coverage)**
   - **Paths & geometry**: add path primitives (move/line/curve/close), rounded rects, ellipses, arcs.
   - **Paint features**: gradients, image patterns, alpha/opacity, blend modes, stroke caps/joins/dashes.
   - **Images/bitmaps**: implement image draw ops and texture/stream asset handling similar to Scenic local driver `put_texture` flow.
   - **Clipping**: implement clip rect/path and save/restore semantics to match Scenic script behavior.

3. **Asset pipeline completeness**
   - Implement streaming asset updates and caching for images/bitmaps from Scenic assets.
   - Validate font aliasing and fallback behavior with Scenic defaults in `ASSETS.md`.

4. **Backend polish & correctness**
   - Wayland: honor resizeable and other window options; ensure redraw scheduling on updates.
   - DRM: confirm atomic commit error paths, consider robust mode selection, and add device selection overrides as needed.
   - Raster: replace deprecated Skia image encode API, ensure deterministic output for tests.

5. **Performance**
   - Consider caching Skia `Picture`s per script id for static sub-graphs.
   - Reduce allocations in script parsing and replay; avoid repeated font loads.
   - Track script dependencies to only redraw affected sub-graphs when possible.

6. **Testing**
   - Add integration tests: render script to raster, validate output properties (dimensions, non-empty image).
   - Add regression test for `draw_script` recursion guard.
   - Add smoke tests for input events and window resize behavior.
