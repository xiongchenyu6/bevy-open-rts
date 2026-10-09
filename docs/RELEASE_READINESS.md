# Steam release quality plan

The goal is a complete, sellable RTS with a guided first session, a finishable
campaign, reliable saves, readable presentation and tested desktop delivery.
Green unit tests or selected screenshots do not establish release readiness.
The project is **not ready for Steam submission**. All nine gates remain in scope.

## Acceptance gates

| Area | Required evidence | Accepted evidence and remaining work |
| --- | --- | --- |
| First session | Fresh profile completes gathering, power, barracks, training, movement, attack and victory; skip/replay work | Seven lessons and sandbox persistence pass at recorded checkpoints; normal-input skip/return/graduation/replay captured. Revalidate final package. |
| Campaign | All six missions through normal controls in one App; correct objectives, retry/next/finale; persistent completion | Older fresh-process six-mission prefix passes. Current paid-AI same-App chain passes missions1–4. Global EMP fix completes mission5 foundations in isolation; later combat selection harness fails. Full chain remains open. Actual Restart resets/replays triggers. Full current chain remains open. |
| Save continuity | Fresh-process restoration of campaign triggers, orders, paid jobs, economy, fog, mines, temporary effects and support; safe missing/corrupt loads | First-wave campaign, paid jobs/fog/economy, harvest references, queued moves and F1/F5 effects pass in bounded runtime proofs. Mines and broader status/damage effects remain open. |
| Interface | English default; coherent icons/tooltips; no clipping at supported sizes; browser inspected | Native cards/tooltips/save menu inspected at1280x720 and900x720; compact unit markers and single finale-map action captured. Browser and remaining overlays require current review. |
| Art/audio | Readable menu/base/economy/combat/defeat/victory; sound, music, commands and volume controls verified | Recorded visual checkpoints only. Human audio validation and complete current presentation review remain open. Candidate campaign art is not shipped evidence. |
| Skirmish/balance | All maps/factions/difficulties/starts playable; reproducible repeated/independent-seed sweeps; no indefinite matches | Paid-AI canonical216 ends215 cases with1 timeout. Alternate starts, repeatability, independent seeds and broad balance remain open. |
| Delivery | Windows/Linux release archives launch without Rust/Nix from arbitrary directories; writable user data | Earlier optimized Linux package passes bounded hardware/input smoke. Windows GNU/Wine harness evidence exists. Current release packages, native Windows/MSVC and Steam Runtime remain open. |
| Reliability | Full suite, packaged smoke, long sessions and representative hardware combat performance | Global EMP full-schedule regressions pass3/3; full current library suite178 passed/0 failed/1 ignored. New runtime acceptance remains open. No representative combat performance or long-session acceptance yet. |
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

## Campaign and EMP blocker

The older d569 campaign checkpoint completes six recipes with growing persisted
prefixes and a fresh all-unlocked menu. It does not prove same-App play.
Current paid-AI recipes include an eastern AA defense against EngineerDrone HQ
capture; mission3 wins in isolation and in the four-mission chain. Mission5
previously left an AA foundation unfinished. The global EMP fix now completes
AA at65.5s and Radar at86.9s in the unchanged actual recipe. At91s the capture
helper fails to select a Tank during combat; HQ remains20HP and the match is
not terminal defeat. This is not a mission victory. See [campaign QA](CAMPAIGN_QA.md).

The first trace shows a selected Worker under EMP when placement occurs.
Construction and harvesting updates deleted their tasks during temporary EMP.
Two isolated expiry regressions pass after changing these updates to pause;
the full library suite passes172/0/1 in96.76s (`logs/emp-task-full.log`).
However, `current-campaign-twin-emp-paused` still fails at82.1s. The actual Combat
schedule also runs a global EMP order canceler, absent from the isolated tests.
That canceler is being removed, with explicit pauses for capture, garrison,
attack-move/patrol and queue progression. Three full-schedule regressions now pass, including actual legal repair units
and queued-task preservation. The full source suite passes173/0/1 in100.63s
(`logs/emp-task-full.log`). The fixture retains RecentDamage to suppress normal
Alliance self-repair and uses MobileRepairCrawler; Worker has no repair ability.
No mission5 victory is claimed yet. Preserve failed traces and test attempts.

Actual pause-menu Restart resets clock45.066→0.033, enemy HMG3→0 and all fired
flags tofalse; the authored wave replays with3HMG at45.066. This is bounded
restart evidence, not the full campaign acceptance.

## Save evidence

Mouse Save/Load and safe missing/corrupt recovery pass at1280x720 and900x720.
A fresh-process first-wave campaign load retains mission identity, fired flags,
nine units,41ore/22crystal,6,222 explored cells and one paid production job.
Normal UI frames advance the clock/job; no duplicate wave or briefing appears.

`screenshots/steam-quality/save-orders-{create,resume}-06` proves real harvest
and two Shift-queued moves survive a new process. The mineral reference remaps,
the route completes, ore32→38 and mineral240→234; original save bytes remain
unchanged. Earlier failed clicks landed on the command HUD, not a serializer bug.

`save-effects-{create,resume}-01` proves normally paid Chaos technology and actual
F1 RadarSweep/F5 ShieldOverdrive restoration. Target identity remaps; reveal,
shield scale/duration and cooldowns advance only by elapsed simulation time.
Both effects expire normally; save bytes remain unchanged. This covers those
two powers only. Mine restoration/trigger and other effects remain unverified.

## Canonical skirmish audit

Immutable audit pin805dbb05e210ab5be9ed73c367bfe0c4a956e49c051ed60bc164e93a41ddf583
runs216 configurations at normal32ore/16crystal:215 complete,1 simulation timeout,
overall exit2. Root validates every configuration/status/raw outcome with zero
errors (`logs/skirmish-current-canonical-all-validation-20261009.json`).
All72 Normal/Normal cases resolve in69.90–228.62 sim seconds. The48 non-mirror
configuration winners are Alliance17/Demon16/Chaos15; these are not win rates.

Case0103 is BigArena, ChaosEasy slot0 versus ChaosBeginner slot1 at600s.
Diagnostic pin4967e56 reproduces the same state: ScoutRover and RocketTrooperRobot
attack a living PowerReactor within range, with active cooldowns and no EMP.
Its HP is approximately5.0. A new read-only trace observes the same Reactor for400s:161 repair starts,
HP3.20–8.70, repeated manual repair and no kill. See
`logs/big-arena-repair-trace-summary-20261009.json`. The force includes one
ground-incapable AA unit. The default armed production pass now requires
ground capability unless the explicit air-threat branch is needed. Four AI
regressions pass (`target/ai-ground-force-focused-02.log`), including the exact
case0103 victory at323.476s with normal resources/costs, actual air-threat AA
selection and Human-controller protection. The full current library suite passes175/0/1 in87.55s
(`target/ai-ground-force-full.log`). Fresh216 matrix and current runtime
revalidation remain open; this does not erase the old failure. A simple chase explanation is contradicted by the report. See
`target/big-arena-timeout-diagnostic-20261009`. Both audit pins predate the global
EMP fix. Alternate starts, repeated runs, independent seeds and final-package
performance are separate requirements.

## Real-peer control evidence

Main pinf2a0d3cf runs two native Human peers plus normal AI slot2 through the
published lobby protocol. Both report success at gameclock50.01365. The remote
Worker executes three validated Move commands; the authoritative host observes
two queued moves and unchanged Human resources/untouched units. AI completes a
Factory and produces a new armed unit. Twenty same-tick snapshot summaries agree.
Normal disconnect and room expiry pass; the service exits0. Evidence:
`target/online-human-ai-live-20261009-01`.

Snapshots omit unit queues: queue preservation is host evidence and resulting
movement is client evidence. Browser peers, forced TURN, eight-player capacity,
long-session reliability, performance and final-package acceptance remain open.

## Delivery, presentation and provenance

See [Linux delivery](DESKTOP_DELIVERY.md), [Windows delivery](WINDOWS_DELIVERY.md),
[asset provenance](ASSET_PROVENANCE.md) and [store draft](STEAM_STORE_DRAFT.md)
for exact checkpoint scopes and hashes. Standard Linux fat-LTO release attempts
failed with OOM. Separately labeled thin-LTO candidates pass bounded no-Nix,
read-only-install rendering and physical input on AMD RADV RENOIR; measured idle
menu FPS is not representative combat performance. Container audio is unproven.
Windows GNU harnesses render under Wine, but the actual launch.cmd capture was
black; native Windows/release/audio acceptance remains open.

Current clean web staging verifies360 runtime files by hash and excludes full
source fonts and unused crosshairs. Full new Wasm/browser acceptance is pending.
Official Kenney153GLBs have matched upstream CC0 provenance; remaining generated
art, icons, VFX and voice-source gaps require resolution. An original generated
campaign-image candidate is outside shipped assets and is not a gameplay capture.

Keep the user-modified `flake.lock` and original ignored full-font source intact.
Use the canonical compiler environment for shared builds, one compiler at a
time, and supervised jobs for long validation. Compilation is not gameplay proof.

## Construction damage regression

The fresh ground-force sweep preserves another timeout: case0050, FourCorners
ChaosEasy slot0 versus ChaosNormal slot1 at600s. Eight ground attackers are
in range of a7.05HP Reactor while two foundations remain unfinished. A source
audit identifies unconditional construction HP overwrites, which erase damage
each frame and again on completion. The fix preserves received damage, awards
only HP earned by actual construction work, and never revives dead sites.
Nine construction regressions pass (`logs/foundation-damage-focused-01.log`),
including real weapon destruction of an unworked foundation, retained damage
through completion and no healing of stalled/dead sites. Full-suite passes178/0/1 in313.94s (`logs/foundation-damage-full-01.log`).
All three shared native binaries build successfully (6m37s). Exact case0050
replay completes at88.866s with Normal slot1 winning; configuration and normal
starting resources match the failed600s baseline (`target/four-corners-foundation-replay-20261010`).
Full matrix revalidation remains open; the frozen788 audit continues
unchanged, and no all-configurations completion is claimed.

The new mission5 native briefing is visually inspected at1280×720
(`current-campaign-twin-early-cover-2/00_briefing.png`): original artwork loads
and the title, objective copy and continue prompt are readable without clipping.
Other mission images/resolutions and browser presentation remain open. Revised
mission5 gameplay still loses HQ before AA preparation completes; this is not
a campaign pass. The new foundation-fix canonical216 audit is running on pin
8c0c4438 under `rts-skirmish-foundation-canonical-all-20261010.service`; the
previous788 sweep finishes215/216 with the preserved FourCorners timeout.

## Actual EMP application acceptance

A further source audit found that `queue_apply_emp_disabled` itself removed all
active tasks and queues when applying or extending EMP. Earlier regressions
attached the component directly and therefore missed this entry point. Commit
cd8067d removes that cancellation; the three full-schedule regressions now use
the actual queued helper and test status extension without task/queue loss.
They pass3/3 in2.72s, and the full suite passes178/0/1 in186.78s
(`logs/emp-application-{focused,full}.log`). All three native programs build in
2m30s; their sealed manifest is under `target/emp-application-shared-20261010-lqbfy3ax`.
The old-pin runtime trace did not observe an active constructor losing its task
to EMP in that particular run; no cancellation timestamp is asserted. Current
mission5 and mine runtime acceptance remain open. The original Wasm build was
stopped for this semantic correction, preserving logs/dependency cache, and
unit02 resumes with refreshed source provenance.

## Completed foundation-fix canonical audit

Pin8c0c4438 finishes all216 canonical configurations with216 normal completions
and no timeouts. Independent validation checks every raw config/status/outcome
and normal32ore/16crystal baseline with zero errors
(`logs/skirmish-foundation-canonical-validation-20261010.json`). This pin
predates the final real-EMP-application correction, and neither alternate starts
nor seeds, eight-peer capacity or performance are covered by this result.

Mine creation06 passes real harvesting, paid expansion/training and mouse Save
on verified helper pin4866d13. Fresh-process restoration passes its early source
reference/arming/layer-state assertions, but the saved mine remains outside the
miner's route and strict causal triggering fails. Mine continuity is not accepted
yet. Retries must confirm the exact helper strings in the sealed executable;
an earlier source change occurred after compiler input and produced a stale bin.

Fresh Wasm e576b058 builds successfully. Initial headless captures were black
despite145/145 assets; a corrected SwiftShader compositor harness displays the
same unchanged program. Actual English menu/setup/match screens are observed
at1280×720 and900×720; complete real selection/movement and paired-frame
inspection remain pending. Software rendering is not a hardware performance claim.
