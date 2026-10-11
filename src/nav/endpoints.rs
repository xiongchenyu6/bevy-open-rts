//! Physical destinations for terrain movement orders.
use super::*;

/// Resolve ground movement destinations against the same physical circles used
/// by movement. Formation offsets and restored/queued orders can place an
/// individual destination inside a building even when the clicked pivot is clear.
pub(crate) fn resolve_ground_move_endpoints(
    grid: Res<NavGrid>,
    bounds: Res<MapBounds>,
    relations: Res<TeamRelations>,
    obstacles: Query<
        (&Transform, &Selectable, Option<&Health>),
        (
            Or<(With<Structure>, With<ResourceNode>, With<TerrainWall>)>,
            Without<Unit>,
        ),
    >,
    mut movers: Query<
        (
            Entity,
            &MovementDomain,
            &Selectable,
            &mut MoveOrder,
            &Team,
            &Health,
            Option<&mut AttackMoveOrder>,
            Option<&mut PatrolOrder>,
        ),
        (
            With<Unit>,
            Without<AttackOrder>,
            Without<FollowOrder>,
            Without<HarvestOrder>,
            Without<RepairOrder>,
            Without<ConstructOrder>,
            Without<CaptureOrder>,
            Without<GarrisonOrder>,
        ),
    >,
) {
    let obstacles = obstacles
        .iter()
        .filter(|(_, _, health)| !health.is_some_and(|health| health.current <= 0.0))
        .map(|(transform, selectable, _)| (transform.translation, selectable.radius))
        .collect::<Vec<_>>();
    let mut destinations = movers
        .iter()
        .filter(|(_, domain, _, _, _, health, ..)| {
            **domain == MovementDomain::Terrain && health.current > 0.0
        })
        .map(|(entity, _, selectable, order, team, ..)| {
            (entity, order.target, selectable.radius, *team)
        })
        .collect::<Vec<_>>();
    for index in 0..destinations.len() {
        let (entity, target, radius, team) = destinations[index];
        if ground_endpoint_clear(target, radius, *bounds, &obstacles, 0.0) {
            continue;
        }
        let reserved = destinations
            .iter()
            .enumerate()
            .filter(|(other, (_, _, _, other_team))| {
                *other != index && relations.are_allied(team, *other_team)
            })
            .map(|(_, (_, point, radius, _))| (*point, *radius))
            .collect::<Vec<_>>();
        let Some(resolved) =
            nearest_ground_endpoint(target, radius, *bounds, &grid, &obstacles, &reserved)
        else {
            continue;
        };
        let Ok((_, _, _, mut order, _, _, attack_move, patrol)) = movers.get_mut(entity) else {
            continue;
        };
        order.target = resolved;
        if let Some(mut attack_move) = attack_move {
            attack_move.destination = resolved;
        }
        if let Some(mut patrol) = patrol {
            if patrol.moving_to_destination {
                patrol.destination = resolved;
            } else {
                patrol.origin = resolved;
            }
        }
        destinations[index].1 = resolved;
    }
}

fn ground_endpoint_clear(
    point: Vec3,
    radius: f32,
    bounds: MapBounds,
    obstacles: &[(Vec3, f32)],
    margin: f32,
) -> bool {
    xz_distance(point, bounds.clamp_ground_point(point, radius)) <= 1e-4
        && obstacles.iter().all(|(position, obstacle_radius)| {
            xz_distance(point, *position) + 1e-4 >= radius + obstacle_radius + margin
        })
}

fn nearest_ground_endpoint(
    target: Vec3,
    radius: f32,
    bounds: MapBounds,
    grid: &NavGrid,
    obstacles: &[(Vec3, f32)],
    reserved: &[(Vec3, f32)],
) -> Option<Vec3> {
    // Search existing navigation cells only when the exact endpoint is illegal.
    // Keep displaced formation members clear of the other destination discs.
    (0..grid.height)
        .flat_map(|z| (0..grid.width).map(move |x| (x, z)))
        .filter(|(x, z)| !grid.is_blocked(*x, *z))
        .map(|(x, z)| grid.cell_center(x, z).with_y(target.y))
        .filter(|point| {
            ground_endpoint_clear(
                *point,
                radius,
                bounds,
                obstacles,
                MOVEMENT_OBSTACLE_CLEARANCE_M,
            )
        })
        .filter(|point| {
            reserved.iter().all(|(other, other_radius)| {
                xz_distance(*point, *other) + 1e-4 >= radius + other_radius + UNIT_SEPARATION_GAP_M
            })
        })
        .min_by(|a, b| xz_distance_squared(*a, target).total_cmp(&xz_distance_squared(*b, target)))
}

#[cfg(test)]
mod endpoint_resolution_tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        let bounds = MapBounds::from_size((10.0, 10.0));
        let mut grid = NavGrid::default();
        grid.rebuild_with_terrain(
            bounds,
            &[(Vec3::ZERO, 1.45)],
            &TerrainHeightField::default(),
        );
        app.insert_resource(TeamRelations::default())
            .insert_resource(bounds)
            .insert_resource(grid)
            .add_systems(Update, resolve_ground_move_endpoints);
        app.world_mut().spawn((
            Structure { id: "PowerReactor" },
            Transform::default(),
            Selectable { radius: 1.45 },
            Health::new(12.0),
        ));
        app
    }

    fn mover(app: &mut App, point: Vec3, radius: f32, domain: MovementDomain) -> Entity {
        app.world_mut()
            .spawn((
                Unit {
                    id: "LightRifleInfantry",
                    speed: 5.0,
                    can_crush: false,
                    can_be_crushed: true,
                },
                domain,
                Team::Player(0),
                Health::new(4.0),
                Selectable { radius },
                MoveOrder { target: point },
            ))
            .id()
    }

    #[test]
    fn blocked_multi_member_goals_use_real_radii_and_keep_destination_separation() {
        let mut app = app();
        let members = [
            (Vec3::ZERO, 0.42),
            (Vec3::X * 0.3, 0.9),
            (Vec3::Z * 0.3, 0.6),
        ]
        .map(|(point, radius)| {
            (
                mover(&mut app, point, radius, MovementDomain::Terrain),
                radius,
            )
        });
        app.update();
        let targets = members
            .map(|(entity, radius)| (app.world().get::<MoveOrder>(entity).unwrap().target, radius));
        for (i, (point, radius)) in targets.iter().enumerate() {
            assert!(
                xz_distance(*point, Vec3::ZERO)
                    >= 1.45 + radius + MOVEMENT_OBSTACLE_CLEARANCE_M - 1e-4
            );
            for (other, other_radius) in targets.iter().skip(i + 1) {
                assert!(
                    xz_distance(*point, *other)
                        >= radius + other_radius + UNIT_SEPARATION_GAP_M - 1e-4
                );
            }
        }
    }

    #[test]
    fn clear_and_air_endpoints_are_preserved_and_boundaries_respect_unit_radius() {
        let mut app = app();
        let clear = Vec3::new(-1.9, 0.0, 0.0);
        let terrain = mover(&mut app, clear, 0.42, MovementDomain::Terrain);
        let air = mover(&mut app, Vec3::ZERO, 0.75, MovementDomain::Air);
        let boundary = mover(
            &mut app,
            Vec3::new(5.0, 0.0, 5.0),
            0.9,
            MovementDomain::Terrain,
        );
        app.update();
        assert_eq!(app.world().get::<MoveOrder>(terrain).unwrap().target, clear);
        assert_eq!(
            app.world().get::<MoveOrder>(air).unwrap().target,
            Vec3::ZERO
        );
        let actual = app.world().get::<MoveOrder>(boundary).unwrap().target;
        assert_eq!(
            app.world()
                .resource::<MapBounds>()
                .clamp_ground_point(actual, 0.9),
            actual
        );
    }

    #[test]
    fn chase_contact_is_preserved_and_attack_move_and_patrol_keep_resolved_goals() {
        let mut app = app();
        let chase = mover(&mut app, Vec3::ZERO, 0.42, MovementDomain::Terrain);
        app.world_mut().entity_mut(chase).insert(AttackOrder {
            target: Entity::PLACEHOLDER,
        });
        let attack_move = mover(&mut app, Vec3::ZERO, 0.42, MovementDomain::Terrain);
        app.world_mut()
            .entity_mut(attack_move)
            .insert(AttackMoveOrder {
                destination: Vec3::ZERO,
            });
        let patrol = mover(&mut app, Vec3::X * 0.3, 0.42, MovementDomain::Terrain);
        app.world_mut().entity_mut(patrol).insert(PatrolOrder {
            origin: Vec3::new(-4.0, 0.0, 0.0),
            destination: Vec3::X * 0.3,
            moving_to_destination: true,
        });
        app.update();
        assert_eq!(
            app.world().get::<MoveOrder>(chase).unwrap().target,
            Vec3::ZERO
        );
        assert_eq!(
            app.world()
                .get::<AttackMoveOrder>(attack_move)
                .unwrap()
                .destination,
            app.world().get::<MoveOrder>(attack_move).unwrap().target
        );
        assert_eq!(
            app.world().get::<PatrolOrder>(patrol).unwrap().destination,
            app.world().get::<MoveOrder>(patrol).unwrap().target
        );
        assert_eq!(
            app.world().get::<PatrolOrder>(patrol).unwrap().origin,
            Vec3::new(-4.0, 0.0, 0.0)
        );
    }
    #[test]
    fn enemy_and_dead_destinations_do_not_divert_ground_arrivals() {
        let blocked = Vec3::new(-0.6, 0.0, -0.9);
        let mut baseline = app();
        let unit = mover(&mut baseline, blocked, 0.42, MovementDomain::Terrain);
        baseline.update();
        let nearest = baseline.world().get::<MoveOrder>(unit).unwrap().target;
        assert!(xz_distance(nearest, blocked) > 0.0);

        let mut hostile = app();
        let unit = mover(&mut hostile, blocked, 0.42, MovementDomain::Terrain);
        let enemy = mover(&mut hostile, nearest, 0.9, MovementDomain::Terrain);
        hostile
            .world_mut()
            .entity_mut(enemy)
            .insert(Team::Player(1));
        let dead = mover(&mut hostile, nearest, 0.9, MovementDomain::Terrain);
        hostile.world_mut().get_mut::<Health>(dead).unwrap().current = 0.0;
        hostile.update();
        assert_eq!(
            hostile.world().get::<MoveOrder>(unit).unwrap().target,
            nearest,
            "Enemy and dead-unit destinations must not divert this arrival"
        );

        let mut allied = app();
        let unit = mover(&mut allied, blocked, 0.42, MovementDomain::Terrain);
        mover(&mut allied, nearest, 0.9, MovementDomain::Terrain);
        allied.update();
        let reserved = allied.world().get::<MoveOrder>(unit).unwrap().target;
        assert!(
            xz_distance(reserved, nearest) >= 0.42 + 0.9 + UNIT_SEPARATION_GAP_M - 1e-4,
            "Living allied arrivals retain their physical separation"
        );
    }
}
