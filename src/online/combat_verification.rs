//! Real-peer combat acceptance: the host stages weapon fire, a support strike,
//! unit deaths and a building collapse through the ordinary simulation, and
//! both peers record what crossed the wire, what was staged on screen, and a
//! digest of every world tick so the two can be compared afterwards.
use super::*;

/// Ticks of world digest kept for the cross-peer comparison (10 Hz).
const DIGEST_HISTORY: usize = 400;
const STRIKE_DELAY_SECONDS: f32 = 1.0;
const COLLAPSE_AFTER_SECONDS: f32 = 6.0;
const FINISH_AFTER_SECONDS: f32 = 10.0;
const EVIDENCE_TIMEOUT_SECONDS: f32 = 40.0;

#[derive(Default, Debug)]
pub(super) struct CombatState {
    staged_at: Option<f32>,
    collapsed: bool,
    pub(super) host_done: bool,
    seen_event_ids: HashSet<u64>,
    weapon_shots: u32,
    unit_deaths: u32,
    strikes: u32,
    structure_destructions: u32,
    presented_muzzle_flashes: u32,
    presented_wrecks: u32,
    presented_shockwaves: u32,
    presented_unit_deaths: u32,
    digests: BTreeMap<u64, String>,
}

impl CombatState {
    /// What this peer still has not seen. Empty once the evidence is complete.
    pub(super) fn missing(&self) -> Vec<&'static str> {
        [
            (self.weapon_shots, "weapon shot events"),
            (self.unit_deaths, "unit death events"),
            (self.strikes, "strike impact events"),
            (self.structure_destructions, "structure destruction events"),
            (self.presented_muzzle_flashes, "muzzle flashes staged"),
            (self.presented_unit_deaths, "unit deaths staged"),
            (self.presented_wrecks, "wrecks staged"),
            (self.presented_shockwaves, "shockwaves staged"),
        ]
        .into_iter()
        .filter_map(|(count, label)| (count == 0).then_some(label))
        .collect()
    }
}

fn enabled(harness: &OnlineVerificationHarness) -> bool {
    harness.config.as_ref().is_some_and(|config| config.combat)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Called with the full world snapshot the host just sent, and with the full
/// snapshot a client just rebuilt from a keyframe or keyframe + delta.
pub(super) fn record_snapshot(
    harness: &mut OnlineVerificationHarness,
    snapshot: &OnlineWorldSnapshot,
) {
    if !enabled(harness) {
        return;
    }
    let combat = &mut harness.combat;
    for event in &snapshot.transient_events {
        if !combat.seen_event_ids.insert(event.id) {
            continue;
        }
        match &event.kind {
            OnlineTransientEventKind::ShotPulse {
                weapon: Some(_), ..
            } => combat.weapon_shots += 1,
            OnlineTransientEventKind::UnitDestruction { .. } => combat.unit_deaths += 1,
            OnlineTransientEventKind::ImpactBurst { radius, .. }
                if *radius >= STRIKE_BURST_MIN_RADIUS =>
            {
                combat.strikes += 1;
            }
            OnlineTransientEventKind::StructureDestruction { .. } => {
                combat.structure_destructions += 1;
            }
            _ => {}
        }
    }
    let state = (
        &snapshot.entities,
        &snapshot.economies,
        &snapshot.build_queue,
        &snapshot.support_cooldowns,
    );
    if let Ok(bytes) = postcard::to_allocvec(&state) {
        combat.digests.insert(
            snapshot.tick,
            format!("{}:{:016x}", snapshot.entities.len(), fnv1a(&bytes)),
        );
        while combat.digests.len() > DIGEST_HISTORY {
            combat.digests.pop_first();
        }
    }
}

pub(super) fn report(harness: &OnlineVerificationHarness) -> serde_json::Value {
    let combat = &harness.combat;
    serde_json::json!({
        "events": {
            "weapon_shots": combat.weapon_shots,
            "unit_deaths": combat.unit_deaths,
            "strikes": combat.strikes,
            "structure_destructions": combat.structure_destructions,
        },
        "presented": {
            "muzzle_flashes": combat.presented_muzzle_flashes,
            "unit_deaths": combat.presented_unit_deaths,
            "wrecks": combat.presented_wrecks,
            "shockwaves": combat.presented_shockwaves,
        },
        "missing": combat.missing(),
        // Per-tick digests are only worth their size in the final report.
        "digests": harness.stage.is_terminal().then(|| combat.digests.clone()),
    })
}

/// Counts what the presentation layer actually staged on this peer.
pub(super) fn tally_presentation(
    mut harness: ResMut<OnlineVerificationHarness>,
    shots: Query<(), Added<ShotWeaponClass>>,
    deaths: Query<(), Added<UnitDeathVfx>>,
    wrecks: Query<(), Added<CombatWreckage>>,
    shockwaves: Query<(), Added<ShockwaveRing>>,
) {
    if !enabled(&harness) {
        return;
    }
    let counts = [
        shots.iter().count(),
        deaths.iter().count(),
        wrecks.iter().count(),
        shockwaves.iter().count(),
    ]
    .map(|count| count as u32);
    if counts == [0; 4] {
        return;
    }
    let combat = &mut harness.combat;
    combat.presented_muzzle_flashes += counts[0];
    combat.presented_unit_deaths += counts[1];
    combat.presented_wrecks += counts[2];
    combat.presented_shockwaves += counts[3];
}

type CombatTarget<'a> = (
    Entity,
    &'a Team,
    &'a Transform,
    &'a mut Health,
    Option<&'a Unit>,
    Option<&'a Structure>,
);

/// Host only: once the client's move command has been observed, stage a fight
/// inside the remote player's base, then let the main harness end the match.
#[allow(clippy::too_many_arguments)]
pub(super) fn stage_host_combat(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_id: ResMut<NextSpawnId>,
    visible_player: Res<VisiblePlayer>,
    session: Res<OnlineSession>,
    mut harness: ResMut<OnlineVerificationHarness>,
    mut targets: Query<CombatTarget>,
) {
    if !enabled(&harness)
        || !session.is_host
        || session.phase != OnlinePhase::InMatch
        || !harness.command_observed
        || harness.combat.host_done
        || harness.stage.is_terminal()
    {
        return;
    }
    let (Some(local_player_id), Some(config)) =
        (session.local_player_id, session.match_config.as_ref())
    else {
        return;
    };
    let Some(local_team) = config.runtime_team_for_player(local_player_id) else {
        return;
    };
    let Some(enemy_team) =
        config
            .slots
            .iter()
            .enumerate()
            .find_map(|(index, slot)| match slot.occupant {
                OnlineSlotOccupant::Human { player_id, .. } if player_id != local_player_id => {
                    Some(Team::Player(index))
                }
                _ => None,
            })
    else {
        return;
    };
    let now = harness.elapsed;

    let Some(staged_at) = harness.combat.staged_at else {
        let victim = targets
            .iter()
            .filter(|(_, team, _, health, unit, _)| {
                **team == enemy_team && unit.is_some() && health.current > 0.0
            })
            .map(|(_, _, transform, ..)| transform.translation)
            .next();
        let outpost = targets
            .iter()
            .filter(|(_, team, _, health, _, structure)| {
                **team == enemy_team
                    && health.current > 0.0
                    && structure
                        .is_some_and(|structure| !is_structure_elimination_anchor(structure))
            })
            .map(|(_, _, transform, ..)| transform.translation)
            .next();
        let Some(victim) = victim else {
            return;
        };
        // Bare starts only have the headquarters, whose loss ends the match:
        // give the remote player an expendable building to lose first.
        let Some(outpost) = outpost else {
            let anchor = targets
                .iter()
                .filter(|(_, team, _, _, _, structure)| {
                    **team == enemy_team && structure.is_some_and(is_structure_elimination_anchor)
                })
                .map(|(_, _, transform, ..)| transform.translation)
                .next();
            if let Some(anchor) = anchor {
                spawn_structure(
                    &mut commands,
                    &asset_server,
                    &mut next_id,
                    "PowerReactor",
                    enemy_team,
                    visible_player.team,
                    Vec3::new(anchor.x + 7.0, 0.0, anchor.z),
                );
            }
            return;
        };
        // Armour parked beside an enemy building opens fire on its own.
        for offset in [Vec3::new(4.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 1.5)] {
            spawn_unit(
                &mut commands,
                &asset_server,
                &mut next_id,
                "RailgunTank",
                local_team,
                outpost + offset,
                0,
                visible_player.team,
            );
        }
        // A real orbital strike on the remote player's units.
        let strike = SupportPowerKind::OrbitalStrike.definition();
        commands.spawn((
            Transform::from_translation(victim + Vec3::Y * 0.03),
            PendingOrbitalStrike {
                remaining: STRIKE_DELAY_SECONDS,
                radius: strike.radius,
                damage: strike.damage,
                impact_scale: strike.damage_scale,
                team: local_team,
            },
            SupportWarning {
                remaining: STRIKE_DELAY_SECONDS,
                radius: strike.radius,
                color: Color::srgba(1.0, 0.72, 0.22, 0.55),
            },
            MatchScopedEntity,
        ));
        harness.combat.staged_at = Some(now);
        return;
    };

    if !harness.combat.collapsed && now - staged_at >= COLLAPSE_AFTER_SECONDS {
        // Guarantee one unit death and one building collapse mid-match, whatever
        // the strike and the tanks managed on their own.
        let mut unit_done = false;
        let mut structure_done = false;
        for (_, team, _, mut health, unit, structure) in &mut targets {
            if *team != enemy_team || health.current <= 0.0 {
                continue;
            }
            if unit.is_some() && structure.is_none() && !unit_done {
                health.current = 0.0;
                unit_done = true;
            } else if structure.is_some_and(|structure| !is_structure_elimination_anchor(structure))
                && !structure_done
            {
                health.current = 0.0;
                structure_done = true;
            }
        }
        harness.combat.collapsed = true;
    }

    if now - staged_at >= FINISH_AFTER_SECONDS {
        let missing = harness.combat.missing();
        if missing.is_empty() {
            harness.combat.host_done = true;
        } else if now - staged_at >= EVIDENCE_TIMEOUT_SECONDS {
            harness.fail(format!(
                "host combat evidence missing: {}",
                missing.join(", ")
            ));
        }
    }
}
