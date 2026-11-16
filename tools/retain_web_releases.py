#!/usr/bin/env python3
"""Keep immutable resources for the current and two previous published releases."""
import argparse
import concurrent.futures
import hashlib
import json
import time
import urllib.error
import urllib.request
from pathlib import Path


def download(base, path, optional=False):
    for attempt in range(4):
        try:
            url = base.rstrip('/') + '/' + path
            if path == 'release-index.json':
                url += '?release-check=' + str(time.time_ns())
            request = urllib.request.Request(url, headers={'Cache-Control': 'no-cache'})
            with urllib.request.urlopen(request, timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if optional and error.code == 404:
                return None
            if error.code < 500 or attempt == 3:
                raise
        except (TimeoutError, OSError):
            if attempt == 3:
                raise
        time.sleep(attempt + 1)


def retain(root, base):
    current = json.loads((root / 'asset-manifest.json').read_text())
    previous = download(base, 'release-index.json', optional=True)
    history = json.loads(previous)['releases'] if previous else []
    versions = [v for v in history if v != current['version']][:2]
    manifests = []
    for version in versions:
        if len(version) != 64 or any(c not in '0123456789abcdef' for c in version):
            raise ValueError('Invalid release identity')
        name = f'_releases/{version}.json'
        data = download(base, name)
        manifest = json.loads(data)
        if manifest['version'] != version:
            raise ValueError('Release manifest mismatch')
        (root / name).write_bytes(data)
        manifests.append(manifest)
    needed = {}
    for manifest in manifests:
        for file in manifest['files']:
            expected = f"_resources/{file['sha256']}/{Path(file['path']).name}"
            if file.get('url') != expected:
                raise ValueError('Invalid immutable resource address')
            needed[expected] = file

    def restore(item):
        name, file = item
        path = root / name
        if path.exists():
            return
        data = download(base, name)
        if len(data) != file['bytes'] or hashlib.sha256(data).hexdigest() != file['sha256']:
            raise ValueError('Historical resource integrity failure: ' + name)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        list(pool.map(restore, needed.items()))
    (root / 'release-index.json').write_text(json.dumps({'releases': [current['version'], *versions]}, indent=2) + '\n')
    old_hashes = {f['sha256'] for f in manifests[0]['files']} if manifests else set()
    report = {}
    for group in sorted({f['group'] for f in current['files']}):
        files = {f['sha256']: f for f in current['files'] if f['group'] == group and f.get('encoding') != 'raw'}
        report[group] = {'total_bytes': sum(f['bytes'] for f in files.values()),
                         'changed_bytes': sum(f['bytes'] for h, f in files.items() if h not in old_hashes)}
    print(json.dumps({'release': current['version'], 'retained_releases': len(versions), 'groups': report}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('--base-url', required=True)
    args = parser.parse_args()
    retain(args.root, args.base_url)
