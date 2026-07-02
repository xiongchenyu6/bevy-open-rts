//! Economy: team resources/power, income ticks, harvesting + dropoff,
//! resource nodes (+tint/visuals) and supply crates.
//!
//! Pure move out of lib.rs (module split); see IMPLEMENTATION_PLAN.md.

use bevy::prelude::*;

use crate::*;

pub(crate) const RESOURCE_ORDER_SCREEN_PICK_MIN_RADIUS_PX: f32 = 48.0;

pub(crate) const RESOURCE_ORDER_SCREEN_PICK_MAX_RADIUS_PX: f32 = 95.0;

pub(crate) const RESOURCE_ORDER_COLLECTOR_SCREEN_PICK_MAX_RADIUS_PX: f32 = 95.0;

pub(crate) const RESOURCE_ENTRY_MARGIN_M: f32 = UNIT_ADHERENCE_MARGIN_M;

pub(crate) const RESOURCE_DROPOFF_ENTRY_MARGIN_M: f32 = 1.2;

pub(crate) const RESOURCE_SEARCH_RADIUS_M: f32 = 30.0;

pub(crate) const ORE_PURIFIER_BONUS_RATIO: f32 = 0.25;

pub(crate) const SUPPLY_CRATE_PICKUP_RADIUS: f32 = 0.85;

pub(crate) const SUPPLY_CRATE_RESOURCE_ORE: i32 = 6;

pub(crate) const SUPPLY_CRATE_RESOURCE_CRYSTAL: i32 = 1;

pub(crate) const SUPPLY_CRATE_REPAIR_RADIUS: f32 = 3.5;

pub(crate) const SUPPLY_CRATE_REPAIR_AMOUNT: f32 = 8.0;

pub(crate) const LOW_POWER_PRODUCTION_SPEED_MULTIPLIER: f32 = 0.5;

pub(crate) const INFILTRATION_RESOURCE_STEAL_MIN: i32 = 1;

#[derive(Component)]
pub(crate) struct ChronoRelay {
    pub(crate) remaining: f32,
    pub(crate) speed_multiplier: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct ResourceSpec {
    pub(crate) kind: ResourceKind,
    pub(crate) amount: i32,
    pub(crate) position: (f32, f32),
}

#[derive(Clone, Copy)]
pub(crate) struct NamedSupplyCrateSpec {
    pub(crate) name: &'static str,
    pub(crate) effect: SupplyCrateEffect,
    pub(crate) position: (f32, f32),
}

#[derive(Component, Clone, Copy)]
pub(crate) struct IncomeSource {
    pub(crate) ore: i32,
    pub(crate) crystal: i32,
    pub(crate) interval: f32,
    pub(crate) remaining: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceKind {
    Ore,
    Crystal,
}

impl ResourceKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ore => t("矿石", "Ore"),
            Self::Crystal => t("水晶", "Crystal"),
        }
    }

    pub(crate) fn collect_seconds(self) -> f32 {
        match self {
            Self::Ore => 1.0,
            Self::Crystal => 2.0,
        }
    }

    pub(crate) fn color(self) -> Color {
        // Match what godot actually renders: Ore (ResourceA) = green, Crystal
        // (ResourceB) = red. (resource_a.material.tres albedo reads blue, but the
        // crystal mesh's vertex colors shift it green in-engine, which is what shows.)
        // Drives the crystal tint, HUD diamonds, and minimap markers.
        match self {
            Self::Ore => Color::srgb(0.0, 0.85, 0.18),
            Self::Crystal => Color::srgb(1.0, 0.0, 0.0),
        }
    }
}

#[derive(Component, Clone, Copy)]
pub(crate) struct ResourceNode {
    pub(crate) kind: ResourceKind,
    pub(crate) amount: i32,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct ResourceCargo {
    pub(crate) capacity: i32,
    pub(crate) ore: i32,
    pub(crate) crystal: i32,
}

impl ResourceCargo {
    pub(crate) fn total(self) -> i32 {
        self.ore + self.crystal
    }

    pub(crate) fn is_full(self) -> bool {
        self.total() >= self.capacity
    }

    pub(crate) fn has_any(self) -> bool {
        self.total() > 0
    }

    pub(crate) fn add_one(&mut self, kind: ResourceKind) -> bool {
        if self.is_full() {
            return false;
        }
        match kind {
            ResourceKind::Ore => self.ore += 1,
            ResourceKind::Crystal => self.crystal += 1,
        }
        true
    }

    pub(crate) fn clear(&mut self) -> (i32, i32) {
        let carried = (self.ore, self.crystal);
        self.ore = 0;
        self.crystal = 0;
        carried
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SupplyCrateEffect {
    Resources,
    Repair,
    Veterancy,
}

impl SupplyCrateEffect {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Resources => t("资源补给", "Resource Crate"),
            Self::Repair => t("维修补给", "Repair Crate"),
            Self::Veterancy => t("老兵补给", "Veterancy Crate"),
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            Self::Resources => Color::srgb(0.24, 0.7, 1.0),
            Self::Repair => Color::srgb(0.25, 0.95, 0.48),
            Self::Veterancy => Color::srgb(1.0, 0.85, 0.22),
        }
    }
}

#[derive(Component, Clone, Copy)]
pub(crate) struct SupplyCrate {
    pub(crate) effect: SupplyCrateEffect,
    pub(crate) pickup_radius: f32,
    pub(crate) resource_ore: i32,
    pub(crate) resource_crystal: i32,
    pub(crate) repair_radius: f32,
    pub(crate) repair_amount: f32,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct HarvestOrder {
    pub(crate) resource: Option<Entity>,
    pub(crate) state: HarvestState,
    pub(crate) collect_remaining: f32,
    /// The mineral type this harvester has been gathering. When its node runs out it
    /// retargets to the nearest node OF THIS KIND only, so a crystal harvester won't
    /// auto-wander off to ore (or vice-versa).
    pub(crate) last_kind: Option<ResourceKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HarvestState {
    MovingToResource,
    Collecting,
    MovingToDropoff,
}

#[derive(Resource)]
pub(crate) struct Economies {
    pub(crate) players: Vec<TeamEconomy>,
}

impl Default for Economies {
    fn default() -> Self {
        Self {
            players: Vec::new(),
        }
    }
}

impl Economies {
    pub(crate) fn get(&self, team: Team) -> &TeamEconomy {
        let Some(index) = team.economy_index() else {
            panic!("neutral team has no economy");
        };
        self.players
            .get(index)
            .unwrap_or_else(|| panic!("player slot {} has no economy", index + 1))
    }

    pub(crate) fn get_mut(&mut self, team: Team) -> &mut TeamEconomy {
        let Some(index) = team.economy_index() else {
            panic!("neutral team has no economy");
        };
        if self.players.len() <= index {
            self.players.resize_with(index + 1, || {
                TeamEconomy::new(
                    BEVY_PLAYTEST_STARTING_RESOURCES.ore,
                    BEVY_PLAYTEST_STARTING_RESOURCES.crystal,
                )
            });
        }
        &mut self.players[index]
    }

    pub(crate) fn apply_starting_resources(&mut self, resources: StartingResources) {
        for economy in &mut self.players {
            economy.ore = resources.ore;
            economy.crystal = resources.crystal;
        }
    }
}

#[derive(Clone)]
pub(crate) struct TeamEconomy {
    pub(crate) ore: i32,
    pub(crate) crystal: i32,
    pub(crate) power_used: i32,
    pub(crate) power_capacity: i32,
    pub(crate) power_sabotage_remaining: f32,
    pub(crate) production_veterancy_ranks: [u8; PRODUCTION_VETERANCY_PRODUCER_COUNT],
}

impl TeamEconomy {
    pub(crate) fn new(ore: i32, crystal: i32) -> Self {
        Self {
            ore,
            crystal,
            power_used: 0,
            power_capacity: 0,
            power_sabotage_remaining: 0.0,
            production_veterancy_ranks: [0; PRODUCTION_VETERANCY_PRODUCER_COUNT],
        }
    }

    pub(crate) fn can_afford(&self, cost: registry::Cost) -> bool {
        self.ore >= cost.ore && self.crystal >= cost.crystal
    }

    pub(crate) fn spend(&mut self, cost: registry::Cost) -> bool {
        if !self.can_afford(cost) {
            return false;
        }
        self.ore -= cost.ore;
        self.crystal -= cost.crystal;
        true
    }

    pub(crate) fn refund(&mut self, cost: registry::Cost) {
        self.ore += cost.ore;
        self.crystal += cost.crystal;
    }

    pub(crate) fn low_power(&self) -> bool {
        self.power_used > self.power_capacity
    }

    pub(crate) fn production_veterancy_rank(&self, producer_id: &str) -> u8 {
        production_veterancy_slot(producer_id)
            .map(|idx| self.production_veterancy_ranks[idx])
            .unwrap_or(0)
    }

    pub(crate) fn grant_production_veterancy_rank(&mut self, producer_id: &str, rank: u8) {
        let Some(idx) = production_veterancy_slot(producer_id) else {
            return;
        };
        self.production_veterancy_ranks[idx] =
            self.production_veterancy_ranks[idx].max(rank.min(VETERANCY_MAX_RANK));
    }
}

pub(crate) fn setup_resource_nodes(
    commands: &mut Commands,
    asset_server: &AssetServer,
    map: &SkirmishMapDef,
) {
    for spec in map.resources {
        spawn_resource_node(
            commands,
            asset_server,
            spec.kind,
            spec.amount,
            map_local_to_world(map, spec.position),
        );
    }
}

pub(crate) fn spawn_resource_node(
    commands: &mut Commands,
    asset_server: &AssetServer,
    kind: ResourceKind,
    amount: i32,
    position: Vec3,
) -> Entity {
    // Match godot's resource models exactly: ResourceA (Ore) = rock_crystalsLargeA,
    // ResourceB (Crystal) = rock_crystalsLargeB. They differ in crystal-cluster
    // shape; the crystal facets are recolored per-mineral by tint_resource_models
    // (Ore=blue, Crystal=red), leaving the grey rock as-is — exactly like godot.
    let (model, scale, radius) = match kind {
        ResourceKind::Ore => ("models/kenney-spacekit/rock_crystalsLargeA.glb", 0.55, 0.6),
        ResourceKind::Crystal => ("models/kenney-spacekit/rock_crystalsLargeB.glb", 0.55, 0.6),
    };
    let entity_id = commands
        .spawn((
            Name::new(format!("{} node", kind.label())),
            Transform::from_translation(position),
            ResourceNode { kind, amount },
            Team::Neutral,
            Selectable { radius },
            VisibilityState { visible: false },
            Visibility::Hidden,
            MatchScopedEntity,
        ))
        .id();
    commands.entity(entity_id).with_children(|parent| {
        parent.spawn((
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(model))),
            Transform::from_translation(Vec3::Y * 0.03).with_scale(Vec3::splat(scale)),
        ));
    });
    entity_id
}

pub(crate) fn setup_supply_crates(
    commands: &mut Commands,
    asset_server: &AssetServer,
    map: &SkirmishMapDef,
) {
    for spec in map.supply_crates {
        spawn_supply_crate(
            commands,
            asset_server,
            spec.effect,
            map_local_to_world(map, spec.position),
        );
    }
}

pub(crate) fn spawn_supply_crate(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect: SupplyCrateEffect,
    position: Vec3,
) -> Entity {
    let entity_id = commands
        .spawn((
            Name::new(effect.label()),
            Transform::from_translation(position),
            SupplyCrate {
                effect,
                pickup_radius: SUPPLY_CRATE_PICKUP_RADIUS,
                resource_ore: SUPPLY_CRATE_RESOURCE_ORE,
                resource_crystal: SUPPLY_CRATE_RESOURCE_CRYSTAL,
                repair_radius: SUPPLY_CRATE_REPAIR_RADIUS,
                repair_amount: SUPPLY_CRATE_REPAIR_AMOUNT,
            },
            Team::Neutral,
            Selectable {
                radius: SUPPLY_CRATE_PICKUP_RADIUS,
            },
            VisibilityState { visible: false },
            Visibility::Hidden,
            MatchScopedEntity,
        ))
        .id();
    // godot's SupplyCrate (non-player/SupplyCrate.tscn): barrels on a small platform,
    // not a single box. Transforms are godot's world values (Geometry 0.62 scale +
    // 0.08 lift folded into each part). recenter_entity_models settles XZ to origin.
    commands.entity(entity_id).with_children(|parent| {
        parent.spawn((
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/kenney-spacekit/platform_small.glb"),
            )),
            Transform::from_translation(Vec3::new(0.0, 0.08, 0.0))
                .with_scale(Vec3::new(0.2604, 0.1116, 0.2604)),
        ));
        parent.spawn((
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/kenney-spacekit/barrels_rail.glb"),
            )),
            Transform::from_translation(Vec3::new(-0.0868, 0.1544, 0.0))
                .with_scale(Vec3::splat(0.2108)),
        ));
        parent.spawn((
            WorldAssetRoot(
                asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset("models/kenney-spacekit/barrel.glb")),
            ),
            Transform::from_translation(Vec3::new(0.1984, 0.1544, -0.124))
                .with_scale(Vec3::splat(0.1736)),
        ));
    });
    entity_id
}

pub(crate) fn active_structure_power_delta(
    structure: &Structure,
    under_construction: Option<&UnderConstruction>,
) -> Option<i32> {
    if !structure_is_constructed(under_construction) {
        return None;
    }
    registry::entity(structure.id).map(|def| def.power_delta)
}

pub(crate) fn powered_repair_offline(
    team: &Team,
    structure: Option<&Structure>,
    economies: &Economies,
) -> bool {
    let Some(structure) = structure else {
        return false;
    };
    if matches!(team, Team::Neutral) || !economies.get(*team).low_power() {
        return false;
    }
    registry::entity(structure.id)
        .is_some_and(|def| def.power_delta < 0 && def.repair_rate > 0.0 && def.repair_radius > 0.0)
}

pub(crate) fn power_readout_text(econ: &TeamEconomy) -> String {
    format!("{}/{}", econ.power_used, econ.power_capacity)
}

pub(crate) fn monitor_low_power_audio_feedback(
    economies: Res<Economies>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    if record_low_power_audio_feedback(&mut feedback, economies.get(player_team).low_power()) {
        record_low_power_battle_log(&mut battle_log);
    }
}

pub(crate) fn resource_target_at_cursor(
    cursor: Vec2,
    camera_q: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    resources: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &ResourceNode,
    )>,
    max_pick_radius: f32,
) -> Option<Entity> {
    let (camera, camera_transform) = camera_q.single().ok()?;
    let mut nearest = None;
    let mut nearest_screen_distance = f32::MAX;
    for (entity, transform, _selectable, _visibility, resource) in resources {
        if resource.amount <= 0 {
            continue;
        }
        let Some((screen_distance, pick_radius)) = resource_cursor_pick_distance(
            cursor,
            camera,
            camera_transform,
            transform.translation,
            resource.kind,
            max_pick_radius,
        ) else {
            continue;
        };
        if screen_distance <= pick_radius && screen_distance < nearest_screen_distance {
            nearest = Some(entity);
            nearest_screen_distance = screen_distance;
        }
    }
    nearest
}

/// Visual height (m) of the rendered resource model above its ground anchor, so
/// the cursor hit-test covers the whole crystal a player actually clicks (not
/// just the ground point under it). Proportional to the `spawn_resource_node`
/// model scale (ore 0.5 / crystal 0.38).
pub(crate) fn resource_visual_height(kind: ResourceKind) -> f32 {
    match kind {
        ResourceKind::Ore => 1.7,
        ResourceKind::Crystal => 1.3,
    }
}

/// Visual half-width (m) of the rendered resource model, for the hit-test radius.
pub(crate) fn resource_visual_half_width(kind: ResourceKind) -> f32 {
    match kind {
        ResourceKind::Ore => 0.85,
        ResourceKind::Crystal => 0.65,
    }
}

/// Cursor hit-test against a resource node treated as a screen-space capsule from
/// its ground anchor up to the top of its visible model. Returns
/// `(distance_to_capsule_axis, pick_radius)`. This fixes the long-standing bug
/// where clicking the visible crystal (which projects *above* the ground point on
/// an angled camera) missed a ground-anchored circular pick.
pub(crate) fn resource_cursor_pick_distance(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    base: Vec3,
    kind: ResourceKind,
    max_pick_radius: f32,
) -> Option<(f32, f32)> {
    let base_screen = camera.world_to_viewport(camera_transform, base).ok()?;
    let top = base + Vec3::Y * resource_visual_height(kind);
    let screen_distance = match camera.world_to_viewport(camera_transform, top).ok() {
        Some(top_screen) => point_to_segment_distance(cursor, base_screen, top_screen),
        None => cursor.distance(base_screen),
    };
    let half_width = resource_visual_half_width(kind);
    let projected_radius = [Vec3::X * half_width, Vec3::Z * half_width]
        .into_iter()
        .filter_map(|offset| {
            camera
                .world_to_viewport(camera_transform, base + offset)
                .ok()
                .map(|edge| edge.distance(base_screen))
        })
        .fold(0.0, f32::max);
    let pick_radius = projected_radius.clamp(
        RESOURCE_ORDER_SCREEN_PICK_MIN_RADIUS_PX,
        max_pick_radius.max(RESOURCE_ORDER_SCREEN_PICK_MIN_RADIUS_PX),
    );
    Some((screen_distance, pick_radius))
}

pub(crate) fn nearest_resource_dropoff_order_target(
    point: Vec3,
    team: Team,
    selectable_q: &Query<SelectableOrderTargetItem<'_>>,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (
        entity,
        transform,
        selectable,
        target_team,
        visibility,
        resource_node,
        supply_crate,
        health,
        _unit,
        structure,
        under_construction,
    ) in selectable_q
    {
        let Some(structure) = structure else {
            continue;
        };
        if !visibility.visible
            || *target_team != team
            || resource_node.is_some()
            || supply_crate.is_some()
            || health.is_none_or(|health| health.current <= 0.0)
            || !structure_is_constructed(under_construction)
            || !is_resource_dropoff_structure(structure)
        {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < nearest_distance {
            nearest = Some(entity);
            nearest_distance = distance;
        }
    }
    nearest
}

pub(crate) fn nearest_supply_crate_target(
    point: Vec3,
    crates: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &SupplyCrate,
    )>,
) -> Option<(Entity, Vec3)> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (entity, transform, selectable, visibility, _crate) in crates {
        if !visibility.visible {
            continue;
        }
        let distance = xz_distance(transform.translation, point);
        if distance <= selectable.radius + 0.45 && distance < nearest_distance {
            nearest = Some((entity, transform.translation));
            nearest_distance = distance;
        }
    }
    nearest
}

pub(crate) fn apply_resource_infiltration(
    capturer_def: &registry::EntityDef,
    capturer_team: Team,
    victim_team: Team,
    economies: &mut Economies,
) {
    if capturer_def.infiltration_resource_steal_ratio <= 0.0
        || capturer_def.infiltration_resource_steal_cap <= 0
        || capturer_team.economy_index().is_none()
        || victim_team.economy_index().is_none()
        || victim_team == capturer_team
    {
        return;
    }

    let victim = economies.get(victim_team);
    let ore = infiltration_steal_amount(
        victim.ore,
        capturer_def.infiltration_resource_steal_ratio,
        capturer_def.infiltration_resource_steal_cap,
    );
    let crystal = infiltration_steal_amount(
        victim.crystal,
        capturer_def.infiltration_resource_steal_ratio,
        capturer_def.infiltration_resource_steal_cap,
    );
    if ore <= 0 && crystal <= 0 {
        return;
    }

    {
        let victim = economies.get_mut(victim_team);
        victim.ore -= ore;
        victim.crystal -= crystal;
    }
    let capturer = economies.get_mut(capturer_team);
    capturer.ore += ore;
    capturer.crystal += crystal;
}

pub(crate) fn can_unit_collect_resources(unit: &Unit) -> bool {
    registry::entity(unit.id).is_some_and(|def| def.resource_capacity > 0)
}

pub(crate) fn is_economy_worker_selection_unit(unit: &Unit) -> bool {
    unit.id == "Worker"
}

pub(crate) fn spawn_refinery_free_worker(
    commands: &mut Commands,
    asset_server: &AssetServer,
    next_id: &mut NextSpawnId,
    structure_id: &'static str,
    team: Team,
    visible_team: Team,
    refinery_position: Vec3,
    build_origin: Vec3,
    spawn_seed: u32,
    bounds: MapBounds,
    visual_faction: Option<SkirmishFaction>,
) {
    if structure_id != "Refinery" {
        return;
    }
    let Some(spawn_at) =
        refinery_free_worker_spawn_position(refinery_position, build_origin, spawn_seed, bounds)
    else {
        return;
    };
    spawn_unit_with_visual_faction(
        commands,
        asset_server,
        next_id,
        "Worker",
        team,
        spawn_at,
        0,
        visual_faction.or_else(|| default_visual_faction(team)),
        visible_team,
    );
}

pub(crate) fn refinery_free_worker_spawn_position(
    refinery_position: Vec3,
    build_origin: Vec3,
    spawn_seed: u32,
    bounds: MapBounds,
) -> Option<Vec3> {
    let refinery_def = registry::entity("Refinery")?;
    let worker_def = registry::entity("Worker")?;
    let mut direction = Vec3::new(
        refinery_position.x - build_origin.x,
        0.0,
        refinery_position.z - build_origin.z,
    );
    if direction.length_squared() <= f32::EPSILON {
        let angle = spawn_seed as f32 * 1.618_034;
        direction = Vec3::new(angle.cos(), 0.0, angle.sin());
    }
    direction = direction.normalize();
    let distance = refinery_def.radius + worker_def.radius + 0.35;
    Some(bounds.clamp_ground_point(refinery_position + direction * distance, 1.0))
}

pub(crate) fn economy_tick(
    time: Res<Time>,
    mut economies: ResMut<Economies>,
    structures: Query<(&Structure, &Team, Option<&UnderConstruction>)>,
    mut income_sources: Query<(&Team, &mut IncomeSource, Option<&UnderConstruction>)>,
) {
    let mut power: Vec<(i32, i32)> = Vec::new();

    for (structure, team, under_construction) in &structures {
        let Some(idx) = team.economy_index() else {
            continue;
        };
        let Some(delta) = active_structure_power_delta(structure, under_construction) else {
            continue;
        };
        if power.len() <= idx {
            power.resize(idx + 1, (0, 0));
        }
        let target = &mut power[idx];
        if delta >= 0 {
            target.1 += delta;
        } else {
            target.0 += -delta;
        }
    }

    let team_count = economies.players.len().max(power.len());
    for team in player_teams(team_count) {
        let idx = team.index();
        let economy = economies.get_mut(team);
        economy.power_sabotage_remaining =
            (economy.power_sabotage_remaining - time.delta_secs()).max(0.0);
        let (power_used, power_capacity) = power.get(idx).copied().unwrap_or((0, 0));
        economy.power_used = power_used;
        economy.power_capacity = if economy.power_sabotage_remaining > 0.0 {
            0
        } else {
            power_capacity
        };
    }

    for (team, mut income, under_construction) in &mut income_sources {
        let can_pay =
            structure_is_constructed(under_construction) && team.economy_index().is_some();
        let (ore, crystal) = advance_income_source(&mut income, time.delta_secs(), can_pay);
        if ore != 0 || crystal != 0 {
            let economy = economies.get_mut(*team);
            economy.ore += ore;
            economy.crystal += crystal;
        }
    }
}

pub(crate) fn advance_income_source(
    income: &mut IncomeSource,
    delta_seconds: f32,
    can_pay: bool,
) -> (i32, i32) {
    income.remaining -= delta_seconds;
    let interval = income.interval.max(0.1);
    let mut ore = 0;
    let mut crystal = 0;
    while income.remaining <= 0.0 {
        if can_pay {
            ore += income.ore;
            crystal += income.crystal;
        }
        income.remaining += interval;
    }
    (ore, crystal)
}

// Idle collectors of every team (human included) auto-resume harvesting, the way
// RA2/SC1 collectors do. The `IdleUnitOrderFilter` guarantees only units with no
// active order are picked, so manually-controlled collectors are never hijacked.
pub(crate) fn auto_assign_idle_resource_collectors(
    mut commands: Commands,
    active_teams: Option<Res<ActiveTeams>>,
    visible_player: Option<Res<VisiblePlayer>>,
    units: Query<
        (
            Entity,
            &Team,
            &Unit,
            &Transform,
            &Health,
            &ResourceCargo,
            Option<&OrderQueue>,
        ),
        (With<Unit>, IdleUnitOrderFilter),
    >,
    resources: Query<(Entity, &Transform, &Selectable, &ResourceNode)>,
    dropoffs: Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    // Only the AI auto-harvests. The human player gathers manually (select a
    // worker, right-click an ore node) so their workers never wander off alone.
    let player_team = controlled_player_team(visible_player.as_deref());
    for (entity, team, unit, transform, health, cargo, order_queue) in &units {
        if player_team == Some(*team)
            || !team_is_active(*team, active_teams.as_deref())
            || !can_unit_collect_resources(unit)
            || health.current <= 0.0
            || cargo.capacity <= 0
        {
            continue;
        }
        if order_queue.is_some_and(|queue| !queue.orders.is_empty()) {
            continue;
        }
        if nearest_resource_dropoff(*team, transform.translation, &dropoffs).is_none() {
            continue;
        }
        let nearest_resource = nearest_resource_entity(
            transform.translation,
            None,
            &resources,
            Some(RESOURCE_SEARCH_RADIUS_M),
        );
        let state = if cargo.has_any() && (cargo.is_full() || nearest_resource.is_none()) {
            HarvestState::MovingToDropoff
        } else {
            HarvestState::MovingToResource
        };
        if nearest_resource.is_none() && state == HarvestState::MovingToResource {
            continue;
        }
        commands.entity(entity).try_insert(HarvestOrder {
            resource: nearest_resource,
            state,
            collect_remaining: 0.0,
            last_kind: None,
        });
    }
}

pub(crate) fn update_harvest_orders(
    mut commands: Commands,
    time: Res<Time>,
    mut economies: ResMut<Economies>,
    mut collectors: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Selectable,
            &Unit,
            &Health,
            &mut ResourceCargo,
            &mut HarvestOrder,
            Option<&MoveOrder>,
            Option<&EmpDisabled>,
        ),
        (With<Unit>, Without<Structure>, Without<ResourceNode>),
    >,
    mut resource_queries: ParamSet<(
        Query<(Entity, &Transform, &Selectable, &ResourceNode)>,
        Query<(Entity, &Transform, &Selectable, &mut ResourceNode)>,
    )>,
    dropoffs: Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) {
    for (
        unit_entity,
        team,
        transform,
        selectable,
        unit,
        health,
        mut cargo,
        mut order,
        move_order,
        emp,
    ) in &mut collectors
    {
        if cargo.capacity <= 0
            || health.current <= 0.0
            || emp.is_some_and(|emp| emp.remaining > 0.0)
        {
            commands
                .entity(unit_entity)
                .try_remove::<HarvestOrder>()
                .try_remove::<MoveOrder>();
            continue;
        }

        if cargo.is_full() {
            order.state = HarvestState::MovingToDropoff;
        }

        match order.state {
            HarvestState::MovingToResource => {
                let target = {
                    let resources = resource_queries.p0();
                    resolve_harvest_resource_target(
                        order.resource,
                        transform.translation,
                        order.last_kind,
                        &resources,
                    )
                };
                let Some(target) = target else {
                    if cargo.has_any() {
                        order.state = HarvestState::MovingToDropoff;
                    } else {
                        commands
                            .entity(unit_entity)
                            .try_remove::<HarvestOrder>()
                            .try_remove::<MoveOrder>();
                    }
                    continue;
                };
                order.resource = Some(target);

                let in_range = {
                    let resources = resource_queries.p0();
                    let Ok((_, resource_transform, resource_selectable, resource)) =
                        resources.get(target)
                    else {
                        order.resource = None;
                        continue;
                    };
                    if resource.amount <= 0 {
                        order.resource = None;
                        continue;
                    }
                    let entry_range = contact_action_entry_range(
                        selectable.radius,
                        resource_selectable.radius,
                        RESOURCE_ENTRY_MARGIN_M,
                    );
                    xz_distance(transform.translation, resource_transform.translation)
                        <= entry_range
                };

                if !in_range {
                    if unit.speed <= 0.0 {
                        commands
                            .entity(unit_entity)
                            .try_remove::<HarvestOrder>()
                            .try_remove::<MoveOrder>();
                        continue;
                    }
                    let resources = resource_queries.p0();
                    if let Ok((_, resource_transform, resource_selectable, _)) =
                        resources.get(target)
                    {
                        if !move_order_targets_contact(
                            move_order,
                            resource_transform.translation,
                            selectable.radius,
                            resource_selectable.radius,
                        ) {
                            commands.entity(unit_entity).try_insert(MoveOrder {
                                target: unit_contact_move_target_position(
                                    transform.translation,
                                    selectable.radius,
                                    resource_transform.translation,
                                    resource_selectable.radius,
                                ),
                            });
                        }
                    }
                    continue;
                }

                commands.entity(unit_entity).try_remove::<MoveOrder>();
                let resources = resource_queries.p0();
                if let Ok((_, _, _, resource)) = resources.get(target) {
                    order.state = HarvestState::Collecting;
                    order.collect_remaining = resource.kind.collect_seconds();
                }
            }
            HarvestState::Collecting => {
                let Some(target) = order.resource else {
                    order.state = HarvestState::MovingToResource;
                    continue;
                };
                let mut depleted_position = None;
                {
                    let mut resources = resource_queries.p1();
                    let Ok((
                        resource_entity,
                        resource_transform,
                        resource_selectable,
                        mut resource,
                    )) = resources.get_mut(target)
                    else {
                        order.resource = None;
                        order.state = HarvestState::MovingToResource;
                        continue;
                    };
                    let entry_range = contact_action_entry_range(
                        selectable.radius,
                        resource_selectable.radius,
                        RESOURCE_ENTRY_MARGIN_M,
                    );
                    if resource.amount <= 0
                        || xz_distance(transform.translation, resource_transform.translation)
                            > entry_range
                    {
                        order.state = HarvestState::MovingToResource;
                        if unit.speed <= 0.0 {
                            commands
                                .entity(unit_entity)
                                .try_remove::<HarvestOrder>()
                                .try_remove::<MoveOrder>();
                        }
                        continue;
                    }

                    order.collect_remaining -= time.delta_secs();
                    let resource_position = resource_transform.translation;
                    let resource_kind = resource.kind;
                    order.last_kind = Some(resource_kind);
                    while order.collect_remaining <= 0.0 && resource.amount > 0 && !cargo.is_full()
                    {
                        resource.amount -= 1;
                        if cargo.add_one(resource_kind) {
                            commands.spawn((
                                ShotPulse {
                                    from: resource_position + Vec3::Y * 0.38,
                                    to: transform.translation + Vec3::Y * 0.45,
                                    ttl: 0.14,
                                    team: *team,
                                },
                                MatchScopedEntity,
                            ));
                        }
                        order.collect_remaining += resource_kind.collect_seconds();
                    }

                    if resource.amount <= 0 {
                        depleted_position = Some(resource_transform.translation);
                        commands.entity(resource_entity).try_despawn();
                    }
                }

                if let Some(position) = depleted_position {
                    commands.spawn((
                        ShotPulse {
                            from: transform.translation + Vec3::Y * 0.25,
                            to: position + Vec3::Y * 0.25,
                            ttl: 0.18,
                            team: *team,
                        },
                        MatchScopedEntity,
                    ));
                    order.resource = None;
                    order.state = if cargo.has_any() {
                        HarvestState::MovingToDropoff
                    } else {
                        HarvestState::MovingToResource
                    };
                } else if cargo.is_full() {
                    order.resource = None;
                    order.state = HarvestState::MovingToDropoff;
                }
            }
            HarvestState::MovingToDropoff => {
                let Some((_, dropoff_position, dropoff_radius)) =
                    nearest_resource_dropoff(*team, transform.translation, &dropoffs)
                else {
                    commands
                        .entity(unit_entity)
                        .try_remove::<HarvestOrder>()
                        .try_remove::<MoveOrder>();
                    continue;
                };

                let entry_range = contact_action_entry_range(
                    selectable.radius,
                    dropoff_radius,
                    RESOURCE_DROPOFF_ENTRY_MARGIN_M,
                );
                if xz_distance(transform.translation, dropoff_position) > entry_range {
                    if unit.speed <= 0.0 {
                        commands
                            .entity(unit_entity)
                            .try_remove::<HarvestOrder>()
                            .try_remove::<MoveOrder>();
                        continue;
                    }
                    if !move_order_targets_contact(
                        move_order,
                        dropoff_position,
                        selectable.radius,
                        dropoff_radius,
                    ) {
                        commands.entity(unit_entity).try_insert(MoveOrder {
                            target: unit_contact_move_target_position(
                                transform.translation,
                                selectable.radius,
                                dropoff_position,
                                dropoff_radius,
                            ),
                        });
                    }
                    continue;
                }

                commands.entity(unit_entity).try_remove::<MoveOrder>();
                let (ore, crystal) = cargo.clear();
                let apply_bonus = resource_dropoff_bonus_applies(*team, &economies, &dropoffs);
                let economy = economies.get_mut(*team);
                economy.ore += resource_amount_after_dropoff_bonus(ore, apply_bonus);
                economy.crystal += resource_amount_after_dropoff_bonus(crystal, apply_bonus);
                commands.spawn((
                    ShotPulse {
                        from: transform.translation + Vec3::Y * 0.35,
                        to: dropoff_position + Vec3::Y * 0.35,
                        ttl: 0.2,
                        team: *team,
                    },
                    MatchScopedEntity,
                ));
                order.state = HarvestState::MovingToResource;
                order.resource = None;
                order.collect_remaining = 0.0;
            }
        }
    }
}

pub(crate) fn resolve_harvest_resource_target(
    current: Option<Entity>,
    position: Vec3,
    prefer_kind: Option<ResourceKind>,
    resources: &Query<(Entity, &Transform, &Selectable, &ResourceNode)>,
) -> Option<Entity> {
    if let Some(current) = current
        && let Ok((_, _, _, resource)) = resources.get(current)
        && resource.amount > 0
    {
        return Some(current);
    }
    // When the node runs out, retarget to the nearest node of the SAME mineral type
    // so a crystal harvester doesn't auto-run to ore (or vice-versa). Only fall back
    // to "any nearest" when this harvester hasn't gathered anything yet (no kind).
    nearest_resource_entity(
        position,
        prefer_kind,
        resources,
        Some(RESOURCE_SEARCH_RADIUS_M),
    )
}

pub(crate) fn nearest_resource_entity(
    position: Vec3,
    prefer_kind: Option<ResourceKind>,
    resources: &Query<(Entity, &Transform, &Selectable, &ResourceNode)>,
    max_distance: Option<f32>,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (entity, transform, selectable, resource) in resources {
        if resource.amount <= 0 {
            continue;
        }
        if prefer_kind.is_some_and(|kind| kind != resource.kind) {
            continue;
        }
        let distance = xz_distance(position, transform.translation) - selectable.radius;
        if max_distance.is_some_and(|max| distance > max) {
            continue;
        }
        if distance < nearest_distance {
            nearest = Some(entity);
            nearest_distance = distance;
        }
    }
    nearest
}

pub(crate) fn nearest_resource_dropoff(
    team: Team,
    position: Vec3,
    dropoffs: &Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) -> Option<(Entity, Vec3, f32)> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (entity, structure, structure_team, transform, selectable, health, under_construction) in
        dropoffs
    {
        if *structure_team != team
            || health.current <= 0.0
            || !structure_is_constructed(under_construction)
        {
            continue;
        }
        if !is_resource_dropoff_structure(structure) {
            continue;
        }
        let distance = xz_distance(position, transform.translation);
        if distance < nearest_distance {
            nearest = Some((entity, transform.translation, selectable.radius));
            nearest_distance = distance;
        }
    }
    nearest
}

pub(crate) fn resource_dropoff_bonus_applies(
    team: Team,
    economies: &Economies,
    structures: &Query<
        (
            Entity,
            &Structure,
            &Team,
            &Transform,
            &Selectable,
            &Health,
            Option<&UnderConstruction>,
        ),
        (With<Structure>, Without<Unit>),
    >,
) -> bool {
    if economies.get(team).low_power() {
        return false;
    }
    structures.iter().any(
        |(_, structure, structure_team, _, _, health, under_construction)| {
            *structure_team == team
                && structure.id == "OrePurifier"
                && health.current > 0.0
                && structure_is_constructed(under_construction)
        },
    )
}

pub(crate) fn resource_amount_after_dropoff_bonus(amount: i32, apply_bonus: bool) -> i32 {
    if amount <= 0 || !apply_bonus {
        return amount;
    }
    amount + ((amount as f32) * ORE_PURIFIER_BONUS_RATIO).ceil() as i32
}

pub(crate) fn is_resource_dropoff_structure(structure: &Structure) -> bool {
    matches!(structure.id, "CommandCenter" | "Refinery")
}

pub(crate) fn collect_supply_crates(
    mut commands: Commands,
    mut economies: ResMut<Economies>,
    relations: Res<TeamRelations>,
    visible_player: Option<Res<VisiblePlayer>>,
    mut audio_feedback: ResMut<AudioFeedback>,
    mut battle_log: ResMut<BattleLog>,
    crates: Query<(Entity, &Transform, &SupplyCrate)>,
    mut units: ParamSet<(
        Query<(Entity, &Team, &Transform, &Selectable, &Unit, &Health)>,
        Query<(&Team, &Transform, &Selectable, &mut Health), With<Unit>>,
        Query<
            (
                Entity,
                &Team,
                &Transform,
                &Selectable,
                &mut Health,
                Option<&mut Weapon>,
                &mut VisionRadius,
                &mut Veterancy,
                Option<&VisibilityState>,
            ),
            With<Unit>,
        >,
    )>,
) {
    let player_team = visible_player_team(visible_player.as_deref());
    let mut collections = Vec::new();
    {
        let unit_scan = units.p0();
        for (crate_entity, crate_transform, supply_crate) in &crates {
            let mut collector = None;
            let mut nearest_distance = f32::MAX;
            for (unit_entity, team, unit_transform, unit_selectable, unit, health) in &unit_scan {
                if team.economy_index().is_none() || health.current <= 0.0 || unit.speed <= 0.0 {
                    continue;
                }
                let distance = xz_distance(crate_transform.translation, unit_transform.translation);
                if distance <= supply_crate.pickup_radius + unit_selectable.radius
                    && distance < nearest_distance
                {
                    collector = Some((unit_entity, *team));
                    nearest_distance = distance;
                }
            }
            if let Some((collector_entity, collector_team)) = collector {
                collections.push((
                    crate_entity,
                    crate_transform.translation,
                    *supply_crate,
                    collector_entity,
                    collector_team,
                ));
            }
        }
    }

    for (crate_entity, crate_position, supply_crate, collector_entity, collector_team) in
        collections
    {
        match supply_crate.effect {
            SupplyCrateEffect::Resources => {
                let economy = economies.get_mut(collector_team);
                economy.ore += supply_crate.resource_ore;
                economy.crystal += supply_crate.resource_crystal;
            }
            SupplyCrateEffect::Repair => {
                let mut repair_targets = units.p1();
                for (team, transform, selectable, mut health) in &mut repair_targets {
                    if !relations.are_allied(collector_team, *team)
                        || health.current <= 0.0
                        || health.current >= health.max
                    {
                        continue;
                    }
                    if xz_distance(crate_position, transform.translation)
                        <= supply_crate.repair_radius + selectable.radius
                    {
                        health.current =
                            (health.current + supply_crate.repair_amount).min(health.max);
                    }
                }
            }
            SupplyCrateEffect::Veterancy => {
                let mut promote_targets = units.p2();
                if !try_grant_veterancy_rank(
                    &mut commands,
                    collector_entity,
                    1,
                    &mut promote_targets,
                ) {
                    let mut best = None;
                    let mut best_distance = f32::MAX;
                    for (entity, team, transform, _, _, _, _, veteran, _) in &promote_targets {
                        if !relations.are_allied(collector_team, *team)
                            || veteran.rank >= VETERANCY_MAX_RANK
                        {
                            continue;
                        }
                        let distance = xz_distance(crate_position, transform.translation);
                        if distance <= supply_crate.repair_radius && distance < best_distance {
                            best = Some(entity);
                            best_distance = distance;
                        }
                    }
                    if let Some(entity) = best {
                        let _ = try_grant_veterancy_rank(
                            &mut commands,
                            entity,
                            1,
                            &mut promote_targets,
                        );
                    }
                }
            }
        }
        commands.spawn((
            ShotPulse {
                from: crate_position + Vec3::Y * 0.35,
                to: crate_position + Vec3::Y * 1.15,
                ttl: 0.24,
                team: collector_team,
            },
            MatchScopedEntity,
        ));
        if collector_team == player_team {
            record_sound_audio_feedback(&mut audio_feedback, SoundEffectKind::SupplyCrate);
            push_battle_log(
                &mut battle_log,
                format!(
                    "{}: {}",
                    t("补给箱", "Supply crate"),
                    supply_crate.effect.label()
                ),
                Some(crate_position),
            );
        }
        commands.entity(crate_entity).try_despawn();
    }
}

pub(crate) fn powered_combat_offline(
    team: &Team,
    structure: Option<&Structure>,
    economies: &Economies,
) -> bool {
    let Some(structure) = structure else {
        return false;
    };
    if matches!(team, Team::Neutral) || !economies.get(*team).low_power() {
        return false;
    }
    registry::entity(structure.id).is_some_and(|def| def.power_delta < 0 && def.weapon.is_some())
}

/// Per-kind tint materials applied to resource models so ore (red) and crystal
/// (green) read as distinct minerals (mirrors godot's resource_a/_b albedo tints).
#[derive(Resource)]
pub(crate) struct ResourceTintMaterials {
    pub(crate) ore: Handle<StandardMaterial>,
    pub(crate) crystal: Handle<StandardMaterial>,
}

/// Marks a resource whose model meshes have been recolored to its mineral tint.
#[derive(Component)]
pub(crate) struct ResourceTinted;

pub(crate) fn resource_tint_material(kind: ResourceKind) -> StandardMaterial {
    let c = kind.color();
    let lin = c.to_linear();
    // Mirror godot's resource_a/_b crystal material: pure metallic albedo (blue/red)
    // with a touch of emissive so the gem facets stay vivid under the lighting.
    StandardMaterial {
        base_color: c,
        metallic: 1.0,
        perceptual_roughness: 0.35,
        emissive: LinearRgba::new(lin.red * 0.30, lin.green * 0.30, lin.blue * 0.30, 1.0),
        ..default()
    }
}

/// The default crystal-facet albedo baked into kenney's rock_crystals GLBs (the
/// teal gems) — godot's Color(0.4687, 0.944, 0.7938), which is an sRGB literal and
/// matches bevy's loaded base_color in sRGB (verified: srgb=(0.469,0.944,0.794)).
/// godot replaces exactly this material; we match it the same way so only the
/// crystals recolor and the grey rock is left intact.
pub(crate) const CRYSTAL_FACET_ALBEDO_SRGB: [f32; 3] = [0.4687, 0.944, 0.7938];

pub(crate) const CRYSTAL_FACET_ALBEDO_EPSILON: f32 = 0.06;

pub(crate) fn is_crystal_facet_albedo(color: Color) -> bool {
    let s = color.to_srgba();
    (s.red - CRYSTAL_FACET_ALBEDO_SRGB[0]).abs() < CRYSTAL_FACET_ALBEDO_EPSILON
        && (s.green - CRYSTAL_FACET_ALBEDO_SRGB[1]).abs() < CRYSTAL_FACET_ALBEDO_EPSILON
        && (s.blue - CRYSTAL_FACET_ALBEDO_SRGB[2]).abs() < CRYSTAL_FACET_ALBEDO_EPSILON
}

/// Recolors each resource node's loaded model meshes with its mineral tint, once
/// the GLB scene has spawned its meshes.
pub(crate) fn tint_resource_models(
    mut commands: Commands,
    tints: Option<Res<ResourceTintMaterials>>,
    materials: Res<Assets<StandardMaterial>>,
    resources: Query<(Entity, &ResourceNode), Without<ResourceTinted>>,
    children_q: Query<&Children>,
    mut material_q: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let Some(tints) = tints else {
        return;
    };
    for (root, node) in &resources {
        let handle = match node.kind {
            ResourceKind::Ore => &tints.ore,
            ResourceKind::Crystal => &tints.crystal,
        };
        // Recolor ONLY the crystal-facet meshes (those carrying the teal gem
        // albedo) and leave the grey rock untouched — exactly godot's behavior.
        // Don't mark ResourceTinted until at least one facet recolors, so we keep
        // retrying while the GLB materials are still streaming in.
        let mut recolored = false;
        let mut stack: Vec<Entity> = children_q
            .get(root)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        while let Some(entity) = stack.pop() {
            if let Ok(children) = children_q.get(entity) {
                stack.extend(children.iter());
            }
            if let Ok(mut material) = material_q.get_mut(entity) {
                let Some(current) = materials.get(&material.0) else {
                    continue;
                };
                if is_crystal_facet_albedo(current.base_color) {
                    material.0 = handle.clone();
                    recolored = true;
                }
            }
        }
        if recolored {
            commands.entity(root).insert(ResourceTinted);
        }
    }
}

/// The resource node currently under the cursor (for hover highlight + so the
/// player knows a click will hit it). Updated by `update_resource_hover`.
#[derive(Resource, Default)]
pub(crate) struct HoveredResource(pub(crate) Option<Entity>);

pub(crate) fn update_resource_hover(
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    resources: Query<(
        Entity,
        &Transform,
        &Selectable,
        &VisibilityState,
        &ResourceNode,
    )>,
    command_mode: Res<CommandMode>,
    mut hovered: ResMut<HoveredResource>,
) {
    hovered.0 = None;
    if command_mode.pending_structure_placement.is_some() {
        return;
    }
    let Ok(window) = window_q.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    hovered.0 = resource_target_at_cursor(
        cursor,
        &camera_q,
        &resources,
        RESOURCE_ORDER_COLLECTOR_SCREEN_PICK_MAX_RADIUS_PX,
    );
}

pub(crate) const HARVEST_CARGO_VISUAL_MAX_SLOTS: usize = 6;

pub(crate) fn harvest_cargo_visual_slots(cargo: ResourceCargo) -> Vec<ResourceKind> {
    let mut slots = Vec::new();
    for _ in 0..cargo.ore.max(0) {
        if slots.len() >= HARVEST_CARGO_VISUAL_MAX_SLOTS {
            return slots;
        }
        slots.push(ResourceKind::Ore);
    }
    for _ in 0..cargo.crystal.max(0) {
        if slots.len() >= HARVEST_CARGO_VISUAL_MAX_SLOTS {
            return slots;
        }
        slots.push(ResourceKind::Crystal);
    }
    slots
}

pub(crate) fn harvest_visual_color(kind: ResourceKind, alpha: f32) -> Color {
    // Beam + cargo dots take the mineral color (ore green, crystal red).
    let c = kind.color().to_srgba();
    Color::srgba(c.red, c.green, c.blue, alpha)
}

/// Brighter tint of the mineral color for the beam's "hot" core + sparks (mixed
/// halfway to white), so the green ore beam glows green and the red crystal red.
pub(crate) fn harvest_visual_hot_color(kind: ResourceKind, alpha: f32) -> Color {
    let c = kind.color().to_srgba();
    Color::srgba(
        (c.red + 1.0) * 0.5,
        (c.green + 1.0) * 0.5,
        (c.blue + 1.0) * 0.5,
        alpha,
    )
}

pub(crate) fn draw_harvest_and_cargo_visuals(
    gizmos: &mut Gizmos,
    hud: &mut Gizmos<HudGizmos>,
    position: Vec3,
    radius: f32,
    harvest_order: Option<&HarvestOrder>,
    cargo: Option<&ResourceCargo>,
    resources: &Query<(
        Entity,
        &Transform,
        &Selectable,
        &ResourceNode,
        &VisibilityState,
    )>,
    elapsed_secs: f32,
    bar_right: Vec3,
) {
    if let Some(cargo) = cargo {
        draw_resource_cargo_visual(
            gizmos,
            hud,
            position,
            radius,
            *cargo,
            elapsed_secs,
            bar_right,
        );
    }

    let Some(order) = harvest_order else {
        return;
    };
    if order.state != HarvestState::Collecting {
        return;
    }
    let Some(target) = order.resource else {
        return;
    };
    let Ok((_, resource_transform, resource_selectable, resource, visibility)) =
        resources.get(target)
    else {
        return;
    };
    if !visibility.visible || resource.amount <= 0 {
        return;
    }

    let resource_position = resource_transform.translation;
    let to_resource = Vec3::new(
        resource_position.x - position.x,
        0.0,
        resource_position.z - position.z,
    )
    .normalize_or(Vec3::Z);
    let front = position + to_resource * (radius + 0.16) + Vec3::Y * 0.34;
    let contact =
        resource_position - to_resource * (resource_selectable.radius * 0.45) + Vec3::Y * 0.28;
    let color = harvest_visual_color(resource.kind, 0.84);
    let hot = harvest_visual_hot_color(resource.kind, 0.78);
    hud.line(front, contact, color);
    hud.line(front + Vec3::Y * 0.06, contact + Vec3::Y * 0.03, hot);

    let side = Vec3::new(-to_resource.z, 0.0, to_resource.x).normalize_or(bar_right);
    let phase = elapsed_secs * 7.5;
    for i in 0..8 {
        let seed = i as f32 * 2.399_963;
        let angle = phase + seed;
        let spread = 0.10 + 0.07 * ((phase * 0.7 + seed).sin() * 0.5 + 0.5);
        let lift = 0.10 + 0.38 * ((phase + seed * 0.5).cos() * 0.5 + 0.5);
        let center = front
            + side * angle.cos() * spread
            + to_resource * angle.sin() * spread * 0.55
            + Vec3::Y * lift;
        let spark_color = if i % 2 == 0 { hot } else { color };
        hud.line(
            center - Vec3::Y * 0.05,
            center + Vec3::Y * 0.06,
            spark_color,
        );
        gizmos.circle(
            Isometry3d::new(center, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
            0.035,
            spark_color,
        );
    }
}

pub(crate) fn draw_resource_cargo_visual(
    gizmos: &mut Gizmos,
    hud: &mut Gizmos<HudGizmos>,
    position: Vec3,
    radius: f32,
    cargo: ResourceCargo,
    elapsed_secs: f32,
    bar_right: Vec3,
) {
    let slots = harvest_cargo_visual_slots(cargo);
    if slots.is_empty() {
        return;
    }
    let side = bar_right.normalize_or(Vec3::X);
    let forward = Vec3::new(-side.z, 0.0, side.x).normalize_or(Vec3::Z);
    let base = position + Vec3::Y * (0.62 + radius * 0.42);
    for (index, kind) in slots.into_iter().enumerate() {
        let col = (index % 3) as f32 - 1.0;
        let row = (index / 3) as f32 - 0.5;
        let bob = 0.025 * (elapsed_secs * 5.0 + index as f32).sin();
        let center = base + side * (col * 0.17) + forward * (row * 0.18) + Vec3::Y * bob;
        let color = harvest_visual_color(kind, 0.92);
        gizmos.circle(
            Isometry3d::new(center, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
            0.075,
            color,
        );
        hud.line(center + Vec3::Y * 0.04, center + Vec3::Y * 0.20, color);
    }
}
