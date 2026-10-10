# Native entity portrait candidates

`entities.json` maps all 70 unique existing registry portrait paths to entity IDs,
roles, faction memberships, ModelMap entries and actual assembly sources. Its
provenance statuses describe the legacy images: 62 unresolved and eight already
recorded. RosterPreview is a separate 71st asset-only group composition.
Neutral TechHospital, TechOilDerrick and TechRepairDepot use the Alliance visual
default without claiming faction ownership. No legacy PNG is a render input.

The exporter accepts this inventory and validates IDs, icon paths and factions:

```sh
cargo run --locked --bin portrait_export -- art/portraits/entities.json /new/staging-directory
python3 art/portraits/check_attachments.py --check
```

Actual registered GLB parts, authored procedural bodies and detail attachments
are spawned through the game model path. Authored contact corrections cover 42
assemblies; `attachment_transforms.json` preserves them during registry generation.
The geometry regression reads raw GLB hierarchy/accessor bounds, rejects detached
Cryo primitives and overlapping aircraft mounts, requires depot primitive floor
support, and retains separate actors with individually attached equipment.

Contextual faction indicators alone carry a dedicated tag. They remain visible
in normal gameplay; the exporter hides them and excludes them from both model
collectors. Manifests record the excluded count and **model-only geometry/material
parity**, never whole-scene parity. The regression retains real Tank geometry,
GLB children and accessories while checking all three faction marker sets.

Validation checkpoint: shared native build passed and the library suite reports
229 passed, zero failed, one ignored; the marker regression passed. Sealed
exporter SHA `8bd848449911baf355df6d3989fffe3154821fecef9b7395e52e5726b1405e95`
produced all 140 model-only PNGs (70 entities twice), with independent checker
success. Root reviewed all 70 at 128px and 72px on dark/light backgrounds and
accepted the candidate visuals. Evidence: `target/model-only-full70-20261010`.
Earlier detached-part renders remain explicitly failed historical evidence.

Paid normal-game model/UI comparison and runtime PNG integration remain pending.
These are staged asset candidates, not proof that units were trained or structures
built through gameplay. No runtime portrait has been replaced by this work.
