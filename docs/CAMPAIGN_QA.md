# Campaign QA

All six missions must win through ordinary mouse/keyboard input, record completion
on disk, and continue to the next briefing (or return to the campaign map after
Mission 6). A successful direct mission launch does not satisfy this gate.

## Verified checkpoint

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
