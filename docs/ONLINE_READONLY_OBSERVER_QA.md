# Read-only frontend observer

The normal lobby needs passive evidence without the verification harness driving Host/Join/Ready or forcing an outcome. `online_observe=1` enables a hidden JSON observer on browsers; `OPEN_BEVY_ONLINE_OBSERVE_STATUS` enables a native report file. Both suppress automatic verification configuration. Default sessions remain disabled, and the observer never submits lobby/game commands or changes actors, resources, timers or match results.

Reports contain public lobby/player/slot state and a bounded40-frame window of actual host-sent or validated guest-received full DTOs. Native publication occurs on snapshot/lobby/phase/applied-tick changes rather than every game-clock frame. Guest receipt is explicitly distinct from world application: last_applied_tick and actual scene/paid-job continuity need independent checks. Session secrets, tokens, SDP contents and TURN credentials are excluded.

Native focused tests passed2/0: a shared-schedule observer app stays outside lobby/match with no room/commands, and disabled/bounded snapshot retention holds. Full library passed255/0/1; main/capture builds passed and source/332 assets stayed unchanged. [Portable proof](qa/online-readonly-observer-native-review.json) records exact binary hashes.

Actual browser opt-in menu behavior remains pending Wasm11. Old Wasm10 normal physical Create public room succeeded, but Join remained at new ICE with no route; closing the own room led to404. That exploration proves neither joined readiness nor paid construction. Eight peers, forced TURN, same-session reconnect and final-artifact coverage remain separate gates.


The opt-in observer now also records up to40 actual guest ECS observations, once per applied tick in `Last`, after the update schedule and deferred entity changes. These rows query actual actors, banks, jobs, support state and match state. They preserve both the rendered `Transform` and any actual interpolation target. A received DTO and an applied-tick counter alone remain insufficient to claim independent world equivalence.

Native verification covers real deferred spawn/despawn, divergent actual health while the received DTO stays unchanged, rendered pose versus interpolation target, disabled behavior, the registered schedule and fresh-match history reset. The full library passed273tests with one ignored; native check and three binary builds passed. [The bounded native review](qa/online-applied-world-observer-native-review.json) records the assertion failures and hashes. Real browser world comparison awaits the matching Wasm15 artifact; existing Web14 evidence retains its earlier scope.
