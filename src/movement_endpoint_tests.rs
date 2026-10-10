//! Measured endpoint geometry fixture; this is not paid gameplay acceptance.
use super::*;

const ENDPOINT: &str = r#"[[85899343417,"AntiAirWalker","Player(0)",[-18.661571502685547,0.05000000074505806,-12.987637519836426],0.75,9.0,null],[597000452253,"AntiAirWalker","Player(0)",[-14.677107810974121,0.05000000074505806,-21.266326904296875],0.75,9.0,null],[124554049481,"Tank","Player(0)",[-13.00489330291748,0.05000000074505806,-21.843130111694336],0.8999999761581421,10.0,null],[141733919029,"Tank","Player(0)",[-28.077659606933594,0.05000000074505806,-18.399391174316406],0.8999999761581421,10.0,null],[231928232475,"Tank","Player(0)",[-20.530014038085938,0.05000000074505806,-11.530892372131348],0.8999999761581421,10.0,null],[4294965817,"PowerReactor","Player(0)",[-26.0,0.0,-19.500001907348633],1.4500000476837158,12.0,null],[12884900087,"Refinery","Player(1)",[23.22801971435547,0.0,-17.153148651123047],1.5,14.0,null],[64424507914,"PowerReactor","Player(1)",[26.971708297729492,0.0,-22.531139373779297],1.4500000476837158,12.0,null],[8589932612,"RadarUplink","Player(0)",[-22.000001907348633,0.0,-28.0],1.399999976158142,12.0,null],[429496728004,"RadarUplink","Player(1)",[24.007646560668945,0.0,-26.57923126220703],1.399999976158142,12.0,null],[562640713672,"RoboticsBay","Player(1)",[25.555601119995117,0.0,-25.515352249145508],1.5499999523162842,14.0,null],[390842021471,"PowerReactor","Player(1)",[26.152584075927734,0.0,-24.784969329833984],1.4500000476837158,12.0,null],[25769801127,"PowerReactor","Player(0)",[-26.5,0.0,-25.5],1.4500000476837158,12.0,null],[261993002448,"TechLab","Player(1)",[26.601774215698242,0.0,-23.955421447753906],1.600000023841858,16.0,null],[652835026510,"OrePurifier","Player(1)",[26.887157440185547,0.0,-23.0562686920166],1.600000023841858,16.0,null],[124554048597,"PowerReactor","Player(1)",[17.0124454498291,0.0,-22.352561950683594],1.4500000476837158,12.0,null],[446676596110,"Worker","Player(0)",[-17.78398895263672,0.05000000074505806,-22.163963317871094],0.6000000238418579,6.0,null],[4294966766,"Worker","Player(0)",[-16.96442985534668,0.05000000074505806,-21.273059844970703],0.6000000238418579,6.0,null],[4294966769,"Worker","Player(0)",[-19.446550369262695,0.05000000074505806,-18.03363800048828],0.6000000238418579,6.0,null],[274877904932,"RocketInfantry","Player(0)",[-16.117345809936523,0.05000000074505806,-13.662184715270996],0.44999998807907104,5.0,null],[257698035695,"RocketInfantry","Player(0)",[-21.11931037902832,0.05000000074505806,-12.800824165344238],0.44999998807907104,5.0,null],[575525614879,"HeavyMachinegunTrooper","Player(0)",[-16.13368034362793,0.05000000074505806,-20.507062911987305],0.47999998927116394,7.0,null],[184683591437,"HeavyMachinegunTrooper","Player(0)",[-17.253896713256836,0.05000000074505806,-14.202048301696777],0.47999998927116394,7.0,null],[455266531233,"HeavyMachinegunTrooper","Player(0)",[-20.311201095581055,0.05000000074505806,-13.355224609375],0.47999998927116394,7.0,null],[141733918548,"RocketInfantry","Player(0)",[-13.709671020507812,0.05000000074505806,-15.897300720214844],0.44999998807907104,5.0,null],[4294966742,"Worker","Player(1)",[25.231542587280273,0.05000000074505806,-17.788570404052734],0.6000000238418579,6.0,null],[244813132745,"Tank","Player(0)",[5.7337236404418945,0.05000000074505806,-26.19712257385254],0.8999999761581421,10.0,null],[2005749724971,"Tank","Player(0)",[6.632181644439697,0.05000000074505806,-24.57994270324707],0.8999999761581421,10.0,null],[68719474045,"Tank","Player(0)",[6.260414123535156,0.05000000074505806,-22.767683029174805],0.8999999761581421,10.0,null],[330712479260,"Tank","Player(0)",[8.209983825683594,0.05000000074505806,-22.264205932617188],0.8999999761581421,10.0,null],[579820582773,"Tank","Player(0)",[8.635043144226074,0.05000000074505806,-20.463701248168945],0.8999999761581421,15.0,null],[38654703350,"Tank","Player(0)",[6.717179298400879,0.05000000074505806,-27.764066696166992],0.8999999761581421,10.0,null],[566935680639,"Tank","Player(0)",[6.776762962341309,0.05000000074505806,-20.99120330810547],0.8999999761581421,10.0,null],[42949670791,"Tank","Player(0)",[8.419459342956543,0.05000000074505806,-24.102306365966797],0.8999999761581421,10.0,null],[4294966711,"wall",null,[0.4400000274181366,0.0,-22.0],1.100000023841858,null,null],[4294966710,"wall",null,[-0.18906250596046448,0.0,-20.545454025268555],1.100000023841858,null,null],[4294966709,"wall",null,[1.196250081062317,0.0,-19.090909957885742],1.100000023841858,null,null],[4294966708,"wall",null,[0.006875000428408384,0.0,-17.636363983154297],1.100000023841858,null,null],[4294966707,"wall",null,[-0.17875000834465027,0.0,-16.18181800842285],1.100000023841858,null,null],[4294966706,"wall",null,[1.0656250715255737,0.0,-14.727272033691406],1.100000023841858,null,null],[4294966705,"wall",null,[-0.38671875,0.0,-13.272727966308594],1.100000023841858,null,null],[4294966704,"wall",null,[-0.4039062559604645,0.0,-11.818181991577148],1.100000023841858,null,null],[4294966703,"wall",null,[0.8525000810623169,0.0,-10.363636016845703],1.100000023841858,null,null],[4294966702,"wall",null,[0.39531251788139343,0.0,-8.909090995788574],1.100000023841858,null,null],[4294966701,"wall",null,[-0.28660157322883606,0.0,-7.454545021057129],1.100000023841858,null,null],[4294966700,"wall",null,[1.1687500476837158,0.0,-6.0],1.100000023841858,null,null],[4294966699,"wall",null,[0.955625057220459,0.0,6.0],1.100000023841858,null,null],[4294966698,"wall",null,[-0.2200000137090683,0.0,7.454545021057129],1.100000023841858,null,null],[4294966697,"wall",null,[1.142968773841858,0.0,8.909090995788574],1.100000023841858,null,null],[4294966696,"wall",null,[1.2220313549041748,0.0,10.363636016845703],1.100000023841858,null,null],[4294966695,"wall",null,[-0.27156251668930054,0.0,11.818181991577148],1.100000023841858,null,null],[4294966694,"wall",null,[0.5465625524520874,0.0,13.272727012634277],1.100000023841858,null,null],[4294966693,"wall",null,[1.0553125143051147,0.0,14.727272033691406],1.100000023841858,null,null],[4294966692,"wall",null,[0.288750022649765,0.0,16.18181800842285],1.100000023841858,null,null],[4294966691,"wall",null,[0.4623437523841858,0.0,17.636363983154297],1.100000023841858,null,null],[4294966690,"wall",null,[0.599414050579071,0.0,19.090909957885742],1.100000023841858,null,null],[4294966689,"wall",null,[0.43656250834465027,0.0,20.545454025268555],1.100000023841858,null,null],[4294966688,"wall",null,[0.7184375524520874,0.0,22.0],1.100000023841858,null,null],[4294966687,"wall",null,[-22.0,0.0,-1.203125],1.100000023841858,null,null],[4294966686,"wall",null,[-20.545454025268555,0.0,-0.19164063036441803],1.100000023841858,null,null],[4294966685,"wall",null,[-19.090909957885742,0.0,-0.21828125417232513],1.100000023841858,null,null],[4294966684,"wall",null,[-17.636363983154297,0.0,-1.299375057220459],1.100000023841858,null,null],[4294966683,"wall",null,[-16.18181800842285,0.0,0.21484375],1.100000023841858,null,null],[4294966682,"wall",null,[-14.727272033691406,0.0,-0.1667187660932541],1.100000023841858,null,null],[4294966681,"wall",null,[-13.272727966308594,0.0,-0.4503125250339508],1.100000023841858,null,null],[4294966680,"wall",null,[-11.818181991577148,0.0,0.2853125035762787],1.100000023841858,null,null],[4294966679,"wall",null,[-10.363636016845703,0.0,-0.27156251668930054],1.100000023841858,null,null],[4294966678,"wall",null,[-8.909090995788574,0.0,-0.9693750739097595],1.100000023841858,null,null],[4294966677,"wall",null,[-7.454545021057129,0.0,-0.2956250011920929],1.100000023841858,null,null],[4294966676,"wall",null,[-6.0,0.0,-0.6930859684944153],1.100000023841858,null,null],[4294966675,"wall",null,[6.0,0.0,-1.2478126287460327],1.100000023841858,null,null],[4294966674,"wall",null,[7.454545021057129,0.0,-1.15500009059906],1.100000023841858,null,null],[4294966673,"wall",null,[8.909090995788574,0.0,-0.25437501072883606],1.100000023841858,null,null],[4294966672,"wall",null,[10.363636016845703,0.0,-1.1309374570846558],1.100000023841858,null,null],[4294966671,"wall",null,[11.818181991577148,0.0,-1.16015625],1.100000023841858,null,null],[4294966670,"wall",null,[13.272727012634277,0.0,0.43656250834465027],1.100000023841858,null,null],[4294966669,"wall",null,[14.727272033691406,0.0,-0.8146875500679016],1.100000023841858,null,null],[4294966668,"wall",null,[16.18181800842285,0.0,-1.2147265672683716],1.100000023841858,null,null],[4294966667,"wall",null,[17.636363983154297,0.0,0.11687500774860382],1.100000023841858,null,null],[4294966666,"wall",null,[19.090909957885742,0.0,-0.7665625214576721],1.100000023841858,null,null],[4294966665,"wall",null,[20.545454025268555,0.0,-0.107421875],1.100000023841858,null,null],[4294966664,"wall",null,[22.0,0.0,-0.35234376788139343],1.100000023841858,null,null],[253403068506,"AntiGroundTurret","Player(0)",[-16.0,0.0,-19.0],0.6000000238418579,8.0,null],[614180321303,"AntiAirTurret","Player(0)",[-16.0,0.0,-22.0],0.6000000238418579,8.0,null],[682899797592,"AntiGroundTurret","Player(0)",[-19.000001907348633,0.0,-28.0],0.6000000238418579,8.0,null],[115964114897,"ScoutRover","Player(1)",[16.12732696533203,0.05000000074505806,-24.197481155395508],0.550000011920929,5.0,null],[532575941713,"ScoutRover","Player(1)",[16.2589168548584,0.05000000074505806,-24.01776695251465],0.550000011920929,5.0,null],[1932735280514,"ScoutRover","Player(1)",[16.501480102539062,0.05000000074505806,-23.56981086730957],0.550000011920929,5.0,null],[4294966771,"Drone","Player(0)",[-24.0,1.5,-24.0],0.6000000238418579,6.0,null],[382252086596,"RocketInfantry","Player(0)",[8.708616256713867,0.05000000074505806,-25.472118377685547],0.44999998807907104,5.0,null],[90194310271,"RocketInfantry","Player(0)",[7.4649786949157715,0.05000000074505806,-25.756553649902344],0.44999998807907104,6.0,null],[794568947205,"Worker","Player(0)",[-23.26566505432129,0.05000000074505806,-18.241825103759766],0.6000000238418579,6.0,null],[4294966786,"CommandCenter","Player(0)",[-22.0,0.0,-22.0],2.0,20.0,null],[4294966763,"CommandCenter","Player(1)",[22.0,0.0,-22.0],2.0,20.0,null],[12884900007,"VehicleFactory","Player(0)",[-28.0,0.0,-16.0],1.5,16.0,null],[51539605481,"VehicleFactory","Player(1)",[14.016521453857422,0.0,-22.513883590698242],1.5,16.0,null],[64424507425,"Barracks","Player(0)",[-18.000001907348633,0.0,-19.5],1.4500000476837158,13.0,null],[356482282765,"Barracks","Player(1)",[13.346650123596191,0.0,-28.602548599243164],1.4500000476837158,13.0,null],[790273979703,"AircraftFactory","Player(1)",[14.789432525634766,0.0,-30.53508949279785],1.5,16.0,null],[51539605769,"Worker","Player(1)",[22.351289749145508,0.05000000074505806,-15.225475311279297],0.6000000238418579,6.0,null],[4294966745,"Worker","Player(1)",[18.2635440826416,0.05000000074505806,-24.031599044799805],0.6000000238418579,6.0,null],[4294966723,"resource","Neutral",[24.0,0.0,16.0],0.6000000238418579,null,140],[4294966729,"resource","Neutral",[24.0,0.0,-16.0],0.6000000238418579,null,137],[4294966733,"resource","Neutral",[16.0,0.0,-24.0],0.6000000238418579,null,138],[4294966731,"resource","Neutral",[14.400001525878906,0.0,-22.600000381469727],0.6000000238418579,null,240],[4294966727,"resource","Neutral",[16.0,0.0,24.0],0.6000000238418579,null,240],[4294966725,"resource","Neutral",[14.400001525878906,0.0,22.599998474121094],0.6000000238418579,null,240],[4294966721,"resource","Neutral",[-16.0,0.0,24.0],0.6000000238418579,null,240],[4294966719,"resource","Neutral",[-14.399999618530273,0.0,22.599998474121094],0.6000000238418579,null,240],[4294966715,"resource","Neutral",[0.0,0.0,-2.0],0.6000000238418579,null,235],[4294966713,"resource","Neutral",[0.0,0.0,2.0],0.6000000238418579,null,240],[4294966717,"resource","Neutral",[-24.0,0.0,16.0],0.6000000238418579,null,140]]"#;

fn measured_app(endpoint: &str) -> (App, std::collections::BTreeMap<u64, Entity>) {
    let mut app = build_game_app(GameAppMode::Headless);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 30.0),
    ));
    {
        let mut setup = app.world_mut().resource_mut::<MatchSetupSettings>();
        setup
            .player_controllers
            .fill(SkirmishPlayerController::Human);
        setup.player_factions.fill(SkirmishFaction::Alliance);
        setup.map_path = SKIRMISH_MAPS
            .iter()
            .find(|map| map.id == "crossfire")
            .unwrap()
            .godot_path;
    }
    app.world_mut()
        .resource_mut::<NextState<AppScreen>>()
        .set(AppScreen::InMatch);
    for _ in 0..8 {
        app.update();
    }
    let entities: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, (With<MatchScopedEntity>, With<Selectable>)>()
        .iter(app.world())
        .collect();
    for entity in entities {
        app.world_mut().despawn(entity);
    }
    *app.world_mut().resource_mut::<TerrainHeightField>() = TerrainHeightField::default();
    assert_eq!(app.world().resource::<MapBounds>().half_width, 32.0);
    assert_eq!(app.world().resource::<MapBounds>().half_depth, 32.0);
    let rows: Vec<(
        u64,
        String,
        Option<String>,
        [f32; 3],
        f32,
        Option<f32>,
        Option<i32>,
    )> = serde_json::from_str(endpoint).unwrap();
    let mut keys = std::collections::BTreeMap::new();
    for (key, id, team, position, radius, health, resource) in rows {
        let team = if team.as_deref() == Some("Player(0)") {
            Team::Player(0)
        } else {
            Team::Player(1)
        };
        let mut actor = app.world_mut().spawn((
            MatchScopedEntity,
            Transform::from_translation(Vec3::from_array(position)),
            Selectable { radius },
            team,
        ));
        if id == "wall" {
            actor.insert(TerrainWall);
        } else if id == "resource" {
            actor.insert(ResourceNode {
                kind: ResourceKind::Ore,
                amount: resource.unwrap(),
            });
        } else {
            let def = registry::entity(&id).unwrap();
            actor.insert((Health::new(health.unwrap()), MovementDomain::Terrain));
            if matches!(def.role, registry::EntityRole::Unit) {
                actor.insert(Unit {
                    id: def.id,
                    speed: def.speed,
                    can_crush: def.can_crush,
                    can_be_crushed: def.can_be_crushed,
                });
                if def.domain == registry::MoveDomain::Air {
                    actor.insert(MovementDomain::Air);
                }
            } else {
                actor.insert(Structure { id: def.id });
            }
        }
        keys.insert(key, actor.id());
    }
    (app, keys)
}

#[test]
fn measured_crossfire_endpoint_tanks_progress_in_real_schedule() {
    let (mut app, keys) = measured_app(ENDPOINT);
    let measured = [
        (
            244813132745u64,
            356482282765u64,
            Vec3::new(12.25, 0.0, -30.25),
        ),
        (
            2005749724971u64,
            51539605481u64,
            Vec3::new(14.75, 0.0, -24.75),
        ),
    ];
    let def = registry::entity("Tank").unwrap();
    let weapon = def.weapon.unwrap();
    let starts: Vec<_> = measured
        .iter()
        .map(|(key, _, _)| app.world().get::<Transform>(keys[key]).unwrap().translation)
        .collect();
    for (key, target, waypoint) in measured {
        let goal = app
            .world()
            .get::<Transform>(keys[&target])
            .unwrap()
            .translation;
        app.world_mut().entity_mut(keys[&key]).insert((
            MoveOrder { target: goal },
            AttackOrder {
                target: keys[&target],
            },
            PlannedPath {
                goal,
                waypoints: vec![waypoint],
                next: 0,
            },
            HoldPosition { enabled: false },
            VisionRadius(def.sight_range),
            Weapon::new(
                weapon.range,
                weapon.damage,
                weapon.cooldown,
                weapon.splash_radius,
                weapon.splash_damage_multiplier,
                weapon.structure_damage_multiplier,
                weapon.can_attack_air,
                weapon.can_attack_ground,
            ),
        ));
    }
    for tick in 0..30 {
        app.update();
        if tick == 0 || tick == 29 {
            for (key, _, _) in measured {
                println!(
                    "endpoint fixture tick{tick} key{key} position {:?} speed {} delta {}",
                    app.world()
                        .get::<Transform>(keys[&key])
                        .unwrap()
                        .translation,
                    app.world().get::<Unit>(keys[&key]).unwrap().speed,
                    app.world().resource::<Time>().delta_secs()
                );
            }
        }
    }
    for (index, (key, _, _)) in measured.iter().enumerate() {
        let end = app.world().get::<Transform>(keys[key]).unwrap().translation;
        assert!(
            xz_distance(starts[index], end) > 1.0,
            "measured Tank failed to progress: start {:?}, end {:?}",
            starts[index],
            end
        );
    }
}

const CONSTRUCTION_ENDPOINT: &str = r#"[[502511171143,"PowerReactor","Player(0)",[-26.5,0.0,-25.5],1.4500000476837158,1.0,null],[4294966786,"CommandCenter","Player(0)",[-22.0,0.0,-22.0],2.0,20.0,null],[4294966763,"CommandCenter","Player(1)",[22.0,0.0,-22.0],2.0,20.0,null],[12884900006,"VehicleFactory","Player(0)",[-28.0,0.0,-16.0],1.5,16.0,null],[167503722572,"VehicleFactory","Player(1)",[14.26703929901123,0.0,-26.40582847595215],1.5,16.0,null],[64424507452,"Barracks","Player(0)",[-18.000001907348633,0.0,-19.5],1.4500000476837158,13.0,null],[111669147841,"Tank","Player(0)",[-28.094417572021484,0.05000000074505806,-18.409860610961914],0.8999999761581421,10.0,null],[98784245621,"Tank","Player(0)",[-24.529062271118164,0.05000000074505806,-17.454702377319336],0.8999999761581421,10.0,null],[515396073129,"Tank","Player(0)",[-29.043148040771484,0.05000000074505806,-13.828981399536133],0.8999999761581421,10.0,null],[270582937007,"Tank","Player(0)",[-28.594188690185547,0.05000000074505806,-18.894807815551758],0.8999999761581421,10.0,null],[339302414454,"RocketInfantry","Player(0)",[-17.3015079498291,0.05000000074505806,-10.66445541381836],0.44999998807907104,5.0,null],[133143984050,"RocketInfantry","Player(0)",[-19.803842544555664,0.05000000074505806,-11.382020950317383],0.44999998807907104,5.0,null],[163208755060,"RocketInfantry","Player(0)",[-15.146588325500488,0.05000000074505806,-12.860942840576172],0.44999998807907104,5.0,null],[176093656818,"HeavyMachinegunTrooper","Player(0)",[-20.708114624023438,0.05000000074505806,-11.00427532196045],0.47999998927116394,7.0,null],[4294966748,"ShieldTrooper","Player(1)",[20.0,0.05000000074505806,-24.0],0.5,9.0,null],[4294966742,"Worker","Player(1)",[20.355274200439453,0.05000000074505806,-27.637361526489258],0.6000000238418579,6.0,null],[4294966711,"wall",null,[0.4400000274181366,0.0,-22.0],1.100000023841858,null,null],[4294966710,"wall",null,[-0.18906250596046448,0.0,-20.545454025268555],1.100000023841858,null,null],[4294966709,"wall",null,[1.196250081062317,0.0,-19.090909957885742],1.100000023841858,null,null],[4294966708,"wall",null,[0.006875000428408384,0.0,-17.636363983154297],1.100000023841858,null,null],[4294966707,"wall",null,[-0.17875000834465027,0.0,-16.18181800842285],1.100000023841858,null,null],[4294966706,"wall",null,[1.0656250715255737,0.0,-14.727272033691406],1.100000023841858,null,null],[4294966705,"wall",null,[-0.38671875,0.0,-13.272727966308594],1.100000023841858,null,null],[4294966704,"wall",null,[-0.4039062559604645,0.0,-11.818181991577148],1.100000023841858,null,null],[4294966703,"wall",null,[0.8525000810623169,0.0,-10.363636016845703],1.100000023841858,null,null],[4294966702,"wall",null,[0.39531251788139343,0.0,-8.909090995788574],1.100000023841858,null,null],[4294966701,"wall",null,[-0.28660157322883606,0.0,-7.454545021057129],1.100000023841858,null,null],[4294966700,"wall",null,[1.1687500476837158,0.0,-6.0],1.100000023841858,null,null],[4294966699,"wall",null,[0.955625057220459,0.0,6.0],1.100000023841858,null,null],[4294966698,"wall",null,[-0.2200000137090683,0.0,7.454545021057129],1.100000023841858,null,null],[4294966697,"wall",null,[1.142968773841858,0.0,8.909090995788574],1.100000023841858,null,null],[4294966696,"wall",null,[1.2220313549041748,0.0,10.363636016845703],1.100000023841858,null,null],[4294966695,"wall",null,[-0.27156251668930054,0.0,11.818181991577148],1.100000023841858,null,null],[4294966694,"wall",null,[0.5465625524520874,0.0,13.272727012634277],1.100000023841858,null,null],[4294966693,"wall",null,[1.0553125143051147,0.0,14.727272033691406],1.100000023841858,null,null],[4294966692,"wall",null,[0.288750022649765,0.0,16.18181800842285],1.100000023841858,null,null],[4294966691,"wall",null,[0.4623437523841858,0.0,17.636363983154297],1.100000023841858,null,null],[4294966690,"wall",null,[0.599414050579071,0.0,19.090909957885742],1.100000023841858,null,null],[4294966689,"wall",null,[0.43656250834465027,0.0,20.545454025268555],1.100000023841858,null,null],[4294966688,"wall",null,[0.7184375524520874,0.0,22.0],1.100000023841858,null,null],[4294966687,"wall",null,[-22.0,0.0,-1.203125],1.100000023841858,null,null],[4294966686,"wall",null,[-20.545454025268555,0.0,-0.19164063036441803],1.100000023841858,null,null],[4294966685,"wall",null,[-19.090909957885742,0.0,-0.21828125417232513],1.100000023841858,null,null],[4294966684,"wall",null,[-17.636363983154297,0.0,-1.299375057220459],1.100000023841858,null,null],[4294966683,"wall",null,[-16.18181800842285,0.0,0.21484375],1.100000023841858,null,null],[4294966682,"wall",null,[-14.727272033691406,0.0,-0.1667187660932541],1.100000023841858,null,null],[4294966681,"wall",null,[-13.272727966308594,0.0,-0.4503125250339508],1.100000023841858,null,null],[4294966680,"wall",null,[-11.818181991577148,0.0,0.2853125035762787],1.100000023841858,null,null],[4294966679,"wall",null,[-10.363636016845703,0.0,-0.27156251668930054],1.100000023841858,null,null],[4294966678,"wall",null,[-8.909090995788574,0.0,-0.9693750739097595],1.100000023841858,null,null],[4294966677,"wall",null,[-7.454545021057129,0.0,-0.2956250011920929],1.100000023841858,null,null],[4294966676,"wall",null,[-6.0,0.0,-0.6930859684944153],1.100000023841858,null,null],[4294966675,"wall",null,[6.0,0.0,-1.2478126287460327],1.100000023841858,null,null],[4294966674,"wall",null,[7.454545021057129,0.0,-1.15500009059906],1.100000023841858,null,null],[4294966673,"wall",null,[8.909090995788574,0.0,-0.25437501072883606],1.100000023841858,null,null],[4294966672,"wall",null,[10.363636016845703,0.0,-1.1309374570846558],1.100000023841858,null,null],[4294966671,"wall",null,[11.818181991577148,0.0,-1.16015625],1.100000023841858,null,null],[4294966670,"wall",null,[13.272727012634277,0.0,0.43656250834465027],1.100000023841858,null,null],[4294966669,"wall",null,[14.727272033691406,0.0,-0.8146875500679016],1.100000023841858,null,null],[4294966668,"wall",null,[16.18181800842285,0.0,-1.2147265672683716],1.100000023841858,null,null],[4294966667,"wall",null,[17.636363983154297,0.0,0.11687500774860382],1.100000023841858,null,null],[4294966666,"wall",null,[19.090909957885742,0.0,-0.7665625214576721],1.100000023841858,null,null],[4294966665,"wall",null,[20.545454025268555,0.0,-0.107421875],1.100000023841858,null,null],[4294966664,"wall",null,[22.0,0.0,-0.35234376788139343],1.100000023841858,null,null],[42949670808,"AntiGroundTurret","Player(0)",[-16.0,0.0,-19.0],0.6000000238418579,8.0,null],[300647708525,"AntiAirTurret","Player(0)",[-16.0,0.0,-22.0],0.6000000238418579,8.0,null],[4294966739,"resource","Neutral",[-16.0,0.0,-24.0],0.6000000238418579,null,224],[4294966737,"resource","Neutral",[-14.399999618530273,0.0,-22.600000381469727],0.6000000238418579,null,238],[4294966733,"resource","Neutral",[16.0,0.0,-24.0],0.6000000238418579,null,234],[4294966731,"resource","Neutral",[14.400001525878906,0.0,-22.600000381469727],0.6000000238418579,null,163],[4294966727,"resource","Neutral",[16.0,0.0,24.0],0.6000000238418579,null,240],[4294966725,"resource","Neutral",[14.400001525878906,0.0,22.599998474121094],0.6000000238418579,null,240],[4294966721,"resource","Neutral",[-16.0,0.0,24.0],0.6000000238418579,null,240],[4294966719,"resource","Neutral",[-14.399999618530273,0.0,22.599998474121094],0.6000000238418579,null,240],[4294966715,"resource","Neutral",[0.0,0.0,-2.0],0.6000000238418579,null,240],[4294966713,"resource","Neutral",[0.0,0.0,2.0],0.6000000238418579,null,240],[4294966717,"resource","Neutral",[-24.0,0.0,16.0],0.6000000238418579,null,140],[4294966735,"resource","Neutral",[-24.0,0.0,-16.0],0.6000000238418579,null,109],[4294966729,"resource","Neutral",[24.0,0.0,-16.0],0.6000000238418579,null,68],[4294966723,"resource","Neutral",[24.0,0.0,16.0],0.6000000238418579,null,140],[244813133528,"HeavyMachinegunTrooper","Player(0)",[-18.57246971130371,0.05000000074505806,-13.38482666015625],0.47999998927116394,7.0,null],[300647708745,"ScoutRover","Player(1)",[16.143310546875,0.05000000074505806,-27.902359008789062],0.550000011920929,5.0,null],[266287970250,"ScoutRover","Player(1)",[13.447906494140625,0.05000000074505806,-24.14994239807129],0.550000011920929,5.0,null],[4294966745,"Worker","Player(1)",[15.10405158996582,0.05000000074505806,-21.104026794433594],0.6000000238418579,6.0,null],[51539605989,"Worker","Player(1)",[22.36515235900879,0.05000000074505806,-16.1888370513916],0.6000000238418579,6.0,null],[4294966769,"Worker","Player(0)",[-27.735198974609375,0.05000000074505806,-12.997254371643066],0.6000000238418579,6.0,null],[4294966766,"Worker","Player(0)",[-17.289873123168945,0.05000000074505806,-23.0220890045166],0.6000000238418579,6.0,null],[4294966771,"Drone","Player(0)",[-24.0,1.5,-24.0],0.6000000238418579,6.0,null],[4294965817,"PowerReactor","Player(0)",[-26.0,0.0,-19.500001907348633],1.4500000476837158,12.0,null],[12884900087,"Refinery","Player(1)",[16.983112335205078,0.0,-17.118316650390625],1.5,14.0,null],[68719474993,"PowerReactor","Player(1)",[26.971708297729492,0.0,-22.531139373779297],1.4500000476837158,12.0,null],[4294965345,"RadarUplink","Player(0)",[-22.000001907348633,0.0,-28.0],1.399999976158142,12.0,null],[55834572617,"RadarUplink","Player(1)",[24.007646560668945,0.0,-26.57923126220703],1.399999976158142,12.0,null],[47244637835,"RoboticsBay","Player(1)",[24.279821395874023,0.0,-13.29354190826416],1.5499999523162842,14.0,null],[253403067974,"PowerReactor","Player(1)",[28.283409118652344,0.0,-18.914751052856445],1.4500000476837158,12.0,null],[627065222697,"TechLab","Player(1)",[20.906574249267578,0.0,-13.066667556762695],1.600000023841858,16.0,null],[335007446418,"OrePurifier","Player(1)",[23.93128204345703,0.0,-30.399999618530273],1.600000023841858,16.0,null],[111669147592,"Tank","Player(0)",[-18.3925724029541,0.05000000074505806,-11.54171085357666],0.8999999761581421,10.0,null],[184683591801,"Tank","Player(0)",[-19.868511199951172,0.05000000074505806,-12.780526161193848],0.8999999761581421,10.0,null],[335007447150,"Tank","Player(0)",[-18.51067543029785,0.05000000074505806,-17.20478057861328],0.8999999761581421,10.0,null],[150323852784,"Tank","Player(0)",[-14.4649019241333,0.05000000074505806,-16.55750274658203],0.8999999761581421,10.0,null],[330712479469,"AntiAirWalker","Player(0)",[-17.0000057220459,0.05000000074505806,-12.88424301147461],0.75,9.0,null],[322122544792,"Tank","Player(0)",[-16.204795837402344,0.05000000074505806,-15.928799629211426],0.8999999761581421,10.0,null],[64424507089,"AntiAirWalker","Player(0)",[-16.351242065429688,0.05000000074505806,-11.476548194885254],0.75,9.0,null]]"#;

#[test]
fn measured_constructor_detours_around_stationary_tank_without_body_overlap() {
    let (mut app, keys) = measured_app(CONSTRUCTION_ENDPOINT);
    let worker = keys[&4294966769];
    let blocking_tank = keys[&515396073129];
    let foundation = keys[&502511171143];
    // Isolate the measured blocking pair; retain every recorded static footprint.
    // Other mobile bodies belong to the later real-population acceptance run.
    let others: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<Unit>>()
        .iter(app.world())
        .filter(|entity| *entity != worker && *entity != blocking_tank)
        .collect();
    for entity in others {
        app.world_mut().despawn(entity);
    }
    let power = registry::entity("PowerReactor").unwrap();
    app.world_mut()
        .entity_mut(foundation)
        .insert(UnderConstruction {
            remaining: 1.0,
            total: 1.0,
            cost: power.cost,
            free_worker_origin: None,
        });
    let start = app.world().get::<Transform>(worker).unwrap().translation;
    let blocker_start = app
        .world()
        .get::<Transform>(blocking_tank)
        .unwrap()
        .translation;
    let worker_radius = app.world().get::<Selectable>(worker).unwrap().radius;
    let tank_radius = app.world().get::<Selectable>(blocking_tank).unwrap().radius;
    let goal = Vec3::new(-27.367902755737305, 0.0, -23.316139221191406);
    let waypoint = Vec3::new(-29.75, 0.0, -13.75);
    let direction = (waypoint - Vec3::new(start.x, 0.0, start.z)).normalize();
    assert!(
        distance_point_to_xz_segment(blocker_start, start, start + direction * (2.5 / 30.0))
            < worker_radius + tank_radius,
        "fixture must reproduce the blocked desired segment"
    );
    app.world_mut()
        .entity_mut(blocking_tank)
        .insert(HoldPosition { enabled: true });
    app.update();
    assert_eq!(
        app.world().get::<Transform>(worker).unwrap().translation,
        start
    );
    assert_eq!(
        app.world()
            .get::<Transform>(blocking_tank)
            .unwrap()
            .translation,
        blocker_start
    );
    // The next point is measured. The remaining static-clear continuation is a
    // fixture route around the Factory; the original later points were not saved.
    let waypoints = vec![
        waypoint,
        Vec3::new(-30.75, 0.0, -14.75),
        Vec3::new(-30.75, 0.0, -18.75),
        goal,
    ];
    let grid = app.world().resource::<NavGrid>();
    let mut anchor = start;
    for point in &waypoints {
        assert!(
            grid.line_clear(anchor, *point),
            "fixture continuation must be static-grid clear"
        );
        anchor = *point;
    }
    let static_bodies: Vec<crate::combat::MovementObstacleSnapshot> = {
        let mut query = app
            .world_mut()
            .query_filtered::<(&Transform, &Selectable, Option<&Structure>), (
                Or<(With<Structure>, With<ResourceNode>, With<TerrainWall>)>,
                Without<Unit>,
            )>();
        query
            .iter(app.world())
            .map(
                |(transform, body, building)| crate::combat::MovementObstacleSnapshot {
                    position: transform.translation,
                    radius: body.radius,
                    is_structure: building.is_some(),
                },
            )
            .collect()
    };
    let mut anchor = start;
    for point in &waypoints {
        assert!(
            crate::combat::static_movement_segment_clear(
                anchor,
                *point,
                worker_radius,
                &static_bodies
            ),
            "fixture continuation must satisfy actual worker-body static guard"
        );
        assert!(
            !app.world()
                .resource::<TerrainHeightField>()
                .step_blocked(anchor, *point)
        );
        anchor = *point;
    }
    app.world_mut().entity_mut(worker).insert((
        ConstructOrder { target: foundation },
        MoveOrder { target: goal },
        PlannedPath {
            goal,
            waypoints,
            next: 0,
        },
    ));
    for tick in 0..900 {
        let before = app.world().get::<Transform>(worker).unwrap().translation;
        app.update();
        let after = app.world().get::<Transform>(worker).unwrap().translation;
        if tick == 0 || tick == 59 || tick == 899 {
            println!(
                "constructor fixture tick{tick} position {after:?} move {:?} path {:?} construct {:?} remaining {:?}",
                app.world().get::<MoveOrder>(worker).map(|o| o.target),
                app.world()
                    .get::<PlannedPath>(worker)
                    .map(|p| (p.next, p.waypoints.clone())),
                app.world().get::<ConstructOrder>(worker).map(|o| o.target),
                app.world()
                    .get::<UnderConstruction>(foundation)
                    .map(|u| u.remaining)
            );
        }
        let blocking_position = app
            .world()
            .get::<Transform>(blocking_tank)
            .unwrap()
            .translation;
        assert!(
            app.world().get::<MoveOrder>(blocking_tank).is_none(),
            "no order may move the held blocker"
        );
        assert!(
            xz_distance(blocking_position, blocker_start) <= UNIT_SEPARATION_GAP_M,
            "held blocker exceeded the normal separation-gap displacement: {:?}",
            blocking_position - blocker_start
        );
        assert!(crate::combat::static_movement_segment_clear(
            before,
            after,
            worker_radius,
            &static_bodies
        ));
        assert!(
            !app.world()
                .resource::<TerrainHeightField>()
                .step_blocked(before, after)
        );
        assert!(
            distance_point_to_xz_segment(blocking_position, before, after) + 1e-4
                >= worker_radius + tank_radius,
            "constructor step tunneled through the Tank"
        );
        if app.world().get::<UnderConstruction>(foundation).is_none() {
            println!(
                "constructor completed at tick{tick}; natural blocker displacement {}",
                xz_distance(blocking_position, blocker_start)
            );
            break;
        }
    }
    let end = app.world().get::<Transform>(worker).unwrap().translation;
    assert!(
        xz_distance(start, end) > 1.0,
        "actual measured constructor made no detour progress: start {start:?}, end {end:?}"
    );
    assert!(
        app.world().get::<UnderConstruction>(foundation).is_none(),
        "site must actually complete within the unchanged 900-tick construction budget"
    );
}

#[test]
fn measured_harvester_routes_around_adjacent_aa_and_delivers_real_cargo() {
    // Measured Worker/resource/AA geometry; isolated mechanism fixture, not paid gameplay.
    let fixture = r#"[
        [1,"Worker","Player(0)",[-15.196226,0.05,-21.040670],0.6,6,null],
        [2,"resource",null,[-14.4,0,-22.6],0.6,null,229],
        [3,"AntiAirTurret","Player(0)",[-16,0,-22],0.6,8,null],
        [4,"CommandCenter","Player(0)",[-22,0,-22],2,20,null],
        [5,"CommandCenter","Player(1)",[22,0,-22],2,20,null]
    ]"#;
    let (mut app, keys) = measured_app(fixture);
    let worker = keys[&1];
    let node = keys[&2];
    let turret = keys[&3];
    let def = registry::entity("Worker").unwrap();
    let start = app.world().get::<Transform>(worker).unwrap().translation;
    let turret_position = app.world().get::<Transform>(turret).unwrap().translation;
    let goal = Vec3::new(-15.304574, 0.0, -21.403446);
    let combined_radius = def.radius + app.world().get::<Selectable>(turret).unwrap().radius;
    assert!(xz_distance(goal, turret_position) < combined_radius);
    assert!(
        (xz_distance(
            goal,
            app.world().get::<Transform>(node).unwrap().translation
        ) - 1.5)
            .abs()
            < 0.0001
    );
    app.world_mut().entity_mut(worker).insert((
        ResourceCargo {
            capacity: def.resource_capacity,
            ore: 0,
            crystal: 0,
        },
        HarvestOrder {
            resource: Some(node),
            state: HarvestState::MovingToResource,
            collect_remaining: 0.0,
            last_kind: Some(ResourceKind::Ore),
        },
        MoveOrder { target: goal },
    ));
    let ore_before = app.world().resource::<Economies>().get(Team::Player(0)).ore;
    let amount_before = app.world().get::<ResourceNode>(node).unwrap().amount;
    for tick in 0..900 {
        let before = app.world().get::<Transform>(worker).unwrap().translation;
        app.update();
        let after = app.world().get::<Transform>(worker).unwrap().translation;
        assert!(
            distance_point_to_xz_segment(turret_position, before, after) + 1e-4 >= combined_radius,
            "harvester step entered AA body"
        );
        if app.world().resource::<Economies>().get(Team::Player(0)).ore > ore_before {
            println!("measured harvest delivered at tick{tick}, start {start:?}, end {after:?}");
            break;
        }
    }
    let removed = amount_before - app.world().get::<ResourceNode>(node).unwrap().amount;
    let gained = app.world().resource::<Economies>().get(Team::Player(0)).ore - ore_before;
    println!(
        "measured harvest removed{removed} gained{gained} position {:?}",
        app.world().get::<Transform>(worker).unwrap().translation
    );
    assert!(
        gained > 0,
        "measured blocked contact never collected and delivered real ore"
    );
    assert_eq!(
        gained, removed,
        "delivered ore must come from the actual node"
    );
    assert_eq!(app.world().get::<Health>(worker).unwrap().current, 6.0);
    assert_eq!(app.world().get::<Health>(turret).unwrap().current, 8.0);
}

#[path = "movement_endpoint_tests/robotics_contact.rs"]
mod robotics_contact;
