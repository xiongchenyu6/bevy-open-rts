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

The existing native exporter currently hardcodes AdvancedReactorPlant,
CommandCenter and Tank. Extend its validated iteration with this list and faction
selection, retaining full actual spawn/model assembly, attachments, rest-pose
geometry parity, assembled AABB framing, transparent RGBA and independent render
reproducibility. Registry models may be empty for authored procedural tanks and
TeslaFenceSegment; that is not missing geometry. ModelMap references are source
correspondence, not permission to replace the actual assembled rendering path.

This inventory neither reads legacy PNGs as render inputs nor generates or
replaces any image. Actual complete exports, paid in-game model comparisons,
readability review and runtime integration remain pending.
