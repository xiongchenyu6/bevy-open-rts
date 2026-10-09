#!/usr/bin/env python3
"""Local first-three portrait staging and reproducible review metadata."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    from PIL import Image
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    binary = args.binary.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    sources = {str(p.relative_to(repo)): sha(p) for p in sorted((repo / 'src').rglob('*.rs'))}
    sources['Cargo.lock'] = sha(repo / 'Cargo.lock')
    assets = {str(p.relative_to(repo)): sha(p) for p in sorted((repo / 'assets').rglob('*')) if p.is_file() and not p.name.endswith('.full.ttf')}
    metadata = {'binary': str(binary), 'binary_sha256': sha(binary), 'source_sha256': sources, 'asset_sha256': assets,
                'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
                'command': [str(binary), str(output / 'renders')], 'asset_only': True, 'runtime_replaced': False,
                'live_gameplay_reference_verified': False, 'reviewer_visual_acceptance': False}
    for name in ['docs/asset-source-hashes.json', 'ASSETS.md', 'docs/ASSET_PROVENANCE.md']:
        path = repo / name
        if path.exists(): metadata.setdefault('provenance_files', {})[name] = {'sha256': sha(path), 'contents': path.read_text()}
    (output / 'provenance-start.json').write_text(json.dumps(metadata, indent=2))
    env = dict(os.environ, BEVY_ASSET_ROOT=str(repo))
    started = time.time()
    with (output / 'export.log').open('w') as log:
        result = subprocess.run(metadata['command'], cwd=repo, env=env, stdout=log, stderr=subprocess.STDOUT)
    metadata.update(exit_code=result.returncode, elapsed_seconds=time.time()-started)
    (output / 'exit.json').write_text(json.dumps({'exit_code': result.returncode, 'elapsed_seconds': metadata['elapsed_seconds']}, indent=2))
    if result.returncode:
        (output / 'provenance.json').write_text(json.dumps(metadata, indent=2))
        return result.returncode
    rows = json.loads((output / 'renders/manifest.json').read_text())
    checks = []
    for entity in ['AdvancedReactorPlant', 'CommandCenter', 'Tank']:
        first, second = [output / 'renders' / f'{entity}-{run}.png' for run in range(2)]
        a, b = Image.open(first).convert('RGBA'), Image.open(second).convert('RGBA')
        if a.size != (512, 512) or a.tobytes() != b.tobytes(): raise RuntimeError(f'{entity}: decoded determinism failed')
        alpha = a.getchannel('A')
        if alpha.getextrema() != (0, 255): raise RuntimeError(f'{entity}: transparent/opaque alpha missing')
        bounds = alpha.getbbox()
        if not bounds or min(bounds[:2]) <= 0 or max(bounds[2:]) >= 512: raise RuntimeError(f'{entity}: clipped/empty silhouette')
        for size in [512, 72]:
            icon = a.resize((size, size), Image.Resampling.LANCZOS)
            for theme, color in [('light', '#e9edf2'), ('dark', '#151d2b')]:
                card = Image.new('RGBA', (size, size), color)
                card.alpha_composite(icon)
                card.convert('RGB').save(output / f'{entity}-{size}-{theme}.png')
        checks.append({'entity': entity, 'sha256': [sha(first), sha(second)], 'byte_identical': first.read_bytes() == second.read_bytes(),
                       'decoded_identical': True, 'alpha_bounds': bounds, 'mesh_count': next(row['geometry']['mesh_count'] for row in rows if row['id'] == entity)})
    changed = [name for name, digest in assets.items() if not (repo/name).exists() or sha(repo/name) != digest]
    metadata.update(checks=checks, assets_changed_during_export=changed, image_library_version=Image.__version__)
    (output / 'provenance.json').write_text(json.dumps(metadata, indent=2))
    if changed: raise RuntimeError('assets changed during export')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
