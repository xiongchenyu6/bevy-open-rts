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
preserved. A fresh same-App six-mission chain is running as
`rts-campaign-chain-live-response-01` with evidence
`current-campaign-chain-live-response/`. Its acceptance is still **pending**. No unit spawns, resource grants, enemy/stat/trigger nerfs or direct
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

Pin the compiled capture executable in an ignored `target/` directory before a
sweep. Record its SHA256, source revision and recipe changes. Use a new profile
directory and run indices 0 through 5 sequentially. After each process, verify that
`bevy-open-rts/campaign.ron` contains exactly the completed prefix:

`hold_the_line`, `strike_the_outpost`, `canyon_siege`,
`highland_bastion_stand`, `twin_front`, `thunder_decapitation`.

Inspect every victory and continuation image, then start a fresh `menu-campaign`
process with the same profile and inspect all six unlocked cards. No unit spawning,
resource grants, clock jumps, AI changes or level nerfs are permitted. Existing
authored reinforcements and supply deliveries remain normal mission behavior.
