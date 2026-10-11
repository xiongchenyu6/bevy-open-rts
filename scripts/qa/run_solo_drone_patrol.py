"""Prepared normal SoloDrone Patrol reproduction; no game setters;600/30 bounds."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import sys
import time
import traceback

HERE = Path(__file__).resolve().parents[2] / "target/same-app-campaign-preparation"
sys.path[:0] = [str(HERE), str(HERE.parent)]
from artifact import validate_ready
from session import PhysicalSession, sha
from recovery_placement06 import Opening, MAP_SIZES
from refresh04 import refresh
from mission2_physical_controls04 import Opening as ExistingOpening
from camera_opening import outside_edge_pan_band
from save_position_ron import fields, split_top
ExistingOpening.refresh = refresh
MAP_SIZES['res://source/match/maps/PlainAndSimple.tscn'] = 50


def rows(path):
    raw = fields(path.read_text())
    result = {}
    for item in split_top(raw['units'][1:-1]):
        unit = fields(item)
        if unit['team'] == '0':
            result[int(unit['key'])] = unit
    return raw, result


def xyz(unit):
    return [float(v) for v in split_top(unit['position'][1:-1])]


def validate_gesture(card_down, ground_down, selected, point, offset):
    assert card_down['left_pressed'], 'Missing actual card DOWN'
    assert any(card['action'] == 'Patrol' and card['contains_physical_cursor']
               and card['enabled'] and card['inherited_visible'] and card['display'] == 'Flex'
               for card in card_down['cards']), 'Actual gesture did not hit Patrol card'
    assert ground_down['right_pressed'] and not ground_down['hud_blocks_cursor'], 'Missing nonHUD world RMB'
    assert all(abs(ground_down['window']['cursor_physical'][i] + offset[i] - point[i]) <= .5
               for i in (0, 1)), 'Actual RMB cursor missed intended ground'
    expected = {(x['local_entity'], x['type']) for x in selected}
    actual = {(x['local_entity'], x['type']) for x in ground_down['state']['selected_owned']}
    assert actual == expected, 'Actual RMB selected cohort changed'


def run(s):
    s.normal_launch_once()
    s.capture_current_png('00-normal-main-menu')
    s.click(1038, 255); s.wait_bounded(.5); s.press('Enter'); s.wait_bounded(10); s.press('Enter')
    s.press('F10')
    own, raw, initial = s.read_own('01-initial')
    assert int(raw['version']) == 4 and [own['ore'], own['crystal']] == [32, 16]
    settings = fields(raw['settings'])
    assert json.loads(settings['map_path']) == 'res://source/match/maps/PlainAndSimple.tscn'
    assert sum(u['id'] == 'Drone' for u in own['units']) == 1
    assert sum(u['id'] == 'Worker' for u in own['units']) == 2
    o = Opening(s.mission_out, s.profile, s.click, s.press, s.send, s.ctrl,
                s.capture_current_png, s.check_wall, s.key)
    o.select_structure('CommandCenter')
    s.key(29, True); s.key(56, True)
    try: s.press(30)
    finally: s.key(56, False); s.key(29, False)
    event = o.refresh(); own = o.state('04-mixed-before')
    selected = event['state']['selected_owned']
    assert [x['type'] for x in selected] == ['Drone'], 'Only normal starter Drone selected'
    drone = next(x['local_entity'] for x in selected if x['type'] == 'Drone')
    definitions = {e['id']: e for e in json.load(open(s.package/'assets/migration/gameplay_registry.json'))['entities']}
    hq = next(x['position'] for x in own['structures'] if x['id'] == 'CommandCenter')
    gun = next(x['position'] for x in own['units'] if x['key'] == drone)
    point = None
    for dx, dz in [(4, 1), (4, -1), (-4, 1), (-4, -1), (0, 4), (0, -4), (6, 2), (-6, 2)]:
        world = [hq[0]+dx, 0, hq[2]+dz]
        if math.hypot(world[0]-gun[0], world[2]-gun[2]) <= 2.5: continue
        if any(math.hypot(world[0]-x['position'][0], world[2]-x['position'][2]) <= definitions[x['id']]['radius']+.45 for x in own['structures']+own['units']): continue
        if any(x['visible'] and x['loaded_mesh_count'] and math.hypot(world[0]-x['global_anchor'][0], world[2]-x['global_anchor'][2]) <= 1.05 for x in event.get('native_resource_geometry', {}).get('resources', [])): continue
        try:
            candidate = o.project_own(world, 0, event)
            assert outside_edge_pan_band(candidate, event)
        except AssertionError: continue
        point = candidate; break
    assert point is not None, 'Bounded current public clear-ground candidate required'
    image = s.capture_current_png('05-current-mixed-before-ground')
    cards = [c for c in event['cards'] if c['action']=='Patrol' and c['enabled'] and c['inherited_visible'] and c['display']=='Flex']
    assert len(cards)==1
    card=cards[0];s.click(round(card['physical_center'][0]+o.window_offset[0]), round(card['physical_center'][1]+o.window_offset[1]))
    down=next(e for e in reversed(json.load(open(s.out/'online-observer.json'))['ui_events']) if e['left_pressed'])
    s.capture_current_png('06-actual-Patrol-mode');s.right(*point)
    issued=o.state('07-mixed-issued');packet=json.load(open(s.out/'online-observer.json'))
    ground=next(e for e in reversed(packet['ui_events']) if e['right_pressed'] and not e['hud_blocks_cursor'])
    validate_gesture(down, ground, selected, point, o.window_offset)
    raw, issued_units=rows(s.mission_out/'07-mixed-issued.ron')
    continuity=fields(issued_units[drone]['continuity'])
    assert [issued['ore'],issued['crystal']]==[32,16], 'Exact normal starter budget unchanged'
    assert 'Patrol(' in continuity['active'], 'Actual selected unarmed Drone receives Patrol'
    dp=xyz(issued_units[drone])
    s.capture_current_png('08-solo-Drone-paused-issued')
    s.press('F10');s.wait_bounded(5);s.press('F10')
    later, raw, path=s.read_own('09-after-natural-five-seconds');_, actual=rows(path)
    after=xyz(actual[drone]);delta=math.hypot(after[0]-dp[0],after[2]-dp[2])
    clock_delta=later['clock']-issued['clock']
    assert clock_delta>=5, 'At least five actual natural game seconds'
    s.capture_current_png('10-solo-Drone-after-natural-motion')
    receipt={'drone_key':drone,'before':dp,'after':after,'xz_delta':delta,'clock_delta':clock_delta,
      'issued_continuity':continuity,'after_continuity':fields(actual[drone]['continuity']),
      'normal_ground_point':point,'selected':selected,'bank': [own['ore'],own['crystal']]}
    (s.out/'patrol-observation.json').write_text(json.dumps(receipt,indent=2)+'\n')
    assert delta>.5, 'Normal enabled Drone Patrol must produce natural movement; observed receipt preserved'
    return receipt



if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--ready',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--check',action='store_true');a=p.parse_args()
    ready=validate_ready(a.ready);assert ready['files']==399
    if a.check: print('Exactartifact399 validated; no compositor/game launched');raise SystemExit(0)
    s=None;result={'scope':'Normal starter SoloDrone Patrol smoke; no spawned/paid extra units; not campaign acceptance','passed':False,'original_wall':600,'original_natural_stage':30}
    try:
        s=PhysicalSession(ready['package'],ready['archive'],ready['binary_sha256'],ready['archive_sha256'],a.out,900)
        s.deadline=min(s.deadline,s.started+600);s.stage_deadline=s.deadline;s.begin_mission(0)
        result['actual_receipt']=run(s);s.normal_close_and_require_exit_zero();result['program_flow_passed']=True
    except Exception as e:
        result.update(error=str(e),error_type=type(e).__name__,traceback=traceback.format_exc())
        if s and s.game and s.game.poll() is None and not s.keys:
            try:s.capture_current_png('99-actual-failure');s.normal_close_and_require_exit_zero()
            except Exception as e:result['normal_failure_close_error']=str(e)
    finally:
        if s:
            s.cleanup();result.update(normal_exits=s.exits,forced_cleanup=s.forced_cleanup,installation_unchanged=s.before==s.inventory(),archive_unchanged=sha(s.archive)==ready['archive_sha256'],screenshots=s.shots,wall_seconds=time.monotonic()-s.started)
            result['passed']=bool(result.get('program_flow_passed') and not s.forced_cleanup and all(x['actual_exit']==0 for x in s.exits) and result['installation_unchanged'] and result['archive_unchanged'])
            (s.out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    raise SystemExit(0 if result['passed'] else 1)
