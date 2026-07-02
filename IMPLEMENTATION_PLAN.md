# Plan: split src/lib.rs (34k lines) into domain modules

Pure-move refactor: no logic changes, one module per stage, every stage compiles,
passes `cargo test --lib`, and is committed separately. Modules re-export into the
crate root (`pub(crate) use module::*;`) so call sites and the capture/test API
stay unchanged.

## Stage 0: Visibility groundwork
**Goal**: all top-level items in lib.rs become `pub(crate)` (already-`pub` capture
API unchanged) so items can move between modules without dependency untangling.
**Success**: bin builds warning-free; `cargo test --lib` passes; zero behavior diff.
**Status**: Complete

## Stage 1: nav.rs (pilot)
**Goal**: NavGrid + A* + PlannedPath + unit separation + their consts/tests.
**Success**: same as Stage 0; nav tests live in nav.rs.
**Status**: Complete

## Stage 2: camera.rs
**Goal**: RtsCamera resource + bevy_rts_camera bridge + camera settings/bookmarks.
**Status**: Not Started

## Stage 3: audio.rs
**Goal**: AudioFeedback, SoundEffectKind, UnitVoiceEvent, play systems.
**Status**: Not Started

## Stage 4: menu.rs
**Goal**: front menu, options, credits, skirmish lobby (setup_main_menu + helpers).
**Status**: Not Started

## Stage 5: hud.rs
**Goal**: in-match HUD (setup_ui, minimap, battle log, command card, selection,
HudHitZones), and the HUD update systems.
**Status**: Not Started

## Stage 6: capture_api.rs
**Goal**: the `pub fn capture_*` harness surface.
**Status**: Not Started

## Later stages (same pattern)
combat.rs, economy.rs, orders.rs, ai.rs — carve once the above land.

## Rules
- Moves only; if a change is not a move or a visibility keyword, it does not
  belong in these commits.
- Each module owns its `use` imports; the crate root keeps
  `pub(crate) use module::*;` re-exports so nothing else changes.
- Remove this file when all stages are done.
