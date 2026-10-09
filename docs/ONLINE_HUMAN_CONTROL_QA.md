# Live native human-control proof — 2026-10-09

Two real native clients connected through the published room service and WSS /
WebRTC transport. The host configured Human0, Human1 and one Normal AI slot
through ordinary lobby commands, with GodotSkirmish and normal 32/16 resources.
The remote player sent one Worker Move and two queued Move commands through the
normal reliable command outbox and host validation path.

Both clients passed at game clock **50.01365**, with **two connected humans**.
The authoritative host observed both queued moves, continuously checked their
ordered destinations, and saw normal completion of the route. Human1 retained
its initial roster and 32/16 resources; untouched units received no automatic
orders. The AI independently completed a VehicleFactory and produced a new
entity with an actual Weapon component.

**20 same-tick authoritative snapshot summaries** agreed after game clock 30
(ticks 301–491). They compare the remote Worker's authoritative position, human
resources and AI entity/development state. Unit order queues are not part of
the current snapshot protocol: queue preservation is a host-only assertion;
the client proves synchronized motion and snapshot agreement. No protocol or
gameplay values were changed. No units, HP, resources or timers were injected,
and no forced match result or pause was used.

Evidence:

- `target/online-human-ai-live-20261009-01/{host,player}.json`
- `target/online-human-ai-live-20261009-01/snapshot_agreement.json`
- `target/online-human-ai-live-20261009-01/result.json`
- `target/online-human-ai-bootstrap.json` contains only safe provider status:
  health/readiness/config 200, supported session protocol 1, WSS, and TURN
  credential **presence**, without credentials.

The native executable is pinned at
`target/online-human-ai-native-20261009/bin/bevy-open-rts`, SHA256
`f2a0d3cf54215ef31d442530c8f82870703af129028d65733069b7b3e6e5b9f9`.
It includes the AI controller protection and the earlier partial EMP task fix;
this run is not evidence that the later global EMP-order-clearing issue was
fixed. There was no EMP intervention in this control-observation window.

Both processes disconnected normally. The service exposes no room DELETE API;
the own QA room expired through the normal host-disconnect grace, and its room
endpoint returned **404**. The final report records `disconnected: true` and
`room_expired: true`. The driver never prints TURN credentials or room tokens.

Reproduce after building the current native binary with the canonical compiler
environment:

```sh
python3 scripts/verify_native_human_ai.py \
  --binary target/debug/bevy-open-rts \
  --output target/online-human-ai-new-run
```

The output directory must be new. The proof is specifically two native humans
plus one local AI; it does not establish browser/cross-platform, forced-relay,
eight-peer or performance acceptance.
