# Steam release quality plan

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
Green unit tests or selected screenshots do not establish release readiness.
The project is **not ready for Steam submission**. All nine gates remain in scope.

## Acceptance gates

| Area | Required evidence | Accepted evidence and remaining work |
| --- | --- | --- |
| First session | Fresh profile completes gathering, power, barracks, training, movement, attack and victory; skip/replay work | Seven lessons and sandbox persistence pass at recorded checkpoints; normal-input skip/return/graduation/replay captured. Revalidate final package. |
| Campaign | All six missions through normal controls in one App; correct objectives, retry/next/finale; persistent completion | Older separate-process six-mission prefix and pre-reset four-mission chain pass. Isolated M5 wins04:49/HQ20. Fresh lifecycle/atomic-queue tests pass; visible-only isolatedM5 wins05:32/HQ20 on8d391. Visible-only same-App chain failed mission1; isolated visible-HQ/scouting retry also loses at172.8s. Mixed retry loses217.3s; Hold auto-fire/chase defect is corrected with full200/0/1, but its M1 retry loses238.9s. A subsequent normal paid two-AntiAirWalker strategy holds HQ20 through440s, then loses443.7s. Heavy-airship samples include27HP and later2HP, but lack actor IDs; no same-actor damage attribution is accepted. Enemy HQ is visibly targeted twice; late route/attack state still needs recording. Full chain/final menu remain open. |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, temporary effects and support; safe missing/corrupt loads | First-wave campaign, paid jobs/fog/economy, harvest references, queued moves and F1/F5 effects pass in bounded runtime proofs. Fresh Mine11 mouseSave/newPIDLoad passes exact4HP, source/timer/index remap and unchanged bytes on reset-core pin. Broader effects, weather and final-format/package revalidation remain open. |
| Interface | English default; coherent icons/tooltips; no clipping at supported sizes; browser inspected | Native cards/tooltips/save menu inspected at1280x720 and900x720; compact unit markers and single finale-map action captured. Fresh browser menu/setup/start/select/move inspected at1280x720/900x720 using software compositor; All20 original semantic glyphs have bounded visual coverage:10 actual gameplay/menu and10 shared-UI presentation fixture at1280x720/900x650. Remaining overlays/final artifacts require review. |
| Art/audio | Readable menu/base/economy/combat/defeat/victory; sound, music, commands and volume controls verified | Recorded visual checkpoints only. New32 voices load through the actual Bevy decoder and34 enum/language events play to natural completion; private-output69.8s recording is unclipped, with338 assets unchanged. Master/Voice/SFX gain spawn tests pass. Natural game triggers, mixing, human listening and complete current presentation review remain open. Six original encoded illustrations are wired; native briefings1/2/3/5 and bounded result panels inspected. Remaining layouts, full presentation and audio remain open. |
| Skirmish/balance | All maps/factions/difficulties/starts playable; reproducible repeated/independent-seed sweeps; no indefinite matches | Foundation-fix pin completes216/216 canonical cases with validated outcomes. Combat-damage checkpoint completes216/216 canonical cases, independently validated. The AI-compatible checkpoint runs3,726 ordered-start cases; prior reset-core run stopped after two timeouts, both standalone compatible-target rechecks now finish.27 overlapping fresh reset reports match the previous pin. TechDivide AllianceBeginner0/AllianceEasy2 wall-times out twice at180s on the same pin; profiling traces repeated constructor contact searches; measured trapped-worker regressions motivate a navigation correction currently under test. A partial independent scan records1,019 terminal cases:1,015 completed, three wall timeouts and one600s simulation timeout (case697); the latter needs separate economy/endgame diagnosis. The corrected fad484 checkpoint completes the exact failed TechDivide case in both trace/no-trace runs: actual exits0, identical results at180.542938 simulation seconds, original600s simulation/180s wall limits and32/16 resources preserved (`target/tech-divide-construction-fixed-pair-05/verification.json`). A fresh3,726 ordered-start sweep is running on an isolated asset copy; no whole-matrix result is claimed. Wider repetition, independent seeds and broad balance remain open. |
| Delivery | Windows/Linux release archives launch without Rust/Nix from arbitrary directories; writable user data | Earlier optimized Linux package passes bounded hardware/input smoke. Windows GNU/Wine harness evidence exists. The pre-Hold Linux336-asset archive passes real skirmish startup, F10, keyboard and mouse Save/Load in a read-only no-Nix Ubuntu/AMD environment. The notice-enhanced archive hashes13 exact license texts. Current-core rebuilding, native Windows/MSVC and Steam Runtime remain open. |
| Reliability | Full suite, packaged smoke, long sessions and representative hardware combat performance | Real EMP application regressions pass3/3; prior full suite178/0/1. Three new combat-damage regressions pass; fresh/load transition regressions2/2 and updated Hold suite200/0/1 and shared native build pass at that checkpoint. The first pending constructor/navigation full suite fails204/4/1. Subsequent physical, arrival and curved-egress corrections pass measured safety cases and the unchanged45s Human/AI test; two600s AI cases still fail, exposing separation and bonus-worker spawn defects. The final constructor/navigation checkpoint passes219 tests, with0 failures and1 ignored; contact safety14/14 and the real-wall fixture pass. All four native binaries build with actual exit0 (`target/constructor-full-27-result.json`). This proves that checkpoint, not final packaged reliability. Matched pre-Hold native/Wasm07 human+AI control passes queue2, completed route and158 exact same-tick DTO comparisons with normal cleanup. The short hardware probe samples52.63FPS/GPU98.6% during early combat; stable representative combat and long-session acceptance remain open. |
| Distribution | Accurate store copy, provenance/licenses/generated inventory and store-ready media | Unpublished store draft, an original reproducible wordmark and verified Steam media export brief exist; final media needs at least five actual1920×1080 gameplay captures. Partial provenance audit exists. Art/audio rights gaps and final media remain open. |

## Current development checkpoint

The paid AI spawn/site correction passes the unchanged marker, eight spatial
and cost regressions, and seven AI development/completion fixtures. Normal
Chaos completes paid Power/Radar/RoboticsBay/TechLab and trains armed advanced
units on the original far-start fixture; the original45s remote-Human control
fixture passes. Costs, profile timers, six-pick budget and physical clearance
remain unchanged. Exact packed-base snapshots motivate angular coverage and
bounded expansion inside normal construction influence.

The current full suite passes229/0/1 in176.20s with actual exit0
(`logs/ai-placement-shared-14-full.log/.exit`). Four native binaries are still
building; no final artifact or whole-map acceptance follows from this suite.
The70 model-only portrait candidates pass140 paired exports, transparent RGBA,
explicit contextual-marker exclusion and complete128px/72px dark/light review
(`art/portraits/model-only-full70-visual-review.json`). Runtime replacement,
normal paid-model comparison and final native/browser presentation remain open.
All nine release gates above still apply.

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

Current capture pin `f6069ba1` wins mission5 at289.117s through normal inputs,
paid production and live attack-move responses to ground artillery/Jammers.
HQ remains20HP and all four Workers survive. The actual child exits0,
telemetry records HumanVictory, and `02_victory.png` displays04:49 and Next Mission
under `screenshots/steam-quality/current-campaign-twin-live-ground-response`.
This is isolated mission5 evidence; the six-mission same-App chain, six exact
persisted prefixes and fresh final menu remain pending.

Earlier stationary-Hold pin `ac27f9ff` loses at106.8s despite completed defenses,
adequate power and correctly held combat units. Its ground-artillery defeat is
preserved, not reclassified. No mission stats, enemy waves or costs have been
weakened. See [campaign QA](CAMPAIGN_QA.md).

## Cross-match state regression

The current same-App chain completes missions1–4 with exact on-disk prefixes,
then loses mission5 at117.8s (`current-campaign-chain-live-response` actual
process exit1). The isolated mission5 victory remains a distinct bounded proof.
AI director state is initialized once and omitted from fresh-match entry reset;
opening-grace flags/cursors/timers therefore carry across matches. A regression
using the actual RestartingMatch→InMatch transition fails before the fix and
retains the seeded previous director (`logs/campaign-ai-reset-red.log`).

The initial loaded-state test also differs after Update because disabled AI
capabilities normalize their timers to profile intervals even at zero delta.
That fixture observation does not prove a serializer defect. Exact restoration
must be checked at the actual load boundary before normal AI updates. Economy
startup likewise resets only ore/crystal, leaving sabotage and producer veterancy;
the expanded actual-transition regression reproduces this leakage while exact
saved-state restoration passes. The minimal correction resets AiDirector before
apply_loaded_save and creates fresh TeamEconomy values at new-match setup.
Focused fresh/load regressions pass2/2 in1.67s; full suite passes183/0/1 in78.24s
(`logs/campaign-ai-reset-{green,full}.log`). Shared native build passes2m39s with actual full/build exits0/0; sealed hashes
and source revision are in `target/ai-match-reset-shared-20261010/artifact.json`.
Corrected same-App chain remains pending. Wasm04 was deliberately stopped and retains its
stop/provenance records.

## Save and combat continuity

Current save format3 adds mandatory weather continuity: previous/current
weather, blend, phase timer and deterministic step. Fresh match/restart resets
weather before loading; an accepted save restores it exactly. Five focused
weather checks pass, including actual mid-fade save/load/advance and real restart;
the coordinated full suite passes190/0/1. Older format2 saves are rejected with
a clear incompatibility message and require a new match. No migration layer is
provided. The runtime proofs below belong to their historical format2 binary
checkpoints. Current format3 actual orders/effects/mines revalidation passes on
the sealed f58e08 capture checkpoint; this is bounded functional evidence, not
acceptance of subsequent HUD contrast changes.

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
migration. Failed fresh creation08 and resource-snapped Rally10 remain recorded.
Fresh-current Mine11 now passes normal paid creation, mouse Save and new-PID
mouse Load, preserving source/timers/index and unchanged bytes before exact
4HP damage. This closes the bounded Chaos LandMine continuity gate, not every
status effect or final-package acceptance.

Current format3 normal mouse Save/new-process Load passes all three bounded
flows (`save-{orders,effects,mines}-schema3-{create,resume}-01`). All six child
results are0 with valid observations, distinct creator/loader PIDs, unchanged
save bytes and inspected screenshots. Orders remap harvest targets, consume
two queued waypoints and advance ore32→38. F1 RadarSweep/F5 ShieldOverdrive
remap target17179868665→12884900677, preserve timers/cooldowns and expire
normally. Mines remap source446676596757→12884901309 and mine73014442576→
12884900726; arming0.150000→0.116667 and cooldown2.133333→2.100000 preserve
index7 over0.033340s, followed by exact player pulse and WorkerHP6→2.
Capture SHA256 is `f58e08a3054eda63952b847238de9b2a98a547fcbf1c03ebcb05a18e4a555e3f`;
result JSONs and observations are under `logs/` and `screenshots/steam-quality/`.
Broader status effects, shield combat mitigation and final-package acceptance
remain open.

The actual full format2 Mine11 file also passes normal mouse Load rejection
(`save-schema-reject-01`): clear incompatibility feedback is visible, the paused
match is unchanged, and original/copied SHA256 remains
`858d007e09d85996cf6ac92a035ebbc1ff8ba52d75c666b51d4d03e8d7b46a07`.
The screenshot was inspected; child3504123 returns0 with valid observations.

## Skirmish and AI

AI pays normal costs and prerequisites and controls only configured AI teams.
Construction now retains damage and cannot resurrect dead foundations. Actual
EMP application and extension pause tasks/queues instead of deleting them;
regressions use the real application helper and complete shared schedule.

Foundation-fix pin8c0c4438 completes216/216 canonical map/faction/difficulty
configurations; independent raw config/status/outcome validation reports zero
errors (`logs/skirmish-foundation-canonical-validation-20261010.json`). Older
BigArena and FourCorners timeout artifacts remain preserved. Combat-damage pin `cbb2d6eb` also completes216/216 canonical cases with zero
independent validation errors
(`logs/skirmish-combat-canonical-final-validation-20261010.json`). This remains a
checkpoint before fresh-match reset3478190.

Its full ordered-start sweep was deliberately stopped after127 successful cases
when superseded; partial evidence and reason remain under
`target/skirmish-combat-all-starts-20261010/semantic-stop.json`. It is not a
completed3,726-case acceptance. The new reset-core audit pin `9dcd4c80` runs the
full3,726 configurations in `target/skirmish-match-reset-all-starts-20261010`,
unit `rts-skirmish-match-reset-all-starts-20261010`, log
`logs/skirmish-match-reset-all-starts-20261010.log`. Its manifest and sealed binary
hash are independently verified; the sweep is still live. Wider repetition,
independent-seed outcomes, statistical win rates and representative hardware
performance remain unverified.

## Repeatability evidence

The canonical and all-start sweeps independently execute27 overlapping exact
configurations on the same immutable current binary. Comparing their complete
raw reports finds27 identical reports and zero winner differences
(`logs/skirmish-current-repeat-comparison-01.json`). This is bounded repeated-
process evidence, not statistical balance or independently seeded matches.

The audit exposes explicit maps/starts rather than a stochastic match seed.
AI placement uses deterministic IDs as geometric seeds; Random Map uses a cursor
to select among authored maps. Do not relabel repeated runs as independent random
seeds. Random-map selection and wider repeated-run coverage remain unverified.

The first27 overlapping fresh-App configurations on the new reset-core pin have
complete raw reports identical to the prior combat checkpoint
(`logs/skirmish-fresh-reset-report-comparison-01.json`). This is bounded
cross-version evidence, not a substitute for the current3,726-case sweep.

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

## Paid mine Save/Load evidence

Fresh Mine11 uses normal Chaos resources plus real6crystal harvesting, paid
prerequisites, a forward relay/factory and two MineLayers (total29ore16crystal;
saved economy3ore6crystal). Real ShieldTrooper scouting/attack and displayed
Rally controls reach a visible healthy Ore Worker. Mouse Save and fresh-process
mouse Load use distinct PIDs3443650/3447197; both runner results are child0 with
valid schema (`logs/save-mines-{create,resume}-11-result.json`). Capture SHA256:
`7fe3cb8024e48b7557df1695a917c5bc47a5e0a47c434840edee32bf8140933e`.

Layer206158428900→12884901309 and mine622770255756→12884900726 remap. Over
the first0.033340-second load frame, arming0.150000→0.116667 and source
cooldown2.133333→2.100000 preserve index7. The loaded mine disappears with
a player pulse from its exact position to the Worker, whose HP changes6→2;
the strict4HP assertion passes. Observations and inspected Saved/Restored/
Triggered PNGs are in `screenshots/steam-quality/save-mines-{create,resume}-11`.
Original quicksave bytes remain SHA256
`858d007e09d85996cf6ac92a035ebbc1ff8ba52d75c666b51d4d03e8d7b46a07`.

Earlier failures remain recorded, including creation08 eligibility and Rally10
resource snapping. Migration09 independently loads the unchanged real Mine07
save on the corrected core and passes exact4HP damage; its observations/logs
remain under `save-mines-migration-resume-09`. No save editing, injected
resources/units/orders/timers, or compatibility layer is used. This evidence
covers Chaos LandMine continuity alongside F1/F5 and order proofs; broader
status effects and final-package acceptance remain open.

The native runner records actual child code/PID/hash/schema, refuses existing
output/log/result paths and terminates timed-out children with exit124. Negative
checks in `target/mine-runner-check-n5ex9f8z` verify child1 propagation, missing
observations failure2, existing-file preservation and real child timeout/reaping.
Asynchronous service launch success is never proof success.

## Visible-only mission5 checkpoint

Capture8d391 on the updated original semantic-icon asset set completes actual
mission5 at332.774s with HumanVictory/HQ20, actual child exit0 and normal Next
Mission. Root inspected `current-campaign-twin-visible-crystal-cover/02_victory.png`.
Targets come only from actual visible enemy roots; unexplored sectors use ordinary
map scouting. Paid production and crystal-field protection preserve the economy.
The single-App six-mission run `rts-campaign-chain-visible-only-01` failed
in mission1 after completing mission0. Its strategic target selection returned
to incidental visible units instead of continuing unexplored-sector scouting.
The capture-only fix targets visible Command Centers and advances scouting
from own-army arrival. Its isolated retry also fails at172.8s: no Tanks remain
and enemy HQ retains20HP. Both failures are preserved; no core balance defect
is established, and a complete six-mission pass remains unproven.

The current Linux thin-LTO candidate completed with explicit Rust1.95, locked
dependencies and one Cargo job: actual exit0 in23m22s, source hashes unchanged.
`target/linux-current-release-20261010/result.json` pins executable SHA256
`b86918e67a239d3f047fed437a6e27213ab597f83c3c53e68053bfa55550761e`.
Packaging exited0; `target/linux-current-package-20261010/verified-package.json`
records archive SHA256 and all360 runtime assets passing exact-set/hash checks.
Actual package launch renders its main menu on AMD RADV RENOIR/Mesa25.2.8
in read-only Ubuntu24.04 without a Nix mount. GLES Sway retry exits0; the
initial pixman compositor attempt failed surface creation and remains retained.
Initial screenshot review showed short-lived keyboard input did not activate
Enter/F10. A persistent virtual pointer then clicked Skirmish and Start Match:
actual normal32/16 gameplay, Command Center and Workers render correctly.
`target/linux-current-runtime-pointer-match-20261010/visual-review.json` records
this bounded360-asset release-package startup evidence. Current pruned-archive
controls are verified below; audible output and combat performance remain open.
Windows, Steam Runtime and distribution-library obligations remain
open; building and packaging alone do not close delivery.

## Current unused-icon cleanup package

Commit c4a25fa removes24 unused legacy PNGs after source-reference checks and
a normal skirmish startup in an independent336-asset QA copy (actual exit0,
inspected screenshot). Original hashes and removed records remain in the audit.
`target/linux-current-package-pruned-20261010/verified-package.json` pins the
new archive, verifies all336 assets and exact equality of358 extracted files.
The executable is the existing thin-LTO checkpoint. Exact extracted archive
SHA256 `da292f12500f6685e681cfad4a03e55a651990b5630be52c5c3e684f4b209ab8`
passes basic controls/Save/Load UI on AMD RADV RENOIR in read-only Ubuntu24.04,
without a Nix mount. `target/linux-release-keyboard-20261010-01` records actual
container0, standard-US persistent keyboard/pointer on seat0, focused Bevy
window, F10 pause, Ctrl+S Save, Ctrl+L Load and Escape menu. Shared physical
key codes must be emitted; prior wtype custom-map logical symbols were not a
valid physical-hotkey test.

`target/linux-release-mouse-load-20261010-02` records container0 and inspected
normal mouse Save/two Load screenshots, including loading an unchanged real
release save copied into a fresh isolated profile. Both loads preserve file
bytes; the original source save is untouched. The menu is reopened after Save
before clicking Load. These are basic release input/Save/Load UI proofs, not
acceptance of all saved state, combat, performance, long sessions or audio.

The apparent distant-unit selection mismatch is diagnosed: the worker click
was swallowed by the selection HUD (actual client298,620). Ordinary empty-ground
deselection followed by the same click selects Worker; diagnostics report
ground distance0.24/screen distance24.916 with both pick tests passing. A model
projection defect is not established. Review JSONs retain images, seat/focus
records, archive/executable identities and save hashes.

## Atomic queue native/browser checkpoint

The matching immutable native/Wasm07 pair passes normal two-human/one-AI
control through the published signaling service. Actual host peak queue2 and
route completion pass; root independently compares158 complete same-tick DTOs
with zero mismatch from403 host/408 player records. Normal disconnect and own
room expiry pass. Evidence:
`target/online-human-ai-browser-native-20261010-07-retry02/independent-snapshot-verification.json`.
The initial run failed due to an unallowed local Origin and is retained.
The retry uses allowed localhost8080 without relaxing queue/snapshot checks.
This checkpoint predates the Hold fix; final-core multi-peer, TURN-only and
long-session acceptance are still unproven.
