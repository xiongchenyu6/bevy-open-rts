# Bounded headless audit progress

Set `OPEN_BEVY_AUDIT_PROGRESS` to a new JSONL path to opt into read-only audit
telemetry. Default runs add no file I/O. The observer flushes before App creation,
periodic update-start/completed samples and final outcomes. Samples include
clock/phase, living types per team, order/path counts, at most32 ordered units
unchanged across samples, maximum/last update wall duration, and Linux process
CPU ticks. The driver records `getconf CLK_TCK` separately. Stationary samples
are observations, not proof of a blocked route. Wall update duration is not CPU time.
The trace caps4096 rows and does not change simulation/world/limits.

Canonical build retry02 exited0 in2m31s, after preserving build01's diagnostic
team-key compile failure. Sealed audit:
`target/reactor-audit-pin-20261010/skirmish_audit`, SHA
`17c785304cfd1ae8988d240ebc1cd16326518383d2b7e83bfcbf43bc89b4d494`.

The exact previously failing case0684 was rechecked with normal32/16 starting
resources: TechDivide, Alliance Beginner player0 at spawn0 versus Alliance Easy
player1 at spawn2, simulation limit600s and wall timeout180s. Evidence in
`target/tech-divide-progress-01` reports wall_timeout/exit124; cleanup completed
at182.42s wall. The final flushed row at179.844s wall records clock435.115,
13055 updates,179.81 CPU seconds, and no frozen update. Maximum update wall time
was0.815s during startup; late updates were approximately14ms. Only seven Workers
and two unarmed Drones remained throughout the late trace despite multiple
production/technology buildings. This does not establish their exact economy
or foundation failure cause.

A bounded read-only8s49Hz perf sample of this owned audit child captured406samples
and zero lost samples. The two leading self buckets (34.96% and31.85%) are vector
allocation/fill with stacks through `NavGrid::find_path`,
`reachable_unit_contact_approach`, and `update_construct_orders`.
Retained perf.data/report support construction-contact routing as a measured CPU
hotspot. Some source-line resolution warnings occurred; symbol/call-stack evidence
remains present. This sample is not a fix or a general performance claim. Next
needed evidence is actual constructor targets, pending foundations, and contact
route validity; timers, limits and gameplay costs remain unchanged.

The detailed constructor collector is external, bounded, and opt-in. It records
actual construction targets/remaining work, contact and endpoint checks, navigation
revision and already computed route-search results. It never inserts an ECS resource
or deferred command, and does not repeat path searches for telemetry. Bevy stores
resources as entities: an abandoned intermediate diagnostic resource shifted game
entity IDs and changed ID-dependent scout scheduling. That intermediate trace was
not accepted as a gameplay fix. After removing the resource, paired trace/no-trace
runs on `target/constructor-external-diagnostic-pin-20261010/skirmish_audit` both
reproduced the original 180-second wall timeout; evidence is retained in
`target/tech-divide-constructor-pair-04`. Construction/movement verification is
recorded separately in `docs/CONSTRUCTION_NAV_QA.md`.
