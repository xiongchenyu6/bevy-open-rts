#!/usr/bin/env bash
# Package an already-built Linux executable; this never changes the build inputs.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="${1:-$ROOT/target/release/bevy-open-rts}"
OUTPUT="${2:-$ROOT/dist}"
for tool in python3 patchelf strip tar; do
  command -v "$tool" >/dev/null || { echo "Required tool missing: $tool" >&2; exit 1; }
done
[[ -f "$BINARY" ]] || { echo "Build the native binary first, or pass its path." >&2; exit 1; }
python3 - "$ROOT" "$BINARY" "$OUTPUT" <<'PY'
import hashlib, json, os, pathlib, shutil, subprocess, sys
root, binary, output = map(lambda p: pathlib.Path(p).resolve(), sys.argv[1:])
name = 'bevy-open-rts-linux-x86_64'
stage = output / name
if stage.exists():
    raise SystemExit(f'Refusing to replace existing package directory: {stage}')
with binary.open('rb') as stream:
    header = stream.read(20)
if header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') != 62:
    raise SystemExit('Expected a Linux x86_64 ELF executable')
output.mkdir(parents=True, exist_ok=True)
subprocess.run(['patchelf', '--print-interpreter', str(binary)], check=True, capture_output=True)
(stage / 'bin').mkdir(parents=True)
(stage / 'lib').mkdir()
shutil.copytree(root / 'assets', stage / 'assets', ignore=shutil.ignore_patterns('*.full.ttf'))
exe = stage / 'bin' / 'bevy-open-rts'
shutil.copy2(binary, exe)
exe.chmod(0o755)
subprocess.run(['strip', '--strip-debug', str(exe)], check=True)
loader = subprocess.check_output(['patchelf', '--print-interpreter', str(binary)], text=True).strip()
queue = [str(binary), loader]
# Winit loads these at runtime, so ldd alone does not include them.
for soname in ['libxkbcommon.so.0', 'libwayland-cursor.so.0', 'libwayland-egl.so.1']:
    found = next((pathlib.Path(p) / soname for p in os.environ.get('LD_LIBRARY_PATH', '').split(':') if p and (pathlib.Path(p) / soname).exists()), None)
    if found is None:
        raise SystemExit(f'Cannot locate runtime dependency {soname}; set LD_LIBRARY_PATH to the build runtime.')
    queue.append(str(found))
seen = set()
provenance = {}
while queue:
    path = pathlib.Path(queue.pop()).resolve()
    if str(path) in seen:
        continue
    seen.add(str(path))
    if path != binary:
        soname = subprocess.run(['patchelf', '--print-soname', str(path)], text=True, capture_output=True, check=True).stdout.strip() or path.name
        target = stage / 'lib' / soname
        if target.exists() and target.read_bytes() != path.read_bytes():
            raise SystemExit(f'Conflicting library versions for {soname}')
        shutil.copy2(path, target)
        target.chmod(0o755)
        provenance[soname] = str(path)
    deps = subprocess.run(['ldd', str(path)], text=True, capture_output=True)
    if 'not found' in deps.stdout:
        raise SystemExit(f'Unresolved dependencies for {path}:\n{deps.stdout}')
    for line in deps.stdout.splitlines():
        words = line.split()
        if '=>' in words:
            dep = words[words.index('=>') + 1]
        elif words and words[0].startswith('/'):
            dep = words[0]
        else:
            continue
        if dep.startswith('/'):
            queue.append(dep)
for elf in [exe, *(path for path in (stage / 'lib').iterdir() if path.name != 'ld-linux-x86-64.so.2')]:
    subprocess.run(['patchelf', '--set-rpath', '$ORIGIN' if elf.parent.name == 'lib' else '$ORIGIN/../lib', str(elf)], check=True)
# The launcher invokes the bundled loader directly; this interpreter also avoids Nix paths.
subprocess.run(['patchelf', '--set-interpreter', '/lib64/ld-linux-x86-64.so.2', str(exe)], check=True)
launcher = stage / 'launch.sh'
launcher.write_text('''#!/bin/sh
set -eu
INSTALL_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export BEVY_ASSET_ROOT="$INSTALL_DIR"
# Include host and Steam container drivers in the bundled loader search.
HOST_LIB_DIRS=/usr/lib/pressure-vessel/overrides/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu:/usr/lib64:/lib64:/usr/lib:/lib
export XKB_CONFIG_ROOT="${XKB_CONFIG_ROOT:-/usr/share/X11/xkb}"
export XLOCALEDIR="${XLOCALEDIR:-/usr/share/X11/locale}"
export ALSA_CONFIG_PATH="${ALSA_CONFIG_PATH:-/usr/share/alsa/alsa.conf}"
export ALSA_CONFIG_DIR="${ALSA_CONFIG_DIR:-/usr/share/alsa}"
export ALSA_PLUGIN_DIR="${ALSA_PLUGIN_DIR:-/usr/lib/x86_64-linux-gnu/alsa-lib}"
case "${XDG_DATA_HOME:-}" in
  /*) ;;
  *) : "${HOME:?HOME or an absolute XDG_DATA_HOME must identify the user data location}"
     export XDG_DATA_HOME="$HOME/.local/share" ;;
esac
mkdir -p "$XDG_DATA_HOME/bevy-open-rts"
# Keep the working directory outside the installation, which may be read-only.
cd "$XDG_DATA_HOME/bevy-open-rts"
exec "$INSTALL_DIR/lib/ld-linux-x86-64.so.2" --library-path "$INSTALL_DIR/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}:$HOST_LIB_DIRS" "$INSTALL_DIR/bin/bevy-open-rts" "$@"
''')
launcher.chmod(0o755)
shutil.copy2(root / 'docs' / 'PLAYER_GUIDE.md', stage / 'README.md')
shutil.copy2(root / 'docs' / 'DESKTOP_DELIVERY.md', stage / 'BUILD_NOTES.md')
(stage / 'docs').mkdir()
for notice in ['ASSET_PROVENANCE.md', 'asset-source-hashes.json']:
    shutil.copy2(root / 'docs' / notice, stage / 'docs' / notice)
notices = root / 'docs' / 'third-party' / 'linux-runtime'
notice_manifest = json.loads((notices / 'manifest.json').read_text())
actual_providers = {str(pathlib.Path(path).parents[1]) for path in provenance.values()}
recorded_providers = {row['packaged_store_path'] for row in notice_manifest['components']}
if actual_providers != recorded_providers:
    raise SystemExit('Bundled library providers changed; regenerate exact-source runtime notices')
library_identities = notice_manifest.get('library_source_sha256', {})
if library_identities:
    if set(library_identities) != set(provenance):
        raise SystemExit('Bundled library identity inventory differs from executable closure')
    for soname, identity in library_identities.items():
        provider = pathlib.Path(provenance[soname])
        if str(provider) != identity['provider_file']:
            raise SystemExit(f'Library provider differs from notice provenance: {soname}')
        with provider.open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != identity['sha256']:
                raise SystemExit(f'Library bytes differ from notice provenance: {soname}')
for row in notice_manifest['components']:
    for notice_name, digest in row['notices'].items():
        path = notices / row['component'] / notice_name
        with path.open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != digest:
                raise SystemExit(f'Runtime notice hash mismatch: {path}')
shutil.copytree(notices, stage / 'docs' / 'third-party' / 'linux-runtime')
shutil.copy2(root / 'docs' / 'RUST_DEPENDENCY_MATERIALS.md', stage / 'RUST_DEPENDENCY_MATERIALS.md')
# These are curated source-component notices, not complete license clearance.
shutil.copytree(root / 'docs' / 'third-party' / 'rust', stage / 'docs' / 'third-party' / 'rust')
shutil.copy2(root / 'ASSETS.md', stage / 'ASSETS.md')
for license_file in root.glob('LICENSE*'):
    if license_file.is_file():
        shutil.copy2(license_file, stage / license_file.name)
(stage / 'BUILD_INFO.json').write_text(json.dumps({
    'rust_notice_sha256': {
        path.relative_to(stage).as_posix(): hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()
        for path in sorted((stage / 'docs' / 'third-party' / 'rust').rglob('*')) if path.is_file()
    },
    'runtime_asset_sha256': {
        path.relative_to(stage).as_posix(): hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()
        for path in sorted((stage / 'assets').rglob('*')) if path.is_file()
    },
    'source_binary': str(binary), 'source_sha256': hashlib.file_digest(binary.open('rb'), 'sha256').hexdigest(),
    'packaged_binary_sha256': hashlib.file_digest(exe.open('rb'), 'sha256').hexdigest(),
    'binary_profile': 'development' if 'debug' in binary.parts else 'release',
    'build_variant': os.environ.get('BEVY_OPEN_RTS_BUILD_VARIANT', 'unrecorded'),
    'build_context': json.loads(pathlib.Path(os.environ['BEVY_OPEN_RTS_BUILD_CONTEXT']).read_text()) if os.environ.get('BEVY_OPEN_RTS_BUILD_CONTEXT') else None,
    'bundled_libraries': provenance,
    'note': 'Library provenance is recorded for auditing; distribution license/source obligations remain a release gate.'
}, indent=2) + '\n')
archive = output / f'{name}.tar.gz'
if archive.exists():
    raise SystemExit(f'Refusing to replace archive: {archive}')
subprocess.run(['tar', '-czf', str(archive), '-C', str(output), name], check=True)
subprocess.run([sys.executable, str(root / 'scripts' / 'verify_package.py'), str(archive)], check=True)
with archive.open('rb') as stream:
    checksum = hashlib.file_digest(stream, 'sha256').hexdigest()
(archive.parent / (archive.name + '.sha256')).write_text(f'{checksum}  {archive.name}\n')
print(f'Archive: {archive}\nSHA256: {checksum}')
PY
