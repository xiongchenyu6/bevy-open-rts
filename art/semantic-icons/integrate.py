"""Install reviewed originals after running QA has released runtime assets."""
import hashlib
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CANDIDATES = ROOT / 'art/semantic-icons/candidates-v3'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


if __name__ == '__main__':
    backup = ROOT / 'target/semantic-icon-legacy-20261010'
    if backup.exists():
        raise SystemExit('Refusing to replace previous icon integration evidence')
    originals = json.loads((ROOT / 'docs/icon-replacement-audit.json').read_text())
    old_by_path = {item['path']: item for item in originals['icons']}
    candidates = json.loads((CANDIDATES / 'provenance.json').read_text())['icons']
    changes = []
    for item in candidates:
        relative = f"assets/ui/icons/{item['id']}.png"
        current = ROOT / relative
        candidate = CANDIDATES / item['runtime_candidate']
        assert digest(current) == old_by_path[relative]['current_sha256'], relative
        assert digest(candidate) == item['png_sha256'], relative
        changes.append({'path': relative, 'previous_sha256': digest(current),
                        'sha256': item['png_sha256'],
                        'svg': f"art/semantic-icons/candidates-v3/{item['source']}",
                        'svg_sha256': item['source_sha256']})
    assert len(changes) == 20
    backup.mkdir()
    inventory_path = ROOT / 'docs/asset-source-hashes.json'
    inventory = json.loads(inventory_path.read_text())
    indexed = {item['path']: item for item in inventory['assets']}
    for change in changes:
        destination = ROOT / change['path']
        shutil.copy2(destination, backup / destination.name)
        shutil.copy2(CANDIDATES / destination.name, destination)
        assert digest(destination) == change['sha256']
        entry = indexed[change['path']]
        entry['sha256'] = change['sha256']
        entry['authored_vector_replacement'] = {**change,
            'generator': 'art/semantic-icons/generate.py', 'rasterizer': 'resvg0.48.1',
            'scope': 'Original geometric SVG; runtime HUD review pending'}
        original = old_by_path[change['path']]
        original['audited_legacy_sha256'] = original['current_sha256']
        original['current_sha256'] = change['sha256']
        original['matches_audited_bytes'] = False
        original['replacement'] = change
    inventory_path.write_text(json.dumps(inventory, indent=2) + '\n')
    originals['scope'] += ' Twenty semantic icons now have original vector replacements; legacy hashes remain recorded separately. Runtime review pending.'
    (ROOT / 'docs/icon-replacement-audit.json').write_text(json.dumps(originals, indent=2) + '\n')
    (ROOT / 'art/semantic-icons/runtime-integration.json').write_text(json.dumps({
        'scope': 'Original semantic PNG replacements; unchanged asset paths; actual HUD review pending',
        'legacy_backup': str(backup.relative_to(ROOT)), 'changes': changes,
    }, indent=2) + '\n')
    print('Installed20 original semantic icons; preserved all legacy bytes outside runtime assets')
