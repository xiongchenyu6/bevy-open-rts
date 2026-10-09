# Steam release quality plan

The goal is a complete, sellable RTS: a new player learns through play, finishes
a meaningful campaign, can save and resume, and installs a reliable desktop
build. Previous visual fixes and green unit tests do not establish that goal.

## Acceptance gates

| Area | Required evidence | Current state |
| --- | --- | --- |
| First session | Fresh profile → guided gathering, power, barracks, training, movement, attack and victory through normal controls; skip/replay work | Seven lessons and sandbox persistence pass; normal-input skip/return/graduation/fresh replay captured |
| Campaign | All six missions complete without cheats; correct objectives, retry/next/finale; completion survives restart | Older six-mission fresh-process prefix passes; current paid-AI same-App chain passes missions1–4, then fails mission5 construction; real Restart resets/replays triggers correctly |
| Save continuity | Mid-mission save/restart/load preserves triggers, orders, paid production, fog, mines, effects and support; missing/corrupt files recover | 14 save tests pass; real Ctrl+S/Ctrl+L restores paid queue/economy/fog and suppresses opening briefing; missing/corrupt loads preserve current match and show revised player-facing feedback; broader campaign runtime pending |
| Interface | English default, coherent icons/tooltips, no clipping at supported desktop sizes; small browser viewport inspected | Compact cards, locked tooltip and queue captured at1280x720/900x720; browser, remaining screens/overlays pending |
| Art/audio | Fresh menu, base, economy, combat, defeat/victory captures; readable silhouettes; commands/music/volume controls verified | Prior material pass only; further review required |
| Skirmish/balance | All shipped maps/factions/difficulties playable; reproducible multi-seed difficulty and faction sweeps; no indefinite matches | Old canonical72 complete; old difficulty144 has5 timeouts. Current paid-AI targeted10 complete, including all5 former timeouts; current216 sweep finishes215/216 with1 timeout. Alternate starts/seeds remain open |
| Delivery | Windows and Linux archives launch from arbitrary working directories without Rust/Nix; writable user saves/config | Corrected Linux development archive starts in no-Nix/read-only Ubuntu; matching separate harness passes save recovery; Windows GNU compiles and Wine menu/save recovery render correctly; native Windows/release profile/Steam Runtime/hardware pending |
| Reliability | Full tests, packaged-build smoke, long matches and measured performance on supported hardware | Current library suite172 passed/0 failed/1 network-dependent ignored; earlier packaged startup passes; standard release OOM retained; current release/performance gates outstanding |
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

`capture mission <index> <directory> [width] [height]` now accepts explicit
viewport dimensions. Check/build/fmt pass; fresh longest-English Mission3 and
Mission4 captures exit0 at900×720. Root inspected full text and continuation
hints in `briefing-complete-{2,3}-900/00_cutscene.png`; neither is clipped.
The online-default thin-LTO candidate is running under
`rts-native-thin-online-candidate-20261008.service` with its exact launch context
recorded in the build log; completion and packaged-runtime acceptance are pending.

## Skirmish setup summary correction

Fresh current UI screenshots cover Options, Credits and Skirmish setup at
1280×720. The setup exposed a stale global difficulty in Operation summary:
AI Normal contradicted the actual AI Easy player row. The redundant aggregate
was removed; individual player rows remain the displayed difficulty source.
Check/build/fmt pass and `ui-current/setup-summary-fixed.png` was inspected
after a successful capture. The live optimized candidate predates this UI fix.

The balance-harness audit also found that existing difficulty duels keep
Alliance/Demon default factions, so difficulty and faction effects are confounded.
They use one default map/spawn pairing and return success even on timeouts;
those diagnostic outputs do not satisfy the all-map/faction/difficulty gate.

## Fresh-process campaign save restoration

`capture save-resume <original-evidence-directory> <output-directory>` loads
existing bytes through the actual mouse match menu in a newly launched process.
It requires an explicit isolated absolute XDG data directory and preserves the
save file. Check/build/fmt and runtime exit0 pass. Root inspected
`save-mission-fresh-process/00_resumed.png` and observations: first-wave flags
remain `[true,true,false,false,false]`, living units9→9, ore/crystal41/22,
6,222 explored cells, mission `hold_the_line`, and one paid infantry job.
The clock46.666→46.933 and job3.0→2.733 advance only by normal UI/load frames.
This closes fresh-process restoration for that first-wave snapshot; later
orders, mines, effects and support powers still require runtime acceptance.

## Packaged physical keyboard and pointer evidence

The offline b416 thin-LTO archive now passes real Enter-to-start, physical F10
pause/resume, left-click worker selection and right-click movement in isolated
Ubuntu/Sway on AMD RADV RENOIR. Root inspected `11_evdev_f10_pause.png` and
`13_worker_moved.png` in `/tmp/rts-native-thin-validation-20261008/perf`.
The old wtype attempt mapped the requested F10 to physical Escape and is not
used as evidence. A persistent evdev keyboard/pointer supplies correct keycodes.
The locale override removes Compose errors; the old archive needed that test
override, while the prepared launcher preserves user overrides and supplies a
host-locale default. This proves the recorded input paths on that exact archive,
not broad combat performance or audible playback.

The online-default thin candidate finished in11m22s with exit0 under
`rts-native-thin-online-candidate-20261008.service`. Its source checkpoint
predates the skirmish-summary correction. New archive hashes and exact-package
runtime checks are required before extending this old artifact's acceptance.

## Finale return action

The actual normal-input finale victory exposed two identical Campaign Map
buttons. The redundant ordinary return action is now hidden on the final
campaign victory; the continuation action remains. Check/build pass, and a
fresh normal-input finale again wins at1:22/HQ20 with exit0. Root inspected
`finale-single-map-button/02_victory.png` (one map button) and `03_continue.png`
(actual return to campaign selection). This isolated run persists the finale
only; the full growing-prefix campaign sweep is separate and still active.
The first systemd launch omitted development library paths and could not load a
GPU; the corrected retry inherited LD_LIBRARY_PATH and succeeds. Both logs and
exact executable/profile metadata are retained alongside the screenshots.

## Canonical faction matrix

All72 Normal/Normal configurations (8 maps ×9 ordered faction pairs, starts0/1)
finish with an authoritative single winner and exit0 on pinned executable8ceee.
Root verified each raw configuration/result/status, initial HQ count, resources,
anchors and no timeout flags. Duration76.399–284.819 sim seconds. This establishes
resolution for those configurations, not general balance: Chaos wins only4 of
the48 non-mirror configurations, so AI/faction diagnosis is active. The144
same-faction adjacent-difficulty cases now run separately; alternate spawn pairs,
repeatability, independent seeds and release-artifact performance remain open.

## Current source revalidation and web staging

The d569 campaign checkpoint completes all six in order with exact persisted
prefixes and a fresh-process all-unlocked menu. Same-App play and pause-menu
Restart are separate checks: a retained trigger-state defect is being corrected
in the common new-match initializer, before saved state restoration. Current
AI technology/worker-construction changes require new behavioral acceptance;
earlier campaign and balance results identify their older executable hashes.

Web staging now builds only the shipped game with the lockfile and excludes
`*.full.ttf`, retaining the runtime subset. Root executed the actual staging
block in an isolated directory: all560 runtime file hashes match, the3,347,880-
byte source font is excluded, and the original font hash is unchanged. Evidence:
`logs/web-assets-subset-qa.json`; shell syntax passes. Full Wasm compilation and
browser runtime acceptance are still pending.

The old canonical adjacent-difficulty matrix is now fully observed:139 completed
and5 real timeouts across144 configurations. All raw reports/statuses were
revalidated. The failed cases remain open for the new AI implementation; current
source is still compiling. Cleaned web staging also passes360 file-by-file
hash checks, with full source fonts and unused crosshairs excluded
(`logs/web-clean-assets-qa-20261009.json`).

Reviewable player-facing store copy is in `docs/STEAM_STORE_DRAFT.md`; internal
claim evidence is separate, and no unmeasured system-requirement numbers or
unverified platform/Steam integration badges are asserted. It is not published.

## Current paid-AI restart evidence

Shared capture build03 finished with exit0 in110m31s after dependency-cache
recovery. The immutable capture SHA is
`bbacd8e42b4cc1e244b95382b6bf5a9b12d722305e3482efac4a2fa5047484f2`.
The real pause-menu Restart test exits0: clock45.066/3enemyHMG resets to0.033/
zeroHMG/all fired flagsfalse; the authored wave then appears again at45.066
with3HMG. Root inspected `current-campaign-restart-paid-ai/03_wave_replayed.png`
and the result JSON. Saved-trigger load continuity still needs its current test.
The same-App campaign chain is supervised under
`open-rts-campaign-chain-paid-ai-2.service`; its earlier foreground attempt
endedSIGTERM and is preserved separately, not counted as full completion.

## Current paid-AI and order-save acceptance

The current library suite passes170 tests with0 failures and1 explicitly ignored
network-bootstrap test (`target/ai-tech-full-tests.log`). A separate current saved
trigger/order/production regression passes (`target/ai-tech-savedflags.log`).
Paid-AI progression and Human-controller protection tests pass: AI pays normal
costs and physically constructs its technology, then produces armed advanced
units; configured Human teams retain their resources and orders. This headless
protection check does not establish live online peer behavior.

The ten exact configuration comparisons in
`target/ai-tech-targeted-20261009` complete, including all five old600-second
timeouts. The pinned current canonical216-case sweep is running under
`rts-skirmish-current-canonical-all-20261009.service`; its terminal summary must
be inspected before acceptance. Current checkpoint:112 completed/216, with
1 unresolved600-second case0103 (Big Arena, Chaos Easy0 versus Beginner1).
These are configuration resolution checks,
not statistical win rates, independent seed coverage or alternate-start proof.

Real unit-order save creation and fresh-process restoration both exit0:
`screenshots/steam-quality/save-orders-{create,resume}-06/observations.json`.
Actual mouse inputs assign harvesting and two Shift-queued waypoints. The
loaded mineral entity reference remaps correctly, both queued targets survive,
the worker finishes its route, ore32→38 and mineral240→234. Original save bytes
remain unchanged; Saved and resumed screenshots were inspected. Earlier failed
inputs landed on the command HUD, so they did not constitute a serializer bug.
Mine deployment/trigger continuity and other status effects or delayed support
strikes still need separate runtime proof. Radar Sweep and Shield Overdrive
now have the bounded fresh-process proof below.

The current same-App campaign reaches four victories, then fails mission5 with
an unfinished foundation. Diagnostic traces are gathering assignment/cancellation
history; a reachable path at the failure snapshot alone does not prove correct
initial assignment. No current six-mission same-App completion is claimed.

## Fresh-process Radar Sweep and Shield Overdrive

`capture save-effects <directory>` uses actual Play, Chaos faction, AI Beginner
and Start Match clicks. Four physically built, normally paid prerequisites
(Power Reactor, Radar Uplink, Robotics Bay and Tech Lab) debit19 ore and16
crystal from the normal32/16 budget, leaving13/0 with sufficient power. Actual
F1/F5 plus target clicks create a Radar reveal and a Worker shield; the match
menu saves while both effects are active. `capture save-effects-resume
<evidence-directory> <output-directory>` loads through the mouse menu in a
separate process and leaves the original save bytes unchanged.

`screenshots/steam-quality/save-effects-{create,resume}-01/observations.json`
and Saved/Restored/Expired screenshots were inspected. The shield target remaps
from8589934089 to12884900480. Over the real0.267-second load advance, shield
remaining7.80→7.53 and reveal7.47→7.20; shield damage scale remains0.25.
Cooldowns18/55 continue from17.47/54.80 to17.20/54.53, then9.63/46.97 after
ordinary simulation removes both effects. Logs
`logs/save-effects-{check,build}-02.log` and
`logs/save-effects-{create,resume}-01.log` exit0. Pinned capture SHA256 is
`659f56be747d78ad9f29d0ae18435c0e58f90e121b8bde9acb4b209a4f453620`.

This proves Radar reveal, one Worker Shield Overdrive buff, their timers and
cooldowns. Shield combat mitigation, other buffs, delayed strikes and mines
are separate acceptance gates; this capture does not establish them.

## EMP task preservation regression

The mission5 first-frame trace proves that the selected Worker is temporarily
EMP-disabled when the AA foundation is placed. The old construction update
deleted its task immediately; harvesting had the same cancellation behavior.
Both systems now pause while EMP is active and retain the task.
`logs/emp-task-focused.log` passes two actual-status-expiry regressions: paid
construction completes after expiry, and harvesting retains its mineral, cargo
and progress before resuming collection. The full current library suite passes
172 tests,0 failures,1 ignored in96.76s (`logs/emp-task-full.log`).
The initial failed compile is preserved separately. Current mission5 gameplay
and the full same-App campaign still require new-build runtime acceptance.

## Completed current canonical matrix

The paid-AI pin805dbb05 finishes the full216 configurations:215 complete,1
simulation timeout, overall exit2. Root independently validates every raw report
against the manifest, status, normal32ore/16crystal baseline and terminal outcome
with zero validation errors (`logs/skirmish-current-canonical-all-validation-20261009.json`).
All72 Normal/Normal faction configurations resolve in69.90–228.62 sim seconds.
The48 non-mirror configurations produce Alliance17/Demon16/Chaos15 winners;
these counts are not statistical win rates. The remaining timeout is BigArena,
ChaosEasy slot0 versus ChaosBeginner slot1, case0103, at600 sim seconds.
Its raw report remains available for new target/weapon diagnostics and replay.
This immutable pin predates the EMP task preservation fix. Alternate starts,
repeatability, independent-seed coverage and release performance remain open.

The post-fix mission5 replay (`current-campaign-twin-emp-paused`) still fails
with an unfinished AA foundation at82.1s. Root identifies a second cancellation
path: `clear_emp_disabled_orders` globally clears all active and queued orders
in the actual Combat schedule. The two isolated regressions did not include
this system. Full-schedule EMP preservation, including capture/garrison pause
semantics, must pass before claiming the campaign defect fixed. The failed
runtime trace and original logs remain intact.
