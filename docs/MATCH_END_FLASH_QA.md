# Match-end combat flash cleanup

Resolved matches stopped the normal flash decay system, leaving transient orange combat flashes permanently over the result scene. The existing match-end HUD cleanup now removes only CombatFlash entities after its Running guard. Units, structures and wreckage remain; Running tactical pause behavior remains intact.

The actual shared-schedule regression failed with the original HUD (child 101, frozen-flash assertion), then passed with the fix for HumanVictory, HumanDefeat and MatchFinished. Full library: 253 passed, 0 failed, 1 ignored. Main and capture builds exited 0; source and 332 assets matched before/after. See [portable evidence](qa/match-end-flash-review.json).

Raw logs, the isolated RED snapshot and immutable binaries remain under target/match-end-flash-shared02-20261011. The initial incomplete workspace snapshot and a post-build wrapper provenance assertion error remain preserved. Independent sealing verified the actual current-root compiler log and Cargo dep-info relative source paths. Natural rendered results and final SDK/package revalidation are pending; the earlier menu SDK build does not include this fix.
