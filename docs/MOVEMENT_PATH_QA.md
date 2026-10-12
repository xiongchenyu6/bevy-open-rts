# Physical pursuit path verification

On 2026-10-10, immutable audit `5ac59455…` timed out in Plain & Simple
case0101 (Demon Hard/Normal, starts1/0). Thirteen attackers kept identical
positions and pursuit orders from roughly100–110 simulated seconds through600.
The opponent had one living Worker and no structures. This was a movement
failure, not ongoing damage or missing production technology.

The recorded Tank at(-14.184266,-8.863881), radius0.9, pursued a Worker at
(-21.337654,-8.938757), with next waypoint(-19.75,-8.25). Two nearby resource
centers were1.80355m apart; the Tank required3.0m between their centers. The
shared raster uses0.3m obstacle inflation. Its planned leg was physically
impassable for this Tank. The most urgent obstacle steering direction also
entered a resource body, so the existing segment guard correctly stopped it.

The fix retains the original desired and steered movement preference. Only when
both fail and an existing path is present, it rebuilds the existing NavGrid with
the actual mover clearance and runs one existing A* search. The initial open-cell
waypoint remains in the route. Every leg must pass the same physical segment
checks, terrain checks and map bounds. The original order goal stays unchanged;
normal movement follows the repaired waypoints. No route means safe stationary
movement. No speed, radius, damage, resource, difficulty or time limit changed.

The actual shared-schedule geometry regression failed before the fix: after90
ticks the Tank remained at exactly its recorded position. After the fix it moved
5.74834m and reduced target distance from7.15378m to2.46768m. Every step asserted
physical safety, terrain safety and the ordinary speed cap. Additional tests
reject an existing resource-body overlap, a solid map-spanning wall, and a real
cliff without a ramp. Focused results were4 passed/0 failed. The full library
suite passed234/failed0/ignored1 in182.09s; all four native builds exited0.

Logs and numeric exits are under `logs/movement-steering-{red-01,green-03}.log`
and `logs/movement-clearance-shared-04-{full,build}.log`, with pipeline results in
`target/movement-clearance-shared-04-result.json`. The immutable four-bin seal is
`target/movement-clearance-shared-20261010/pin.json`, including78 source inputs
and338 asset hashes. Its audit SHA256 is
`0295f5cffe11512192f0810c0fe05815683a0100b25ff78469870aa7eda7b449`.

All six observed timeout configurations completed in exact rechecks:

| Case | Map | Old outcome | New simulated seconds |
| --- | --- | --- | ---: |
|0101|Plain & Simple|600s timeout|86.232|
|0199|Plain & Simple|600s timeout|179.109|
|0238|Plain & Simple|600s timeout|501.999|
|0366|Four Corners|600s timeout|180.943|
|0387|Four Corners|600s timeout|173.175|
|0400|Four Corners|600s timeout|193.912|

Prior regression configurations0012 and0684 also completed at76.066s and180.743s.
Every run used normal32 ore/16 crystal startup, the original600 simulated-second
and180 wall-second limits, and a private338-file asset copy. Independent
configuration, initial HQ position, resource, typed count, outcome and asset hash
checks passed for all eight. Raw reports, commands, process exits and verification
are retained in `target/movement-clearance-failed-rechecks-20261010`.

This is bounded regression evidence. The older5ac594 sweep was semantically
superseded after441 observations:435 completed and6 simulation timeouts; inflight
0441 and all raw evidence were retained. Its owned process group was confirmed
terminal. It is not a completed sweep or a pass for the corrected source.
A fresh3726-case sweep runs on immutable0295f5 in
`target/skirmish-movement-clearance-all-starts-20261010`, with an isolated
pre-portrait338-file asset snapshot and unchanged configuration/time budgets.
That headless run does not validate the subsequently replaced71 portrait images.
No general balance, complete map coverage or release readiness claim follows
from the bounded rechecks.
