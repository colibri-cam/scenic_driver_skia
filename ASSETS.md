# Assets Guide

Scenic assets are compiled into the static assets library at build time. This project
uses a local `assets/` folder and Scenic defaults for fonts.

## Sources
- `assets/` (local, project-owned assets)
- `{ :scenic, "assets" }` (Scenic defaults in `/workspace/scenic/assets`)

## Default Fonts
Scenic expects these fonts and auto-aliases them:
- `fonts/roboto.ttf` -> `:roboto`
- `fonts/roboto_mono.ttf` -> `:roboto_mono`

The local copies live in `assets/fonts/` and are included in the assets module.

## Configuration
Asset configuration is handled by:
- `lib/scenic/driver/skia/assets.ex`
- `config/config.exs`

When you add or change assets, force a recompile by touching
`lib/scenic/driver/skia/assets.ex` or running `mix compile`.
