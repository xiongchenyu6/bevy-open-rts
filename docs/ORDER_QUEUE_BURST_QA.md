# Same-frame order queue QA

The real native-host/browser-player control run exposed lost Shift-click commands. Two separate normal queued Move packets arrived in the same host update. Each copied the same pre-update queue and scheduled an insertion; the later insertion overwrote the earlier destination. The authoritative host correctly failed its preservation check.

Cross05 evidence is in `target/online-human-ai-browser-native-20261010-05`. The browser sent its two queued packets together at game time 33.6589. At host time 35.7316, the Worker still had the first active move but retained only the final queued destination. The middle destination disappeared. The run is **failed**; client return-to-origin alone did not establish the intended route.

Independent replay of its persisted JSONL confirms 46 matching full snapshot DTOs after 30 seconds with no mismatches. Both clients disconnected and the own QA room expired with HTTP 404. `independent-snapshot-verification.json` records the JSONL hashes and preservation of 16 resource identities above JavaScript's exact-number range.

## Correction and regression evidence

Each normal order input now performs its queue decision and append against the current entity state when its deferred command executes. FIFO appends and nonqueued replacements are atomic within the existing deferred command stream. No per-packet global flush, transport change, stat adjustment, or test-oracle reduction was introduced. Obsolete pre-frame active/queue arguments and unused query members were removed from local, minimap, and online input paths.

- Two real normal-start Worker regressions failed before the fix: existing active Move plus two queued commands, and a fresh nonqueued Move followed by two queued commands in one input batch. Actual RED exit 101 is recorded in `target/order-burst-red-20261010`.
- Four focused checks passed after the fix, covering both FIFO cases, replacement cancellation, disallowed queuing, and a despawned actor. They use normally spawned Workers and the actual order-input method; no units, health, resources, or timers are injected.
- Full core suite: **196 passed, 0 failed, 1 ignored**, 109.72 seconds. Actual step exit records are in `target/order-burst-green-full-20261010/exit.json`.
- Narrow cleanup checks passed (4 queue checks and the real remote Human fixture), and the final shared native build exited 0 without the stale-query warnings. These results are tracked separately in `target/order-burst-final-shared-20261010-02`. Earlier cleanup typecheck failures remain preserved.

The core regression passes are distinct from the final live cross-platform gate, which still requires a matching native/Wasm run with host queue length two, preserved destinations, completed route, normal AI development, at least 20 exact matching DTO ticks, and room cleanup.
