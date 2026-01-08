# Scenic Driver Skia Plan

## Goal
Implement a Scenic driver backed by a Rust Skia renderer via Rustler, starting with a minimal driver that only logs callbacks and then layering in rendering and input support.

## Incremental Steps
1. **Scaffold driver module**
   - Provide a Scenic driver module that logs each callback.
   - Ensure the module can be registered as a ViewPort driver.
   - Add tests that start a ViewPort and assert logging callbacks.

2. **Define and validate options**
   - Add an options schema (backend selection, window settings, debug flags).
   - Validate options using `NimbleOptions`.
   - Add tests that verify option validation errors and defaults.

3. **Rust NIF integration**
   - Add NIF APIs for start/stop and basic render lifecycle.
   - Ensure Elixir driver starts/stops the renderer cleanly.
   - Add tests that exercise NIF start/stop from the driver.

4. **Script handling**
   - Fetch scripts from the ViewPort and serialize them.
   - Feed scripts into the renderer incrementally (per update batch).
   - Add tests that confirm script updates trigger renderer calls.

5. **Input plumbing**
   - Forward requested inputs to the backend.
   - Emit Scenic input events from backend sources.
   - Add tests for requested input updates and emitted input events.

6. **Backends and cleanup**
   - Support Wayland and DRM/KMS paths behind a clean abstraction.
   - Ensure proper shutdown and resource cleanup across backends.
   - Add tests that cover backend selection and shutdown paths.

## Done So Far
- Driver module logs callbacks and returns driver state.
- Local Scenic dependency wired for development.
