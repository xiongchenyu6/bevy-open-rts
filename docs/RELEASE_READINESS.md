# Steam release quality plan

The goal is a complete, sellable RTS: a new player learns through play, finishes
a meaningful campaign, can save and resume, and installs a reliable desktop
build. Previous visual fixes and green unit tests do not establish that goal.

## Acceptance gates

| Area | Required evidence | Current state |
| --- | --- | --- |
| First session | Fresh profile → guided gathering, power, barracks, training, movement, attack and victory through normal controls; skip/replay work | Seven lessons and sandbox persistence pass; normal-input skip/return/graduation/fresh replay captured |
| Campaign | All six missions complete without cheats; correct objectives, retry/next/finale; completion survives restart | All six recipes have win/continuation evidence at recorded checkpoints; latest missions3/5 pass; complete current sequence/restart/retry audit remains |
| Save continuity | Mid-mission save/restart/load preserves triggers, orders, paid production, fog, mines, effects and support; missing/corrupt files recover | 14 save tests pass; real Ctrl+S/Ctrl+L restores paid queue/economy/fog and suppresses opening briefing; missing/corrupt loads preserve current match and show revised player-facing feedback; broader campaign runtime pending |
| Interface | English default, coherent icons/tooltips, no clipping at supported desktop sizes; small browser viewport inspected | Compact cards, locked tooltip and queue captured at1280x720/900x720; browser, remaining screens/overlays pending |
| Art/audio | Fresh menu, base, economy, combat, defeat/victory captures; readable silhouettes; commands/music/volume controls verified | Prior material pass only; further review required |
| Skirmish/balance | All shipped maps/factions/difficulties playable; reproducible multi-seed difficulty and faction sweeps; no indefinite matches | Partial old evidence; full sweep required |
| Delivery | Windows and Linux archives launch from arbitrary working directories without Rust/Nix; writable user saves/config | Corrected Linux development archive starts in no-Nix/read-only Ubuntu; matching separate harness passes save recovery; Windows GNU compiles and Wine menu/save recovery render correctly; native Windows/release profile/Steam Runtime/hardware pending |
| Reliability | Full tests, packaged-build smoke, long matches and measured performance on supported hardware | 168 library tests pass; no-Nix/read-only development startup passes; standard release OOM retained; current release/performance gates outstanding |
| Distribution | Accurate feature/system-requirement copy, asset provenance/licenses and generated-content inventory; store-ready media | Restricted logo removed; upstream/font notices added; unresolved art/audio provenance in ASSET_PROVENANCE.md |

## Verification policy

Use the shared game scene and real input harnesses. Each visible fix needs a
fresh screenshot. Complete a current 450-frame, 30-fps proof sequence and inspect
its opening, interactions, combat and ending. Build/check/format and meaningful
behavior tests must pass. Claims about Windows, browser, performance or content
completion require evidence for those targets; native screenshots cannot prove
them. Preserve all gates until verified rather than reducing the release scope.

Steam reviews the store presence and build, including whether the build launches
and its advertised features work. Technical review is a release gate alongside
our player-experience criteria. See [Steam build review](https://partner.steamgames.com/doc/store/review_process)
and [content survey](https://partner.steamgames.com/doc/gettingstarted/contentsurvey).

The working title remains Bevy Open RTS. Desktop mouse/keyboard RTS play is the
primary product; controller/Deck claims need separate verification. Actual store
submission uses the owner's Steamworks account after reviewable artifacts exist.

## Current save acceptance evidence

`logs/steam-save-flow-final.log` and
`screenshots/steam-quality/save-load-final/observations.json` record normal-input
construction, paid infantry production, Ctrl+S, a second paid order, and Ctrl+L.
Ore returns from 24 to 25, two queued jobs return to the saved one, and all 5,771
explored cells remain explored. The production timer and match clock advance by
one normal frame during restoration. `02_restored.png` shows Game loaded with
the opening briefing dismissed. This is skirmish runtime evidence; campaign
triggers, entity remapping and delayed effects are covered by save tests and
still need broader campaign runtime acceptance. Save/load is now available
through mouse buttons in the match menu as well as those shortcuts.

`logs/steam-save-recovery-runtime.log` adds ordinary Ctrl+L attempts with absent
and corrupt files in an explicit disposable XDG sandbox. Both attempts leave the
paused simulation observation byte-equivalent, retain the corrupt bytes and
display Load failed. The first captures exposed raw OS/parser errors; player
error copy now explains missing, damaged and incompatible saves without showing
serialization types. `logs/steam-save-friendly-tests.log` verifies that copy and
file preservation alongside the other 13 save regressions.

The revised feedback is verified in `logs/steam-save-friendly-runtime.log` and
`screenshots/steam-quality/save-friendly/{missing_save,corrupt_save}.png`.
The current integration passes 166 library tests, bins check/build and a
12-second native launch (intentional timeout, empty warning/error log).
`screenshots/result/13` contains 450 raw frames and a 15-second/30-fps video:
4.24m movement and 113 siege damage, with opening/movement/combat/ending inspected.

## Command shortcut regression

An actively gathering Worker can omit the idle-worker command and shift Barracks
onto grid slot 7. The old S binding stopped the worker instead of building;
other grid bindings collided with Guard, Hold, Scatter and numbered groups.
Grid keys now reserve all those shortcuts. The library regression rejects
duplicate/reserved bindings. Campaign macros observe actual tab keys rather
than assuming fixed T/Y positions; complete mission acceptance is still pending.

## Desktop checkpoint limits

The corrected Linux archive and exact-source executable hashes are recorded in
`docs/DESKTOP_DELIVERY.md`. Its read-only installation launches from `/tmp`
without Nix. A separate stripped/patched validation harness uses an asset
snapshot matching all 560 archive files and passes ordinary save/load and
missing/corrupt file recovery. `container-save-load-result.txt` beside that
archive records exit 0. Container ALSA has no audio device; this proves neither
audible sound nor physical GPU compatibility.

Windows GNU game and capture binaries compile. The menu and ordinary save/load
flow render under Wine with a read-only installation and separate user sandbox:
`screenshots/windows-wine/menu.png`, `save-load/02_restored.png` and
`save-load/observations.json`. Both images were inspected. This is useful Windows
executable evidence, but clean native Windows, MSVC distribution, release
performance and audible playback remain open. Both packagers exclude ignored
full-font source files and preserve the original user file.

## Mouse save menu acceptance

`capture save-menu <directory> [width]` uses ordinary Esc, mouse clicks and a
disposable XDG sandbox. The keyboard proof scenario now also runs through
Save/Load buttons, including a second paid order and missing/corrupt files.
`logs/steam-save-menu-final-{1280,900}.log` and matching screenshot directories
record successful restoration and safe rejected loads. Both layouts were
inspected; labels and buttons fit at 1280x720 and 900x720. The offline
Main Menu button reaches the main menu through an actual click.
Online saves are disabled; browser menus omit disk-save buttons. Online return
labels now say Return to Lobby; that label still needs online runtime capture.

The updated source passes 166 library tests, bins check/build and a 12-second
native launch with an empty warning/error log. Proof14 contains 450 frames at
30fps/15s; movement and siege behavior remain verified. Existing desktop archive
hashes identify the earlier checkpoint. The standard Linux release build is
running and must be packaged/tested before a current release claim.

## Construction and remaining campaign proof

An exact live snapshot (`campaign-contact-fixture-2/failure-contact.json`) shows
healthy workers, an open source cell and an AA foundation with no path. The
worker-inflated mobile-unit mask closes a65-cell component although static
routing has a contact route. The helper now prefers the original occupied-unit
route and falls back to static routing when needed; occupied endpoints remain
excluded and normal movement still handles collisions/yielding.

Six construction tests pass, including actual walking, physical non-overlap,
per-frame speed bound, stored cost and completed8HP AA. All168 library tests,
check/build/fmt and native12s smoke pass. In the same normal-input scene the AA
at(-22,-13) completes49.3s instead of remaining stalled at73.7s. Mission3 then
wins183.7s/HQ20 and proceeds to Mission4; Mission5 wins257.8s/HQ20 with no building
losses and proceeds to Mission6. Victory/next screenshots were inspected.
Logs: `steam-aa-contact-full-tests.log`, `steam-campaign-contact-fixed-2.log`,
`steam-campaign-twin-guard-4.log`; screenshots use the corresponding directories.

Mission3's log has no explicit assault: surviving tanks advanced to the eastern
enemy base through ordinary automatic pursuit. The current resolver requires
all living hostile units/structures eliminated, rather than a survival timer.
Mission5 uses explicit assault and home guard response. These proofs do not
replace the complete current campaign/profile sequence or broad balance sweeps.

Current video proof15 has450raw frames/15s/30fps; motion/siege and four phases
were inspected. Standard release fat LTO failed with host OOM, so no current
release executable/package is claimed. Its preserved evidence is under
`/tmp/rts-standard-release-failed-20261008`; retry keeps the profile unchanged
and requires source/memory gates. Human audio/performance/hardware remain open.

## First-wave campaign save acceptance

`capture save-mission <directory>` starts Hold the Line with authored settings,
uses normal construction, waits through its45s wave, then clicks Save/Load in a
fresh explicit XDG sandbox. `logs/steam-save-mission-runtime.log` exits0 and
`screenshots/steam-quality/save-mission/observations.json` records mission identity,
trigger flags `[true,true,false,false,false]`, nine living units before/after,
6,222 explored cells, restored41ore/22crystal and the original one paid job.
The changed state had40ore/two jobs. Clock/production advance eight normal frames
through UI/restart; no wave duplication or opening briefing replay was observed.
The restored screenshot was inspected. This covers the first-wave scenario;
mines/support/effect and long post-load campaign behavior remain separate gates.

## Current profile sweep and compiler constraints

The fresh persisted-profile baseline uncovered an actual first-mission loss at
3:18. Mission2 persists successfully; Mission3 persists its observed victory.
Mission4 reaches authored5:00 victory and writes completion, but the legacy
capture incorrectly reports a Tank quota failure before clicking Next Mission.
These are incomplete sequence results, not a closed full-campaign/profile gate.
The remaining baseline is supervised separately; next coherent sequence will pin
one binary and correct normal-input survival replacement/terminal handling.

The unchanged fat-LTO release retry also suffered global OOM, despite resource
pauses; no standard release executable exists. A separately identified native
thin-LTO candidate reuses cached dependencies and changes only final compiler
LTO/codegen/thread flags. It is still unproven and retains all runtime, hardware,
performance and packaging acceptance requirements.

## Optimized native checkpoint and public online bootstrap

A final-bin-only thin-LTO candidate built successfully in8m58s with cached
release dependencies and full features. Its explicit flags override final LTO
and codegen/thread counts; it is distinct from the two failed standard fat-LTO
attempts. Source executable SHA256:
`fadde65718e2dc0f9aac3207e62e57ab1efe517aa55f0a7db52226e3d636e01d`.
The offline b416 checkpoint archive is under
`/tmp/rts-linux-native-thin-offline-b416-20261008`, SHA256
`370755ccb228ac8103f244253c6dfe60dc940e2eb327385a2c64b96cebdb2007`.
It retains the player guide, provenance/build context and excludes full fonts.

The exact read-only archive renders the actual1280x720 menu without Nix on AMD
RADV RENOIR/Mesa25.2.8/Vulkan. Root inspected
`/tmp/rts-native-thin-validation-20261008/perf/01_initial.png` and adapter/window
logs. This is hardware startup/rendering evidence, not input or combat FPS.
The isolated environment has no audio device and logs missing Compose locales;
keyboard/pointer/data-path and performance acceptance remain open.

Online source and smoke defaults now use the documented, healthy public Worker
(e4fa01a), retaining explicit overrides and custom service fields.31 existing
online tests pass. An isolated no-override test through the actual client HTTP
path passes compatible session protocol/WSS/TURN-configuration checks without
room writes or credential logging. Logs: steam-online-default-{unit-tests,
bootstrap}.log. The old b416 artifact has the previous default; a new candidate
and real multiplayer/forced-TURN traffic are still required.

## Current campaign and packaged-input audit

The coherent current-profile run `current-campaign-profile-coherent-2` passed
missions 1–3 through normal input and recorded the exact growing completion
prefix after each successful process exit. Mission 4 then lost its headquarters
around 274 seconds; the sequence stopped. This is a real defense failure, so
older isolated wins do not close the current campaign acceptance gate.

The exact offline thin-LTO archive now also enters Skirmish setup and launches
a match through a persistent virtual pointer on the isolated AMD/Ubuntu desktop.
Root inspected `/tmp/rts-native-thin-validation-20261008/perf/07_match_mouse.png`.
This extends startup evidence to mouse navigation and match rendering. Keyboard
mapping, combat performance and audible playback remain unverified. The Compose
locale fix is being incorporated into the next package.

Apparent truncated continuation briefings were captured after only two seconds
of the real 28-character/second typewriter animation. Capture acceptance must
wait for the actual complete-text state before reviewing wrapping and clipping;
these early frames alone do not prove a UI truncation bug.

The complete-text predicate and bounded wait pass compilation and fresh mission
3/4 captures. Root inspected `briefing-complete-{2,3}/00_cutscene.png`: the
longest English briefing and the full five-minute defense sentence fit at
1280×720. Both capture processes exit 0 with ordinary Enter dismissal.
