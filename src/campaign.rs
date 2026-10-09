//! The campaign: data-driven missions on top of the skirmish runtime.
//!
//! A mission is a const table entry — map, factions, victory kind and a list of
//! timed triggers (messages, reinforcement waves, resource grants). Missions
//! reuse the whole match pipeline: MatchSetupSettings drives the world, the
//! existing annihilation/headquarters evaluation decides defeat, and a small
//! checker grants SurviveFor victories. Tier-3 design: docs/TIER3_DESIGN.md.

use bevy::prelude::*;

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MissionVictory {
    /// Destroy every living enemy unit and structure, including surviving armies.
    DestroyAllEnemies,
    /// Hold out for this many simulated seconds.
    SurviveFor(f32),
    /// Destroy every enemy command center (VictoryCondition::Headquarters).
    Headquarters,
}

#[derive(Clone, Copy)]
pub(crate) enum MissionAction {
    Message {
        zh: &'static str,
        en: &'static str,
    },
    SpawnWave {
        team_index: usize,
        units: &'static [(&'static str, usize)],
        at: (f32, f32),
        attack_to: Option<(f32, f32)>,
    },
    GrantResources {
        team_index: usize,
        ore: i32,
        crystal: i32,
    },
}

#[derive(Clone, Copy)]
pub(crate) struct MissionTrigger {
    pub(crate) at_sec: f32,
    pub(crate) action: MissionAction,
}

#[derive(Clone, Copy)]
pub(crate) struct MissionDef {
    /// Stable profile/save identity, independent of menu ordering.
    pub(crate) id: &'static str,
    pub(crate) name_zh: &'static str,
    pub(crate) name_en: &'static str,
    pub(crate) briefing_zh: &'static str,
    pub(crate) briefing_en: &'static str,
    pub(crate) map_id: &'static str,
    pub(crate) spawn_slots: [usize; 2],
    pub(crate) player_faction: SkirmishFaction,
    pub(crate) enemy_faction: SkirmishFaction,
    pub(crate) enemy_difficulty: AiDifficulty,
    pub(crate) starting_resources: StartingResources,
    pub(crate) victory: MissionVictory,
    pub(crate) triggers: &'static [MissionTrigger],
    /// Briefing keyart shown by the mission cutscene (assets-relative path).
    pub(crate) art: &'static str,
}

pub(crate) const CAMPAIGN_MISSIONS: &[MissionDef] = &[
    MissionDef {
        id: "hold_the_line",
        name_zh: "第一关:站稳脚跟",
        name_en: "Mission 1: Hold the Line",
        briefing_zh: "敌军波次将穿过峡谷进攻。坚守基地 4 分钟,增援即至。",
        briefing_en: "Enemy waves will push through the canyon. Hold your base for 4 minutes.",
        map_id: "canyon_pass",
        spawn_slots: [0, 1],
        player_faction: SkirmishFaction::Alliance,
        enemy_faction: SkirmishFaction::Demon,
        enemy_difficulty: AiDifficulty::Beginner,
        starting_resources: StartingResources::new(48, 24),
        victory: MissionVictory::SurviveFor(240.0),
        triggers: &[
            MissionTrigger {
                at_sec: 5.0,
                action: MissionAction::Message {
                    zh: "指挥部: 敌军正在集结,构筑防线!",
                    en: "Command: enemy massing — dig in!",
                },
            },
            MissionTrigger {
                at_sec: 45.0,
                action: MissionAction::SpawnWave {
                    team_index: 1,
                    units: &[("HeavyMachinegunTrooper", 3)],
                    at: (46.0, 10.0),
                    attack_to: Some((10.0, 28.0)),
                },
            },
            MissionTrigger {
                at_sec: 110.0,
                action: MissionAction::SpawnWave {
                    team_index: 1,
                    units: &[("HeavyMachinegunTrooper", 3), ("FlameAssaultBuggy", 2)],
                    at: (46.0, 46.0),
                    attack_to: Some((10.0, 28.0)),
                },
            },
            MissionTrigger {
                at_sec: 120.0,
                action: MissionAction::GrantResources {
                    team_index: 0,
                    ore: 30,
                    crystal: 15,
                },
            },
            MissionTrigger {
                at_sec: 180.0,
                action: MissionAction::SpawnWave {
                    team_index: 1,
                    units: &[("Tank", 2), ("HeavyMachinegunTrooper", 4)],
                    at: (46.0, 28.0),
                    attack_to: Some((10.0, 28.0)),
                },
            },
        ],
        art: "campaign/mission_01-v2.jpg",
    },
    MissionDef {
        id: "strike_the_outpost",
        name_zh: "第二关:摧毁前哨",
        name_en: "Mission 2: Strike the Outpost",
        briefing_zh: "斩首行动:摧毁敌方指挥中心即告胜利。小心交叉火线的卡口。",
        briefing_en: "Decapitation: destroy the enemy command center. Mind the crossfire gaps.",
        map_id: "crossfire",
        spawn_slots: [0, 1],
        player_faction: SkirmishFaction::Alliance,
        enemy_faction: SkirmishFaction::Chaos,
        enemy_difficulty: AiDifficulty::Easy,
        starting_resources: StartingResources::new(64, 32),
        victory: MissionVictory::Headquarters,
        triggers: &[
            MissionTrigger {
                at_sec: 5.0,
                action: MissionAction::Message {
                    zh: "指挥部: 侦察显示敌方指挥中心防御薄弱,速战速决!",
                    en: "Command: their HQ is lightly guarded — strike fast!",
                },
            },
            MissionTrigger {
                at_sec: 150.0,
                action: MissionAction::SpawnWave {
                    team_index: 0,
                    units: &[("Tank", 2)],
                    at: (10.0, 10.0),
                    attack_to: None,
                },
            },
        ],
        art: "campaign/mission_02-v2.jpg",
    },
    MissionDef {
        id: "canyon_siege",
        name_zh: "第三关:峡谷合围",
        name_en: "Mission 3: Canyon Siege",
        briefing_zh: "突破环形谷地中的包围。控制中央矿区,消灭所有敌方部队和建筑。",
        briefing_en: "Break the siege in Ring Valley. Secure the central mine and eliminate every enemy unit and structure.",
        map_id: "ring_valley",
        spawn_slots: [0, 1],
        player_faction: SkirmishFaction::Alliance,
        enemy_faction: SkirmishFaction::Demon,
        enemy_difficulty: AiDifficulty::Hard,
        starting_resources: StartingResources::new(64, 32),
        victory: MissionVictory::DestroyAllEnemies,
        triggers: &[
            MissionTrigger {
                at_sec: 5.0,
                action: MissionAction::Message {
                    zh: "指挥部: 敌军实力强劲,控制中央矿区者控制战局。",
                    en: "Command: they are strong — whoever holds the centre holds the day.",
                },
            },
            MissionTrigger {
                at_sec: 240.0,
                action: MissionAction::Message {
                    zh: "指挥部: 增援已抵达西侧入口!",
                    en: "Command: reinforcements at the west entrance!",
                },
            },
            MissionTrigger {
                at_sec: 241.0,
                action: MissionAction::SpawnWave {
                    team_index: 0,
                    units: &[("Tank", 3), ("HeavyMachinegunTrooper", 4)],
                    at: (4.0, 30.0),
                    attack_to: None,
                },
            },
        ],
        art: "campaign/mission_03-v2.jpg",
    },
    MissionDef {
        id: "highland_bastion_stand",
        name_zh: "第四关:高地堡垒",
        name_en: "Mission 4: Highland Bastion",
        briefing_zh: "敌军将分波强攻高地。据守西侧坡道 5 分钟,空投补给中途抵达。",
        briefing_en: "Enemy waves will storm the highland. Hold the west ramp for 5 minutes.",
        map_id: "highland_bastion",
        spawn_slots: [0, 1],
        player_faction: SkirmishFaction::Alliance,
        enemy_faction: SkirmishFaction::Demon,
        enemy_difficulty: AiDifficulty::Normal,
        starting_resources: StartingResources::new(56, 28),
        victory: MissionVictory::SurviveFor(300.0),
        triggers: &[
            MissionTrigger {
                at_sec: 5.0,
                action: MissionAction::Message {
                    zh: "指挥部: 高地只有一条坡道可上,把守住它!",
                    en: "Command: one ramp leads up here — hold it!",
                },
            },
            MissionTrigger {
                at_sec: 60.0,
                action: MissionAction::SpawnWave {
                    team_index: 1,
                    units: &[("HeavyMachinegunTrooper", 4)],
                    at: (36.0, 30.0),
                    attack_to: Some((12.0, 30.0)),
                },
            },
            MissionTrigger {
                at_sec: 140.0,
                action: MissionAction::SpawnWave {
                    team_index: 1,
                    units: &[("GrenadierTrooper", 3), ("FlameAssaultBuggy", 2)],
                    at: (37.0, 20.0),
                    attack_to: Some((12.0, 30.0)),
                },
            },
            MissionTrigger {
                at_sec: 175.0,
                action: MissionAction::Message {
                    zh: "指挥部: 空投补给已落在你的基地。",
                    en: "Command: supply drop landed at your base.",
                },
            },
            MissionTrigger {
                at_sec: 176.0,
                action: MissionAction::GrantResources {
                    team_index: 0,
                    ore: 40,
                    crystal: 20,
                },
            },
            MissionTrigger {
                at_sec: 250.0,
                action: MissionAction::SpawnWave {
                    team_index: 1,
                    units: &[("Tank", 2), ("HeavyMachinegunTrooper", 4)],
                    at: (37.0, 40.0),
                    attack_to: Some((12.0, 30.0)),
                },
            },
        ],
        art: "campaign/mission_04-v2.jpg",
    },
    MissionDef {
        id: "twin_front",
        name_zh: "第五关:两线鏖战",
        name_en: "Mission 5: Twin Front",
        briefing_zh: "敌军主力盘踞对角,两翼另有伏兵。全歼四角战场上的所有敌军。",
        briefing_en: "Their main force holds the far corner; flanks hide more. Annihilate them all.",
        map_id: "four_corners",
        spawn_slots: [0, 2],
        player_faction: SkirmishFaction::Alliance,
        enemy_faction: SkirmishFaction::Chaos,
        enemy_difficulty: AiDifficulty::Hard,
        starting_resources: StartingResources::new(72, 36),
        victory: MissionVictory::DestroyAllEnemies,
        triggers: &[
            MissionTrigger {
                at_sec: 5.0,
                action: MissionAction::Message {
                    zh: "指挥部: 敌军不止一处营地,提防两翼突袭!",
                    en: "Command: more than one enemy camp — watch both flanks!",
                },
            },
            MissionTrigger {
                at_sec: 90.0,
                action: MissionAction::SpawnWave {
                    team_index: 1,
                    units: &[("HeavyMachinegunTrooper", 4)],
                    at: (10.0, 62.0),
                    attack_to: Some((10.0, 10.0)),
                },
            },
            MissionTrigger {
                at_sec: 205.0,
                action: MissionAction::Message {
                    zh: "指挥部: 侦察警报——东北角装甲集群正在逼近!",
                    en: "Command: scouts report armour inbound from the north-east!",
                },
            },
            MissionTrigger {
                at_sec: 210.0,
                action: MissionAction::SpawnWave {
                    team_index: 1,
                    units: &[("Tank", 2), ("FlameAssaultBuggy", 2)],
                    at: (62.0, 10.0),
                    attack_to: Some((10.0, 10.0)),
                },
            },
            MissionTrigger {
                at_sec: 300.0,
                action: MissionAction::Message {
                    zh: "指挥部: 友军装甲连抵达战场,转守为攻!",
                    en: "Command: allied armour has arrived — go on the offensive!",
                },
            },
            MissionTrigger {
                at_sec: 301.0,
                action: MissionAction::SpawnWave {
                    team_index: 0,
                    units: &[("Tank", 3), ("GrenadierTrooper", 4)],
                    at: (24.0, 10.0),
                    attack_to: None,
                },
            },
        ],
        art: "campaign/mission_05-v2.jpg",
    },
    MissionDef {
        id: "thunder_decapitation",
        name_zh: "第六关:雷霆斩首",
        name_en: "Mission 6: Thunder Decapitation",
        briefing_zh: "终局:补给线被切断,以现有储备直取敌指挥中枢。后勤纵队正在赶来。",
        briefing_en: "Endgame: supply lines cut. Take their command centre with what you have.",
        map_id: "tech_divide",
        spawn_slots: [0, 1],
        player_faction: SkirmishFaction::Alliance,
        enemy_faction: SkirmishFaction::Demon,
        enemy_difficulty: AiDifficulty::Hard,
        starting_resources: StartingResources::new(24, 8),
        victory: MissionVictory::Headquarters,
        triggers: &[
            MissionTrigger {
                at_sec: 5.0,
                action: MissionAction::Message {
                    zh: "指挥部: 储备见底,别恋战——摧毁敌指挥中心即告胜利。",
                    en: "Command: stores are dry — strike the HQ, nothing else matters.",
                },
            },
            MissionTrigger {
                at_sec: 90.0,
                action: MissionAction::Message {
                    zh: "指挥部: 后勤纵队突破封锁,物资已送达!",
                    en: "Command: the supply column broke through — materiel delivered!",
                },
            },
            MissionTrigger {
                at_sec: 91.0,
                action: MissionAction::GrantResources {
                    team_index: 0,
                    ore: 60,
                    crystal: 30,
                },
            },
            MissionTrigger {
                at_sec: 240.0,
                action: MissionAction::Message {
                    zh: "指挥部: 最后一批装甲增援已就位,发起总攻!",
                    en: "Command: final armour reserves committed — all-out assault!",
                },
            },
            MissionTrigger {
                at_sec: 241.0,
                action: MissionAction::SpawnWave {
                    team_index: 0,
                    units: &[("Tank", 2), ("HeavyMachinegunTrooper", 3)],
                    at: (10.0, 6.0),
                    attack_to: None,
                },
            },
        ],
        art: "campaign/mission_06-v2.jpg",
    },
];

/// The mission currently being played (index into [`CAMPAIGN_MISSIONS`]).
#[derive(Resource, Default)]
pub(crate) struct ActiveMission(pub(crate) Option<usize>);

/// Which triggers already fired for the active mission.
#[derive(Resource, Default)]
pub(crate) struct MissionTriggerState {
    pub(crate) fired: Vec<bool>,
}

pub(crate) fn mission_by_index(index: usize) -> Option<&'static MissionDef> {
    CAMPAIGN_MISSIONS.get(index)
}

/// Builds the full match settings for a mission (player slot 0, enemy slot 1).
pub(crate) fn match_settings_for_mission(mission: &MissionDef) -> Option<MatchSetupSettings> {
    let map = SKIRMISH_MAPS.iter().find(|map| map.id == mission.map_id)?;
    let team_ids = vec![0usize, 1usize];
    let active_teams = vec![true, true];
    Some(MatchSetupSettings {
        map_path: map.godot_path,
        starting_resources: mission.starting_resources,
        visible_player: VisiblePlayer::per_player(Team::Player(0)),
        ai_difficulties: AiDifficultySettings {
            players: vec![mission.enemy_difficulty; 2],
        },
        team_relations: skirmish_team_relations_from_team_ids(&active_teams, &team_ids),
        startup_loadout: StartupLoadoutMode::GodotSkirmish,
        victory_condition: match mission.victory {
            MissionVictory::Headquarters => VictoryCondition::Headquarters,
            _ => VictoryCondition::Annihilation,
        },
        active_teams,
        player_factions: vec![mission.player_faction, mission.enemy_faction],
        player_color_slots: vec![0, 1],
        player_controllers: vec![
            SkirmishPlayerController::Human,
            SkirmishPlayerController::Ai(mission.enemy_difficulty),
        ],
        player_spawn_slots: mission.spawn_slots.to_vec(),
    })
}

/// Fires due mission triggers against the running match.
pub(crate) fn run_mission_triggers(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    active: Res<ActiveMission>,
    match_state: Res<MatchState>,
    selected_map: Res<SelectedSkirmishMap>,
    mut state: ResMut<MissionTriggerState>,
    mut next_id: ResMut<NextSpawnId>,
    mut economies: ResMut<Economies>,
    mut battle_log: ResMut<BattleLog>,
    visible_player: Res<VisiblePlayer>,
) {
    let Some(mission) = active.0.and_then(mission_by_index) else {
        return;
    };
    if state.fired.len() != mission.triggers.len() {
        state.fired = vec![false; mission.triggers.len()];
    }
    let clock = match_state.start_time_sec;
    let map = selected_map.definition();
    for (index, trigger) in mission.triggers.iter().enumerate() {
        if state.fired[index] || clock < trigger.at_sec {
            continue;
        }
        state.fired[index] = true;
        match trigger.action {
            MissionAction::Message { zh, en } => {
                push_battle_log(&mut battle_log, t(zh, en), None);
            }
            MissionAction::SpawnWave {
                team_index,
                units,
                at,
                attack_to,
            } => {
                let origin = map_local_to_world(map, at);
                let attack_target = attack_to.map(|point| map_local_to_world(map, point));
                let mut slot = 0usize;
                for (unit_id, count) in units {
                    let Some(def) = registry::entity(unit_id) else {
                        continue;
                    };
                    for _ in 0..*count {
                        let position = origin + formation_offset(slot, 12);
                        slot += 1;
                        let entity = spawn_unit(
                            &mut commands,
                            &asset_server,
                            &mut next_id,
                            def.id,
                            Team::Player(team_index),
                            position,
                            0,
                            visible_player.team,
                        );
                        if let Some(destination) = attack_target {
                            commands
                                .entity(entity)
                                .insert(AttackMoveOrder { destination });
                        }
                    }
                }
                push_battle_log(
                    &mut battle_log,
                    if team_index == 0 {
                        t("增援抵达", "Reinforcements arrived")
                    } else {
                        t("敌军进攻波抵达", "Enemy wave incoming")
                    },
                    Some(origin),
                );
            }
            MissionAction::GrantResources {
                team_index,
                ore,
                crystal,
            } => {
                if let Some(economy) = economies.players.get_mut(team_index) {
                    economy.ore += ore;
                    economy.crystal += crystal;
                }
                push_battle_log(&mut battle_log, t("补给送达", "Supplies delivered"), None);
            }
        }
    }
}

/// Grants SurviveFor victories once the clock outlasts the requirement.
pub(crate) fn check_mission_victory(
    active: Res<ActiveMission>,
    mut match_state: ResMut<MatchState>,
    mut match_flow: ResMut<MatchFlow>,
    mut audio_feedback: ResMut<AudioFeedback>,
) {
    let Some(mission) = active.0.and_then(mission_by_index) else {
        return;
    };
    if !match_state.is_running() {
        return;
    }
    if let MissionVictory::SurviveFor(seconds) = mission.victory
        && match_state.start_time_sec >= seconds
    {
        record_voice_audio_feedback(&mut audio_feedback, UnitVoiceEvent::Victory);
        finalize_match(
            &mut match_state,
            &mut match_flow,
            MatchPhase::HumanVictory,
            t("坚守成功", "Held the line"),
        );
    }
}

/// The mission objective line shown by the top-center tracker while a
/// SurviveFor mission runs.
pub(crate) fn mission_survive_remaining(
    active: &ActiveMission,
    match_state: &MatchState,
) -> Option<f32> {
    let mission = active.0.and_then(mission_by_index)?;
    match mission.victory {
        MissionVictory::SurviveFor(seconds) => {
            Some((seconds - match_state.start_time_sec).max(0.0))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn objective_app(index: usize) -> App {
        let mut app = App::new();
        app.init_resource::<MatchState>()
            .init_resource::<MatchFlow>()
            .init_resource::<AudioFeedback>()
            .init_resource::<TeamRelations>()
            .insert_resource(VisiblePlayer::per_player(Team::Player(0)))
            .insert_resource(match_settings_for_mission(&CAMPAIGN_MISSIONS[index]).unwrap())
            .insert_resource(ActiveMission(Some(index)))
            .add_systems(Update, (evaluate_match_end, check_mission_victory).chain());
        app.world_mut().spawn((
            Structure {
                id: "CommandCenter",
            },
            Team::Player(0),
            Health::new(20.0),
        ));
        app
    }

    #[test]
    fn annihilation_campaign_requires_surviving_armies_and_structures_to_fall() {
        let mut app = objective_app(2);
        let tank = app
            .world_mut()
            .spawn((
                Unit {
                    id: "Tank",
                    speed: 1.0,
                    can_crush: true,
                    can_be_crushed: false,
                },
                Team::Player(1),
                Health::new(10.0),
            ))
            .id();
        let barracks = app
            .world_mut()
            .spawn((
                Structure { id: "Barracks" },
                Team::Player(1),
                Health::new(13.0),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::Running
        );
        app.world_mut().despawn(tank);
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::Running
        );
        app.world_mut().despawn(barracks);
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::HumanVictory
        );
    }

    #[test]
    fn survival_requires_the_timer_even_when_enemy_base_is_gone() {
        let mut app = objective_app(0);
        app.world_mut().resource_mut::<MatchState>().start_time_sec = 239.0;
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::Running
        );
        app.world_mut().resource_mut::<MatchState>().start_time_sec = 240.0;
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::HumanVictory
        );
    }

    #[test]
    fn survival_still_loses_when_the_player_base_and_workers_are_destroyed() {
        let mut app = objective_app(0);
        let mut query = app.world_mut().query_filtered::<Entity, With<Structure>>();
        let entities = query.iter(app.world()).collect::<Vec<_>>();
        for entity in entities {
            app.world_mut().despawn(entity);
        }
        app.world_mut().resource_mut::<MatchState>().start_time_sec = 240.0;
        app.update();
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::HumanDefeat
        );
    }

    #[test]
    fn every_mission_references_real_maps_and_units() {
        for mission in CAMPAIGN_MISSIONS {
            assert!(
                SKIRMISH_MAPS.iter().any(|map| map.id == mission.map_id),
                "mission {} references unknown map {}",
                mission.id,
                mission.map_id
            );
            let settings = match_settings_for_mission(mission).expect("settings must build");
            let map = skirmish_map_by_path(settings.map_path).unwrap();
            for slot in &settings.player_spawn_slots {
                assert!(
                    *slot < map.spawn_points.len(),
                    "{} has an invalid spawn",
                    mission.id
                );
            }
            assert_eq!(settings.player_controllers.len(), 2);
            for trigger in mission.triggers {
                if let MissionAction::SpawnWave { units, .. } = trigger.action {
                    for (unit_id, _) in units {
                        assert!(
                            registry::entity(unit_id).is_some(),
                            "mission {} spawns unknown unit {unit_id}",
                            mission.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn twin_front_places_the_main_force_in_the_far_corner_and_waves_on_both_flanks() {
        let mission = &CAMPAIGN_MISSIONS[4];
        let settings = match_settings_for_mission(mission).unwrap();
        let map = skirmish_map_by_path(settings.map_path).unwrap();
        let player = map_local_to_world(map, map.spawn_points[settings.player_spawn_slots[0]]);
        let enemy = map_local_to_world(map, map.spawn_points[settings.player_spawn_slots[1]]);
        assert!(map.spawn_points.iter().all(|point| {
            map_local_to_world(map, *point).distance_squared(player)
                <= enemy.distance_squared(player)
        }));
        let flanks = mission
            .triggers
            .iter()
            .filter_map(|trigger| match trigger.action {
                MissionAction::SpawnWave {
                    team_index: 1, at, ..
                } => Some(map_local_to_world(map, at)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            flanks
                .iter()
                .any(|point| point.x == player.x && point.z == enemy.z)
        );
        assert!(
            flanks
                .iter()
                .any(|point| point.x == enemy.x && point.z == player.z)
        );
    }

    #[test]
    fn survival_does_not_add_unscheduled_skirmish_armies() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        app.world_mut().resource_mut::<ActiveMission>().0 = Some(0);
        *app.world_mut().resource_mut::<MatchSetupSettings>() =
            match_settings_for_mission(&CAMPAIGN_MISSIONS[0]).unwrap();
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..40 {
            app.update();
        }
        let mut initial_query = app.world_mut().query::<(Entity, &Unit, &Team)>();
        let initial_army = initial_query
            .iter(app.world())
            .filter(|(_, unit, team)| {
                **team == Team::Player(1)
                    && registry::entity(unit.id).is_some_and(|def| def.weapon.is_some())
            })
            .map(|(entity, _, _)| entity)
            .collect::<Vec<_>>();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Enter);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        for _ in 0..900 {
            app.update();
        }
        assert!(app.world().resource::<MatchState>().start_time_sec > 25.0);
        let mut query = app.world_mut().query::<(Entity, &Unit, &Team)>();
        let attackers = query
            .iter(app.world())
            .filter(|(entity, unit, team)| {
                **team == Team::Player(1)
                    && !initial_army.contains(entity)
                    && registry::entity(unit.id).is_some_and(|def| def.weapon.is_some())
            })
            .count();
        assert_eq!(attackers, 0, "first authored wave arrives at 45 seconds");
    }

    #[test]
    fn survive_mission_grants_victory_and_triggers_fire() {
        let mut app = build_game_app(GameAppMode::Headless);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 30.0),
        ));
        let mission_index = 0usize; // hold_the_line (SurviveFor 240s)
        let mission = mission_by_index(mission_index).unwrap();
        let settings = match_settings_for_mission(mission).unwrap();
        app.world_mut().resource_mut::<ActiveMission>().0 = Some(mission_index);
        *app.world_mut().resource_mut::<MissionTriggerState>() = MissionTriggerState::default();
        *app.world_mut().resource_mut::<MatchSetupSettings>() = settings;
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::InMatch);
        for _ in 0..40 {
            app.update();
        }
        // Jump the clock past the first enemy wave and let the trigger run.
        app.world_mut().resource_mut::<MatchState>().start_time_sec = 46.0;
        for _ in 0..5 {
            app.update();
        }
        let world = app.world_mut();
        let enemy_units = {
            let mut q = world.query_filtered::<(&Team, &Unit), ()>();
            q.iter(world)
                .filter(|(team, _)| **team == Team::Player(1))
                .count()
        };
        assert!(enemy_units >= 3, "wave 1 spawned enemy attackers");
        // Jump past the survive requirement: victory must land.
        app.world_mut().resource_mut::<MatchState>().start_time_sec = 241.0;
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<MatchState>().phase,
            MatchPhase::HumanVictory,
            "surviving the timer wins the mission"
        );
    }
}
