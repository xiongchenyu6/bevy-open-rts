# Steam release quality plan

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
Green unit tests or selected screenshots do not establish release readiness.
The project is **not ready for Steam submission**. All nine gates remain in scope.

## Acceptance gates

| Area | Required evidence | Accepted evidence and remaining work |
| --- | --- | --- |
| First session | Fresh profile completes gathering, power, barracks, training, movement, attack and victory; skip/replay work | Seven lessons and sandbox persistence pass at recorded checkpoints; normal-input skip/return/graduation/replay captured. Revalidate final package. |
| Campaign | All six missions through normal controls in one App; correct objectives, retry/next/finale; persistent completion | Older fresh-process six-mission prefix and current four-mission chain pass. Actual Restart resets/replays triggers. Current M5 loses at106.8s despite verified persistent Hold and completed defenses; normal counter-artillery tactics and full chain remain incomplete. |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, temporary effects and support; safe missing/corrupt loads | First-wave campaign, paid jobs/fog/economy, harvest references, queued moves and F1/F5 effects pass in bounded runtime proofs. Real older-save→current-core mine migration passes exact4HP triggering; fresh-current creation and broader status/damage effects remain open. |
| Interface | English default; coherent icons/tooltips; no clipping at supported sizes; browser inspected | Native cards/tooltips/save menu inspected at1280x720 and900x720; compact unit markers and single finale-map action captured. Fresh browser menu/setup/start/select/move inspected at1280x720/900x720 using software compositor; remaining overlays/final artifacts require review. |
| Art/audio | Readable menu/base/economy/combat/defeat/victory; sound, music, commands and volume controls verified | Recorded visual checkpoints only. Human audio validation and complete current presentation review remain open. Six original encoded illustrations are wired; mission5 native briefing inspected. Other images/layouts/audio remain open. |
| Skirmish/balance | All maps/factions/difficulties/starts playable; reproducible repeated/independent-seed sweeps; no indefinite matches | Foundation-fix pin completes216/216 canonical cases with validated outcomes. Latest damage pin has55/216 completed in a live sweep; alternate starts, repeatability, independent seeds and broad balance remain open. |
| Delivery | Windows/Linux release archives launch without Rust/Nix from arbitrary directories; writable user data | Earlier optimized Linux package passes bounded hardware/input smoke. Windows GNU/Wine harness evidence exists. Current release packages, native Windows/MSVC and Steam Runtime remain open. |
| Reliability | Full suite, packaged smoke, long sessions and representative hardware combat performance | Real EMP application regressions pass3/3; prior full suite178/0/1. Three new combat-damage regressions pass; updated full suite181/0/1 and shared native build pass; final-package/runtime acceptance remains incomplete. No representative combat performance or long-session acceptance yet. |
| Distribution | Accurate store copy, provenance/licenses/generated inventory and store-ready media | Unpublished store draft and partial provenance audit exist. Art/audio rights gaps and final media remain open. |

## Verification policy

Use the shared game scene and real input for player-flow acceptance. Inspect
screenshots and recordings; code alone cannot establish visual correctness.
Keep exact binary hashes, raw observations and failed attempts. Compile/check,
formatting and meaningful behavior regressions must pass. Native screenshots
cannot prove browser or Windows behavior; menu FPS cannot prove combat FPS.
Revalidate final artifacts after core changes. Never narrow the gates to match
already completed work or present configuration counts as statistical win rates.

Presentation acceptance includes a fresh450-frame sequence at30fps, with its
opening, interactions, combat and ending inspected. Prior proof bundles identify
older checkpoints and require final-source review.

Steam submission also requires the advertised features to work and the build to
launch. See [Steam build review](https://partner.steamgames.com/doc/store/review_process)
and [content survey](https://partner.steamgames.com/doc/gettingstarted/contentsurvey).
Submission needs the owner's Steamworks account and explicit publication scope.
No store submission is authorized by the quality goal alone.

## Current campaign evidence

Older separate-process runs complete all six missions with exact persisted
prefixes. They do not prove the required same-App chain. Current paid-AI runs
complete missions1–4. Real menu Restart resets clock45.066→0.033, enemy HMG3→0
and fired flags; the normal wave replays at45.066.

Current capture pin `ac27f9ff` loses mission5 at106.8s. All nine sites complete
by79.0s, power44/use27, and the army reaches9 Tanks/3 HMG/3 Rockets/4 Workers.
Hold remains active on surviving combat units; no accidental toggle causes the
loss. Enemy ground artillery outranges the stationary Tanks, destroys defenses
and workers, then HQ. A normal input counter-artillery recipe is being prepared;
no mission stats, enemy waves or costs have been weakened. See
[campaign QA](CAMPAIGN_QA.md) for raw failed attempts.

## Save and combat continuity

Actual mouse Save and new-process Load preserve harvest references, two queued
moves, economy and mineral depletion (`save-orders-{create,resume}-06`). Paid
Chaos F1 RadarSweep and F5 ShieldOverdrive restore/remap, advance and expire
normally (`save-effects-{create,resume}-01`). Original bytes remain unchanged.
First-wave campaign loading preserves identity, triggers, fog and a paid job.
These bounded proofs do not cover every status or final package.

Mine07 correctly restores source/timers/index and triggers, but6→2.04HP exposed
missing RecentDamage. Commit9d6a666 corrects mine/orbital/crushing damage and
recovery ordering. Full shared-schedule regressions pass3/3; full suite passes
181/0/1 in167.04s (`logs/combat-damage-full-01.log`). Shared native build passes.

Current-core migration09 loads the unchanged real Mine07 save in an isolated
profile/new PID, remaps source and mine, preserves timers/index7, and observes
that exact mine disappear with its player pulse and enemyHP6→2.0. Original and
copied bytes remain identical. This proves the causal damage fix and older-save
migration. Fresh-current creation08 fails eligibility after normal mines kill
the target Worker; complete fresh-current Save/Load acceptance remains open.

## Skirmish and AI

AI pays normal costs and prerequisites and controls only configured AI teams.
Construction now retains damage and cannot resurrect dead foundations. Actual
EMP application and extension pause tasks/queues instead of deleting them;
regressions use the real application helper and complete shared schedule.

Foundation-fix pin8c0c4438 completes216/216 canonical map/faction/difficulty
configurations; independent raw config/status/outcome validation reports zero
errors (`logs/skirmish-foundation-canonical-validation-20261010.json`). Older
BigArena and FourCorners timeout artifacts remain preserved. Current damage
pin `cbb2d6eb` runs a fresh216-case sweep in
`target/skirmish-combat-canonical-all-20261010`; observed55 completions with no
failures at the latest check. It is still running. This covers canonical starts
only, not all3,726 ordered-start configurations, independent seeds, statistical
win rates or representative hardware performance.

## Browser and online

Wasm02 builds and renders actual English menu/setup/match/selection/movement
at1280×720 and900×720 with inspected paired frames. The initial black headless
canvas required a corrected SwiftShader compositor harness; the same unchanged
program renders afterward. Software rendering proves no hardware FPS claim.

Wasm03 builds successfully in9m17s with the current combat core and read-only
online diagnostics: buildId `f0c40da533de8f6c`, Wasm SHA-256
`10adaa091fb0fe3d818ea7ecda1daa15c5009556703da8a6baec866763f165f7`.
Its source hashes are preserved in
`target/web-current-20261009/build-provenance-03.json`.

Earlier two-native-Human-plus-AI verification observes normal paid AI growth,
three actual remote Move packets, two authoritative queued moves and20 matching
snapshots. Browser/native attempt02 enters and completes the client route but
fails host queue/control assertions; matching poses disprove an ID-bootstrap
explanation. Attempt03 is live with matched native/Wasm diagnostics and unchanged
pacing. No browser-peer, forced-TURN, eight-player or long-session acceptance is
claimed. See [web QA](WEB_CURRENT_QA.md).

## Delivery, presentation and provenance

See [Linux delivery](DESKTOP_DELIVERY.md), [Windows delivery](WINDOWS_DELIVERY.md),
[asset provenance](ASSET_PROVENANCE.md) and [store draft](STEAM_STORE_DRAFT.md).
Earlier labeled thin-LTO Linux candidates launch from read-only no-Nix installs
with physical input on AMD RADV. Menu FPS does not prove combat performance.
Windows GNU/Wine harness evidence exists, but launch.cmd window capture was
black. Final current releases, native Windows, Steam Runtime and audio remain
unverified.

Six original individually generated campaign illustrations are encoded and wired;
mission5's actual briefing is inspected. Other briefings, current full UI/media
and human listening remain open. Current runtime tree has366 files excluding the
full source font; the provenance source-hash inventory has360 records. These
counts have different scopes. Six obsolete campaign JPEGs are retained only for
older sealed QA executables and must be removed before final distribution.
Official Kenney153GLBs match upstream CC0 archives. Remaining icon/VFX/voice
rights gaps remain open; source membership alone is not commercial permission.

Preserve user-modified `flake.lock`, original AGENTS.md and ignored full-font
source. No Steam upload or external publication has occurred.

## Bounded real-save migration and mine damage regression

The actual normally paid Mine07 save, produced by mouse controls, is copied
byte-for-byte into a new isolated XDG sandbox and loaded through the mouse menu
in a fresh process on the corrected core. This is an old-real-save→new-core
migration and causal damage regression; it does not replace failed current
creation08 or close the full fresh-save gate.

`logs/save-mines-migration-resume-09-result.json` records child0, valid schema,
PID3364171 versus real creator3264555, and capture SHA256
`ac27f9ff0e99d276c36b4759a238347ed944590f7b4d3757a5760f09f08aed23`.
`screenshots/steam-quality/save-mines-migration-resume-09/observations.json`
records layer206158428216→12884901309 and mine867583392440→12884900465.
Across the real0.033333-second first load frame, arming0.183333→0.150000 and
layer cooldown2.133333→2.100000 retain deployment index7. The restored mine
then disappears and emits a player pulse from its exact location to the enemy
Worker, whose HP changes6→2. The strict4HP assertion passes unchanged.
Restored and triggered screenshots were inspected.

The original and copied quicksave remain SHA256
`a78b22b1a38e6e8deea0844f84b0395e257e805311975ad7d59ded6a8d168b50` after
loading. Provenance is in
`target/save-mines-migration-sandbox-20261010-09/original-save-provenance.json`.
No save editing, injected resources/units/timers, or compatibility layer is used.

The native proof runner persists the actual child return code, PID, executable
hash and observation-schema outcome. It refuses existing output/log/result
paths and terminates a timed-out child with recorded exit124. Negative checks
retain evidence in `target/mine-runner-check-n5ex9f8z`: child1 propagates1;
child0 without observations fails2; existing paths preserve their original
bytes; an actual hanging child is terminated and reaped. Asynchronous service
launch success is never treated as proof success.
