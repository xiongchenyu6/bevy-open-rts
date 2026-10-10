# Native entity portrait candidates

`entities.json` maps all 70 unique existing registry portrait paths to entity IDs,
roles, faction memberships, ModelMap entries and actual assembly sources. Its
current image hashes/provenance point to the 70 integrated original native
outputs; pre-integration image records are retained only as historical evidence. RosterPreview is a separate 71st asset-only group composition.
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

All 70 entity portraits are now integrated at their existing runtime paths.
Complete paid normal-game coverage remains unverified; the bounded representative
model/UI reviews are separate from all-70 asset-only parity/readability.

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
The approved roster now occupies its existing runtime path; this remains an
asset-only formation, not proof of an actual mixed-faction battle.

The readable roster checker requires run IDs exactly `{0, 1}`, validates every
member's ID/faction/f32 layout against the source configuration, and checks any
recorded PNG hashes. It also rejects clipped/empty silhouettes and compares
independent decoded RGBA. Copied-stage negatives for duplicate runs, wrong ID,
faction, position and PNG hash all reject; original evidence is unchanged.
Checks: `target/roster-checker-negative-20261010/result.json` and the original
stage's new `independent-review-02.json`; prior reports are not overwritten.

## Verified runtime integration

`runtime-integration.json` pins the selected 70 run-0 portrait images and roster
run-0 image, decoded RGBA, source/model/layout hashes and bounded review records.
Exactly 71 existing paths changed; all other 267 tracked runtime hashes remain
unchanged and the exact file count is 338. Prior bytes and provenance remain
outside assets at `target/portrait-legacy-before-integration-20261010`.
The active source ledger has zero hash mismatches; derived-source hash refreshes
retain their prior records explicitly as history. Regenerating `entities.json`
checks installed image hashes and retains current original-output provenance.

Post-integration native UI and final-package checks are distinct from the private
candidate menu/HUD reviews. All-70 paid gameplay/UI coverage and final package
acceptance are not implied by this integration.

## Current material change awaiting new exports

The reviewed original armor texture was integrated by `ad16eb6` after the
71-image render checkpoint. Existing portraits and roster still depict the
previous surface. Their rendering evidence remains historical; it does not
establish exact current-material parity or remove the previous surface input
source gap from those derived images. Regenerate all70 portraits twice and the
roster from a newly built exporter, validate output/source hashes and framing,
then review and integrate the new outputs before final asset acceptance.
The live inventory now contains332 files following unused VFX retirement.

## Current original armor render refresh

All70 portraits were exported twice with the original matte armor texture.
Independent paired-byte, RGBA/framing and model-only reference checks passed;
all70 were visually reviewed at128/72px on dark/light backgrounds. The roster
also passed independent paired checks and visual review. These71 reviewed
outputs are now installed;70 image hashes changed and the roster remained
byte-identical. `current-armor-runtime-integration.json` is the active evidence
record, while `runtime-integration.json` preserves the preceding checkpoint.
Current inventory is332 files. Post-integration UI, all-roster paid creation,
provider/distribution review and final package validation remain open.
