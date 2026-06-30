# Dependency Decisions

## bevy_fluent 0.15.0

- Status: introduced.
- Reason: `bevy_fluent 0.15` tracks Bevy `0.19` and registers Fluent `.ftl` /
  bundle asset loaders through `FluentPlugin`.
- Current integration: `add_game_scenes()` registers `FluentPlugin` so the
  runtime and capture apps can load Fluent assets.
- Migration rule: keep the existing lightweight `Locale`/`t()` path for current
  UI until text keys are organized. Move screens to Fluent incrementally by
  adding `assets/locales/<locale>/*.ftl` bundles and replacing only one screen
  at a time.

## bevy_egui 0.40.1

- Status: researched, not introduced.
- Compatibility: `bevy_egui 0.40.1` tracks Bevy `0.19`.
- Good fit: developer/debug tooling such as AI inspector panels, entity stat
  overlays, map/model harness controls, and temporary tuning windows.
- Poor fit for shipped menu/lobby UI: the project already has styled Bevy UI
  screens matching the Godot/RTS direction, and `bevy_egui` default features add
  clipboard, URL opening, render, Bevy UI bridge, and picking integration.
- Native concern: the crate README documents Linux XCB package requirements for
  its default desktop path, while this project intentionally targets Wayland-only
  native runtime.
- Decision: do not use `bevy_egui` for the front menu, lobby, settings, or
  command panel. If debug tooling needs it later, add it behind a dedicated
  feature with `default-features = false` and the smallest feature set that
  still renders the debug overlay.
