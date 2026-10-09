#!/usr/bin/env python3
"""Package GNU executable inside the matching cross-build container."""
import hashlib
import json
import pathlib
import re
import shutil
import struct
import subprocess
import sys
import zipfile

root = pathlib.Path(__file__).resolve().parent.parent
profile = sys.argv[1]
package_tag = sys.argv[2] if len(sys.argv) > 2 else ''
if not re.fullmatch(r'[a-zA-Z0-9_-]*', package_tag):
    raise SystemExit('Package tag must contain letters, numbers, underscores or hyphens')
if profile not in ('debug', 'release'):
    raise SystemExit('Profile must be debug or release')
binary = root / 'target/windows-cross/x86_64-pc-windows-gnu' / profile / 'bevy-open-rts.exe'
name = f'bevy-open-rts-windows-gnu-x86_64-{profile}' + (f'-{package_tag}' if package_tag else '')
stage = root / 'dist' / name
archive = stage.with_suffix('.zip')
if stage.exists() or archive.exists():
    raise SystemExit('Refusing to replace an existing Windows package')
with binary.open('rb') as stream:
    if stream.read(2) != b'MZ':
        raise SystemExit('Expected PE executable')
    stream.seek(0x3c)
    offset = struct.unpack('<I', stream.read(4))[0]
    stream.seek(offset)
    if stream.read(4) != b'PE\0\0' or stream.read(2) != b'\x64\x86':
        raise SystemExit('Expected AMD64 PE executable')
stage.mkdir(parents=True)
shutil.copy2(binary, stage / binary.name)
subprocess.run(['x86_64-w64-mingw32-strip', '--strip-debug', str(stage / binary.name)], check=True)
shutil.copytree(root / 'assets', stage / 'assets', ignore=shutil.ignore_patterns('*.full.ttf', *(f'mission_{i:02}.jpg' for i in range(1, 7))))
shutil.copy2(root / 'docs/PLAYER_GUIDE.md', stage / 'README.md')
shutil.copy2(root / 'docs/WINDOWS_DELIVERY.md', stage / 'BUILD_NOTES.md')
for notice in ('ASSET_PROVENANCE.md', 'asset-source-hashes.json'):
    shutil.copy2(root / 'docs' / notice, stage / notice)
shutil.copy2(root / 'ASSETS.md', stage / 'ASSETS.md')
for license_file in root.glob('LICENSE*'):
    if license_file.is_file():
        shutil.copy2(license_file, stage / license_file.name)
# GNU runtime DLLs are installed by the exact compiler used for this executable.
# Copy them beside it, then retain all PE imports for dependency auditing.
imports = subprocess.check_output(['x86_64-w64-mingw32-objdump', '-p', str(binary)], text=True)
required = {name.lower() for name in re.findall(r'DLL Name:\s*(\S+)', imports)}
runtime_files = []
candidates = {'libgcc_s_seh-1.dll', 'libstdc++-6.dll', 'libwinpthread-1.dll'}
while pending := (required & candidates) - set(runtime_files):
    dll = sorted(pending)[0]
    path = pathlib.Path(subprocess.check_output(
        ['x86_64-w64-mingw32-gcc', f'-print-file-name={dll}'], text=True).strip())
    if path.is_file():
        shutil.copy2(path, stage / dll)
        runtime_files.append(dll)
        dependencies = subprocess.check_output(['x86_64-w64-mingw32-objdump', '-p', str(path)], text=True)
        required.update(name.lower() for name in re.findall(r'DLL Name:\s*(\S+)', dependencies))
    else:
        raise SystemExit(f'Required GNU runtime DLL missing: {dll}')
(stage / 'PE_IMPORTS.txt').write_text(imports)
launcher = '''@echo off
setlocal
if not defined LOCALAPPDATA exit /b 1
set "BEVY_ASSET_ROOT=%~dp0"
set "GAME_DATA=%LOCALAPPDATA%\\bevy-open-rts"
if not exist "%GAME_DATA%" mkdir "%GAME_DATA%"
if not exist "%GAME_DATA%" exit /b 1
cd /d "%GAME_DATA%"
if errorlevel 1 exit /b 1
"%~dp0bevy-open-rts.exe" %*
exit /b %errorlevel%
'''
(stage / 'launch.cmd').write_bytes(launcher.replace('\n', '\r\n').encode('ascii'))
commit = subprocess.check_output(['git', '-c', f'safe.directory={root}', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
(stage / 'BUILD_INFO.json').write_text(json.dumps({
    'target': 'x86_64-pc-windows-gnu', 'profile': profile,
    'package_tag': package_tag,
    'rust_version': subprocess.check_output(['rustc', '--version'], text=True).strip(),
    'source_commit': commit,
    'source_worktree_dirty': bool(subprocess.check_output(['git', '-c', f'safe.directory={root}', '-C', str(root), 'status', '--porcelain'])),
    'binary_sha256': hashlib.file_digest(binary.open('rb'), 'sha256').hexdigest(),
    'runtime_dlls': runtime_files,
    'packaged_binary_sha256': hashlib.file_digest((stage / binary.name).open('rb'), 'sha256').hexdigest(),
    'debug_sections_stripped': True,
    'validation': 'Cross-build package; requires actual Wine and clean Windows runtime validation.',
}, indent=2) + '\n')
with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
    for path in sorted(stage.rglob('*')):
        if path.is_file():
            bundle.write(path, path.relative_to(stage.parent))
checksum = hashlib.file_digest(archive.open('rb'), 'sha256').hexdigest()
archive.with_name(archive.name + '.sha256').write_text(f'{checksum}  {archive.name}\n')
print(f'Archive: {archive}\nSHA256: {checksum}')
