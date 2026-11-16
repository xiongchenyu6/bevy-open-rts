#!/usr/bin/env python3
"""Seal the deployed runtime files into a reproducible service-worker manifest."""
import argparse
import hashlib
import json
from pathlib import Path

TOKEN = '/* BUILD_MANIFEST */ null'
EXCLUDED = {'sw.js', 'asset-manifest.json', 'build.json', 'release-index.json', '.nojekyll'}


def resource_group(path: str):
    if path.startswith(('THIRD_PARTY_NOTICES/', 'assets/licenses/')) or path.endswith(('.md', '.txt', '.d.ts', '.br')):
        return 'optional'
    if not path.startswith('assets/') or path.startswith(('assets/fonts/', 'assets/ui/', 'assets/campaign/', 'assets/data/', 'assets/migration/', 'assets/translations/', 'assets/logos/')):
        return 'core'
    return 'combat'


def generate(root: Path, revision: str, template: Path):
    files = []
    for path in sorted(root.rglob('*')):
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        if relative in EXCLUDED or relative.startswith(('_resources/', '_releases/')):
            continue
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        url = f'_resources/{digest}/{path.name}'
        entry = {'path': relative, 'sha256': digest, 'bytes': len(data),
                 'group': resource_group(relative), 'url': url}
        immutable = root / url
        immutable.parent.mkdir(parents=True, exist_ok=True)
        immutable.write_bytes(data)
        if relative.endswith('.wasm'):
            entry['encoding'] = 'raw'
        elif relative.endswith('.wasm.gz'):
            entry['encoding'] = 'gzip'
        files.append(entry)
    if not any(f['path'] == 'index.html' for f in files):
        raise ValueError('index.html is required')
    source = template.read_text()
    canonical = json.dumps({'files': files, 'worker': hashlib.sha256(source.encode()).hexdigest()}, sort_keys=True, separators=(',', ':'))
    manifest = {'version': hashlib.sha256(canonical.encode()).hexdigest(),
                'revision': revision, 'files': files}
    if source.count(TOKEN) != 1:
        raise ValueError('SW template must contain exactly one manifest placeholder')
    serialized = json.dumps(manifest, separators=(',', ':'))
    (root / 'asset-manifest.json').write_text(serialized + '\n')
    (root / '_releases').mkdir(exist_ok=True)
    (root / '_releases' / (manifest['version'] + '.json')).write_text(serialized + '\n')
    (root / 'sw.js').write_text(source.replace(TOKEN, serialized))
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--template', type=Path, default=Path('web/sw.js'))
    args = parser.parse_args()
    generate(args.root, args.revision, args.template)
