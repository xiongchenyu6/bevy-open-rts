# Entity portrait export inventory

`entities.json` identifies the 70 registry entity portraits from the current
unresolved ledger by exact runtime path, registry ID, source line, scene,
ModelMap entry, assembled model source and faction production membership.
All 70 files exist; paths are unique, with no shared-icon aliases. Each row
explicitly records provenance status: 62 unresolved and eight with already
recorded provenance. A consistent full-model visual style should cover all 70;
three preliminary exporter images do not complete this set. RosterPreview is
a separate 71st asset-only composition task, not an entity ID.

Faction choice prefers Alliance, then Demon, then Chaos among actual memberships.
Neutral TechHospital, TechOilDerrick and TechRepairDepot have no production-faction
membership and explicitly use the Alliance visual default, without claiming
ownership. Multi-faction entities retain all memberships. Regenerate the source
inventory with `python3 art/portraits/audit_entities.py`.

The native exporter now accepts this audited inventory and faction selection:

```sh
cargo run --locked --bin portrait_export -- art/portraits/entities.json /new/staging-directory
```

It validates registry IDs, matching icon paths, uniqueness and faction names
before creating the staging directory. It retains actual spawn/model assembly,
attachments, independent MainMenu asset-reference geometry parity, assembled
AABB framing, transparent RGBA and two separate rendering runs. The sealed native exporter completed all140 renders (70 entities twice);
independent byte, RGBA, alpha and framing checks pass. Root reviewed all70
at128px: multiple real model accessories are detached, so visual acceptance
fails. See `full70-visual-review.json`; correct the registered models and
rerender before replacing runtime icons. Registry models may be empty for authored procedural tanks and
TeslaFenceSegment; that is not missing geometry. ModelMap references are source
correspondence, not permission to replace the actual assembled rendering path.

This inventory neither reads legacy PNGs as render inputs nor generates or
replaces any image. Initial complete exports and visual review are recorded. Corrected-model
exports, paid in-game comparisons and runtime integration remain pending.

## Model-only reference scope

Contextual faction ground indicators are tagged separately from registered GLB
and authored procedural geometry. The exporter hides only those tagged
indicators; the independent full-game asset reference compares the same model
meshes and materials. Runtime indicators remain visible in ordinary gameplay.
Manifests explicitly record model-only parity and the excluded indicator count.
The checker rejects older marker-inclusive scope; a new complete70-entity
render and visual review are required before runtime integration. The marker
regression and new render remain pending native validation.
