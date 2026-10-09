# AI development verification — 2026-10-09

The AI pays registry construction costs, assigns a living Worker to each
foundation, and develops power and technology before filling its army with
opening units. Power planning includes existing foundations. Training prefers
available armed products; useful support units remain eligible, while unarmed
scouting Drones cannot bypass the battle-group cap.

Automation reads the existing match controller table. Human slots receive no
AI construction, harvesting, scouting, deployment, capture, support, or attack
orders. Missing controller settings disable automation. Generic opponent
iteration still includes human enemies. No faction statistics, difficulty
profile values, starting resources, or gameplay grants were changed.

## Verified regressions

`target/ai-tech-green-03.log`: **2 passed, 0 failed**, 49.78 seconds.

- Normal Chaos starts through the real headless game with GodotSkirmish's
  CommandCenter, Workers and opening unit, using normal 32 ore / 16 crystal.
  The original Plain & Simple match ends in **HumanDefeat** before its late
  TechLab build; the recorded phase confirms that this is an AI victory.
- A legitimate Big Arena match with starting slots 0 and 4 gives the same
  Normal AI enough time to develop the full chain. PowerReactor, RadarUplink,
  RoboticsBay and TechLab each begin with their normal refundable cost,
  observed resource debit, actual Worker contact pulse and completed state.
  Normal costs are respectively **3/1, 4/3, 5/5 and 7/7 ore/crystal**.
  TechLab's observed resource transaction is **95/31 → 88/24**.
- Newly spawned AntiAirWalker, MirageScoutTank, RocketTrooperRobot and
  TeslaCrawlerMk2 entities have actual Weapon components and technology
  requirements. Initial units are excluded from this production evidence.
  No unarmed scouting Drone is produced.
- In a Human0 / Human1 / AI2 match viewed as player 0, Human1 retains its
  32/16 resources, original units and no active orders for 45 game seconds;
  AI2 completes a VehicleFactory and trains an armed unit. Human1 remains a
  valid opponent for target selection.

The original missing-technology failure is preserved in
`target/ai-tech-red.log`; its test executable hash is recorded in
`target/ai-tech-red-binary.sha256`.

`target/ai-tech-savedflags.log`: the existing
`mission_orders_and_production_survive_load_without_replaying_triggers`
regression **passes (1/0)** in 1.61 seconds, verifying saved mission orders,
production and fired-trigger continuity.

## Full suite and exact configuration comparisons

`target/ai-tech-full-tests.log`: **170 passed, 0 failed, 1 ignored**,
96.30 seconds. The existing ignored test is the read-only production HTTP
bootstrap, which requires Internet access.

`target/ai-tech-targeted-20261009/comparison.json` and `status.json`:
**10/10 completed, 0 simulation timeouts, 0 validation failures**, 139.28 wall
seconds. Every result preserves the baseline configuration, normal 32/16
resources, GodotSkirmish loadout and initial headquarters positions. Typed
unit and structure counts sum to their respective totals.

All five actual 600-second timeouts in the older canonical difficulty sweep
resolve in this exact-configuration rerun:

| Baseline case | Map / factions | Difficulty order | New game seconds | Winner |
| --- | --- | --- | ---: | --- |
| 0049 | Tech Divide / Chaos | Easy, Beginner | 272.655 | Easy, player 0 |
| 0054 | Big Arena / Alliance | Beginner, Easy | 202.514 | Easy, player 1 |
| 0055 | Big Arena / Alliance | Easy, Beginner | 204.715 | Easy, player 0 |
| 0084 | Canyon Pass / Chaos | Beginner, Easy | 251.391 | Easy, player 1 |
| 0085 | Canyon Pass / Chaos | Easy, Beginner | 249.924 | Easy, player 0 |

The five Plain & Simple Normal comparisons also resolve. Chaos versus Demon
with Chaos in slot 1 changes from a Demon victory to a Chaos victory; the Chaos
mirror changes winning slot. Those individual outcomes do not establish a
faction win rate. Raw typed rosters are retained in each case's `result.json`.

The pinned audit executable is
`target/ai-tech-targeted-20261009/skirmish_audit`, SHA256
`805dbb05e210ab5be9ed73c367bfe0c4a956e49c051ed60bc164e93a41ddf583`.
The manifest is `target/ai-tech-targeted-20261009/cases.json`.
The full current 72-faction and 144-difficulty sweeps remain separate acceptance
work. This focused proof does not establish general balance or Steam release
readiness.

All compiler commands use locked dependencies, one Cargo job, Rust 1.95 and the
same restricted systemd compiler environment. Runtime asset roots are absolute.
