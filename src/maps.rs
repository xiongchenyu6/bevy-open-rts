//! Skirmish map catalog: map definitions, per-map resource/tech/wall/terrain
//! data tables, map bounds, random-map selection, and terrain-wall spawning.

use bevy::prelude::*;

use crate::*;

#[derive(Clone, Copy)]
pub(crate) struct NeutralTechSpec {
    pub(crate) name: &'static str,
    pub(crate) id: &'static str,
    pub(crate) position: (f32, f32),
}

#[derive(Clone, Copy)]
pub(crate) struct SkirmishMapDef {
    pub(crate) id: &'static str,
    pub(crate) godot_path: &'static str,
    pub(crate) name: &'static str,
    pub(crate) name_key: &'static str,
    pub(crate) players: usize,
    pub(crate) size: (f32, f32),
    pub(crate) spawn_points: &'static [(f32, f32)],
    pub(crate) resources: &'static [ResourceSpec],
    pub(crate) neutral_tech: &'static [NeutralTechSpec],
    pub(crate) supply_crates: &'static [NamedSupplyCrateSpec],
    pub(crate) terrain_walls: &'static [TerrainWallSpec],
    pub(crate) terrain_plateaus: &'static [TerrainPlateauSpec],
    pub(crate) terrain_ramps: &'static [TerrainRampSpec],
}

impl SkirmishMapDef {
    pub(crate) fn contains_point(self, point: (f32, f32)) -> bool {
        point.0 >= 0.0 && point.1 >= 0.0 && point.0 <= self.size.0 && point.1 <= self.size.1
    }

    pub(crate) fn is_catalog_consistent(self) -> bool {
        !self.id.is_empty()
            && !self.godot_path.is_empty()
            && !self.name.is_empty()
            && !self.name_key.is_empty()
            && self.players > 0
            && self.size.0 > 0.0
            && self.size.1 > 0.0
            && self.spawn_points.len() == self.players
            && self.resources.len() >= self.players * 3
            && self
                .spawn_points
                .iter()
                .copied()
                .all(|point| self.contains_point(point))
            && self
                .resources
                .iter()
                .all(|resource| self.contains_point(resource.position))
            && self.neutral_tech.iter().all(|tech| {
                !tech.name.is_empty() && !tech.id.is_empty() && self.contains_point(tech.position)
            })
            && self.supply_crates.iter().all(|crate_spec| {
                !crate_spec.name.is_empty()
                    && matches!(
                        crate_spec.effect,
                        SupplyCrateEffect::Resources
                            | SupplyCrateEffect::Repair
                            | SupplyCrateEffect::Veterancy
                    )
                    && self.contains_point(crate_spec.position)
            })
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectedSkirmishMap {
    pub(crate) godot_path: &'static str,
}

impl Default for SelectedSkirmishMap {
    fn default() -> Self {
        Self {
            godot_path: SKIRMISH_MAPS[0].godot_path,
        }
    }
}

impl SelectedSkirmishMap {
    pub(crate) fn definition(self) -> &'static SkirmishMapDef {
        skirmish_map_by_path(self.godot_path).unwrap_or(&SKIRMISH_MAPS[0])
    }
}

pub(crate) fn random_map_label() -> &'static str {
    t("随机地图", "Random Map")
}

pub(crate) fn localized_skirmish_map_name(map: &SkirmishMapDef) -> &'static str {
    match map.name_key {
        "MAP_NAME_PLAIN_AND_SIMPLE" => t("简明战场", "Plain & Simple"),
        "MAP_NAME_FOUR_CORNERS" => t("四角战场", "Four Corners"),
        "MAP_NAME_TECH_DIVIDE" => t("科技分界线", "Tech Divide"),
        "MAP_NAME_BIG_ARENA" => t("大型竞技场", "Big Arena"),
        "MAP_NAME_CANYON_PASS" => t("峡谷通道", "Canyon Pass"),
        "MAP_NAME_HIGHLAND_BASTION" => t("高地要塞", "Highland Bastion"),
        "MAP_NAME_CROSSFIRE" => t("交叉火线", "Crossfire"),
        "MAP_NAME_RING_VALLEY" => t("环形谷地", "Ring Valley"),
        _ => map.name,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RouletteWheel<T> {
    pub(crate) values_w_accumulated_shares: Vec<(T, f32)>,
}

impl<T> RouletteWheel<T> {
    fn new(value_to_share_mapping: impl IntoIterator<Item = (T, f32)>) -> Self {
        let values_w_positive_shares = value_to_share_mapping
            .into_iter()
            .filter(|(_, share)| *share > 0.0)
            .collect::<Vec<_>>();
        let total_share = values_w_positive_shares
            .iter()
            .map(|(_, share)| *share)
            .sum::<f32>();

        if total_share <= 0.0 {
            return Self {
                values_w_accumulated_shares: Vec::new(),
            };
        }

        let mut accumulated_share = 0.0;
        let mut values_w_accumulated_shares = Vec::with_capacity(values_w_positive_shares.len());
        for (value, share) in values_w_positive_shares {
            accumulated_share += share / total_share;
            values_w_accumulated_shares.push((value, accumulated_share));
        }

        Self {
            values_w_accumulated_shares,
        }
    }

    fn get_value(&self, probability: f32) -> Option<&T> {
        if self.values_w_accumulated_shares.is_empty() {
            return None;
        }

        let normalized_probability = probability.clamp(0.0, 1.0);
        for (value, accumulated_share) in &self.values_w_accumulated_shares {
            if normalized_probability <= *accumulated_share {
                return Some(value);
            }
        }

        self.values_w_accumulated_shares
            .last()
            .map(|(value, _)| value)
    }
}

pub(crate) fn is_random_map_index(index: usize) -> bool {
    index == random_map_index()
}

pub(crate) fn random_map_candidates_for_required_slots(
    required_player_slots: usize,
) -> impl Iterator<Item = &'static SkirmishMapDef> {
    let required_player_slots = required_player_slots.max(2);
    SKIRMISH_MAPS
        .iter()
        .filter(move |map| map.players >= required_player_slots)
}

pub(crate) fn random_map_for_required_slots(
    required_player_slots: usize,
    seed: u32,
) -> &'static SkirmishMapDef {
    let candidates =
        random_map_candidates_for_required_slots(required_player_slots).collect::<Vec<_>>();
    if candidates.is_empty() {
        return largest_skirmish_map();
    }

    let probability = roulette_bucket_probability(seed, candidates.len());
    let wheel = RouletteWheel::new(candidates.into_iter().map(|map| (map, 1.0)));
    wheel
        .get_value(probability)
        .copied()
        .unwrap_or_else(largest_skirmish_map)
}

pub(crate) fn roulette_bucket_probability(seed: u32, bucket_count: usize) -> f32 {
    if bucket_count == 0 {
        return 0.0;
    }
    let bucket = seed as usize % bucket_count;
    (bucket as f32 + 0.5) / bucket_count as f32
}

pub(crate) fn largest_skirmish_map() -> &'static SkirmishMapDef {
    SKIRMISH_MAPS
        .iter()
        .max_by(|left, right| {
            left.players
                .cmp(&right.players)
                .then_with(|| skirmish_map_area(left).total_cmp(&skirmish_map_area(right)))
        })
        .unwrap_or(&SKIRMISH_MAPS[0])
}

pub(crate) fn skirmish_map_area(map: &SkirmishMapDef) -> f32 {
    map.size.0 * map.size.1
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RandomMapCursor(pub(crate) u32);

impl Default for RandomMapCursor {
    fn default() -> Self {
        Self(0x4f1b_2c3d)
    }
}

impl RandomMapCursor {
    pub(crate) fn next_seed(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct MapBounds {
    pub(crate) half_width: f32,
    pub(crate) half_depth: f32,
}

impl Default for MapBounds {
    fn default() -> Self {
        Self {
            half_width: MAP_HALF_EXTENT,
            half_depth: MAP_HALF_EXTENT,
        }
    }
}

impl MapBounds {
    pub(crate) fn from_size(size: (f32, f32)) -> Self {
        Self {
            half_width: size.0 * 0.5,
            half_depth: size.1 * 0.5,
        }
    }

    pub(crate) fn from_map(map: &SkirmishMapDef) -> Self {
        Self::from_size(map.size)
    }

    pub(crate) fn contains_ground_point(self, point: Vec3) -> bool {
        point.x >= -self.half_width
            && point.x <= self.half_width
            && point.z >= -self.half_depth
            && point.z <= self.half_depth
    }

    pub(crate) fn clamp_ground_point(self, point: Vec3, margin: f32) -> Vec3 {
        let half_width = (self.half_width - margin).max(0.0);
        let half_depth = (self.half_depth - margin).max(0.0);
        Vec3::new(
            point.x.clamp(-half_width, half_width),
            point.y,
            point.z.clamp(-half_depth, half_depth),
        )
    }

    pub(crate) fn minimap_local_position(self, world: Vec3) -> Vec2 {
        let rect = self.minimap_content_rect();
        let x = ((world.x + self.half_width) / (self.half_width * 2.0)).clamp(0.0, 1.0);
        let z = ((world.z + self.half_depth) / (self.half_depth * 2.0)).clamp(0.0, 1.0);
        // Minimap top = world -Z, matching the view convention (screen-up = -Z,
        // same as the edge-pan / WASD fixes). Previously top mapped to +Z, so the
        // minimap was inverted vs the world and clicks moved the camera the wrong way.
        Vec2::new(rect.left + x * rect.width, rect.top + z * rect.height)
    }

    pub(crate) fn minimap_world_position(self, local: Vec2) -> Vec3 {
        let rect = self.minimap_content_rect();
        let x = ((local.x - rect.left) / rect.width).clamp(0.0, 1.0);
        let z = ((local.y - rect.top) / rect.height).clamp(0.0, 1.0);
        Vec3::new(
            x * self.half_width * 2.0 - self.half_width,
            0.0,
            z * self.half_depth * 2.0 - self.half_depth,
        )
    }

    pub(crate) fn minimap_world_position_checked(self, local: Vec2) -> Option<Vec3> {
        let rect = self.minimap_content_rect();
        (local.x >= rect.left
            && local.x <= rect.left + rect.width
            && local.y >= rect.top
            && local.y <= rect.top + rect.height)
            .then(|| self.minimap_world_position(local))
    }

    pub(crate) fn minimap_content_rect(self) -> MinimapContentRect {
        let map_width = (self.half_width * 2.0).max(1.0);
        let map_height = (self.half_depth * 2.0).max(1.0);
        let scale = (MINIMAP_SIZE_PX / map_width).min(MINIMAP_SIZE_PX / map_height);
        let width = map_width * scale;
        let height = map_height * scale;
        MinimapContentRect {
            left: (MINIMAP_SIZE_PX - width) * 0.5,
            top: (MINIMAP_SIZE_PX - height) * 0.5,
            width,
            height,
        }
    }
}

pub(crate) const SKIRMISH_MAP_ORE_AMOUNT: i32 = 240;
pub(crate) const SKIRMISH_MAP_CRYSTAL_AMOUNT: i32 = 140;

macro_rules! map_ore {
    ($x:expr, $z:expr) => {
        ResourceSpec {
            kind: ResourceKind::Ore,
            amount: SKIRMISH_MAP_ORE_AMOUNT,
            position: ($x, $z),
        }
    };
}

macro_rules! map_crystal {
    ($x:expr, $z:expr) => {
        ResourceSpec {
            kind: ResourceKind::Crystal,
            amount: SKIRMISH_MAP_CRYSTAL_AMOUNT,
            position: ($x, $z),
        }
    };
}

pub(crate) const PLAIN_AND_SIMPLE_SPAWNS: &[(f32, f32)] =
    &[(10.0, 7.0), (40.0, 7.0), (40.0, 43.0), (10.0, 43.0)];

pub(crate) const PLAIN_AND_SIMPLE_RESOURCES: &[ResourceSpec] = &[
    map_ore!(7.52981, 15.5708),
    map_ore!(9.4963, 15.3833),
    map_ore!(9.07351, 17.1366),
    map_ore!(40.5191, 14.7519),
    map_ore!(41.412, 13.361),
    map_ore!(42.5933, 14.65),
    map_crystal!(14.2571, 15.2579),
    map_crystal!(36.3143, 14.8547),
    map_ore!(40.685, 34.3027),
    map_ore!(41.533, 35.8383),
    map_ore!(42.7592, 34.2008),
    map_crystal!(36.4802, 34.4055),
    map_ore!(9.6306, 33.1921),
    map_ore!(8.46063, 34.3596),
    map_ore!(7.58274, 32.8476),
    map_crystal!(13.7599, 33.992),
];

pub(crate) const FOUR_CORNERS_SPAWNS: &[(f32, f32)] =
    &[(10.0, 10.0), (62.0, 10.0), (62.0, 62.0), (10.0, 62.0)];

pub(crate) const FOUR_CORNERS_RESOURCES: &[ResourceSpec] = &[
    map_ore!(15.0, 16.0),
    map_ore!(18.0, 15.0),
    map_ore!(17.0, 18.0),
    map_crystal!(22.0, 20.0),
    map_ore!(57.0, 16.0),
    map_ore!(54.0, 15.0),
    map_ore!(55.0, 18.0),
    map_crystal!(50.0, 20.0),
    map_ore!(57.0, 56.0),
    map_ore!(54.0, 57.0),
    map_ore!(55.0, 54.0),
    map_crystal!(50.0, 52.0),
    map_ore!(15.0, 56.0),
    map_ore!(18.0, 57.0),
    map_ore!(17.0, 54.0),
    map_crystal!(22.0, 52.0),
    map_ore!(32.0, 36.0),
    map_ore!(40.0, 36.0),
    map_crystal!(36.0, 32.0),
    map_crystal!(36.0, 40.0),
];

pub(crate) const FOUR_CORNERS_NEUTRAL_TECH: &[NeutralTechSpec] = &[
    NeutralTechSpec {
        name: "WestOilDerrick",
        id: "TechOilDerrick",
        position: (24.0, 36.0),
    },
    NeutralTechSpec {
        name: "EastOilDerrick",
        id: "TechOilDerrick",
        position: (48.0, 36.0),
    },
    NeutralTechSpec {
        name: "NorthTechAirport",
        id: "TechAirport",
        position: (36.0, 24.0),
    },
    NeutralTechSpec {
        name: "SouthTechAirport",
        id: "TechAirport",
        position: (36.0, 48.0),
    },
    NeutralTechSpec {
        name: "NorthEastTechHospital",
        id: "TechHospital",
        position: (48.0, 24.0),
    },
    NeutralTechSpec {
        name: "SouthWestTechHospital",
        id: "TechHospital",
        position: (24.0, 48.0),
    },
    NeutralTechSpec {
        name: "NorthWestTechRepairDepot",
        id: "TechRepairDepot",
        position: (24.0, 24.0),
    },
    NeutralTechSpec {
        name: "SouthEastTechRepairDepot",
        id: "TechRepairDepot",
        position: (48.0, 48.0),
    },
    NeutralTechSpec {
        name: "NorthCenterTechBunker",
        id: "TechBunker",
        position: (36.0, 28.0),
    },
    NeutralTechSpec {
        name: "SouthCenterTechBunker",
        id: "TechBunker",
        position: (36.0, 44.0),
    },
];

pub(crate) const FOUR_CORNERS_CRATES: &[NamedSupplyCrateSpec] = &[
    NamedSupplyCrateSpec {
        name: "NorthWestResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (27.0, 27.0),
    },
    NamedSupplyCrateSpec {
        name: "NorthEastRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (45.0, 27.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthEastVeterancyCrate",
        effect: SupplyCrateEffect::Veterancy,
        position: (45.0, 45.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthWestResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (27.0, 45.0),
    },
];

pub(crate) const TECH_DIVIDE_SPAWNS: &[(f32, f32)] = &[
    (10.0, 16.0),
    (10.0, 42.0),
    (10.0, 68.0),
    (74.0, 16.0),
    (74.0, 42.0),
    (74.0, 68.0),
];

pub(crate) const TECH_DIVIDE_RESOURCES: &[ResourceSpec] = &[
    map_ore!(15.0, 12.0),
    map_ore!(18.0, 15.0),
    map_ore!(14.0, 18.0),
    map_crystal!(21.0, 20.0),
    map_ore!(14.0, 38.0),
    map_ore!(18.0, 42.0),
    map_ore!(14.0, 46.0),
    map_crystal!(22.0, 42.0),
    map_ore!(14.0, 66.0),
    map_ore!(18.0, 69.0),
    map_ore!(15.0, 72.0),
    map_crystal!(21.0, 64.0),
    map_ore!(70.0, 12.0),
    map_ore!(66.0, 15.0),
    map_ore!(69.0, 18.0),
    map_crystal!(63.0, 20.0),
    map_ore!(70.0, 38.0),
    map_ore!(66.0, 42.0),
    map_ore!(70.0, 46.0),
    map_crystal!(62.0, 42.0),
    map_ore!(69.0, 66.0),
    map_ore!(66.0, 69.0),
    map_ore!(70.0, 72.0),
    map_crystal!(63.0, 64.0),
    map_ore!(38.0, 42.0),
    map_ore!(46.0, 42.0),
    map_crystal!(42.0, 38.0),
    map_crystal!(42.0, 46.0),
];

pub(crate) const TECH_DIVIDE_NEUTRAL_TECH: &[NeutralTechSpec] = &[
    NeutralTechSpec {
        name: "NorthOilDerrick",
        id: "TechOilDerrick",
        position: (42.0, 16.0),
    },
    NeutralTechSpec {
        name: "SouthOilDerrick",
        id: "TechOilDerrick",
        position: (42.0, 68.0),
    },
    NeutralTechSpec {
        name: "WestOilDerrick",
        id: "TechOilDerrick",
        position: (30.0, 42.0),
    },
    NeutralTechSpec {
        name: "EastOilDerrick",
        id: "TechOilDerrick",
        position: (54.0, 42.0),
    },
    NeutralTechSpec {
        name: "NorthTechAirport",
        id: "TechAirport",
        position: (42.0, 28.0),
    },
    NeutralTechSpec {
        name: "SouthTechAirport",
        id: "TechAirport",
        position: (42.0, 56.0),
    },
    NeutralTechSpec {
        name: "NorthWestTechHospital",
        id: "TechHospital",
        position: (24.0, 30.0),
    },
    NeutralTechSpec {
        name: "SouthEastTechHospital",
        id: "TechHospital",
        position: (60.0, 54.0),
    },
    NeutralTechSpec {
        name: "SouthWestTechRepairDepot",
        id: "TechRepairDepot",
        position: (24.0, 54.0),
    },
    NeutralTechSpec {
        name: "NorthEastTechRepairDepot",
        id: "TechRepairDepot",
        position: (60.0, 30.0),
    },
    NeutralTechSpec {
        name: "NorthWestTechBunker",
        id: "TechBunker",
        position: (36.0, 36.0),
    },
    NeutralTechSpec {
        name: "NorthEastTechBunker",
        id: "TechBunker",
        position: (48.0, 36.0),
    },
    NeutralTechSpec {
        name: "SouthEastTechBunker",
        id: "TechBunker",
        position: (48.0, 48.0),
    },
    NeutralTechSpec {
        name: "SouthWestTechBunker",
        id: "TechBunker",
        position: (36.0, 48.0),
    },
];

pub(crate) const TECH_DIVIDE_CRATES: &[NamedSupplyCrateSpec] = &[
    NamedSupplyCrateSpec {
        name: "NorthResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (42.0, 22.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (42.0, 62.0),
    },
    NamedSupplyCrateSpec {
        name: "WestVeterancyCrate",
        effect: SupplyCrateEffect::Veterancy,
        position: (26.0, 42.0),
    },
    NamedSupplyCrateSpec {
        name: "EastResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (58.0, 42.0),
    },
    NamedSupplyCrateSpec {
        name: "NorthWestRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (30.0, 30.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthEastVeterancyCrate",
        effect: SupplyCrateEffect::Veterancy,
        position: (54.0, 54.0),
    },
];

pub(crate) const BIG_ARENA_SPAWNS: &[(f32, f32)] = &[
    (10.0, 30.0),
    (35.0, 10.0),
    (65.0, 10.0),
    (90.0, 30.0),
    (90.0, 70.0),
    (65.0, 90.0),
    (35.0, 90.0),
    (10.0, 70.0),
];

pub(crate) const BIG_ARENA_RESOURCES: &[ResourceSpec] = &[
    map_ore!(13.8154, 23.5681),
    map_ore!(12.3442, 21.3867),
    map_ore!(9.51845, 22.2094),
    map_crystal!(3.60055, 23.3712),
    map_ore!(27.4944, 3.46134),
    map_ore!(25.3909, 5.04189),
    map_ore!(26.3566, 7.82198),
    map_crystal!(27.8187, 13.6729),
    map_ore!(73.3526, 12.7052),
    map_ore!(75.2773, 10.9112),
    map_ore!(74.0231, 8.24873),
    map_crystal!(71.9507, 2.58511),
    map_ore!(94.8869, 25.3542),
    map_ore!(93.3679, 23.2059),
    map_ore!(90.561, 24.0908),
    map_crystal!(84.6702, 25.3831),
    map_ore!(84.7421, 73.7477),
    map_ore!(86.059, 76.0256),
    map_ore!(88.9349, 75.4003),
    map_crystal!(94.9189, 74.6505),
    map_ore!(71.2856, 94.4847),
    map_ore!(73.2918, 92.7823),
    map_ore!(72.163, 90.0643),
    map_crystal!(70.3568, 84.3103),
    map_ore!(29.1414, 85.0513),
    map_ore!(27.0299, 86.6211),
    map_ore!(27.9815, 89.4061),
    map_crystal!(29.4138, 95.2644),
    map_ore!(4.39248, 74.789),
    map_ore!(5.9203, 76.9311),
    map_ore!(8.72352, 76.0348),
    map_crystal!(14.609, 74.7184),
];

pub(crate) const BIG_ARENA_NEUTRAL_TECH: &[NeutralTechSpec] = &[
    NeutralTechSpec {
        name: "NorthOilDerrick",
        id: "TechOilDerrick",
        position: (50.0, 34.0),
    },
    NeutralTechSpec {
        name: "EastOilDerrick",
        id: "TechOilDerrick",
        position: (66.0, 50.0),
    },
    NeutralTechSpec {
        name: "SouthOilDerrick",
        id: "TechOilDerrick",
        position: (50.0, 66.0),
    },
    NeutralTechSpec {
        name: "WestOilDerrick",
        id: "TechOilDerrick",
        position: (34.0, 50.0),
    },
    NeutralTechSpec {
        name: "WestTechAirport",
        id: "TechAirport",
        position: (38.0, 44.0),
    },
    NeutralTechSpec {
        name: "EastTechAirport",
        id: "TechAirport",
        position: (62.0, 56.0),
    },
    NeutralTechSpec {
        name: "NorthTechHospital",
        id: "TechHospital",
        position: (56.0, 38.0),
    },
    NeutralTechSpec {
        name: "SouthTechHospital",
        id: "TechHospital",
        position: (44.0, 62.0),
    },
    NeutralTechSpec {
        name: "WestTechRepairDepot",
        id: "TechRepairDepot",
        position: (38.0, 56.0),
    },
    NeutralTechSpec {
        name: "EastTechRepairDepot",
        id: "TechRepairDepot",
        position: (62.0, 44.0),
    },
    NeutralTechSpec {
        name: "NorthWestTechBunker",
        id: "TechBunker",
        position: (44.0, 38.0),
    },
    NeutralTechSpec {
        name: "NorthEastTechBunker",
        id: "TechBunker",
        position: (62.0, 38.0),
    },
    NeutralTechSpec {
        name: "SouthEastTechBunker",
        id: "TechBunker",
        position: (56.0, 62.0),
    },
    NeutralTechSpec {
        name: "SouthWestTechBunker",
        id: "TechBunker",
        position: (38.0, 62.0),
    },
];

pub(crate) const BIG_ARENA_CRATES: &[NamedSupplyCrateSpec] = &[
    NamedSupplyCrateSpec {
        name: "CenterResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (50.0, 50.0),
    },
    NamedSupplyCrateSpec {
        name: "NorthRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (50.0, 42.0),
    },
    NamedSupplyCrateSpec {
        name: "EastVeterancyCrate",
        effect: SupplyCrateEffect::Veterancy,
        position: (58.0, 50.0),
    },
    NamedSupplyCrateSpec {
        name: "SouthResourceCrate",
        effect: SupplyCrateEffect::Resources,
        position: (50.0, 58.0),
    },
    NamedSupplyCrateSpec {
        name: "WestRepairCrate",
        effect: SupplyCrateEffect::Repair,
        position: (42.0, 50.0),
    },
];

pub(crate) const EMPTY_NEUTRAL_TECH: &[NeutralTechSpec] = &[];
pub(crate) const EMPTY_NAMED_CRATES: &[NamedSupplyCrateSpec] = &[];

/// A run of impassable rocks along a map-local segment: ground units and
/// structure placement are blocked, air flies over (the gameplay value of
/// cliffs without leaving godot's flat-terrain model).
#[derive(Clone, Copy)]
pub(crate) struct TerrainWallSpec {
    pub(crate) start: (f32, f32),
    pub(crate) end: (f32, f32),
    pub(crate) width: f32,
}

#[derive(Component)]
pub(crate) struct TerrainWall;

pub(crate) const EMPTY_TERRAIN_WALLS: &[TerrainWallSpec] = &[];

// ---- bevy-original tier-3 maps (terrain-wall showcases) ----

pub(crate) const CANYON_PASS_SPAWNS: &[(f32, f32)] = &[(10.0, 28.0), (46.0, 28.0)];
pub(crate) const CANYON_PASS_RESOURCES: &[ResourceSpec] = &[
    map_ore!(8.0, 18.0),
    map_ore!(9.6, 19.4),
    map_ore!(7.2, 20.6),
    map_crystal!(12.0, 38.0),
    map_ore!(48.0, 18.0),
    map_ore!(46.4, 19.4),
    map_ore!(48.8, 20.6),
    map_crystal!(44.0, 38.0),
    map_ore!(28.0, 8.0),
    map_ore!(28.0, 48.0),
];
/// Two long walls leave a north and a south pass through the canyon.
pub(crate) const CANYON_PASS_WALLS: &[TerrainWallSpec] = &[
    TerrainWallSpec {
        start: (24.0, 0.0),
        end: (24.0, 20.0),
        width: 2.4,
    },
    TerrainWallSpec {
        start: (24.0, 36.0),
        end: (24.0, 56.0),
        width: 2.4,
    },
    TerrainWallSpec {
        start: (32.0, 0.0),
        end: (32.0, 20.0),
        width: 2.4,
    },
    TerrainWallSpec {
        start: (32.0, 36.0),
        end: (32.0, 56.0),
        width: 2.4,
    },
];

pub(crate) const CROSSFIRE_SPAWNS: &[(f32, f32)] =
    &[(10.0, 10.0), (54.0, 10.0), (54.0, 54.0), (10.0, 54.0)];
pub(crate) const CROSSFIRE_RESOURCES: &[ResourceSpec] = &[
    map_ore!(16.0, 8.0),
    map_ore!(17.6, 9.4),
    map_crystal!(8.0, 16.0),
    map_ore!(48.0, 8.0),
    map_ore!(46.4, 9.4),
    map_crystal!(56.0, 16.0),
    map_ore!(48.0, 56.0),
    map_ore!(46.4, 54.6),
    map_crystal!(56.0, 48.0),
    map_ore!(16.0, 56.0),
    map_ore!(17.6, 54.6),
    map_crystal!(8.0, 48.0),
    map_ore!(32.0, 30.0),
    map_ore!(32.0, 34.0),
];
/// A cross of walls splits the map into quadrants with four gaps near the rim.
pub(crate) const CROSSFIRE_WALLS: &[TerrainWallSpec] = &[
    TerrainWallSpec {
        start: (32.0, 10.0),
        end: (32.0, 26.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (32.0, 38.0),
        end: (32.0, 54.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (10.0, 32.0),
        end: (26.0, 32.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (38.0, 32.0),
        end: (54.0, 32.0),
        width: 2.2,
    },
];

pub(crate) const RING_VALLEY_SPAWNS: &[(f32, f32)] =
    &[(8.0, 8.0), (52.0, 8.0), (52.0, 52.0), (8.0, 52.0)];
pub(crate) const RING_VALLEY_RESOURCES: &[ResourceSpec] = &[
    map_ore!(6.0, 14.0),
    map_ore!(7.4, 15.6),
    map_ore!(54.0, 14.0),
    map_ore!(52.6, 15.6),
    map_ore!(54.0, 46.0),
    map_ore!(52.6, 44.4),
    map_ore!(6.0, 46.0),
    map_ore!(7.4, 44.4),
    map_crystal!(28.0, 28.0),
    map_crystal!(32.0, 28.0),
    map_crystal!(28.0, 32.0),
    map_crystal!(32.0, 32.0),
    map_ore!(30.0, 24.0),
    map_ore!(30.0, 36.0),
];
/// A ring of rock with four gates guarding the rich centre.
pub(crate) const RING_VALLEY_WALLS: &[TerrainWallSpec] = &[
    TerrainWallSpec {
        start: (20.0, 20.0),
        end: (26.0, 20.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (34.0, 20.0),
        end: (40.0, 20.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (40.0, 20.0),
        end: (40.0, 26.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (40.0, 34.0),
        end: (40.0, 40.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (34.0, 40.0),
        end: (40.0, 40.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (20.0, 40.0),
        end: (26.0, 40.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (20.0, 34.0),
        end: (20.0, 40.0),
        width: 2.2,
    },
    TerrainWallSpec {
        start: (20.0, 20.0),
        end: (20.0, 26.0),
        width: 2.2,
    },
];

/// Highland Bastion: each base sits on its own plateau; the crystal high
/// grounds in the middle are only reachable over ramps, so holding a ramp
/// mouth holds the expansion.
pub(crate) const HIGHLAND_BASTION_SPAWNS: &[(f32, f32)] = &[(12.0, 30.0), (48.0, 30.0)];
pub(crate) const HIGHLAND_BASTION_RESOURCES: &[ResourceSpec] = &[
    map_ore!(8.0, 24.0),
    map_ore!(6.8, 25.6),
    map_ore!(9.2, 26.8),
    map_ore!(52.0, 24.0),
    map_ore!(53.2, 25.6),
    map_ore!(50.8, 26.8),
    map_crystal!(30.0, 10.0),
    map_crystal!(30.0, 50.0),
    map_ore!(27.0, 10.0),
    map_ore!(33.0, 50.0),
];
pub(crate) const HIGHLAND_BASTION_PLATEAUS: &[TerrainPlateauSpec] = &[
    // West and east base plateaus.
    TerrainPlateauSpec {
        min: (2.0, 18.0),
        max: (20.0, 42.0),
        level: 1,
    },
    TerrainPlateauSpec {
        min: (40.0, 18.0),
        max: (58.0, 42.0),
        level: 1,
    },
    // North and south crystal high grounds.
    TerrainPlateauSpec {
        min: (24.0, 2.0),
        max: (36.0, 16.0),
        level: 1,
    },
    TerrainPlateauSpec {
        min: (24.0, 44.0),
        max: (36.0, 58.0),
        level: 1,
    },
];
pub(crate) const HIGHLAND_BASTION_RAMPS: &[TerrainRampSpec] = &[
    // Down from each base plateau toward the valley floor.
    TerrainRampSpec {
        min: (20.0, 27.0),
        max: (27.0, 33.0),
        level: 1,
        direction: RampDirection::MinusX,
    },
    TerrainRampSpec {
        min: (33.0, 27.0),
        max: (40.0, 33.0),
        level: 1,
        direction: RampDirection::PlusX,
    },
    // Single ramp onto each crystal high ground: a defensible choke.
    TerrainRampSpec {
        min: (28.0, 16.0),
        max: (32.0, 23.0),
        level: 1,
        direction: RampDirection::MinusZ,
    },
    TerrainRampSpec {
        min: (28.0, 37.0),
        max: (32.0, 44.0),
        level: 1,
        direction: RampDirection::PlusZ,
    },
];

pub(crate) const SKIRMISH_MAPS: &[SkirmishMapDef] = &[
    SkirmishMapDef {
        id: "plain_and_simple",
        godot_path: "res://source/match/maps/PlainAndSimple.tscn",
        name: "Plain & Simple",
        name_key: "MAP_NAME_PLAIN_AND_SIMPLE",
        players: 4,
        size: (50.0, 50.0),
        spawn_points: PLAIN_AND_SIMPLE_SPAWNS,
        resources: PLAIN_AND_SIMPLE_RESOURCES,
        neutral_tech: EMPTY_NEUTRAL_TECH,
        supply_crates: EMPTY_NAMED_CRATES,
        terrain_walls: EMPTY_TERRAIN_WALLS,
        terrain_plateaus: EMPTY_TERRAIN_PLATEAUS,
        terrain_ramps: EMPTY_TERRAIN_RAMPS,
    },
    SkirmishMapDef {
        id: "four_corners",
        godot_path: "res://source/match/maps/FourCorners.tscn",
        name: "Four Corners",
        name_key: "MAP_NAME_FOUR_CORNERS",
        players: 4,
        size: (72.0, 72.0),
        spawn_points: FOUR_CORNERS_SPAWNS,
        resources: FOUR_CORNERS_RESOURCES,
        neutral_tech: FOUR_CORNERS_NEUTRAL_TECH,
        supply_crates: FOUR_CORNERS_CRATES,
        terrain_walls: EMPTY_TERRAIN_WALLS,
        terrain_plateaus: EMPTY_TERRAIN_PLATEAUS,
        terrain_ramps: EMPTY_TERRAIN_RAMPS,
    },
    SkirmishMapDef {
        id: "tech_divide",
        godot_path: "res://source/match/maps/TechDivide.tscn",
        name: "Tech Divide",
        name_key: "MAP_NAME_TECH_DIVIDE",
        players: 6,
        size: (84.0, 84.0),
        spawn_points: TECH_DIVIDE_SPAWNS,
        resources: TECH_DIVIDE_RESOURCES,
        neutral_tech: TECH_DIVIDE_NEUTRAL_TECH,
        supply_crates: TECH_DIVIDE_CRATES,
        terrain_walls: EMPTY_TERRAIN_WALLS,
        terrain_plateaus: EMPTY_TERRAIN_PLATEAUS,
        terrain_ramps: EMPTY_TERRAIN_RAMPS,
    },
    SkirmishMapDef {
        id: "big_arena",
        godot_path: "res://source/match/maps/BigArena.tscn",
        name: "Big Arena",
        name_key: "MAP_NAME_BIG_ARENA",
        players: 8,
        size: (100.0, 100.0),
        spawn_points: BIG_ARENA_SPAWNS,
        resources: BIG_ARENA_RESOURCES,
        neutral_tech: BIG_ARENA_NEUTRAL_TECH,
        supply_crates: BIG_ARENA_CRATES,
        terrain_walls: EMPTY_TERRAIN_WALLS,
        terrain_plateaus: EMPTY_TERRAIN_PLATEAUS,
        terrain_ramps: EMPTY_TERRAIN_RAMPS,
    },
    SkirmishMapDef {
        id: "canyon_pass",
        godot_path: "bevy://maps/CanyonPass",
        name: "Canyon Pass",
        name_key: "MAP_NAME_CANYON_PASS",
        players: 2,
        size: (56.0, 56.0),
        spawn_points: CANYON_PASS_SPAWNS,
        resources: CANYON_PASS_RESOURCES,
        neutral_tech: EMPTY_NEUTRAL_TECH,
        supply_crates: EMPTY_NAMED_CRATES,
        terrain_walls: CANYON_PASS_WALLS,
        terrain_plateaus: EMPTY_TERRAIN_PLATEAUS,
        terrain_ramps: EMPTY_TERRAIN_RAMPS,
    },
    SkirmishMapDef {
        id: "crossfire",
        godot_path: "bevy://maps/Crossfire",
        name: "Crossfire",
        name_key: "MAP_NAME_CROSSFIRE",
        players: 4,
        size: (64.0, 64.0),
        spawn_points: CROSSFIRE_SPAWNS,
        resources: CROSSFIRE_RESOURCES,
        neutral_tech: EMPTY_NEUTRAL_TECH,
        supply_crates: EMPTY_NAMED_CRATES,
        terrain_walls: CROSSFIRE_WALLS,
        terrain_plateaus: EMPTY_TERRAIN_PLATEAUS,
        terrain_ramps: EMPTY_TERRAIN_RAMPS,
    },
    SkirmishMapDef {
        id: "ring_valley",
        godot_path: "bevy://maps/RingValley",
        name: "Ring Valley",
        name_key: "MAP_NAME_RING_VALLEY",
        players: 4,
        size: (60.0, 60.0),
        spawn_points: RING_VALLEY_SPAWNS,
        resources: RING_VALLEY_RESOURCES,
        neutral_tech: EMPTY_NEUTRAL_TECH,
        supply_crates: EMPTY_NAMED_CRATES,
        terrain_walls: RING_VALLEY_WALLS,
        terrain_plateaus: EMPTY_TERRAIN_PLATEAUS,
        terrain_ramps: EMPTY_TERRAIN_RAMPS,
    },
    SkirmishMapDef {
        id: "highland_bastion",
        godot_path: "bevy://maps/HighlandBastion",
        name: "Highland Bastion",
        name_key: "MAP_NAME_HIGHLAND_BASTION",
        players: 2,
        size: (60.0, 60.0),
        spawn_points: HIGHLAND_BASTION_SPAWNS,
        resources: HIGHLAND_BASTION_RESOURCES,
        neutral_tech: EMPTY_NEUTRAL_TECH,
        supply_crates: EMPTY_NAMED_CRATES,
        terrain_walls: EMPTY_TERRAIN_WALLS,
        terrain_plateaus: HIGHLAND_BASTION_PLATEAUS,
        terrain_ramps: HIGHLAND_BASTION_RAMPS,
    },
];

#[cfg(test)]
pub(crate) fn skirmish_maps() -> &'static [SkirmishMapDef] {
    SKIRMISH_MAPS
}

pub(crate) fn skirmish_map_by_path(path: &str) -> Option<&'static SkirmishMapDef> {
    SKIRMISH_MAPS.iter().find(|map| map.godot_path == path)
}

pub(crate) fn map_local_to_world(map: &SkirmishMapDef, point: (f32, f32)) -> Vec3 {
    Vec3::new(point.0 - map.size.0 * 0.5, 0.0, point.1 - map.size.1 * 0.5)
}

pub(crate) fn team_start_position(map: &SkirmishMapDef, team: Team) -> Vec3 {
    let spawn_index = team.economy_index().unwrap_or(0);
    team_start_position_for_spawn_slot(map, spawn_index)
}

pub(crate) fn team_start_position_for_spawn_slot(map: &SkirmishMapDef, spawn_index: usize) -> Vec3 {
    map.spawn_points
        .get(spawn_index)
        .copied()
        .map(|spawn_point| map_local_to_world(map, spawn_point))
        .unwrap_or_else(|| fallback_team_start_position_for_spawn_slot(map, spawn_index))
}

/// Spawns the rock line for every terrain wall on the map. Each rock is an
/// obstacle entity (TerrainWall + Selectable radius) so the nav grid, unit
/// steering and structure placement all treat it as solid; air ignores it.
pub(crate) fn spawn_terrain_walls(
    commands: &mut Commands,
    asset_server: &AssetServer,
    map: &SkirmishMapDef,
) {
    const ROCK_MODELS: [(&str, f32); 3] = [
        ("models/kenney-spacekit/rock_largeA.glb", 1.15),
        ("models/kenney-spacekit/rock_largeB.glb", 1.3),
        ("models/kenney-spacekit/rock_largeA.glb", 0.95),
    ];
    for (wall_index, wall) in map.terrain_walls.iter().enumerate() {
        let start = map_local_to_world(map, wall.start);
        let end = map_local_to_world(map, wall.end);
        let length = xz_distance(start, end).max(0.1);
        let steps = (length / 1.5).ceil().max(1.0) as usize;
        let direction = (end - start) / length;
        let perpendicular = Vec3::new(-direction.z, 0.0, direction.x);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            // Deterministic jitter/rotation so maps look identical every load.
            let seed = (wall_index * 131 + step * 7919) as f32;
            let jitter = (seed.sin() * 43758.547).fract() - 0.5;
            let position = start.lerp(end, t) + perpendicular * jitter * wall.width * 0.4;
            let (model, scale) = ROCK_MODELS[(wall_index + step) % ROCK_MODELS.len()];
            commands.spawn((
                Name::new("Terrain Wall Rock"),
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(model))),
                Transform::from_translation(position)
                    .with_scale(Vec3::splat(scale))
                    .with_rotation(Quat::from_rotation_y(seed)),
                TerrainWall,
                Selectable {
                    radius: (wall.width * 0.5).max(0.7),
                },
                MatchScopedEntity,
            ));
        }
    }
}
