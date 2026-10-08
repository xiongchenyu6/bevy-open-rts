# Skirmish QA and Balance Audit

Audit date: 2026-10-08. This describes current harness coverage and the next
acceptance matrix; it does not claim that the full matrix has passed.

## Existing diagnostic scope

`capture match`, `ai-duel`, `faction-duel` and `arena` reuse the real headless
Bevy systems. They use a fixed **1/30-second timestep**, with match-result checks
in five-second batches after 20 startup updates. Reported duration is approximate;
requested limits round up to the next batch.

- `capture ai-duel normal hard 120` changes difficulty, but retains default
  factions: player 0 Alliance, player 1 Demon. This confounds difficulty with
  faction and cannot establish a difficulty curve.
- `capture faction-duel alliance demon 600` explicitly sets both factions and
  uses Normal difficulty on both sides.
- Both duels currently use Plain & Simple, physical spawn slots 0/1, standard
  minimal startup, and default 32 ore/16 crystal. No joint map/faction/difficulty/
  spawn configuration is exposed through their CLI.
- Duel commands return success even when the phase remains `Running`. Their
  printed winner is inferred from surviving structure/unit counts, rather than
  the authoritative victory condition. Treat these as diagnostics, not gates.
- `arena` uses equal unit counts and identical factions. It isolates a combat
  matchup, but does not normalize purchase cost or test a complete economy.

`diag_ai_vs_ai_match_resolves` checks resolution on the default setup.
`diag_all_three_factions_playable` checks that each faction produces more units
than its opening roster; it logs resolution but does not require it. Neither
covers all maps, start positions or difficulties.

## Reproducible variation

There is no exposed AI seed parameter. Placement uses spawn-ID/counter-derived
values; `RandomMapCursor` selects a map rather than an independent AI strategy.
Use explicit configurations and repeated identical runs to check repeatability.
Do not report repetitions as independent seeds or estimate statistical win rates.
Fixed-step execution alone does not prove identical results across ECS scheduling,
platforms or builds; record executable hash, setup and observed repeatability.

## Required scenario matrix

Factions: **Alliance, Demon, Chaos**. Difficulties: **Beginner, Easy, Normal, Hard**.

| Map ID | Start slots | Ordered distinct start pairs |
| --- | ---: | ---: |
| `plain_and_simple` | 4 | 12 |
| `four_corners` | 4 | 12 |
| `tech_divide` | 6 | 30 |
| `big_arena` | 8 | 56 |
| `canyon_pass` | 2 | 2 |
| `crossfire` | 4 | 12 |
| `ring_valley` | 4 | 12 |
| `highland_bastion` | 2 | 2 |

First run each map's canonical 0/1 starts with all nine ordered faction pairs at
Normal: **72 scenarios**. Expand to all 138 ordered legal start pairs:
**1,242 scenarios**. These are configuration observations, not independent trials.

For difficulty comparisons, hold faction identical on both sides. Test each
adjacent pair (Beginner/Easy, Easy/Normal, Normal/Hard) in both assignments:
**144 canonical-start scenarios**, or **2,484 across all start pairs**.
Beginner has intentionally disabled offense; a passive mirror timeout is not
itself a broken match. Resolution expectations must follow the selected profiles.
Do not change resources, grant armies or nerf units to make the matrix pass.

## Missing harness inputs and evidence

Expose map ID, both factions/difficulties, both physical spawn slots, explicit
active slots, starting resources/loadout and simulation limit together. Return
structured setup and **authoritative winning side/anchors**, phase, timeout flag,
simulated elapsed time, surviving units/structures, economy and pending construction.
A timeout must remain explicit even when the diagnostic process exits normally.
Keep a wall-clock cap and run sequentially while release compilation is active.

## Joint acceptance executable

The new `skirmish_audit` binary accepts all scenario inputs together:

```sh
cargo build --locked --bin skirmish_audit
BEVY_ASSET_ROOT="$PWD" timeout 180s target/debug/skirmish_audit \
  plain_and_simple alliance normal 0 demon hard 1 120 \
  /tmp/skirmish-normal-hard.json
```

Arguments are map ID or shipped map path, faction/difficulty/spawn for player A,
then faction/difficulty/spawn for B, simulation limit (1–3600 seconds), and a new
JSON evidence path. Slots must be distinct and legal for that map. Other players
are inactive; economy is 32/16, minimal normal startup, Annihilation victory and
1/30-second steps. No synthetic seed or resource grants are introduced.

The report preserves actual match clock, original setup, resolved map, initially
spawned command-center coordinates, authoritative active anchors and winning
teams, phase/reason, surviving counts, economy and pending construction. Winners
are never inferred from army size. Completed matches exit 0; simulation timeouts
write their report and exit 2; invalid inputs/I/O fail with exit 1. Existing
evidence is never overwritten. External wall-clock expiry remains distinct from
an in-game timeout and may leave no completed report.

Compile/runtime acceptance passed on 2026-10-08. Eight invalid-config/overwrite
checks passed (`logs/skirmish-joint-invalid-config.json`). A swapped-slot Canyon
case timed out at actual clock 1.00000036 seconds with exit 2 and preserved JSON:
player 0 spawned at [18,0,0], player 1 at [-18,0,0]. The shipped map-path alias
also passed. Plain & Simple Alliance Normal versus Demon Hard completed at
88.79899 seconds with exit 0, active anchors [false,true] and authoritative
winning team [1]. Evidence is under `target/skirmish-audit-validation/`, with
build log `logs/skirmish-joint-build.log`. These are bounded pilot cases; the
full matrix remains unrun.

## Current bounded evidence

The cached debug executable completed `ai-duel normal hard 120` without compilation
or GPU work: **MatchFinished at approximately 90 seconds**. It reported player 1
Hard with 18 units/10 structures versus player 0 with 2 units/4 structures.
Evidence: `logs/skirmish-audit-normal-hard-120.log`. The printed winner remains
heuristic; the factions differ, and this is one default-map checkpoint rather
than current-release or full-matrix acceptance.
