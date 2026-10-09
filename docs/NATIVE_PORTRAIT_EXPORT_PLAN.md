# Native portrait export plan

Implemented first-three staging exporter, 2026-10-10. Runtime replacement remains gated.
The existing single-page harness is insufficient: its minimum24m orthographic
width and floor leave assembled models tiny. Tight camera fitting alone must
not omit body meshes, attachments or authored game materials.

## Actual geometry parity

Use a dedicated offscreen native capture module and a renderer-only App. Spawn
one actual registry entity through `spawn_unit_with_visual_faction` or
`spawn_structure_for_visual_faction`, with a fixed faction/palette and rest pose.
These paths call `spawn_entity_models` (`src/spawn.rs:447`): authored procedural
tanks, all transformed Kenney render parts, procedural Tesla/mine models,
CommandCenter service details and faction markers. Include shared
`recenter_entity_models` and `apply_team_color_materials` with their required
resources, and finish streamed geometry/material initialization before framing.
Do not run economy/combat/orders, terrain, weather, particles, HUD or UI systems.
This is asset rendering, not a proof of normally paid gameplay.

The current generic authored structure detail function is
`structure_details::spawn_structure_details` and currently only decorates
CommandCenter corridor geometry. Other buildings use their complete registry
assemblies; there is no general authored structure-body helper in current source.
Do not invent a filled body or silently substitute a primary GLB. If another
owner adds a shared authored body path, use that same path in gameplay/export.
Require descendant mesh/material parity against an actual-game rest-state
reference, especially AdvancedReactorPlant, CommandCenter and an authored tank.
The current harness root lacks Unit/Structure/Team/Selectable markers, so it
misses the real recentering and game material processing even where meshes match.

## Ready geometry and exact tight camera

Local dependencies are Bevy0.19.1, not an older SceneSpawner API.

- `bevy::world_serialization::WorldInstance` dereferences to InstanceId;
  `WorldInstanceSpawner::instance_is_ready(*instance)` or `WorldInstanceReady`
  confirms each `WorldAssetRoot` has spawned. Also require AssetServer dependency
  readiness, no failed assets, every expected part present and nonzero meshes.
- Traverse all descendants using `Query<&Children>::iter_descendants::<Children>`
  (or the existing explicit Children stack). Include procedural details, body
  and attachments, not only `WorldAssetRoot` subtrees.
- Query `(&Mesh3d, &GlobalTransform, Option<&Aabb>)`. Use
  `bevy::camera::primitives::{Aabb, MeshAabb}`; `MeshAabb::compute_aabb` reads
  Float32x3 positions or cached final bounds when an Aabb component is absent.
  Never derive bounds from Selectable.radius or the declared primary model.
- For each local Aabb, transform all8 corners with
  `GlobalTransform::transform_point`. This handles node transforms, root scale,
  rotation and negative scale. Reject missing/nonfinite/empty geometry.
- Fixed three-quarter camera direction: normalize `(6,5,7)`, Y-up. Let C be
  the union world-bounds center and D be `2*length(world_bounds_size)+1`.
  Initial camera is `Transform::from_translation(C+direction*D).looking_at(C,Y)`.
  Project all world corners onto its right/up axes; obtain xmin/xmax/ymin/ymax.
  Correct focus to `C + right*(xmin+xmax)/2 + up*(ymin+ymax)/2`.
- For square512px exports, `span=max(xmax-xmin,ymax-ymin)/0.84` gives8% padding
  on each side. Use `Projection::Orthographic(OrthographicProjection {
  scaling_mode: ScalingMode::Fixed { width:span,height:span }, scale:1,
  near:0.01, far:D+length(world_bounds_size)+1, ..default_3d() })`.
  No24m minimum, floor or UI. After transform propagation, use
  `Camera::world_to_viewport` to require every bound corner inside the padding.

## Transparent native output

Use an `Image::new_target_texture(512,512,Rgba8UnormSrgb,None)`,
`RenderTarget::Image`, transparent ClearColor and no HDR/Bloom/sky/fog. Fixed
ambient/key lights may shade models but no ground receives shadows.

Do not call current `shoot`/Bevy `save_to_disk`: Bevy's screenshot writer at
`bevy_render-0.19.1/src/view/window/screenshot.rs:141` calls `to_rgb8`, explicitly
discarding alpha because HDR can store brightness there. Instead observe
`ScreenshotCaptured`, call `image.try_into_dynamic()?.to_rgba8()` and write PNG
with native image I/O. Validate actual transparent-background pixels and opaque
model pixels; do not chroma-key an opaque screenshot. Confirm correct edge alpha
by compositing exports on light/dark command-card backgrounds.

## Review and provenance gate

Render to a new staging directory only. First export AdvancedReactorPlant,
CommandCenter and one procedural tank; compare full silhouette/body/attachments
against actual gameplay, inspect512px and72px command-card composites. Then
render all62 eligible unresolved entity icons and compose roster art separately.
Twenty referenced semantic icons remain a separate original-art task.

Pin executable/source/model/texture hashes, license/archive correspondence,
entity/faction/palette, complete mesh counts/bounds, camera/light settings,
backend/GPU/driver, image conversion and output SHA256. The authored armor
texture `textures/armor-painted-steel.png` is a game-material input with its own
retained provenance; include it rather than claiming only GLBs are consumed.
Run two fresh exports on the same backend and compare bytes/decoded pixels.
Fixed scene/time alone is not proof of cross-GPU deterministic PNG bytes.
Replacement/rights/readability gates remain open until those real renders pass.

## First-three staging evidence

The dedicated `portrait_export` binary and `scripts/export_native_portraits.py`
now implement the above full-model renderer. The independent reference uses a
separate full game App after normal asset startup, with MainMenu gating simulation;
it explicitly spawns asset references and runs the shared material/recenter systems.
This verifies game-context asset geometry, not paid gameplay or live-match appearance.

Canonical native build 02 exited 0 in 51.48 seconds. Its sealed executable is
`target/portrait-export-pin-20261010/portrait_export`, SHA-256
`68c59d0c66f1149601f9442c8f6b8c19fe12899857aa2dd3003554f15a8783f9`.
Stage `target/portrait-first-three-01` exited 0 in 10.35 seconds. The retained
manifest/reference JSONs prove matching canonical mesh attributes, indices,
materials and transformed bounds: AdvancedReactorPlant 16 meshes, CommandCenter
94 meshes, Tank 9 meshes. Both fresh exports of each model had identical PNG
bytes and decoded RGBA pixels on this backend. Alpha bounds were respectively
[57,48,455,467], [43,120,471,391] and [43,149,423,429]; every output contains
transparent background and fully opaque model pixels. All input assets remained
unchanged during export. Full source/assets/backend/output hashes and retained
license inventory are in the staging provenance JSON.

The light/dark 512px and 72px composites were inspected in `review-sheet.png`.
HQ and tank silhouettes remain recognizable at 72px. The reactor includes
separated/floating authored parts in both independent signatures; the exporter
preserves that existing geometry rather than inventing a body. Actual live-match
Tank/HQ comparison, advanced-building gameplay reference, final visual approval,
and all62 exports/runtime replacement remain open. No runtime icons changed.
