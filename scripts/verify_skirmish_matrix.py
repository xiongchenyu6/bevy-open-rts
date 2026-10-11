#!/usr/bin/env python3
"""Independently verify a frozen skirmish matrix's terminal observations, read-only."""
import argparse
import hashlib
import itertools
import json
import math
import struct
from collections import Counter
from pathlib import Path

# Authored map spawn coordinates, independently transcribed from src/maps.rs.
MAPS = {
    'plain_and_simple': (50, [(10,7),(40,7),(40,43),(10,43)]),
    'four_corners': (72, [(10,10),(62,10),(62,62),(10,62)]),
    'tech_divide': (84, [(10,16),(10,42),(10,68),(74,16),(74,42),(74,68)]),
    'big_arena': (100, [(10,30),(35,10),(65,10),(90,30),(90,70),(65,90),(35,90),(10,70)]),
    'canyon_pass': (56, [(10,28),(46,28)]),
    'crossfire': (64, [(10,10),(54,10),(54,54),(10,54)]),
    'ring_valley': (60, [(8,8),(52,8),(52,52),(8,52)]),
    'highland_bastion': (60, [(12,30),(48,30)]),
}
FACTIONS = ('alliance','demon','chaos')
LEVELS = ('beginner','easy','normal','hard')


def expected_start_positions(case):
    width, starts = MAPS[case['map']]
    # Both Highland base spawns lie inside authored level-one plateaus.
    # Terrain settling assigns the Rust f32 TERRAIN_LEVEL_HEIGHT (1.4).
    height = struct.unpack('f', struct.pack('f', 1.4))[0] if case['map'] == 'highland_bastion' else 0.0
    return [{'player': team, 'position': [starts[slot][0]-width/2, height, starts[slot][1]-width/2]}
            for team, slot in enumerate(case['spawn_slots'])]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def expected_plan():
    cases = []
    for name, (_, starts) in MAPS.items():
        for pair in itertools.permutations(range(len(starts)), 2):
            groups = [(list(f), ['normal','normal']) for f in itertools.product(FACTIONS, repeat=2)]
            groups += [([f,f], list(d)) for f in FACTIONS for a,b in zip(LEVELS, LEVELS[1:]) for d in [(a,b),(b,a)]]
            cases += [dict(map=name,factions=f,difficulties=d,spawn_slots=list(pair),max_seconds=600) for f,d in groups]
    return cases


def verify(matrix, pin_path, asset_root):
    errors, failures, observations = [], [], []
    def require(ok, where, message):
        if not ok:
            errors.append({'where':where,'error':message})
    pin = json.loads(pin_path.read_text())
    manifest = json.loads((matrix/'manifest.json').read_text())
    plan = expected_plan()
    require(manifest.get('scenarios') == plan, 'manifest', 'ordered 3726-case plan differs')
    require(len(plan) == 3726 and len({json.dumps(c,sort_keys=True) for c in plan}) == 3726, 'manifest', 'missing/duplicate plan cases')
    require(manifest.get('matrix') == 'all' and manifest.get('all_slots') is True and manifest.get('wall_seconds') == 180, 'manifest', 'matrix mode or wall limit differs')
    require(manifest.get('schema') == 1, 'manifest', 'schema differs')
    audit_hash = digest(matrix/'skirmish_audit')
    require(audit_hash == manifest.get('source_binary_sha256') == pin['bins']['skirmish_audit'], 'binary', 'manifest/copied audit/source pin mismatch')
    require(digest(pin_path.parent/'skirmish_audit') == audit_hash, 'binary', 'sealed source audit differs')
    actual_assets = {'assets/'+p.relative_to(asset_root/'assets').as_posix():digest(p) for p in sorted((asset_root/'assets').rglob('*')) if p.is_file()}
    require(actual_assets == pin['assets'], 'assets', 'private assets differ from source pin')
    require(len(actual_assets) == pin.get('asset_count') == 332, 'assets', 'asset count differs')
    statuses = sorted(matrix.glob('[0-9][0-9][0-9][0-9]/status.json'))
    indices = []
    for path in statuses:
        index = int(path.parent.name)
        try:
            status = json.loads(path.read_text()); indices.append(index)
            require(index < len(plan), str(path), 'index outside plan')
            if index >= len(plan): continue
            case = plan[index]; kind = status.get('classification'); code = status.get('exit_code')
            require(status.get('index') == index and status.get('config') == case, str(path), 'index/config differs')
            wall = status.get('wall_seconds')
            require(isinstance(wall,(int,float)) and math.isfinite(wall) and wall >= 0, str(path), 'invalid wall elapsed')
            require(kind in ['completed','simulation_timeout','wall_timeout','process_error'],str(path),'unrecognized terminal classification')
            if kind in ['completed','simulation_timeout']:
                result_path=path.parent/'result.json'; result=json.loads(result_path.read_text()); complete=kind=='completed'
                require(result.get('schema') == 1, str(result_path), 'raw schema differs')
                require(wall <= 181, str(path), 'terminal wall elapsed exceeds subprocess limit plus result-write overhead')
                require(code == (0 if complete else 2),str(path),'exit classification differs')
                require(result.get('config') == case,str(result_path),'raw config differs')
                require(result.get('completed') is complete and result.get('timeout') is (not complete),str(result_path),'false completion/timeout claim')
                require(result.get('starting_resources') == {'ore':32,'crystal':16} and result.get('startup_loadout') == 'GodotSkirmish',str(result_path),'opening differs')
                require(result.get('resolved_map_id') == case['map'] and result.get('resolved_map_path') == ({'plain_and_simple':'res://source/match/maps/PlainAndSimple.tscn','four_corners':'res://source/match/maps/FourCorners.tscn','tech_divide':'res://source/match/maps/TechDivide.tscn','big_arena':'res://source/match/maps/BigArena.tscn','canyon_pass':'bevy://maps/CanyonPass','crossfire':'bevy://maps/Crossfire','ring_valley':'bevy://maps/RingValley','highland_bastion':'bevy://maps/HighlandBastion'}[case['map']]),str(result_path),'resolved map differs')
                require(result.get('victory_condition') == 'Annihilation' and result.get('active_player_slots') == [0,1],str(result_path),'victory/active player policy differs')
                positions=expected_start_positions(case)
                require(result.get('initial_command_center_positions') == positions,str(result_path),'actual start coordinates differ')
                winners=result.get('winning_teams');teams=result.get('teams',[])
                require(status.get('winning_teams') == winners,str(path),'winner disagreement')
                anchors={t['player']:t['structure_types'].get('CommandCenter',0)+t['unit_types'].get('Worker',0) for t in teams}
                living=[team for team,count in sorted(anchors.items()) if count]
                require(result.get('remaining_anchors') == sum(anchors.values()),str(result_path),'HQ/Worker anchor sum differs')
                require([i for i,v in enumerate(result.get('active_anchor_teams',[])) if v] == living,str(result_path),'active anchor teams differ')
                require(winners == living if complete else winners == [],str(result_path),'winner/anchor policy differs')
                require(len(living)==1 if complete else len(living)==2,str(result_path),'completion versus surviving teams differs')
                require(result.get('phase') == ('MatchFinished' if complete else 'Running'),str(result_path),'phase differs')
                clock=result.get('clock_seconds',-1);step=result.get('timestep_seconds',-1)
                require(math.isclose(step,1/30,abs_tol=1e-12),str(result_path),'timestep differs')
                require(0 <= clock <= 600.1 and (complete or clock >= 600),str(result_path),'simulation clock outside fixed limit')
                for team in teams:
                    require(sum(team['unit_types'].values()) == team['units'] and sum(team['structure_types'].values()) == team['structures'],str(result_path),'typed totals differ')
                observations.append({'index':index,'classification':kind,'clock':clock,'wall':wall,'winning_teams':winners,'status_sha256':digest(path),'result_sha256':digest(result_path)})
            elif kind=='wall_timeout':
                require(code is None and wall >= 180,str(path),'wall timeout exit/elapsed differs')
            elif kind=='process_error':
                require(code not in [0,None],str(path),'process error has pass exit')
            if kind != 'completed': failures.append({'index':index,'config':case,'classification':kind,'exit_code':code,'evidence':str(path.parent)})
        except (OSError,ValueError,KeyError,TypeError) as error:
            errors.append({'where':str(path),'error':str(error)})
    require(indices == list(range(len(indices))),'terminal coverage','missing/duplicate/noncontiguous terminal indices')
    counts=Counter(o['classification'] for o in observations)
    for f in failures:
        if f['classification'] not in ['completed','simulation_timeout']:counts[f['classification']]+=1
    root=pin_path.resolve().parents[1].parent
    changed=[name for name,value in pin.get('sources',{}).items() if not (root/name).is_file() or digest(root/name)!=value]
    added=[str(p.relative_to(root)) for p in sorted((root/'src').rglob('*.rs')) if str(p.relative_to(root)) not in pin.get('sources',{})]
    return {'scope':'Partial read-only verification of the fixed source pin; not full-matrix or current247-library equivalence','planned':len(plan),'terminal_snapshot':len(statuses),'counts':dict(counts),'complete_matrix':len(statuses)==len(plan),'integrity_verified':not errors,'errors':errors,'failures':failures,'matrix':str(matrix),'source_pin':str(pin_path),'audit_sha256':audit_hash,'manifest_sha256':digest(matrix/'manifest.json'),'private_asset_count':len(actual_assets),'private_asset_hashes_match':actual_assets==pin['assets'],'current_source_files_differing_from_pin':changed,'current_source_files_added_since_pin':added,'observations':observations,'inflight_directories':[p.name for p in sorted(matrix.glob('[0-9][0-9][0-9][0-9]')) if not (p/'status.json').exists()], 'interpretation':'Completion means current Annihilation policy: only Command Centers and Workers retain a team. Other surviving units/structures do not invalidate this policy. No independent seeds/repeats/random maps claimed.'}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['matrix','pin','assets','report']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();report=verify(args.matrix,args.pin,args.assets)
    args.report.parent.mkdir(parents=True,exist_ok=True);args.report.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ['observations','current_source_files_differing_from_pin']},indent=2))
    return (2 if report['failures'] else 0) if report['integrity_verified'] else 1

if __name__=='__main__':raise SystemExit(main())
