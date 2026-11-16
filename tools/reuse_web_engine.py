#!/usr/bin/env python3
"""Reuse verified published engine bytes when compiled inputs are unchanged.

Exit 1 means rebuild normally. Nothing is installed into the output directory
until the receipt, manifest and all three immutable engine files are verified.
"""
import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
import urllib.request
from pathlib import Path
from urllib.parse import urljoin, urlsplit

ENGINE_FILES = ('pkg/bevy_open_rts.js', 'pkg/bevy_open_rts_bg.wasm', 'pkg/bevy_open_rts_bg.wasm.gz')
DIGEST = re.compile(r'^[0-9a-f]{64}$')


def read_json(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers={'Cache-Control': 'no-cache'}), timeout=30) as response:
        data = response.read(8 * 1024 * 1024 + 1)
    if len(data) > 8 * 1024 * 1024:
        raise ValueError('Release metadata is unexpectedly large')
    return json.loads(data)


def reuse(base_url: str, output: Path, expected_fingerprint: str):
    if not DIGEST.fullmatch(expected_fingerprint):
        raise ValueError('Invalid engine input fingerprint')
    base = base_url.rstrip('/') + '/'
    if urlsplit(base).scheme not in ('https', 'http'):
        raise ValueError('Engine source must be an HTTP(S) release')
    receipt = read_json(urljoin(base, 'build.json'))
    if receipt.get('engine_inputs_sha256') != expected_fingerprint:
        raise ValueError('Published compiled inputs differ or have no fingerprint')
    manifest = read_json(urljoin(base, 'asset-manifest.json'))
    if not receipt.get('revision') or manifest.get('revision') != receipt['revision']:
        raise ValueError('Release receipt and resource manifest revisions differ')
    entries = []
    for name in ENGINE_FILES:
        matches = [entry for entry in manifest.get('files', []) if entry.get('path') == name]
        if len(matches) != 1:
            raise ValueError('Missing or duplicate engine entry: ' + name)
        entry = matches[0]
        digest, size = entry.get('sha256'), entry.get('bytes')
        if not isinstance(digest, str) or not DIGEST.fullmatch(digest) or type(size) is not int or size <= 0:
            raise ValueError('Invalid engine integrity metadata: ' + name)
        if entry.get('url') != f'_resources/{digest}/{Path(name).name}':
            raise ValueError('Engine source is not a content-addressed resource: ' + name)
        entries.append(entry)
    wasm = entries[1]
    if (wasm['sha256'], wasm['bytes']) != (receipt.get('wasm_sha256'), receipt.get('wasm_bytes')):
        raise ValueError('WASM receipt does not match the resource manifest')
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.engine-reuse-', dir=output.parent) as temporary:
        staging = Path(temporary)
        for entry in entries:
            (staging / entry['path']).parent.mkdir(parents=True, exist_ok=True)
            digest, size = hashlib.sha256(), 0
            with urllib.request.urlopen(urljoin(base, entry['url']), timeout=30) as response, (staging / entry['path']).open('wb') as target:
                while chunk := response.read(1024 * 1024):
                    size += len(chunk)
                    if size > entry['bytes']:
                        raise ValueError('Engine resource exceeds expected size: ' + entry['path'])
                    digest.update(chunk)
                    target.write(chunk)
            if size != entry['bytes'] or digest.hexdigest() != entry['sha256']:
                raise ValueError('Engine integrity check failed: ' + entry['path'])
        output.mkdir(parents=True, exist_ok=True)
        for entry in entries:
            (output / entry['path']).parent.mkdir(parents=True, exist_ok=True)
            os.replace(staging / entry['path'], output / entry['path'])
    return {'revision': receipt['revision'], 'bytes': sum(entry['bytes'] for entry in entries),
            'engine_inputs_sha256': expected_fingerprint}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-url', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--fingerprint', required=True)
    args = parser.parse_args()
    try:
        result = reuse(args.base_url, args.output, args.fingerprint)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'Published engine reuse unavailable; rebuilding: {error}', file=sys.stderr)
        sys.exit(1)
    print(f'Reused verified web engine from {result["revision"]} ({result["bytes"]} bytes)')
