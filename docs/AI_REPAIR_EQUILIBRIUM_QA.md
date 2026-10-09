# Big Arena repair-equilibrium regression — 2026-10-09

The canonical current 216-case sweep (`805dbb05…`) had one genuine failure:
case 0103, Big Arena Chaos Easy in slot 0 versus Chaos Beginner in slot 1.
Both anchors remained alive after 600 game seconds. A second immutable audit
reproduced it and identified the attacked target as a **PowerReactor**, not a
Worker or an unreachable target. ScoutRover and RocketTrooperRobot were in
range and firing; the third battle-group member was an AntiAirWalker whose
weapon cannot attack ground targets.

The opt-in read-only audit trace uses the real simulation, normal 32/16
resources and GodotSkirmish startup. It neither alters actors nor adds economy,
health, timers or substitute simulation. Its RED evidence is:

- `target/big-arena-repair-trace-20261009/trace.jsonl`: **12,003 frames** from
  200.014 to 600.008 game seconds, always attacking that PowerReactor.
- Reactor HP oscillates between **3.200 and 8.700**, with no kill in those
  400 seconds. ManualStructureRepair is active on **10,801 frames**, and its
  component starts again **161 times**.
- Uninterrupted repair frames directly deplete the component by approximately
  **0.1 HP per 1/30-second tick**, matching the existing **3 HP/second** rule.
  Independent HP observations include repeated healing and damage drops.
- The defender's bank changes from **125/153 to 684/39**. Worker cargo dropoffs
  are estimates, and the residual bank calculation is not exact repair
  spending. At 155 component restarts with no observed cargo dropoff, the bank
  decreases by 1/1 and the current-damage cost rule also calculates 1/1; these
  are paired observations, not a complete transaction ledger.

`analysis.json` retains the measured summary. The RED trace executable SHA256
is `2a7f7c4b6b93663f69d0558fa11fb50db40fdb988fe58f2c57f6f84b688d2b98`.
Registry damage/cooldown division is only nominal: actual faction damage and
cooldown modifiers must be considered. The diagnosis rests primarily on the
observed HP recovery and actual repair-component depletion.

## Minimal correction and focused GREEN

The ordinary armed-product search now requires `can_attack_ground`. The
existing explicit air-threat branch remains first and unchanged, so an AA-only
product stays available when there is a real flying threat. No unit statistics,
difficulty profiles, battle-group caps, costs, repair behavior, resource grants
or tactical target-selection rules were changed.

`target/ai-ground-force-focused-02.log`: **4 passed, 0 failed**, 90.10 seconds.
The exact normal-start 0103 regression now ends with **Easy player 0 winning at
323.476 game seconds**. Its living ground force is ScoutRover,
RocketTrooperRobot and MirageScoutTank, with no AA-only unit occupying the
three-unit cap. An isolated registry/profile/prerequisite/cost fixture starts
its cursor on AntiAirWalker: no air threat selects a ground-capable product;
an actual Drone threat still selects AntiAirWalker.

`target/ai-ground-force-full.log`: **175 passed, 0 failed, 1 ignored**,
87.55 seconds. The existing ignored test is the read-only production HTTP
bootstrap.

The trace can be enabled on the audit binary with
`OPEN_BEVY_REPAIR_TRACE=<new-jsonl-path>` and optional
`OPEN_BEVY_REPAIR_TRACE_FROM_SECONDS=200`. Output refuses to overwrite an
existing trace. This regression and the library suite do not replace the
fresh immutable 216-case sweep or alternate-start-pair acceptance.
