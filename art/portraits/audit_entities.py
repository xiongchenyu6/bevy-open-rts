#!/usr/bin/env python3
"""Read registry/model-map source; never read portrait PNGs as render inputs."""
import hashlib,json,pathlib,re
ROOT=pathlib.Path(__file__).resolve().parents[2]
s=(ROOT/'src/generated_registry.rs').read_text(); model=(ROOT/'assets/data/godot_model_map.model_map.ron').read_text()
line=lambda text,offset:text.count('\n',0,offset)+1
arrays={m.group(1):re.findall(r'"([^"]+)"',m.group(2)) for m in re.finditer(r'const (\w+): &\[&str\] = &\[(.*?)\];',s,re.S)}
members={f:set(v for k,vs in arrays.items() if k.startswith(f.upper()+'_STRUCTURES') or k.startswith(f.upper()+'_PRODUCTS_') for v in vs) for f in ['alliance','demon','chaos']}
start=s.index('pub const ENTITY_DEFS:');end=s.index('const ALLIANCE_STRUCTURES:')
rows=[]
for b in re.finditer(r'    EntityDef \{(.*?)\n    \},',s[start:end],re.S):
 t=b.group(1);get=lambda k:re.search(k+r': "([^"]+)"',t).group(1)
 eid=get('id');icon=re.search(r'icon: Some\("([^"]+)"\)',t)
 if not icon:continue
 model_match=re.search(r'model_assets: (\w+)',t);models=model_match.group(1) if model_match else '';parts_match=re.search(r'render_parts: (\w+)',t);parts=parts_match.group(1) if parts_match else ''
 pm=re.search(r'const '+parts+r': &\[RenderPart\] = &\[(.*?)\];',s,re.S)
 mm=re.search(r'id: "'+re.escape(eid)+r'",',model)
 factions=[f for f in members if eid in members[f]]
 rows.append({'runtime_path':icon.group(1),'entity_id':eid,'label':get('label'),'role':re.search(r'role: EntityRole::(\w+)',t).group(1),'available_factions':factions,'portrait_faction':factions[0] if factions else 'alliance','registry_source':f'src/generated_registry.rs:{line(s,start+b.start())}','scene_path':get('scene_path'),'model_assets':arrays.get(models,[]),'render_parts_symbol':parts,'registry_render_part_count':pm.group(1).count('RenderPart {') if pm else 0,'model_map_source':f'assets/data/godot_model_map.model_map.ron:{line(model,mm.start())}' if mm else None,'assembly_source':'src/tank_models.rs::spawn_tank_model' if not arrays.get(models) and eid!='TeslaFenceSegment' else ('authored Tesla geometry through normal spawn/model processing' if eid=='TeslaFenceSegment' else 'normal spawn/model processing plus shared structure detail attachments'),'render_input':'full actual assembled game geometry; never legacy PNG'})
audit=json.loads((ROOT/'docs/icon-replacement-audit.json').read_text())
target_ids={r.get('entity') for r in audit['icons'] if r.get('entity')}
existing=[r for r in rows if r['entity_id'] not in target_ids]
for r in rows:
 r['provenance_status']='unresolved' if r['entity_id'] in target_ids else 'already_provenance_recorded'
 r['ownership']='neutral' if not r['available_factions'] else 'faction_roster'

paths={}
for r in rows:paths.setdefault(r['runtime_path'],[]).append(r['entity_id'])
report={'schema':1,'scope':'Source-only export inventory; no images generated or replaced','source_sha256':{p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in ['src/generated_registry.rs','assets/data/godot_model_map.model_map.ron','src/capture_api/portrait_export.rs']},'registry_icon_count':len(rows),'unresolved_count':len(rows)-len(existing),'already_provenance_count':len(existing),'entity_count':len(rows),'unique_runtime_paths':len(paths),'shared_icon_paths':{k:v for k,v in paths.items() if len(v)>1},'missing_faction_membership':[r['entity_id'] for r in rows if not r['available_factions']],'missing_runtime_files':[r['runtime_path'] for r in rows if not (ROOT/'assets'/r['runtime_path']).is_file()],'entities':rows,'roster_preview':{'runtime_path':'ui/icons/RosterPreview.png','entity_id':None,'references':['src/menu.rs:363','src/lib.rs:2603'],'scope':'separate composed roster image; not a registry entity portrait'},'exporter':{'source':'src/capture_api/portrait_export.rs','current_hardcoded_ids':['AdvancedReactorPlant','CommandCenter','Tank'],'entrypoint':'export_portrait_stage','required_extension':'iterate validated entities and portrait_faction; retain actual spawn, rest-pose geometry parity, assembled AABB framing, RGBA alpha and independent rerender checks'}}
assert len(rows)==70 and len(paths)==70 and len(existing)==8
(ROOT/'art/portraits/entities.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:report[k] for k in ['entity_count','unique_runtime_paths','shared_icon_paths','missing_faction_membership','missing_runtime_files']}))
