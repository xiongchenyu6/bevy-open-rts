use super::*;

fn pending_mine(state: &Value, layer: u64, at: Vec3) -> Option<&Value> {
    let layer = actor(state, layer)?;
    if !layer["active"].as_array()?.is_empty() || layer["layer"].as_array()?[0].as_f64()? < 1.3 {
        return None;
    }
    state["units"].as_array()?.iter().find(|u| {
        let Some(m) = mine(u) else {
            return false;
        };
        u["team"] == 0
            && m[4] == layer["key"]
            && distance(point(&u["position"]), at) < 2.0
            && m[3].as_f64().is_some_and(|r| r > 0.15)
            && state["units"].as_array().unwrap().iter().any(|enemy| {
                enemy["team"] == 1
                    && enemy["id"] == "Worker"
                    && enemy["visible"] == true
                    && enemy["health"].as_f64().is_some_and(|hp| hp >= 5.99)
                    && distance(point(&enemy["position"]), point(&u["position"])) < 1.4
            })
    })
}

fn construct(app: &mut App, builder: u64, id: &'static str, at: Vec3) -> Result<Vec3, String> {
    select(app, builder)?;
    let stand = at - Vec3::X * 3.0;
    move_to(app, stand)?;
    if !wait_until(app, 1200, |a| {
        capture_live_mine_state(a).is_ok_and(|s| {
            actor(&s, builder).is_some_and(|u| distance(point(&u["position"]), stand) < 0.6)
        })
    }) {
        return Err(format!("real Worker did not reach {id} build site"));
    }
    select(app, builder)?;
    for production in [true, false] {
        if capture_enabled_build_hotkey_for(app, id).is_some() {
            break;
        }
        if let Some(key) = bevy_open_rts::capture_worker_build_tab_hotkey(app, production) {
            tap_key(app, key);
            for _ in 0..4 {
                app.update();
            }
        }
    }
    let build = capture_enabled_build_hotkey_for(app, id)
        .ok_or_else(|| format!("paid {id} command unavailable"))?;
    tap_key(app, build);
    let mut placed = None;
    for candidate in [at, at + Vec3::Z * 2.0, at - Vec3::Z * 2.0] {
        click(app, candidate, MouseButton::Left)?;
        if capture_campaign_state(app)
            .structures
            .iter()
            .any(|s| s.id == id && distance(s.position, candidate) < 0.5)
        {
            placed = Some(candidate);
            break;
        }
        println!("[capture] {id} placement rejected at {candidate:?}");
    }
    let placed = placed.ok_or_else(|| {
        format!("actual {id} placement rejected; see captured placement feedback")
    })?;
    if !wait_until(app, 1200, |a| {
        capture_campaign_state(a)
            .structures
            .iter()
            .any(|s| s.id == id && s.completed && distance(s.position, placed) < 0.5)
    }) {
        return Err(format!("real {id} construction did not finish"));
    }
    println!("[capture] paid forward {id} completed at {placed:?}");
    Ok(placed)
}

pub(super) fn prepare(app: &mut App, handle: &Handle<Image>, dir: &Path) -> Result<Value, String> {
    let initial = capture_live_mine_state(app)?;
    if initial["ore"] != 32 || initial["crystal"] != 16 {
        return Err("normal32ore/16crystal starting budget required".into());
    }
    let crystal = capture_campaign_resources(app)
        .into_iter()
        .filter(|r| r.kind == "crystal")
        .min_by(|a, b| {
            a.position
                .length_squared()
                .total_cmp(&b.position.length_squared())
        })
        .ok_or("visible crystal unavailable")?;
    let worker = capture_campaign_units(app)
        .into_iter()
        .filter(|u| u.id == "Worker")
        .min_by(|a, b| {
            distance(a.position, crystal.position)
                .total_cmp(&distance(b.position, crystal.position))
        })
        .ok_or("Worker unavailable")?;
    select(app, worker.entity.to_bits())?;
    click(app, crystal.click_position, MouseButton::Right)?;
    if !wait_until(app, 1200, |a| {
        capture_live_mine_state(a)
            .is_ok_and(|s| s["crystal"].as_i64() > initial["crystal"].as_i64())
    }) {
        return Err("actual crystal harvest did not deliver".into());
    }
    let harvested = capture_live_mine_state(app)?;
    for id in ["PowerReactor", "RadarUplink", "RoboticsBay"] {
        build_campaign_structure_via_real_input(app, id, &mut |_| {})?;
    }
    let base = capture_campaign_state(app)
        .headquarters
        .ok_or("home HQ unavailable")?
        .position;
    let scout = capture_campaign_units(app)
        .into_iter()
        .find(|u| u.id == "ShieldTrooper")
        .ok_or("normal starting ShieldTrooper cover unavailable")?;
    select(app, scout.entity.to_bits())?;
    move_to(app, Vec3::new(-base.x - 2.0, 0.0, base.z + 4.0))?;
    if !wait_until(app, 900, |a| {
        capture_live_mine_state(a).is_ok_and(|s| enemy_base(&s).is_some())
            && capture_campaign_resources(a)
                .iter()
                .any(|r| r.kind == "ore" && r.position.x > 0.0)
    }) {
        return Err("real scout did not expose enemy HQ/mining patch".into());
    }
    let scouted = capture_live_mine_state(app)?;
    let miner = scouted["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| u["team"] == 1 && u["visible"] == true && u["id"] == "Worker")
        .min_by(|a, b| point(&a["position"]).x.total_cmp(&point(&b["position"]).x))
        .ok_or("actual visible mining Worker unavailable")?;
    let mining_point = point(&miner["position"]).with_y(0.0);
    let crystal_node = capture_campaign_resources(app)
        .into_iter()
        .filter(|r| r.kind == "crystal" && r.position.x > 0.0)
        .min_by(|a, b| {
            distance(a.position, mining_point).total_cmp(&distance(b.position, mining_point))
        })
        .ok_or("visible enemy crystal approach unavailable")?;
    let corridor = mining_point
        + (mining_point - crystal_node.position)
            .with_y(0.0)
            .normalize_or_zero()
            * 0.3;
    let proxy = corridor - Vec3::X * 6.0;
    // Clear the actually observed starting defender with the existing unit.
    // The unarmed construction Worker then approaches the outpost safely.
    let defender = scouted["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["team"] == 1 && u["visible"] == true && u["id"] == "RocketInfantry")
        .cloned()
        .ok_or("observed starting Rocket defender unavailable")?;
    let defender_key = defender["key"].as_u64().unwrap();
    select(app, scout.entity.to_bits())?;
    click(app, point(&defender["position"]), MouseButton::Right)?;
    let mut miners_untouched = true;
    if !wait_until(app, 600, |a| {
        capture_live_mine_state(a).is_ok_and(|s| {
            miners_untouched &= s["units"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|u| u["team"] == 1 && u["id"] == "Worker")
                .all(|u| u["health"].as_f64().unwrap() >= 6.0);
            actor(&s, defender_key).is_none_or(|u| u["health"].as_f64().unwrap() <= 0.0)
        })
    }) {
        return Err("real starting Trooper failed to defeat observed Rocket defender".into());
    }
    let cleared = capture_live_mine_state(app)?;
    write(dir, "01_defender_cleared.json", cleared.clone())?;
    let home = point(&actor(&initial, scout.entity.to_bits()).unwrap()["position"]);
    if !capture_entity_is_selected(app, scout.entity) {
        select(app, scout.entity.to_bits())?;
    }
    move_to(app, home)?;
    if !wait_until(app, 900, |a| {
        capture_live_mine_state(a).is_ok_and(|s| {
            miners_untouched &= s["units"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|u| u["team"] == 1 && u["id"] == "Worker")
                .all(|u| u["health"].as_f64().unwrap() >= 6.0);
            actor(&s, scout.entity.to_bits())
                .is_some_and(|u| distance(point(&u["position"]), home) < 0.6)
        })
    }) {
        return Err("real cover Trooper did not return Home after defender clearance".into());
    }
    if !miners_untouched {
        return Err("combat cover damaged an enemy mining Worker".into());
    }
    write(dir, "01_cover_retired.json", capture_live_mine_state(app)?)?;
    let builder = capture_campaign_units(app)
        .into_iter()
        .filter(|u| u.id == "Worker" && !u.constructing)
        .min_by(|a, b| distance(a.position, proxy).total_cmp(&distance(b.position, proxy)))
        .ok_or("proxy builder unavailable")?;
    let builder_key = builder.entity.to_bits();
    write(dir, "01_scouted.json", scouted.clone())?;
    let robotics = capture_campaign_state(app)
        .structures
        .into_iter()
        .find(|s| s.id == "RoboticsBay" && s.completed)
        .unwrap();
    let relay = robotics.position.lerp(proxy, 0.5).with_y(0.0);
    let before_relay = capture_live_mine_state(app)?;
    construct(app, builder_key, "PowerReactor", relay)?;
    let paid_relay = capture_live_mine_state(app)?;
    if before_relay["ore"].as_i64().unwrap() - paid_relay["ore"].as_i64().unwrap() != 3
        || before_relay["crystal"].as_i64().unwrap() - paid_relay["crystal"].as_i64().unwrap() != 1
    {
        return Err("forward anchor did not pay normal3ore1crystal".into());
    }
    write(dir, "02_paid_relay.json", paid_relay.clone())?;
    let before_factory = capture_live_mine_state(app)?;
    construct(app, builder_key, "VehicleFactory", proxy)?;
    let factory = capture_campaign_state(app)
        .structures
        .into_iter()
        .find(|s| s.id == "VehicleFactory" && s.completed)
        .unwrap();
    let paid_factory = capture_live_mine_state(app)?;
    if before_factory["ore"].as_i64().unwrap() - paid_factory["ore"].as_i64().unwrap() != 6 {
        return Err("proxy factory did not pay normal6ore".into());
    }
    save_effects::focus(app, factory.position);
    let screen = capture_world_to_screen(app, factory.click_position).ok_or("factory offscreen")?;
    capture_click_screen(app, screen, MouseButton::Left);
    if !capture_entity_is_selected(app, factory.entity) {
        return Err("proxy factory was not selected through real click".into());
    }
    let rally = bevy_open_rts::capture_first_enabled_rally_hotkey(app)
        .ok_or("actual Rally Point command unavailable")?;
    tap_key(app, rally);
    click(app, corridor, MouseButton::Right)?;
    let rally_state = capture_live_mine_state(app)?;
    let set_rally = rally_state["structures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["key"].as_u64() == Some(factory.entity.to_bits()))
        .unwrap();
    if set_rally["rally"]
        .as_array()
        .is_none_or(|r| distance(point(&r[0]), corridor) > 0.1)
    {
        return Err("actual Rally hotkey/click did not assign intended ground waypoint".into());
    }
    write(dir, "03_rally_set.json", rally_state)?;
    let train = capture_enabled_train_hotkey_for(app, "DroneMineLayer")
        .ok_or("normal mine-layer training unavailable")?;
    let before_train = capture_live_mine_state(app)?;
    tap_key(app, train);
    let paid_train = capture_live_mine_state(app)?;
    if before_train["ore"].as_i64().unwrap() - paid_train["ore"].as_i64().unwrap() != 4
        || before_train["crystal"].as_i64().unwrap() - paid_train["crystal"].as_i64().unwrap() != 3
    {
        return Err("mine-layer did not pay normal4ore3crystal".into());
    }
    if !wait_until(app, 600, |a| {
        capture_campaign_units(a)
            .iter()
            .any(|u| u.id == "DroneMineLayer")
    }) {
        return Err("paid mine-layer did not finish training".into());
    }
    let first_layer = capture_campaign_units(app)
        .into_iter()
        .find(|u| u.id == "DroneMineLayer")
        .unwrap()
        .entity
        .to_bits();
    if !wait_until(app, 300, |a| {
        capture_live_mine_state(a).is_ok_and(|s| {
            actor(&s, first_layer)
                .is_some_and(|u| u["layer"][1].as_u64().is_some_and(|index| index >= 4))
        })
    }) {
        return Err("first paid Layer did not deploy its normal opening mines".into());
    }
    let first_deployment = capture_live_mine_state(app)?;
    write(
        dir,
        "04_first_layer_deployment.json",
        first_deployment.clone(),
    )?;
    let ore_worker = first_deployment["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| {
            u["team"] == 1
                && u["id"] == "Worker"
                && u["visible"] == true
                && u["health"].as_f64().is_some_and(|hp| hp >= 5.99)
        })
        .max_by(|a, b| point(&a["position"]).x.total_cmp(&point(&b["position"]).x))
        .ok_or("visible healthy Ore Worker unavailable for second paid Layer")?;
    let ore_position = point(&ore_worker["position"]).with_y(0.0);
    let ore_node = capture_campaign_resources(app)
        .into_iter()
        .filter(|r| r.kind == "ore" && r.position.x > 0.0)
        .min_by(|a, b| {
            distance(a.position, ore_position).total_cmp(&distance(b.position, ore_position))
        })
        .ok_or("visible Ore node approach unavailable")?;
    // The outermost visible Ore Worker has open terrain to its east. Avoid
    // clicking inside the nearby resource cluster, which correctly snaps the
    // rally to a resource target rather than accepting the ground waypoint.
    let corridor = ore_position + Vec3::X * 1.5;
    if capture_campaign_resources(app)
        .iter()
        .any(|r| distance(r.position, corridor) < 1.7)
    {
        return Err(
            "second Layer terrain approach still overlaps visible resource click radius".into(),
        );
    }
    println!("[capture] second Layer open-terrain Rally east of visible Ore Worker");
    save_effects::focus(app, factory.position);
    let factory_screen = capture_world_to_screen(app, factory.click_position)
        .ok_or("factory offscreen for second training")?;
    capture_click_screen(app, factory_screen, MouseButton::Left);
    if !capture_entity_is_selected(app, factory.entity) {
        return Err("second paid Layer factory selection failed".into());
    }
    let rally = bevy_open_rts::capture_first_enabled_rally_hotkey(app)
        .ok_or("second Layer Rally shortcut unavailable")?;
    tap_key(app, rally);
    click(app, corridor, MouseButton::Right)?;
    let second_rally = capture_live_mine_state(app)?;
    let factory_rally = second_rally["structures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["key"].as_u64() == Some(factory.entity.to_bits()))
        .unwrap();
    write(
        dir,
        "05_second_rally_attempt.json",
        serde_json::json!({"wanted":corridor.to_array(),"visible_ore_node":ore_node.position.to_array(),"observed":factory_rally["rally"],"factory_selected":capture_entity_is_selected(app,factory.entity),"state":second_rally}),
    )?;
    if factory_rally["rally"]
        .as_array()
        .is_none_or(|r| distance(point(&r[0]), corridor) > 0.1)
    {
        return Err("second Layer real Rally did not match Ore approach".into());
    }
    let second_train = capture_enabled_train_hotkey_for(app, "DroneMineLayer")
        .ok_or("second paid Layer training shortcut unavailable")?;
    let before_second_train = capture_live_mine_state(app)?;
    tap_key(app, second_train);
    let paid_second_train = capture_live_mine_state(app)?;
    if before_second_train["ore"].as_i64().unwrap() - paid_second_train["ore"].as_i64().unwrap()
        != 4
        || before_second_train["crystal"].as_i64().unwrap()
            - paid_second_train["crystal"].as_i64().unwrap()
            != 3
    {
        return Err("second Layer did not pay normal4ore3crystal".into());
    }
    if !wait_until(app, 600, |a| {
        capture_campaign_units(a)
            .iter()
            .any(|u| u.id == "DroneMineLayer" && u.entity.to_bits() != first_layer)
    }) {
        return Err("second paid Layer did not complete".into());
    }
    let layer = capture_campaign_units(app)
        .into_iter()
        .find(|u| u.id == "DroneMineLayer" && u.entity.to_bits() != first_layer)
        .unwrap()
        .entity
        .to_bits();
    println!("[capture] second paid Layer targets visible healthy Ore Worker");
    if !wait_until(app, 1500, |a| {
        capture_live_mine_state(a).is_ok_and(|s| pending_mine(&s, layer, corridor).is_some())
    }) {
        return Err("real rally/deployment did not yield a saveable corridor mine".into());
    }
    let active = capture_live_mine_state(app)?;
    let chosen = pending_mine(&active, layer, corridor).unwrap();
    let chosen_position = point(&chosen["position"]);
    println!("[capture] saving authored unarmed mine inside observed healthy Worker trigger range");
    // The ordinary Save menu pauses simulation. Capture its pending mine
    // before clicking Save; no timer or tactical-pause state is altered.
    save_effects::focus(app, chosen_position + Vec3::X * 4.0);
    tap_key(app, KeyCode::Escape);
    for _ in 0..8 {
        app.update();
    }
    shoot(app, handle, dir.join("00_saved_mines.png"));
    let save_button = bevy_open_rts::capture_match_menu_button_center(app, "save")
        .ok_or("mouse Save button unavailable")?;
    capture_click_screen(app, save_button, MouseButton::Left);
    let expected = capture_disk_mine_state()?;
    let saved = expected["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| {
            mine(u).is_some_and(|m| m[4].as_u64() == Some(layer))
                && distance(point(&u["position"]), chosen_position) < 0.15
        })
        .ok_or("selected mine triggered before mouse Save")?;
    let mine_key = saved["key"].as_u64().unwrap();
    Ok(
        serde_json::json!({"process_id":std::process::id(),"initial":initial,"harvested":harvested,"scouted":scouted,"before_relay":before_relay,"paid_relay":paid_relay,"before_factory":before_factory,"paid_factory":paid_factory,"before_train":before_train,"paid_train":paid_train,"first_layer_key":first_layer,"first_deployment":first_deployment,"second_rally":second_rally,"before_second_train":before_second_train,"paid_second_train":paid_second_train,"layer_key":layer,"mine_key":mine_key,"corridor":corridor.to_array(),"expected":expected}),
    )
}
