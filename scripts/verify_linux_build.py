#!/usr/bin/env python3
"""Verify a frozen Linux build before accepting its archive as current code."""
import argparse
import hashlib
import json
import shlex
from pathlib import Path


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify(snapshot, dep_info):
    snapshot = snapshot.resolve(strict=True)
    source = snapshot / 'source'
    inputs = json.loads((snapshot / 'input.json').read_text())
    result = json.loads((snapshot / 'result.json').read_text())
    if result.get('actual_exit') != 0 or result.get('frozen_inputs_unchanged') is not True:
        raise ValueError('Build failed or frozen inputs changed')
    for name, expected in inputs['files'].items():
        path = (source / name).resolve(strict=True)
        if not path.is_relative_to(source) or digest(path) != expected:
            raise ValueError(f'Frozen input mismatch: {name}')
    binary = snapshot / 'bevy-open-rts'
    if digest(binary) != result['binary_sha256'] or binary.stat().st_size != result['binary_bytes']:
        raise ValueError('Built executable identity differs from result')
    log = (snapshot / 'build.log').read_text()
    # Cargo can silently reuse an executable built in another relocated clone.
    compiled_paths = [Path(line.rsplit('(', 1)[1].rstrip(')')).resolve()
                      for line in log.splitlines()
                      if 'Compiling bevy-open-rts ' in line and '(' in line]
    if source not in compiled_paths:
        raise ValueError('No actual game compilation from this source snapshot')
    dependencies = shlex.split(dep_info.read_text())
    if not any(token.endswith('/src/lib.rs') and Path(token).resolve() == source / 'src/lib.rs'
               for token in dependencies):
        raise ValueError('Executable dependency record points to another source snapshot')
    return {'scope': 'Frozen build provenance only; no gameplay or release acceptance',
            'passed': True, 'frozen_files': len(inputs['files']),
            'binary_sha256': result['binary_sha256']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--snapshot', type=Path, required=True)
    parser.add_argument('--dep-info', type=Path, required=True)
    args = parser.parse_args()
    try:
        result = verify(args.snapshot, args.dep_info)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({'passed': False, 'error': str(error)}))
        return 1
    print(json.dumps(result, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
