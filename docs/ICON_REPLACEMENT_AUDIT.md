# Unresolved icon replacement audit

Read-only audit, 2026-10-10. No rendering, source/asset replacement or GPU use.
All107 unresolved icon files still match the previous provenance audit:105
absent upstream and2 transformed/different. Generated-sheet records exist for
27 current files but do not establish provider output rights. Exact SHA256,
runtime line references and model inputs are in [icon-replacement-audit.json](icon-replacement-audit.json).

62 unresolved runtime entity icons can be rendered from their actual models:
45 Kenney-part assemblies,16 authored procedural tanks and1 authored Tesla
fence model. All referenced GLBs still byte-match the pinned official CC0
Space Kit correspondence. Procedural paths use repository meshes/materials,
not unknown icon PNGs. This establishes viable inputs, not finished readability
or distribution acceptance. RosterPreview can be composed from those renders.
20 referenced action/support/status icons have no entity portrait equivalent;
they need original semantic artwork.24 unresolved files have no current Rust
literal reference or registry icon entry; check package exclusion before removal.

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
| AmphibiousAssaultAPC.png | No current runtime reference |
| AntiAirWalker.png | Actual Kenney model portrait |
| ArcCoilDefenseTower.png | Actual Kenney model portrait |
| AttackMove.png | Original semantic artwork |
| Barracks.png | Actual Kenney model portrait |
| BattleLab.png | No current runtime reference |
| BomberVTOL.png | Actual Kenney model portrait |
| BuildQueue.png | No current runtime reference |
| CancelOrder.png | No current runtime reference |
| ChronoRelay.png | Original semantic artwork |
| CombatEngineerTeam.png | No current runtime reference |
| CryoSprayer.png | Actual Kenney model portrait |
| Defeat.png | No current runtime reference |
| DeployMode.png | Original semantic artwork |
| DroneMineLayer.png | Actual authored tank portrait |
| EmpPulse.png | Original semantic artwork |
| EngineerDrone.png | Actual Kenney model portrait |
| FieldMedic.png | Actual Kenney model portrait |
| FlakHoverTank.png | Actual authored tank portrait |
| FlakRocketTeam.png | Actual Kenney model portrait |
| FlakRocketTeamMk2.png | Actual Kenney model portrait |
| FlakTrooper.png | No current runtime reference |
| FlameAssaultBuggy.png | Actual authored tank portrait |
| GrenadierTrooper.png | Actual Kenney model portrait |
| GuardArea.png | Original semantic artwork |
| HammerSiegeTank.png | Actual authored tank portrait |
| HazardSprayerInfantry.png | No current runtime reference |
| HeavyBombardmentAirship.png | Actual Kenney model portrait |
| HeavyCargoDropship.png | No current runtime reference |
| HeavyMachinegunTrooper.png | Actual Kenney model portrait |
| HeavySiegeWalker.png | Actual Kenney model portrait |
| HoldPosition.png | Original semantic artwork |
| InterceptorJetVTOL.png | No current runtime reference |
| InterceptorVTOL.png | Actual Kenney model portrait |
| JammerVehicle.png | Actual authored tank portrait |
| LanceBeamDefenseTower.png | Actual Kenney model portrait |
| LanceBeamTank.png | Actual authored tank portrait |
| LightRifleInfantry.png | Actual Kenney model portrait |
| LongbowMissileCrawler.png | Actual authored tank portrait |
| LowPower.png | No current runtime reference |
| MineClearingRover.png | No current runtime reference |
| MirageScoutTank.png | Actual authored tank portrait |
| MobileConstructionVehicle.png | No current runtime reference |
| MobileRepairCrawler.png | Actual authored tank portrait |
| MobileSensorJammer.png | No current runtime reference |
| MobileShieldProjector.png | Actual authored tank portrait |
| ModularMissileCarrier.png | Actual authored tank portrait |
| MortarTeam.png | Actual Kenney model portrait |
| NaniteRepairSwarm.png | Original semantic artwork |
| OrbitalStrike.png | Original semantic artwork |
| OrePurifier.png | Actual Kenney model portrait |
| OreRefineryExtension.png | No current runtime reference |
| Paradrop.png | Original semantic artwork |
| Patrol.png | Original semantic artwork |
| PhaseSaboteur.png | Actual Kenney model portrait |
| PowerReactor.png | Actual Kenney model portrait |
| PrismDefenseObelisk.png | Actual Kenney model portrait |
| PrismLanceTank.png | No current runtime reference |
| PulseRifleCommando.png | Actual Kenney model portrait |
| RadarDome.png | No current runtime reference |
| RadarSweep.png | Original semantic artwork |
| RadarUplink.png | Actual Kenney model portrait |
| RailArtilleryWalker.png | Actual Kenney model portrait |
| RailCannonBunker.png | Actual Kenney model portrait |
| RailSniperTeam.png | Actual Kenney model portrait |
| RailgunCommando.png | No current runtime reference |
| RailgunTank.png | Actual authored tank portrait |
| RallyPoint.png | Original semantic artwork |
| Refinery.png | Actual Kenney model portrait |
| Repair.png | Original semantic artwork |
| RepairPad.png | Actual Kenney model portrait |
| ResourceDrop.png | No current runtime reference |
| RoboticsBay.png | Actual Kenney model portrait |
| RocketArtilleryCrawler.png | No current runtime reference |
| RocketGunship.png | Actual Kenney model portrait |
| RocketInfantry.png | Actual Kenney model portrait |
| RocketTrooperRobot.png | Actual Kenney model portrait |
| RosterPreview.png | Compose actual roster renders |
| SaboteurInfiltrator.png | Actual Kenney model portrait |
| Scatter.png | Original semantic artwork |
| ScoutRover.png | Actual authored tank portrait |
| SellStructure.png | Original semantic artwork |
| ShieldGenerator.png | No current runtime reference |
| ShieldOverdrive.png | Original semantic artwork |
| ShieldTrooper.png | Actual Kenney model portrait |
| ShockTrooper.png | Actual Kenney model portrait |
| SiegeAirship.png | Actual Kenney model portrait |
| SiegeArtilleryVehicle.png | Actual authored tank portrait |
| SiegeDrillTank.png | Actual authored tank portrait |
| SniperScout.png | Actual Kenney model portrait |
| StopCommand.png | Original semantic artwork |
| StrategicMissile.png | Original semantic artwork |
| TacticalOfficer.png | Actual Kenney model portrait |
| TechHospital.png | Actual Kenney model portrait |
| TechLab.png | Actual Kenney model portrait |
| TechLocked.png | Original semantic artwork |
| TechOilDerrick.png | Actual Kenney model portrait |
| TechRepairDepot.png | Actual Kenney model portrait |
| TeslaCrawlerMk2.png | Actual authored tank portrait |
| TeslaFenceSegment.png | Actual authored Tesla model portrait |
| Victory.png | No current runtime reference |
| Warning.png | No current runtime reference |
| WeatherControlSpire.png | Actual Kenney model portrait |
| WeatherStorm.png | Original semantic artwork |
| Worker.png | Actual Kenney model portrait |
| texture_01.png | No current runtime reference |
