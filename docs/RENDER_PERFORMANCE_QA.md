# Shadow resolution verification

The shared default shadow map is2048 rather than4096. Four cascades, Gaussian
filtering, HDR/ACES, Bloom,4xMSAA, geometry and game rules are unchanged. This
reduces nominal four-layer32bit shadow depth storage256→64MiB; actual measured
results below support adopting the bounded rendering improvement.

Both normal-input runs use the DEBUG profile, Normal Alliance AI0 versus
Normal Demon AI1,32/16 resources,1x speed, the same camera/timing route and fresh
profiles on AMD RADV RENOIR/Mesa25.2.8. Frozen assets and Nix runtime are mounted
read-only; this is not a release-package benchmark. Baseline is older Hold
`dcff4dd8e60eca3260e709b35afe5794b3acffb36f4ef9b8c5a7463a55ef6e55`;
candidate is current Hold/reactor/observer context
`a5f4e213cacaff715455aacc88fc574e8e779e94af43050e7906e710da62ca87`.
That source-context difference and different live outcomes limit attribution.

| Periodic sample metric |4096 baseline|2048 candidate|
|---|---:|---:|
| Finite samples |599|600|
| Mean sampled FPS |55.74|60.43|
| Median sampled FPS |56.47|60.91|
| P95 sampled frame time |22.30ms|19.21ms|
| Mean GPU load |92.46%|67.28%|

Both actual containers return0. Raw CSV/result JSONs are under
`target/shadow-ab-{baseline4096,candidate2048}-20261010-01`; summary and CSV
hashes are in `target/shadow-ab-review-20261010-01.json`. Sampling is about100ms
for60s, not a per-frame trace. Constructor compilation was concurrent. No
independent1%low or stable60 claim follows from these samples.

The first512 captures were obscured by early Results and were not accepted.
Separate early normal-input visual runs return0 for both variants: normal/close
1280×720, actual512×512 window resize and five live motion samples per variant,
then physical F10 pause. All16 images were inspected. Candidate silhouette,
roof/service detail and contact shadows remain readable; shadow edges are
softer, without observed coarse jaggies or sampled motion instability. Evidence:
`target/shadow-visual-review-20261010-02.json` and the two referenced output trees.

Stable60 in representative dense combat remains the target. Final post-navigation
release artifact testing, uncontended repeats, larger/longer fights, raised maps,
zoom/tilt extremes and full-rate motion/per-frame tail analysis remain open.
The earlier release52.63FPS probe is historical context, not this A/B baseline.
