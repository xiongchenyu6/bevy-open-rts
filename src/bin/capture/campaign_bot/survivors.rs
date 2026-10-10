//! Continue annihilation missions through normal army input after HQ loss.
use super::*;

pub(super) fn run(
    app: &mut App,
    watch: &mut SetupWatch,
    remaining_rounds: usize,
) -> Result<(), String> {
    let state = capture_campaign_state(app);
    let x = state.half_width * 0.7;
    let z = state.half_depth * 0.7;
    let route = [
        Vec3::new(x, 0.0, z),
        Vec3::new(-x, 0.0, z),
        Vec3::new(-x, 0.0, -z),
        Vec3::new(x, 0.0, -z),
        Vec3::ZERO,
    ];
    let mut leg = 0;
    println!(
        "[capture] HQ lost while match remains Running; continuing existing army, remaining rounds={remaining_rounds}"
    );
    for _ in 0..remaining_rounds {
        if capture_match_phase_label(app) != "Running" {
            break;
        }
        // Ctrl+Alt+A is the player's select-all-army command. No Selected,
        // group, resource, health, fog or actor state is written by this runner.
        capture_key(app, KeyCode::ShiftLeft, false);
        capture_key(app, KeyCode::ControlLeft, true);
        capture_key(app, KeyCode::AltLeft, true);
        tap_key(app, KeyCode::KeyA);
        capture_key(app, KeyCode::AltLeft, false);
        capture_key(app, KeyCode::ControlLeft, false);
        running(app)?;
        let units = capture_campaign_units(app);
        let expected: HashSet<_> = units
            .iter()
            .filter(|u| u.id != "Worker")
            .map(|u| u.entity)
            .collect();
        let actual: HashSet<_> = units
            .iter()
            .filter(|u| u.selected)
            .map(|u| u.entity)
            .collect();
        if actual != expected {
            return Err(format!(
                "survivor army selection mismatch expected={expected:?} actual={actual:?}"
            ));
        }
        if !actual.is_empty() {
            let center = units
                .iter()
                .filter(|u| u.selected)
                .map(|u| u.position)
                .sum::<Vec3>()
                / actual.len() as f32;
            let contacts = visible_enemy_contacts(app)?;
            let target = if let Some(contact) = contacts.iter().min_by(|a, b| {
                a.position
                    .distance_squared(center)
                    .total_cmp(&b.position.distance_squared(center))
            }) {
                println!(
                    "[capture] survivor visible enemy {} at {:?}",
                    contact.id, contact.position
                );
                contact.position
            } else {
                if center.distance(route[leg % route.len()]) < 6.0 {
                    leg += 1;
                }
                route[leg % route.len()]
            };
            ground_order(app, target, true)?;
        }
        log_state(app, &mut watch.trace, "survivor-assault")?;
        watch.advance(app, 150);
    }
    let phase = capture_match_phase_label(app);
    if phase != "HumanVictory" {
        return Err(format!("surviving army resolved as {phase}"));
    }
    log_state(app, &mut watch.trace, "result")
}
