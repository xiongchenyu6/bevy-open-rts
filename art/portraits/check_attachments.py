#!/usr/bin/env python3
"""Evaluate actual GLB hierarchy bounds under registry transforms, no PNG input."""
import itertools,json,math,pathlib,re,struct
ROOT=pathlib.Path(__file__).resolve().parents[2]
def matrix(t,r,s):
 x,y,z,w=r
 return [[(1-2*y*y-2*z*z)*s[0],(2*x*y-2*z*w)*s[1],(2*x*z+2*y*w)*s[2],t[0]],[(2*x*y+2*z*w)*s[0],(1-2*x*x-2*z*z)*s[1],(2*y*z-2*x*w)*s[2],t[1]],[(2*x*z-2*y*w)*s[0],(2*y*z+2*x*w)*s[1],(1-2*x*x-2*y*y)*s[2],t[2]],[0,0,0,1]]
def mul(a,b):return [[sum(a[i][k]*b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]
def bounds(path,part, split=False):
 b=(ROOT/'assets'/path).read_bytes();n=struct.unpack_from('<I',b,12)[0];g=json.loads(b[20:20+n]);points=[];primitive_boxes=[]
 def visit(idx,parent):
  node=g['nodes'][idx];m=mul(parent,matrix(node.get('translation',[0]*3),node.get('rotation',[0,0,0,1]),node.get('scale',[1]*3)))
  assert 'matrix' not in node,'explicit matrix requires decoder extension'
  if 'mesh' in node:
   for primitive in g['meshes'][node['mesh']]['primitives']:
    a=g['accessors'][primitive['attributes']['POSITION']]
    primitive_points=[[sum(m[i][j]*p[j] for j in range(3))+m[i][3] for i in range(3)] for p in itertools.product(*zip(a['min'],a['max']))]
    points.extend(primitive_points)
    primitive_boxes.append({'node':node.get('name'),'bounds':[[min(p[i] for p in primitive_points) for i in range(3)],[max(p[i] for p in primitive_points) for i in range(3)]]})
  for c in node.get('children',[]):visit(c,m)
 for root in g['scenes'][g.get('scene',0)]['nodes']:visit(root,part)
 if split:return primitive_boxes
 return [[min(p[i] for p in points) for i in range(3)],[max(p[i] for p in points) for i in range(3)]]
def gap(a,b):return math.sqrt(sum(max(0,a[0][i]-b[1][i],b[0][i]-a[1][i])**2 for i in range(3)))
def audit():
 s=(ROOT/'src/generated_registry.rs').read_text();out=[]
 for entity in json.loads((ROOT/'art/portraits/entities.json').read_text())['entities']:
  symbol=entity['render_parts_symbol'];m=re.search(r'const '+symbol+r': &\[RenderPart\] = &\[(.*?)\];',s,re.S) if symbol else None
  if not m:continue
  parts=[]
  for b in re.finditer(r'RenderPart \{(.*?)\}',m.group(1),re.S):
   t=b.group(1);path=re.search(r'model: "([^"]+)"',t).group(1);v=lambda k:json.loads(re.search(k+r': (\[[^]]+\])',t).group(1));parts.append({'model':path,'bounds':bounds(path,matrix(v('translation'),v('rotation'),v('scale'))),'primitives':bounds(path,matrix(v('translation'),v('rotation'),v('scale')),split=True)})
  pairs=[{'a':i,'b':j,'gap':gap(a['bounds'],b['bounds'])} for i,a in enumerate(parts) for j,b in enumerate(parts) if j>i]
  connected={0} if parts else set()
  while True:
   expanded=connected|{p['b'] for p in pairs if p['a'] in connected and p['gap']<=.035}|{p['a'] for p in pairs if p['b'] in connected and p['gap']<=.035}
   if expanded==connected:break
   connected=expanded
  out.append({'entity':entity['entity_id'],'parts':parts,'pairs':pairs,'disconnected_part_indices':[i for i in range(len(parts)) if i not in connected]})
 return out
if __name__=='__main__':
 import argparse
 p=argparse.ArgumentParser();p.add_argument('--output',type=pathlib.Path);p.add_argument('--check',action='store_true');args=p.parse_args();rows=audit()
 if args.output:args.output.write_text(json.dumps({'scope':'AABB separation candidates; intentional multi-module structures require visual review','entities':rows},indent=2)+'\n')
 if args.check:
  cryo=next(r for r in rows if r['entity']=='CryoSprayer')
  actor=cryo['parts'][0]['primitives']
  barrel=next(p for p in cryo['parts'] if p['model'].endswith('/barrel.glb'))
  for primitive in barrel['primitives']:
   measured=min(gap(primitive['bounds'],body['bounds']) for body in actor)
   assert measured<=.005,f"CryoSprayer: individual {primitive['node']} primitive gap {measured:.6f}m to actual actor meshes"
  for id in ['RocketGunship','SiegeAirship']:
   row=next(r for r in rows if r['entity']==id)
   hull=row['parts'][0]['bounds'];mounts=row['parts'][1:]
   centers=[[sum(p['bounds'][a][i] for a in [0,1])/2 for i in range(3)] for p in mounts]
   assert abs(centers[0][0]-centers[1][0])>.3,f'{id}: duplicated mounts overlap'
   for center in centers:
    for axis in [0,2]:
     assert hull[0][axis]+.1<center[axis]<hull[1][axis]-.1,f'{id}: mount touches only hull extremity'
  depot=next(r for r in rows if r['entity']=='TechRepairDepot')
  floor=depot['parts'][0]['bounds']
  for part in depot['parts'][1:]:
   for primitive in part['primitives']:
    box=primitive['bounds']
    assert box[0][1]<=floor[1][1]+.005,f"depot: unsupported {primitive['node']}"
    for axis in [0,2]:
     assert floor[0][axis]-.005<=box[0][axis] and box[1][axis]<=floor[1][axis]+.005,f"depot: {primitive['node']} outside floor"
  corrected=json.loads((ROOT/'art/portraits/attachment_transforms.json').read_text())['entities']
  for id in corrected:
   row=next(r for r in rows if r['entity']==id)
   actors=[i for i,p in enumerate(row['parts']) if pathlib.Path(p['model']).stem.startswith(('astronaut','alien'))]
   if len(actors)>1:
    # Crew members remain separate; every carried accessory contacts an actor.
    for i,part in enumerate(row['parts']):
     if i not in actors:
      actor_primitives=[primitive for j in actors for primitive in row['parts'][j]['primitives']]
      for primitive in part['primitives']:
       assert min(gap(primitive['bounds'],body['bounds']) for body in actor_primitives)<=.005, f'{id}: detached individual carried primitive {i}'
   else:
    assert not row['disconnected_part_indices'],f"{id}: detached actual GLB accessories {row['disconnected_part_indices']}"

 print(json.dumps({'audited':len(rows),'separation_candidates':[r['entity'] for r in rows if r['disconnected_part_indices']]}))
