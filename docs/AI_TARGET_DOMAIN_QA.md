# AI target compatibility QA

On Plain & Simple with Alliance Easy in spawn 1 against Alliance Beginner in spawn 3, the Easy army stalled for 600 seconds. Its Tank, Scout Rover, and Flame Assault Buggy repeatedly targeted a nearby flying Drone even though all three weapons could only attack ground targets. Attack-order validation cleared those unsupported orders, and wave restoration assigned them again.

The director and wave restoration now select a living hostile target separately for each unit, using its actual runtime Weapon and the existing `attack_order_target_valid` rule. Valid air targets remain available to air-capable weapons. Unarmed support behavior remains available. Construction retains its original target query. No faction statistics, costs, difficulty timers, army caps, resource grants, or protocol fields changed. The obsolete generic target helper was removed.

## Evidence

- The exact real headless RED fixture used normal 32 ore/16 crystal, Godot startup, starts `[1,3]`, and the unchanged 600-second limit. It ended Running at 600.008 seconds. Before-clear assignment traces recorded 13,503 unsupported Air Drone assignments per ground unit, 40,509 total, from engine time 150.0 through 600.067. The Drone remained at 6 HP.
- RED artifacts: `target/ai-alt-spawn-domain-red-20261010/result.json`, `wave.jsonl`, `trace-summary.json`, and `red-02.log`. Temporary game I/O instrumentation was removed after preserving this evidence.
- The same real configuration now ends with Easy winning at 176.5087 seconds. `target/ai-alt-spawn-domain-green-20261010/result.json` records the unchanged settings. The first mixed-domain test fixture lacked an authored air unit; its failure was retained and the fixture corrected to normal Alliance startup.
- The corrected mixed-domain check passed using actual normal-start targets and a local registry-derived runtime Weapon value. It verifies ground-only rejection of air targets, preservation of valid air targets, and living hostile ground selection. It does not modify world units, health, resources, or registry statistics.
- Current full library suite: **192 passed, 0 failed, 1 ignored**, 106.30 seconds. Actual exit codes for the focused check, full suite, and shared build are recorded in `target/ai-compatible-target-full-20261010/exit.json`.
- Existing paid technology, remote Human control protection, anti-air production selection, and Big Arena repair-defense regression also passed. Big Arena Chaos Easy versus Beginner still resolves at 323.4759 seconds.

Shared native artifacts are sealed in `target/ai-compatible-target-shared-20261010`, with hashes in `provenance.json`. The standalone replay and its actual exit record are written to `target/ai-alt-spawn-domain-current-recheck-20261010`.

This proves the tested target-selection correction. The wider spawn-position sweep and affected campaign runs still require verification on the sealed current artifacts; earlier canonical results do not establish every spawn configuration.
