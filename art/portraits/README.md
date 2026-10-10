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

## Original roster composition candidate

`roster.json` specifies a mixed-faction asset-only formation of six actual units,
with validated faction memberships and preserved game-model scales:

```sh
portrait_export --roster art/portraits/roster.json /new/roster-output
python3 scripts/verify_roster_stage.py /new/roster-output
```

The sealed native exporter built successfully (source/assets unchanged), then
completed two fresh-App 512px RGBA renders in 9.46 seconds. Both decoded images
match; independent transparency/framing checks and per-member model-only
references pass. Evidence: `target/roster-original-render-20261010-01`.
This does not prove paid unit creation or an actual mixed-faction match. Root accepted the 512px/260px composition and normal English menus at
1920×1080 and 900×720 with private candidate assets; the group was recognizable
without clipping or overlap. `native-menu-candidate-visual-review.json` records
that bounded menu verdict. `native-hud-candidate-visual-review.json` covers
representative Worker/build/queue/tooltip views at both sizes; it does not
prove all 70 portraits were displayed or created through paid gameplay.
Repository runtime images remain unchanged.

The readable roster checker requires run IDs exactly `{0, 1}`, validates every
member's ID/faction/f32 layout against the source configuration, and checks any
recorded PNG hashes. It also rejects clipped/empty silhouettes and compares
independent decoded RGBA. Copied-stage negatives for duplicate runs, wrong ID,
faction, position and PNG hash all reject; original evidence is unchanged.
Checks: `target/roster-checker-negative-20261010/result.json` and the original
stage's new `independent-review-02.json`; prior reports are not overwritten.

## Runtime integration after the shared seal

1. Recheck the approved model-only 70-image manifest and roster pair against
   their sealed binaries, source inventories and image hashes. Preserve old PNG
   bytes outside the runtime tree as historical proof, with no fallback routes.
2. Replace only the 70 validated registry icon paths and RosterPreview at their
   existing paths. The exact 71-path replacement should leave the other 267
   payload hashes unchanged and the payload count at 338; verify rather than
   assume the final inventory is unchanged.
3. Record selected PNG SHA256 and decoded RGBA hashes, actual model/CC0 or authored
   source references, renderer/model-map/layout/source hashes, marker exclusion
   and the bounded visual reviews. Update active asset-source/provenance/package
   inventories; retire unresolved legacy-image claims from the current inventory.
4. Repeat normal menu/HUD loading and representative paid model views against
   the integrated tree, then verify the final packaged exact set and hashes.
   Preserve the distinction between all-70 asset-only model parity/readability
   and the smaller normal-game/UI sample. Full-roster paid gameplay and final
   package acceptance remain separate gates.

Runtime replacement waits for the coordinated shared source/assets seal.
