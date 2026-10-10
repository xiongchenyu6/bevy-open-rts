#!/usr/bin/env python3
"""Stage the audited portrait roster and preserve reproducible QA evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from verify_portrait_stage import verify


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--inventory', type=Path)
    parser.add_argument('--timeout', type=float, default=1200)
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error('--timeout must be positive')
    repo = Path(__file__).resolve().parents[1]
    binary = args.binary.resolve(strict=True)
    inventory = (args.inventory or repo / 'art/portraits/entities.json').resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    assets = {str(p.relative_to(repo)): sha(p) for p in sorted((repo / 'assets').rglob('*'))
              if p.is_file() and not p.name.endswith('.full.ttf')}
    metadata = {
        'binary': str(binary), 'binary_sha256': sha(binary),
        'inventory': str(inventory), 'inventory_sha256': sha(inventory),
        'asset_sha256': assets,
        'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
        'command': [str(binary), str(inventory), str(output / 'renders')],
        'scope': 'Asset-only model export; paid gameplay and visual acceptance remain separate',
        'runtime_replaced': False, 'reviewer_visual_acceptance': False,
    }
    (output / 'provenance-start.json').write_text(json.dumps(metadata, indent=2) + '\n')
    started = time.monotonic()
    code = 1
    try:
        with (output / 'export.log').open('x') as log:
            child = subprocess.run(metadata['command'], cwd=repo,
                                   env=dict(os.environ, BEVY_ASSET_ROOT=str(repo)),
                                   stdout=log, stderr=subprocess.STDOUT, timeout=args.timeout)
        code = child.returncode
        metadata['child_exit'] = code
        if code == 0:
            report = verify(inventory, output / 'renders')
            (output / 'independent-verification.json').write_text(json.dumps(report, indent=2) + '\n')
            metadata['verification_passed'] = True
            changed = [name for name, digest in assets.items()
                       if not (repo / name).is_file() or sha(repo / name) != digest]
            current_paths = {str(p.relative_to(repo)) for p in (repo / 'assets').rglob('*')
                             if p.is_file() and not p.name.endswith('.full.ttf')}
            added = sorted(current_paths - assets.keys())
            metadata['assets_changed_during_export'] = changed
            metadata['assets_added_during_export'] = added
            if changed or added:
                raise RuntimeError('Runtime assets changed during export')
            if sha(binary) != metadata['binary_sha256'] or sha(inventory) != metadata['inventory_sha256']:
                raise RuntimeError('Binary or inventory changed during export')
    except subprocess.TimeoutExpired as error:
        code = 124
        metadata['error'] = str(error)
    except Exception as error:
        code = 1
        metadata['error'] = repr(error)
    finally:
        metadata.update(exit_code=code, elapsed_seconds=time.monotonic() - started)
        (output / 'provenance.json').write_text(json.dumps(metadata, indent=2) + '\n')
        (output / 'exit.json').write_text(json.dumps({'exit_code': code}, indent=2) + '\n')
    return code


if __name__ == '__main__':
    raise SystemExit(main())
