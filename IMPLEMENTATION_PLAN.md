# Plan: godot-open-rts → bevy migration completion

## Status: migration is ~95% complete (verified by parity audit 2026-06-26)

A 6-cluster parity audit (combat, special units, economy/structures, support
powers/alliances/fog, HUD/UX, AI) compared every godot `source/` subsystem and
manual test against `src/lib.rs`. Result: codex actually ported the vast majority
of mechanics with real behavior (not just registry data). Confirmed DONE at
parity: all unit orders (move/attack-move/auto-attack/hold/scatter/patrol/queued),
splash, veterancy, crushing, wreckage; engineer capture/repair, saboteur
infiltration+power-sabotage, mine layer + mines, mobile shield, siege deploy, MCV
deploy, garrison; harvesting, refinery dropoff + free worker, ore purifier,
tech oil/hospital/repair-depot/pad, structure selling/destruction, supply crates,
power/low-power, tiered tech; all 9 support powers + AI use; alliances combat
gating, player colors, vision radius, elimination win; minimap, resources bar,
battle notifications/focus, briefing, production queue HUD, command panel, camera
bookmarks/edge-pan, control groups, army hotkeys, rally points, unit voices; and
all AI controllers (economy/construction/defense/offense/battlegroup/capture/
saboteur/crates/garrison/support/difficulty).

## Gaps closed this session
- **Worker-only economy**: removed the separate `OreHarvester` playable entity,
  its icon manifest entry, VehicleFactory production access, AI production demand,
  and runtime classification paths. Workers now carry resources and are the only
  trained collector unit; refineries spawn a free Worker on completion. Worker
  visuals/icons now use infantry/engineer art instead of the old rover/miner
  silhouette.
- **Shared allied vision** (was MISSING): `update_visibility` now treats any
  `are_allied(visible_team, team)` unit/structure as a fog revealer, and allied
  entities are always visible. Test `allied_vision_is_shared_through_allies`.
- **Command-panel icons** (was text-only): `command_action_icon_path` maps each
  `BuildAction` to its `ui/icons/*.png` (Train/Build use the entity's registry
  icon; standing orders use mirrored godot icons). `command_button_icon` +
  `CommandSlotIcon`, rendered in `refresh_command_panel`. Test
  `command_actions_resolve_to_existing_icon_assets` locks the mapping + asset
  existence.
- **Selection portrait** (was text-only): `update_selection_portrait` shows the
  primary selected entity's icon as a 64px portrait (`SelectionPortrait`).

## Localization / i18n — DONE (Chinese / English)
`Language`/`Locale` resource + a process-global flag (`CURRENT_LANGUAGE`) synced by
`sync_locale`, read by `t(zh, en)` — no per-system param threading (needed because
`update_hud` is at the 16-param limit). F12 toggles language live (and nudges the
menu selection so its dynamic rows rebuild). Static `Text::new` labels use a
`LocalizedText {zh, en}` component + `update_localized_text` so they re-translate on
toggle too. ALL ~170 user-facing strings converted: menu (titles, rules, lobby,
summaries), in-match HUD (status, selection, production queue, objective tracker,
briefing), battle log, support-power messages, match-end/match-menu, placement
feedback, radar/power notices. Verified end-to-end in capture: full menu AND
in-match HUD render correctly in both languages. Default is Chinese.

## Other remaining gaps (none block core playability)
- AI drone scouting behavior — AI has inherent clairvoyance; godot's active
  IntelligenceController drone-roam is not a distinct behavior.
- Tesla fence inter-segment linking — both godot and bevy are per-segment (≈parity).

## Fog "explored terrain" shroud — DONE
Textured fog-of-war shroud over the map: a `FOG_OVERLAY_RES`² CPU texture on a
transparent unlit plane (`spawn_fog_overlay`), repainted each frame by
`update_fog_overlay` from the viewing player's + allies' revealers — clear where
seen now, dim (`FOG_OVERLAY_EXPLORED_ALPHA`) where explored before, black where
never seen. Hidden in spectator/all-visible mode. Skipped when `Assets<Image>` is
absent (pure-headless logic tests). Verified in capture (force-render darkened the
whole map; normal mode clears the base area = correct UV alignment).

## DONE this session (beyond the 3 listed above)
- Idle defense-tower scan rotation — `update_idle_tower_scan` rotates constructed
  defense structures while idle (`weapon.cooldown_left == 0` proxy). Test
  `idle_defense_tower_scans_when_not_engaging`.
- Production-structure cycle — ALREADY at parity: `select_production_structures_for_hotkey`
  cycles to the next producer of a type (non-shift) / selects all (shift), matching
  godot's `select_next_structure`/`select_all_structures`. Audit over-flagged it.
- Capture self-verifies UI (see section above) — biggest fix.

## Capture now self-verifies the UI (fixed)
The HUD/menu DID render in the live game but were missing from offscreen captures.
Root cause: Bevy 0.19 `DefaultUiCamera::get()` only falls back to a camera whose
`RenderTarget` is the primary window; `retarget_capture_camera` had switched the
capture camera to `RenderTarget::Image`, so no camera qualified and the UI was
dropped. Fix: insert `bevy::ui::IsDefaultUiCamera` on the retargeted camera. Now
`capture screenshot/harvest/menu/play/factions` render the full HUD
(command-bar icons, selection portrait, resource bar, briefing, minimap) and the
setup menu/lobby. `capture harvest` selects a click-safe Worker and exits
non-zero unless a `HarvestOrder` is issued. `capture play` now hard-checks
select, movement delta, train queue insertion, and Worker building placement.
`capture factions` exits non-zero unless all three factions can train and build
through the human command panel. Capture apps disable `LogPlugin` so multi-app
capture runs no longer emit duplicate global-logger errors.

## Verification notes
- 20/20 `cargo test current_tests` pass; `cargo build --bins` clean.
- Command icons + portrait + lobby dropdowns are confirmed in capture output
  (no longer need `cargo run` eyeballing for UI in this headless environment).
- `capture play`, `capture harvest`, and `capture factions` are hard
  self-checking smoke tests for the human input loop.
