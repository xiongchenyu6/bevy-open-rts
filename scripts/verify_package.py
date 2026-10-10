#!/usr/bin/env python3
"""Check release archive identities without extraction or runtime claims."""
import argparse
import hashlib
import json
import pathlib
import stat
import tarfile
import zipfile


def verify(path):
    with zipfile.ZipFile(path) if zipfile.is_zipfile(path) else tarfile.open(path) as archive:
        files = {}
        entries = archive.infolist() if isinstance(archive, zipfile.ZipFile) else archive.getmembers()
        for entry in entries:
            name = entry.filename if isinstance(archive, zipfile.ZipFile) else entry.name
            parts = pathlib.PurePosixPath(name)
            if parts.is_absolute() or '..' in parts.parts or '\\' in name or ':' in name:
                raise ValueError(f'Unsafe archive path: {name}')
            if isinstance(archive, zipfile.ZipFile):
                if stat.S_ISLNK(entry.external_attr >> 16):
                    raise ValueError(f'Archive link: {name}')
                directory = entry.is_dir()
            else:
                if not (entry.isfile() or entry.isdir()):
                    raise ValueError(f'Unsupported archive member: {name}')
                directory = entry.isdir()
            if directory:
                continue
            if name in files:
                raise ValueError(f'Duplicate archive file: {name}')
            files[name] = entry
        manifests = [name for name in files if name.endswith('/BUILD_INFO.json')]
        if len(manifests) != 1:
            raise ValueError('Expected one BUILD_INFO.json')
        prefix = manifests[0].removesuffix('BUILD_INFO.json')
        if any(not name.startswith(prefix) for name in files):
            raise ValueError('Files outside package root')

        def read(name):
            entry = files[name]
            if isinstance(archive, zipfile.ZipFile):
                return archive.open(entry)
            return archive.extractfile(entry)

        with read(manifests[0]) as stream:
            info = json.loads(stream.read().decode('utf-8-sig'))
        counts = {}
        for field, directory in [('runtime_asset_sha256', 'assets/'), ('rust_notice_sha256', 'docs/third-party/rust/')]:
            expected = info.get(field)
            if not isinstance(expected, dict) or not expected:
                raise ValueError(f'Missing hash inventory: {field}')
            actual = {name[len(prefix):] for name in files if name.startswith(prefix + directory)}
            if actual != set(expected):
                raise ValueError(f'Inventory differs from archive: {field}')
            for name, digest in expected.items():
                with read(prefix + name) as stream:
                    if hashlib.file_digest(stream, 'sha256').hexdigest() != digest:
                        raise ValueError(f'Hash mismatch: {name}')
            counts[field] = len(expected)
        binaries = [name for name in files if name in (prefix + 'bin/bevy-open-rts', prefix + 'bevy-open-rts.exe')]
        if len(binaries) != 1 or not info.get('packaged_binary_sha256'):
            raise ValueError('Missing unique packaged executable identity')
        with read(binaries[0]) as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != info['packaged_binary_sha256']:
                raise ValueError('Packaged executable hash mismatch')
        return {'scope': 'Archive integrity only; no runtime or license clearance', 'files': len(files), **counts}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=pathlib.Path)
    args = parser.parse_args()
    try:
        print(json.dumps(verify(args.archive), indent=2))
    except (ValueError, KeyError, OSError, tarfile.TarError, zipfile.BadZipFile) as error:
        parser.exit(1, f'Package verification failed: {error}\n')
