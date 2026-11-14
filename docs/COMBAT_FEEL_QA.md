# Combat feel and visual pass

Presentation-only combat impact layer (`src/combat_feel.rs`) plus a lighting/grading pass. No hit-stop: an RTS clock never stalls for one explosion.

What it adds: weapon-class muzzle flashes; solid shockwave rings for explosive, heavy and siege hits; a full detonation for support-power strikes (previously one thin line); delayed secondary explosions on collapsing buildings; a white hit flash on damaged units; distance-attenuated, budget-limited camera shake for siege hits, strikes and building destruction only; layered, pitch-varied impact audio with a per-frame and voice cap; bolder order-acknowledgement markers. Feel for fights hidden by fog is suppressed.

Sync: the layer only reads effect entities the simulation already emits (`ImpactBurst`, `StructureDestructionVfx`, `CombatWreckage`) and unit `Health`. The wire format and `RTS_ONLINE_PROTOCOL` are unchanged; a strike is announced as an ordinary `ImpactBurst`, so clients receive it through the existing transient-event replication. In-flight effects and shake are cleared when a match resolves.

Evidence (2026-11-14, Linux, nix dev shell): `cargo test` 331 passed, 0 failed, 1 ignored. A twin-run test advances two identical matches 90 ticks, one bombarded with feel events, and requires bit-identical actor positions and health. Headless outcomes from the pre-change and post-change capture binaries match exactly: `arena HammerSiegeTank RailgunTank 5 90`, `arena RocketInfantry RailgunTank 6 90`, `match 120`. `capture combat-feel <dir>` records siege fire, a strike and a move order tick by tick.

Not verified: a live two-peer online match, audio by ear, the browser build at runtime (only `cargo check` for wasm32), and frame-time cost on low-end GPUs. Known gap, unchanged by this pass: online clients get no death explosion for units, because unit wreckage is not replicated.
