# Campaign QA: Preserve the Headquarters

Scope: capture indices **2 / 4** are **Mission 3: Canyon Siege / Mission 5:
Twin Front**, respectively. These failed runs do not establish that either
mission is impossible; completing all six remains a release gate.

## Findings grounded in the current run

| Evidence | What it establishes |
| --- | --- |
| `logs/steam-campaign-2.log` and `campaign/2/failure.png` | Factory construction and eight-tank training succeed. Defeat occurs at 02:44 with 26 enemy units/10 buildings destroyed, seven friendly unit losses/two building losses. Surviving selected tanks show full force HP. Combat and an offensive advance occur; this is not the previous construction deadlock. |
| `logs/steam-campaign-4.log` and `campaign/4/failure.png` | Construction/training succeed; defeat at 01:52 follows 11 enemy kills, zero enemy buildings destroyed and ten unit/three building losses. The screenshot shows a substantial enemy force near friendly structures. |
| `render_campaign_play` in `src/bin/capture.rs` | Non-survival missions briefly defend with the first three tanks, then select the whole army for each offensive order. No defender group, static defenses or offensive replenishment is retained. Only survival missions receive mixed infantry/rocket production. |
| `campaign.rs` | Both failures are Hard annihilation missions. Mission 3's friendly reinforcements arrive at 241s, after this defeat. Mission 5's first flank wave arrives at 90s; its friendly reinforcements arrive at 301s, after this defeat. |

The harness **orders its temporary defenders away**. HQ abandonment is the leading
explanation, rather than a demonstrated navigation fault. Exact HQ attackers and
damage timings are not logged, so this is not a complete causal proof. Mission 3's result also shows zero teams left; log HQ/anchor elimination order to distinguish
simultaneous losses from a result-counter issue. That image alone proves neither. Aircraft-like
shapes in Mission 5's image suggest checking air pressure: registry Tank weapons
cannot attack air; RocketInfantry can attack both domains. Unit identities need
read-only telemetry before attributing defeat to that mismatch.

## Next normal-input recipe

1. Keep gathering. Build Power, Barracks and **two AntiGroundTurrets** covering
   incoming approaches, then the factory. Retain the first three tanks at home.
2. Train three HMG infantry, build Radar, then three RocketInfantry. Use enabled
   UI commands and actual costs/queues. Treat these numbers as the next experiment,
   not verified optimal balance.
3. Shift-click defensive units, assign **Ctrl+1**, position them outside the HQ
   footprint, and enable Hold Position after arrival. On Four Corners cover both
   north and east approaches. Do not repeatedly toggle an already-enabled hold.
4. Train a **separate six-tank assault**, plus two rockets if air threats are
   observed. Count product types, not the global army count: that count includes
   infantry. Click/Shift-click newcomers, assign **Ctrl+2**; never reselect all army.
5. Recall **2**, issue the enabled Attack-Move command toward the nearest enemy
   force/structure. Target selection must use that group's position, not defenders'
   blended position. Keep **1** at home.
6. Check normal gameplay every 3–5 simulated seconds. Replenish defensive losses
   first; continuously train replacements at a safe rally/staging point. Add
   attackers with **Ctrl+Shift+2** and send them to the surviving assault group.
   Refill groups through clicks and keyboard input, not component mutation.
7. If the HQ takes damage, recall/command **1** or temporarily bring **2** home.
   Regroup before another advance. Survive until existing scripted reinforcements;
   merge them through selection, then hunt remaining enemies for annihilation.

Record HQ health, living defenders/attackers, pending production, attackers near
the base and their domains, plus checkpoints at first attack/base contact/HQ
damage. A stationary ordered force with an unobstructed reachable destination is
a separate navigation repro; a dead HQ with attackers abroad is a strategy failure.
Victory, persistent completion and continuation must still be captured through
normal input. No spawning, resource grants, AI changes, speed changes or clock jumps.
