Current runtime update: all 70 entity portraits and RosterPreview now use original native model outputs; see [runtime integration](../art/portraits/runtime-integration.json). The audit below records retired import-era images, not remaining current portrait gaps.

# Unresolved icon replacement audit

Initial read-only audit, 2026-10-10:107 legacy icon files (105 absent upstream,
2 transformed/different). Twenty runtime semantic icons now use original
repository-authored SVG/PNG replacements; exact legacy and current hashes are
retained separately in the JSON. 24 unused legacy files were removed after reference and package-start checks;
63 referenced legacy files remain unchanged. Generated-sheet records exist for
27 current files but do not establish provider output rights. Exact SHA256,
runtime line references and model inputs are in [icon-replacement-audit.json](icon-replacement-audit.json).

62 unresolved runtime entity icons can be rendered from their actual models:
45 Kenney-part assemblies,16 authored procedural tanks and1 authored Tesla
fence model. All referenced GLBs still byte-match the pinned official CC0
Space Kit correspondence. Procedural paths use repository meshes/materials,
not unknown icon PNGs. This establishes viable inputs, not finished readability
or distribution acceptance. RosterPreview can be composed from those renders.
20 referenced action/support/status icons now use original geometric SVGs
rasterized byresvg0.48.1. All20 reproduce byte-identically and have validated
128px transparent RGBA output. Ten are reviewed in actual gameplay/menu UI. The remaining10 pass a shared
UI-only fixture at1280x720 and900x650, with actual hover tooltips. This closes
glyph presentation coverage only, not paid unlocking or combat effects. See `art/semantic-icons/runtime-integration.json`
and `screenshots/steam-quality/semantic-icons-native-review-03.json` plus
`semantic-icons-native-review-04.json` and `semantic-icons-native-review-07.json`.The24 unreferenced paths are now removed; their original hashes remain in
`removed_unused_icons` in the JSON. The independent336-asset package enters
normal skirmish (actual exit0, inspected screenshot); evidence is
`target/unused-icon-removal-result-20261010.json`.

## Existing native rendering path

- `capture model-harness <output> 1 [page]`: one actual registry entity per
 1600×1000 page, all registry entries covered; `manifest.md` maps page to ID.
 Fixed1/30s updates,180 settle frames, orthographic camera and fixed lighting.
 This is an existing roster renderer, not a tight/transparent portrait exporter.
- `build_model_harness_capture_app`, `capture_model_harness_manifest`,
 `capture_spawn_model_harness_page`: `src/capture_api.rs:198,260,285`.
- `spawn_entity_models_for_harness`: `src/spawn.rs:509` mirrors Kenney assemblies,
 authored tank selection and procedural Tesla geometry. It also adds structure
 details; no game-economy/spawn injection is needed for asset export.
- `capture factions <output>` captures three real starting bases and checks paid
 controls; useful comparison, unsuitable as individual portraits.
- Front-menu roster currently loads the unresolved RosterPreview PNG
 (`src/menu.rs:363`); no independent portrait exporter exists.

For reproduction, pin executable/source/model hashes, absolute BEVY_ASSET_ROOT,
graphics backend/device/driver and crop/resampling parameters. Run twice on the
same renderer before claiming deterministic PNG bytes; fixed scene/time alone
does not guarantee byte identity across GPUs. Review final72px command-card
readability and capture actual HUD/menu usage. Preserve CC0 archive/license
hashes and authored-source provenance with every exported file.

## Exact unresolved paths

Every filename below is under `assets/ui/icons/`; the JSON records full paths.

| Filename | Replacement input / current use |
| --- | --- |
| AdvancedReactorPlant.png | Actual Kenney model portrait |
| AntiAirWalker.png | Actual Kenney model portrait |
| ArcCoilDefenseTower.png | Actual Kenney model portrait |
| AttackMove.png | Original vector replacement; remaining HUD review pending |
| Barracks.png | Actual Kenney model portrait |
| BomberVTOL.png | Actual Kenney model portrait |
| ChronoRelay.png | Original vector replacement; remaining HUD review pending |
| CryoSprayer.png | Actual Kenney model portrait |
| DeployMode.png | Original vector replacement; remaining HUD review pending |
| DroneMineLayer.png | Actual authored tank portrait |
| EmpPulse.png | Original vector replacement; remaining HUD review pending |
| EngineerDrone.png | Actual Kenney model portrait |
| FieldMedic.png | Actual Kenney model portrait |
| FlakHoverTank.png | Actual authored tank portrait |
| FlakRocketTeam.png | Actual Kenney model portrait |
| FlakRocketTeamMk2.png | Actual Kenney model portrait |
| FlameAssaultBuggy.png | Actual authored tank portrait |
| GrenadierTrooper.png | Actual Kenney model portrait |
| GuardArea.png | Original vector replacement; remaining HUD review pending |
| HammerSiegeTank.png | Actual authored tank portrait |
| HeavyBombardmentAirship.png | Actual Kenney model portrait |
| HeavyMachinegunTrooper.png | Actual Kenney model portrait |
| HeavySiegeWalker.png | Actual Kenney model portrait |
| HoldPosition.png | Original vector replacement; remaining HUD review pending |
| InterceptorVTOL.png | Actual Kenney model portrait |
| JammerVehicle.png | Actual authored tank portrait |
| LanceBeamDefenseTower.png | Actual Kenney model portrait |
| LanceBeamTank.png | Actual authored tank portrait |
| LightRifleInfantry.png | Actual Kenney model portrait |
| LongbowMissileCrawler.png | Actual authored tank portrait |
| MirageScoutTank.png | Actual authored tank portrait |
| MobileRepairCrawler.png | Actual authored tank portrait |
| MobileShieldProjector.png | Actual authored tank portrait |
| ModularMissileCarrier.png | Actual authored tank portrait |
| MortarTeam.png | Actual Kenney model portrait |
| NaniteRepairSwarm.png | Original vector replacement; remaining HUD review pending |
| OrbitalStrike.png | Original vector replacement; remaining HUD review pending |
| OrePurifier.png | Actual Kenney model portrait |
| Paradrop.png | Original vector replacement; remaining HUD review pending |
| Patrol.png | Original vector replacement; remaining HUD review pending |
| PhaseSaboteur.png | Actual Kenney model portrait |
| PowerReactor.png | Actual Kenney model portrait |
| PrismDefenseObelisk.png | Actual Kenney model portrait |
| PulseRifleCommando.png | Actual Kenney model portrait |
| RadarSweep.png | Original vector replacement; remaining HUD review pending |
| RadarUplink.png | Actual Kenney model portrait |
| RailArtilleryWalker.png | Actual Kenney model portrait |
| RailCannonBunker.png | Actual Kenney model portrait |
| RailSniperTeam.png | Actual Kenney model portrait |
| RailgunTank.png | Actual authored tank portrait |
| RallyPoint.png | Original vector replacement; remaining HUD review pending |
| Refinery.png | Actual Kenney model portrait |
| Repair.png | Original vector replacement; remaining HUD review pending |
| RepairPad.png | Actual Kenney model portrait |
| RoboticsBay.png | Actual Kenney model portrait |
| RocketGunship.png | Actual Kenney model portrait |
| RocketInfantry.png | Actual Kenney model portrait |
| RocketTrooperRobot.png | Actual Kenney model portrait |
| RosterPreview.png | Compose actual roster renders |
| SaboteurInfiltrator.png | Actual Kenney model portrait |
| Scatter.png | Original vector replacement; remaining HUD review pending |
| ScoutRover.png | Actual authored tank portrait |
| SellStructure.png | Original vector replacement; remaining HUD review pending |
| ShieldOverdrive.png | Original vector replacement; remaining HUD review pending |
| ShieldTrooper.png | Actual Kenney model portrait |
| ShockTrooper.png | Actual Kenney model portrait |
| SiegeAirship.png | Actual Kenney model portrait |
| SiegeArtilleryVehicle.png | Actual authored tank portrait |
| SiegeDrillTank.png | Actual authored tank portrait |
| SniperScout.png | Actual Kenney model portrait |
| StopCommand.png | Original vector replacement; remaining HUD review pending |
| StrategicMissile.png | Original vector replacement; remaining HUD review pending |
| TacticalOfficer.png | Actual Kenney model portrait |
| TechHospital.png | Actual Kenney model portrait |
| TechLab.png | Actual Kenney model portrait |
| TechLocked.png | Original vector replacement; remaining HUD review pending |
| TechOilDerrick.png | Actual Kenney model portrait |
| TechRepairDepot.png | Actual Kenney model portrait |
| TeslaCrawlerMk2.png | Actual authored tank portrait |
| TeslaFenceSegment.png | Actual authored Tesla model portrait |
| WeatherControlSpire.png | Actual Kenney model portrait |
| WeatherStorm.png | Original vector replacement; remaining HUD review pending |
| Worker.png | Actual Kenney model portrait |
