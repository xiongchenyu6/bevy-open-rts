# Current fixed-pin skirmish matrix review

Latest checkpoint: [2,005 terminal cases](qa/current244-matrix-2005-review.json), with 2,004 completed and one simulation timeout. Big Arena Chaos Easy versus Chaos Beginner at starts 2/1 remained Running after 600 seconds. The old244 scan continues and retains this failure. The exact configuration now resolves on current267 at 415.653 simulation seconds within unchanged limits ([bounded fix proof](qa/case1885-no-path-pursuit-review.json)); this does not revalidate the entire old sweep on new code. The 692-case snapshot below is historical.

The independently verified snapshot covers 692 of 3726 planned cases: all 692 completed, with zero simulation timeouts, wall timeouts, process errors or integrity errors. Plain & Simple and Four Corners each have all 324 planned cases; Tech Divide has the next 44. The matrix remains active. This is partial evidence, not completion or a general balance verdict. Exact hashes, version differences and the live process observation are in [the portable record](qa/current244-skirmish-partial-review.json).

The verifier checks the independently enumerated ordered-start plan, unique contiguous terminal indices, raw configuration, actual exit classifications, 32 Ore / 16 Crystal, Godot Skirmish opening, authored spawn coordinates, map identity, fixed 600 simulation / 180 wall-second limits, typed totals, phase and winning teams. Annihilation follows the current game rule: Command Centers and Workers retain a team; surviving other troops or structures are not automatically a false victory. All 332 isolated asset hashes match the source pin. The actual running child uses the copied audit binary and that isolated asset root.

The audit SHA is `c4f395dd8853a41da1768c8752e7116159b7ccef3c1be44afe1def300f8616f1`, from `target/objective-foundation-shared-20261010/pin.json`. The subsequent 247-test library checkpoint includes settlement UI, menu labels and read-only QA changes; differing existing files and added files are explicitly recorded. These are separate version boundaries. The older movement-clearance matrix remains active and contributes no observations to this report.

The readonly verifier accepted a copied actual one-case prefix and rejected six deliberate corruptions: false pass exit, resource mismatch, wrong spawn, wrong winner, duplicated index and missing manifest case. These edits were confined to temporary JSON files; no matrix evidence, assets, processes or simulation code changed. The full hash list and negative-check output remain under `target/current244-matrix-independent-partial-20261010.json` and `target/current244-matrix-validator-negative-checks-20261010.json`.

To refresh the partial verification without changing or restarting the run:

```sh
python3 scripts/verify_skirmish_matrix.py \
  --matrix target/current244-all-starts-20261010 \
  --pin target/objective-foundation-shared-20261010/pin.json \
  --assets target/objective-foundation-audit-assets-20261010 \
  --report target/current244-matrix-independent-partial-20261010.json
```

The verifier returns 0 for valid observations without observed match failures, 2 for valid observations containing a match failure, and 1 for integrity errors. A zero exit on a partial snapshot is never a complete-matrix pass.

Safe next steps are to continue this existing serial run, refresh verification at milestones, and preserve/report the first actual new failure with its exact config. After its terminal result, verify all 3726 raw pairs and the unchanged pin/assets before reporting coverage. Then schedule a small same-pin exact-config repeat set, including slow cases and opposite starts, in a separate output with unchanged limits and serialized CPU ownership. Repeated deterministic configurations are repeats, not independent seeds. Random-map coverage requires authored generation, explicit recorded seed semantics and normal resource/physical checks before execution; it remains open. All tests across a newer gameplay source pin need their own provenance and cannot inherit this matrix's pass status.
