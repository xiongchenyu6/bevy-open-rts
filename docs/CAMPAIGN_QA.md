# Campaign QA

All six missions must win through ordinary mouse/keyboard input, record completion
on disk, and continue to the next briefing (or return to the campaign map after
Mission 6). A successful direct mission launch does not satisfy this gate.

## Verified six-process checkpoint

The fresh-profile sequential sweep in
`screenshots/steam-quality/current-campaign-profile-coherent-3/` completed all six
missions. `results.json` records exit code zero and the exact growing completion
prefix after every process; `menu-result.json` records a seventh fresh menu
process loading all six completion IDs. Each mission includes `02_victory.png`,
`03_continue.png`, and `campaign-after.ron`. Logs are
`logs/current-campaign-profile-coherent-3-{0..5}.log`.

| Index / mission | Observed victory time | Continuation |
| --- | --- | --- |
| 0 / Hold the Line | 04:00 | Mission 2 briefing |
| 1 / Strike the Outpost | 01:40 | Mission 3 briefing |
| 2 / Canyon Siege | 03:03 | Mission 4 briefing |
| 3 / Highland Bastion | 05:00 | Mission 5 briefing |
| 4 / Twin Front | 04:34 | Mission 6 briefing |
| 5 / Thunder Decapitation | 01:22 | Campaign map |

The executable SHA256 is
`d5697bacc5a1360ff436c523c9c7974a18bc89d87a6c4b0179e8e646a8d0457c`;
it is pinned at `target/campaign-finale-ground-pin-opfn9htj/capture`.
`environment.json` records source revision, the capture-only recipe corrections,
absolute asset root and fresh absolute `XDG_DATA_HOME`. The sweep explicitly set
`OPEN_RTS_CAPTURE_PLAYER_PROFILE=1`.

This proves six separate mission processes sharing one fresh, growing profile,
plus actual Next Mission clicks launching the following briefing in each process.
It does not prove playing all six missions in one application instance: direct
`campaign-play` entry bypasses campaign-menu locks. The seventh fresh menu process
verifies persisted unlocks. Same-application full play and Retry/Restart reset
remain separate acceptance checks.

A subsequent finale run in
`screenshots/steam-quality/finale-single-map-button/` won through normal input at
01:22 and verified one Map button and actual return after the duplicate-button
correction. Core gameplay changes after either pinned checkpoint require rerunning
the affected campaign missions before treating those changes as accepted.

## Current-source acceptance status

Current six-mission same-application acceptance is **not complete**. The fresh
`current-campaign-chain-east-aa/` attempt won Missions 1–4, verified the exact
four-ID disk prefix, then failed during Twin Front setup. Its partial prefix must
not be reported as a completed campaign. Earlier interrupted attempts and
isolated mission starts do not prove coherent progression.

The pause-menu Restart correction was verified in
`current-campaign-restart-paid-ai/`: the first wave fired at 45.066s with three
enemy HMG; an actual Restart click reset the clock to 0.033s, removed those units
and cleared all five trigger flags; the same wave fired again at 45.066s.
Screenshots show the real menu, fresh briefing and replayed wave banner. The
saved-trigger/order/production continuity regression also passed, confirming
loaded saves restore their flags after the fresh-match reset.

Temporary EMP originally erased active tasks and queues through three paths:
construction/harvest updates, the global EMP cleanup system, and the actual EMP
application helper. The global cleanup and application deletion were removed;
ability and queue processing now pause until expiry. Three regressions use the
actual queued application helper and full shared match schedule, including real
expiry and duration extension. The final full suite passed **178 tests, zero
failures, one ignored** (`logs/emp-application-full.log`); capture, main and audit
builds all exited zero. Earlier tests which attached an EMP component directly
omitted the real application path and are not sufficient evidence on their own.

Twin Front exposed successive ordinary-play issues: a captured HQ after late AA,
construction interrupted by EMP, a target dying during a mouse click, producer
queues stopping at five jobs, delayed second cover and infantry chasing outside
its static defenses. The paid opening now unlocks Radar/rockets early, refills
queues, starts separate-worker construction concurrently, places east AA before
army movement can occupy its pad, and uses the real Ctrl+Alt+A army shortcut.
Hold is a toggle; the helper now checks current holding flags before pressing it.
It logs actual held/moving counts during the opening defense window. The latest
current-core run, `current-campaign-twin-capability-hold-current-core`, used sealed
capture SHA256 `ac27f9ff0e99d276c36b4759a238347ed944590f7b4d3757a5760f09f08aed23`
and a fresh profile. Its actual child exited **1** with HumanDefeat at 106.8 seconds.
All nine paid structures completed by 79.0 seconds; the full 15 combat units
stayed held with zero move orders, then declined through ground combat. All
three turrets were destroyed before the first HQ damage at 102.9 seconds. This
verifies the Hold predicate correction, but did not verify victory. The shared
current-core suite passed 181 tests, zero failures, one ignored.

The capture-only response now uses ordinary attack-move on live ground threats
within 12 metres of HQ, prioritizing missile carriers and jammers, then regroups
when local threats clear. Sealed capture SHA256
`f6069ba1cb44540f4749802956020ea00b4dad60bcd4ba540741db94884f985a`
passed actual isolated Twin Front in **04:49**, with HQ **20/20**, actual child
exit **0**, saved completion and real Next Mission continuation. Evidence is
`current-campaign-twin-live-ground-response/`; the previous losing run remains
preserved. The fresh same-App run `rts-campaign-chain-live-response-01`, evidence
`current-campaign-chain-live-response/`, completed Missions 1–4 and verified all
four exact disk prefixes, then lost Twin Front at 117.8 seconds. Its actual child
exited **1**; acceptance is **incomplete**. This failure exposed retained
`AiDirector` timers, production cursors and opening-grace flags across match
transitions. Two actual shared restart/load regressions reproduced the leak and now pass:
fresh matches reset the AI director plus economy sabotage/veterancy, while loads
restore all ten saved AI vectors and saved economy state exactly before Update.
Commit `3478190` passed the full suite (183 passed, zero failures, one ignored)
and all three native builds. Sealed current capture SHA256
`7fe3cb8024e48b7557df1695a917c5bc47a5e0a47c434840edee32bf8140933e`
ran `current-campaign-chain-fresh-match-reset/`: Missions 1–2 won with both
exact saved prefixes verified; Canyon Siege then stopped at an unfinished second
ground turret at 118.7 seconds, HQ still 20/20. Actual child exit **1** and
`accepted: false` are preserved. One worker disappeared during that construction;
the remaining worker was idle and had a verified reachable contact to the site.
The initial selected-builder trace was not enabled, so the precise death/order
transition is not yet proven. Normal-input abandoned-foundation recovery and
additional paid-queue lifecycle regressions are prepared for the next build.
Full same-App campaign acceptance remains **incomplete**.

The next lifecycle pass reproduced an old paid job refunding into a new match
(32 starting ore became 34) and an old battle-event camera target persisting.
Commit `648beff` resets both before loaded-save restoration and adds ordinary
RMB recovery for a living foundation without an active or queued constructor.
Recovery is bounded to three attempts and reads the actual builder/target before
and after input. Weather save version 3 preserves the actual transition state;
fresh entry resets it before loading. Five lifecycle regressions and five weather
checks passed, followed by **190 passed, zero failures, one ignored**.

Current capture SHA256
`f58e08a3054eda63952b847238de9b2a98a547fcbf1c03ebcb05a18e4a555e3f`
ran `current-campaign-chain-lifecycle-final/` in one App with a fresh profile.
Missions 1–4 won (04:00, 01:55, 02:29, 05:00), with four exact saved prefixes
and all four victory/continuation pairs inspected. Twin Front survived its
opening but lost at 244.7 seconds while a blocking six-tank training quota waited
with zero crystal; the actual child exited **1**. The constructor recovery branch
did not run, so its input behavior is compiled but not yet verified by that run.
The construction trace and failed four-mission prefix remain preserved.

The capture recipe now leaves replacement quotas in normal paid queues and
immediately enters the common harvesting/defense/assault loop after the opening.
The next pin also needs the independently diagnosed AI compatible-target fix.
All six victories, each exact saved prefix, and the seventh fresh campaign menu
remain required before acceptance.

The compatible-target AI build passed 192 tests, zero failures, one ignored.
Its capture SHA256 `4a56196766e0f3dc9ce7c72fe3ea03cc4fba8682708e9f276b10511ee47163eb`
passed the actual pause-menu Restart check (`current-campaign-restart-compatible-ai/`):
45.066 seconds/three HMG/flags TTFFF, reset to 0.033 seconds/zero HMG/all flags
false, then the same wave again at 45.066 seconds. Menu/reset/replayed images were
inspected. Isolated M5 stopped at 122.8 seconds on a selection mismatch, HQ20;
the log lacked enough identity detail to classify the precise target-death frame.

Capture-only SHA256 `b8438329396aec3ff81633a5afa09cc7003535d5254f9395240e8d39aaee2f33`
adds requested/clicked/dead diagnostics, ordinary empty-ground selection recovery
when all requested targets die, and caller guards that skip subsequent controls
on an empty selection. Its M5 attempt (`current-campaign-twin-dead-selection/`)
stopped at 171.1 seconds with HQ20, Factory16, no combat units and zero crystal;
actual child exit1. The empty-selection branch did not occur, so that branch
remains compiled-only. The crystal collector's observed field was outside the
original army staging/range. An ordinary paid-input staging change toward that
field is prepared; no additional resource grants or stat changes are used.
No unit spawns, resource grants, enemy/stat/trigger nerfs or direct
combat-state/time injection are used by these recipes.

The ignored supervisor `target/run-campaign-chain-supervised.py` watches the real
profile every 100ms, copies and verifies each growing on-disk prefix, records the
actual subprocess exit, and opens a fresh campaign-menu process only after all
six missions and all six disk checkpoints succeed. First mission entry is direct;
Missions 2–6 must arrive through the actual Next Mission clicks in the same App.

## Recipe behavior

Survival production uses paid type-specific queues, reserves Radar's actual cost,
and produces six tanks early alongside eight HMG and four rocket infantry.
Highland guards use ordinary selection, control groups and hold/attack commands.
The finale immediately builds Factory after Power, queues two paid tanks while a
north-facing ground turret builds, keeps three defender tanks and replenishes its
separate assault. Its nearby enemy HQ makes ground defense the opening priority.

Game-authoritative HumanVictory still proceeds through persistence and continuation
checks if a production gesture encounters the terminal state. Running and defeat
errors remain failures. Continuation screenshots wait for the briefing typewriter
to reveal the full text.

## Reproducible acceptance

Pin the compiled capture executable in an ignored `target/` directory before the
run. Record its SHA256, source revision and recipe changes. Create a fresh absolute
`XDG_DATA_HOME`, set the absolute `BEVY_ASSET_ROOT` and
`OPEN_RTS_CAPTURE_PLAYER_PROFILE=1`, and run the supervised **single**
`capture campaign-chain <evidence-directory>` process. The same App must win all
six missions; Missions 2–6 start through the real Next Mission buttons.

During that process, observe the actual `bevy-open-rts/campaign.ron` file and copy
it after each victory. Require exactly the growing ordered prefix, with no
regressions or extra completed IDs:

`hold_the_line`, `strike_the_outpost`, `canyon_siege`,
`highland_bastion_stand`, `twin_front`, `thunder_decapitation`.

Require the actual child exit zero, all six disk-prefix checkpoints and the
same-App chain result. Inspect every victory and continuation image. Then start
a seventh fresh `menu-campaign` process with the same profile, require its actual
exit zero and inspect all six unlocked cards. The current supervisor implements
these process and persistence checks; screenshots still require visual review.

Separate `campaign-play <index>` processes are **diagnostic only**: they can
reproduce a mission recipe and its continuation, but cannot establish that all
six missions complete in one App. Preserve their failures and clearly label
historical separate-process checkpoints.

No unit spawning, resource grants, clock jumps, AI changes or level nerfs are
permitted. Existing authored reinforcements and supply deliveries remain normal
mission behavior.

## Latest visible-only attempt

Capture SHA256 `8d391c35c44e2d2778e604c8991b1e2e5d4e5077d55f113d1f63c7cc7870ffdf`
uses the actual visibility boolean for all campaign maintenance/pressure/assault
decisions. Unknown enemy sectors are explored through ordinary attack-move using
only own HQ and known map geometry. Raw enemy data remains diagnostic only.
Updated runtime-icon hashes are archived alongside the process artifacts.

`current-campaign-twin-visible-crystal-cover/` passed isolated M5 in **05:32**,
HQ20/20, zero building losses, saved completion and actual Next Mission verified.
Victory and continuation screenshots were inspected. This does not establish a
full campaign chain.

`current-campaign-chain-visible-only/` won M0 and verified its exact saved prefix,
then lost M1 with **HumanDefeat**, actual child exit1/acceptedfalse. The generic
visible anchor selected a local ScoutRover and RocketGunship, redirecting the
whole army away from the headquarters objective. The first single-sector scout
was not sufficient. An objective-focused visible-HQ-only helper with persistent
clockwise known-map scouting is prepared outside Rust source while the Linux
release build is frozen. It needs an actual isolated M1 victory before another
six-mission chain. Current-source campaign acceptance remains **incomplete**.

## Hold correction and remaining M1 counterplay

Held guards previously suppressed automatic acquisition entirely, and an existing
attack target could make them pursue despite Hold. Commit `3c7c0d1` fixes both:
held mobile units acquire compatible enemies only within weapon reach, keep
their position, and stop pursuing a target that leaves range. Four full-schedule
regressions reproduced three failures before the fix and all passed afterward.
The full suite passed **200 tests, zero failures, one ignored**, and all three
native binaries built successfully.

Capture SHA256 `373886ba9cc2dfe08499b092dfdcfb7ea89b711cc1107d1fa0a930756eb4648b`
ran `current-campaign-m1-hold-fixed/`. Actual child exit1/HumanDefeat at238.9
seconds remains a failed mission proof. Gunship HP fell17→14.8 and it was absent
from the final global contact snapshot, confirming actual air firing. Repeated
new Tank entity births demonstrate continuing production; the five-job queue
does not prove a production stall. Factory stayed16HP, no EMP/construction,
normal power, and substantial ore/crystal remained. The later heavy airship
retained27HP while rocket guards and AA cover were lost.

That run also exercised requested-target death during camera settling: the
normal empty-ground click cleared selection and actual selected[] was verified.
The helper then skipped subsequent controls for the empty result.

The next capture-only plan adds two paid AntiAirWalkers and visible-air
interception with air-capable home guards, prioritizing counter replacements
before more Tanks. It is prepared outside source while the constructor diagnostic
build is frozen. No enemy stats, timers, resource grants or direct queue edits are
used. Isolated M1 and the full same-App campaign remain **unaccepted**.

## Paid AA Walker attempt

Capture SHA256 `a7d68de26f8ae90fd366f7f8d0839d776857ba6652bdc720efc700835dc9b849`
ran `current-campaign-m1-aa-counter/` with two normally paid AntiAirWalkers.
The archived log records Walker queues at34/44 seconds; periodic telemetry first
observes one live walker at64.1 seconds and both at77 seconds in home defense.
These sample times are not exact birth times. Raw HeavyAirship entries include
27HP earlier and2HP at201.9 seconds, but lack entity keys; they do not establish
that both samples describe the same airship or identify the damage source.

The run ended **HumanDefeat at443.7 seconds**, actual child exit1 in
`process-result.json`. Telemetry records own HQ20HP at440.014 seconds,
11.3375HP at440.581, then absent at443.747. The failure contact snapshot records
the enemy HQ at20HP. Late telemetry records308ore/78crystal, Factory16HP and
normal power; these observations do not establish the cause of the defeat or
whether any queued job was blocked.

The log records two **visible CommandCenter assault anchors** at lines160/173,
bounded by logged clocks201.9–215.9 and215.9–232.3. Thus the enemy HQ was visible
when those actions were selected; the exact event times and HQ HP were not logged.
The construction trace contains245 snapshots from1.9 to104.8 seconds, so it
cannot explain the later assault path. Periodic telemetry lacks attack targets,
AttackMove destinations, PlannedPath details and EMP state. Unit absence in a
later sample is not independently classified as a death.

The read-only analyzer `scripts/analyze_campaign_observations.py` records source
hashes, observations and missing fields. It was run against this evidence and
passed `python3 -m py_compile`; its generated report is
`target/campaign-m1-aa-observations-report.json`. It issues no gameplay inputs.
The next diagnostic run awaits the constructor/Nav seal and will capture the
existing order, path and visibility views without changing quotas or stats.
**M1 and the full same-App campaign remain unaccepted.**

## Prepared native observation capture

The capture defaults now render1920×1080 directly into the native image target;
window size, cursor centers and click bounds share those dimensions. Explicit
semantic/HUD fixture dimensions and simulation tick arrays retain their settings.
Campaign telemetry now joins own entity keys to active Move/AttackMove/Attack
orders, path goal/index/next waypoint and EMP snapshots without an intervening
application update. Missing joins are explicit, and absent components remain
null. Enemy HQ observations require actual visible=true, team1 and positive HP;
raw diagnostic threats remain separate from visible action decisions.

The unified full27 library run passed219 tests,0 failed,1 ignored. The native
build and fresh unchanged-quota M1 observation run are still pending at this
checkpoint. Neither1920 gameplay PNG dimensions nor the new telemetry fields
have yet been accepted from an actual run. This change supplies observations;
it does not accept M1 or the six-mission campaign.

## Native 1080p observation result

The sealed construction/Nav capture `47b3c7188216b90928fcef1d3114d16dfc688e423ba27ebc67ade9ea0267d363`
ran fresh-profile M1 with the same paid AA quotas in
`current-campaign-m1-nav-1080/`. All archived PNG headers are1920×1080 and the
periodic joined order/path/EMP fields are present, with identical campaign/live/
contact clocks. Root inspected the assault screenshot: it is mainly fog/empty
space with long order lines, so it is **not accepted as a polished store image**.
The original images are preserved.

The process exhausted its existing240-round macro budget and exited1 with phase
**Running**, not HumanDefeat; the failure contact clock is1390.215 seconds and
own HQ remains20HP. Two Tank entity keys244813132745/2005749724971 have precisely
unchanged position, Move/Attack orders, goal and next waypoint in224 samples
from213.517 to1389.682 seconds (1176.166 seconds). The endpoint identifies their
attack targets as enemy Barracks/VehicleFactory. A read-only endpoint geometry
replay found no static rejection for their next desired step; the authored
Crossfire terrain is flat. This does not identify the live movement cause.

The measured endpoint fixture (`movement_endpoint_tests`) rebuilds112 archived
actor footprints in the real headless schedule with registry-default runtime
components. Its focused03 result is1 passed/0 failed in0.95 seconds; both Tanks
advance approximately2.75m in30 ticks. Focused01 typecheck failure and focused02
HUD fixture panic are retained. The passing fixture rejects the proposed static
geometry explanation under those defaults; it does not reproduce or fix the
live stall, or accept paid gameplay.

Additional read-only speed/domain/Chrono and simulation-gating telemetry is
prepared for the next shared capture. Those runtime fields still await an actual
run; **M1 and the six-mission same-App campaign remain unaccepted**.
