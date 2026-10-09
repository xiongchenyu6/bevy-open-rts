"""Install inspected original crests; keep historical hashes outside runtime."""
from pathlib import Path
import hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[2]
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
if __name__=='__main__':
 reviewed=json.loads((ROOT/'screenshots/steam-quality/original-faction-emblems-01/visual-review.json').read_text());assert reviewed['visual_review']['all_three_distinct_and_readable'];assert all(r['exit']==0 for r in reviewed['results'])
 candidates=ROOT/'art/faction-emblems/candidates-v1';manifest=json.loads((candidates/'manifest.json').read_text());backup=ROOT/'target/faction-emblem-legacy-originals-20261010';backup.mkdir(exist_ok=False);changes=[]
 inventory_path=ROOT/'docs/asset-source-hashes.json';inventory=json.loads(inventory_path.read_text());by_path={r['path']:r for r in inventory['assets']}
 for row in manifest['records']:
  path='assets/ui/factions/'+row['faction']+'_emblem.png';assert digest(candidates/(row['faction']+'.png'))==row['png_sha256'];assert digest(ROOT/path)==by_path[path]['sha256']
 for row in manifest['records']:
  name=row['faction'];source=candidates/(name+'.png');path='assets/ui/factions/'+name+'_emblem.png';target=ROOT/path;old=digest(target);shutil.copy2(target,backup/target.name);shutil.copy2(source,target)
  change={'path':path,'old_sha256':old,'sha256':digest(target),'svg':'art/faction-emblems/candidates-v1/'+name+'.svg','svg_sha256':row['svg_sha256']};changes.append(change);by_path[path]['sha256']=change['sha256'];by_path[path]['authored_vector_replacement']={**change,'generator':'art/faction-emblems/generate.py','rasterizer':manifest['rasterizer'],'scope':'Original vector crest; actual native1280setup/dropdown reviewed'}
 inventory_path.write_text(json.dumps(inventory,indent=2)+'\n');(ROOT/'art/faction-emblems/runtime-integration.json').write_text(json.dumps({'scope':'Original3vectorcrest replacements, unchanged runtime paths; actual1280sharedmenu reviewed; broader layouts/browser pending','changes':changes,'legacy_backup':str(backup.relative_to(ROOT))},indent=2)+'\n')
