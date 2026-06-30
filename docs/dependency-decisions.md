# Dependency Decisions

## iyes_progress 0.17.0

- Status: introduced.
- Reason: `iyes_progress 0.17` tracks Bevy `0.19` and is designed for loading
  screens that track asset/world preparation before transitioning to the next
  app state.
- Current integration: `AppScreen::AssetLoading` is the default startup state.
  `ProgressPlugin::<AppScreen>` tracks startup assets with
  `with_asset_tracking()` and transitions to `AppScreen::MainMenu` when the
  tracked work completes.
- Scope: startup preload currently covers UI font/backgrounds, faction emblems,
  cursor assets, registry icons, migrated GLB world assets, and the Godot model
  mapping asset. Headless tests opt out through `StartupLoadingPolicy` so they
  do not require render asset preparation.
- Migration rule: use this same state-transition pattern for any future
  match-specific loading state instead of ad hoc timers or manual asset polling.

## moonshine-kind 0.5.1

- Status: introduced.
- Reason: `moonshine-kind` tracks Bevy `0.19` and provides `Instance<T>` typed
  entity references, which are a good fit for places where naked `Entity` values
  have caused harness drift.
- Current integration: model harness root entities are spawned and passed as
  `Instance<ModelHarnessRoot>`, and the harness-owned
  `MODEL_HARNESS_ENTITY_IDS` const table has a length check against the runtime
  registry. This lets the compiler catch harness coverage shape changes.
- Scope: intentionally narrow for now. Use it first at harness and editor-like
  boundaries where a typed root gives compile-time clarity without rewriting the
  live RTS simulation.

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
