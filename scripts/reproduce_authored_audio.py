#!/usr/bin/env python3
"""Reproduce six locally synthesized assets using pinned FFmpeg 9.0.1.

Usage: python3 scripts/reproduce_authored_audio.py /tmp/rts-audio-run
Outputs mirror assets/ under that new directory, plus reproduction.json.
Historical oscillator/mix recipes live in docs/asset-source-hashes.json.
No files are copied into the game automatically.
"""
import array
import hashlib
import json
import pathlib
import shlex
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSION = '9.0.1'
PATHS = {
    'assets/sfx/error.wav': 0.36,
    'assets/sfx/explosion_small.wav': 0.55,
    'assets/sfx/structure_lost.wav': 0.40,
    'assets/sfx/support_power_fire.wav': 0.36,
    'assets/sfx/weapon_hit.wav': 0.12,
    'assets/music/rts_battle_loop.ogg': 24.0,
}
SEED = 20261008


def run(*args):
    return subprocess.check_output(args, stderr=subprocess.PIPE)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    output = pathlib.Path(sys.argv[1]).resolve()
    if output.exists():
        raise SystemExit('Refusing to replace an existing reproduction directory')
    ffmpeg = pathlib.Path(shutil.which('ffmpeg') or '').resolve()
    version = run(str(ffmpeg), '-version').decode().splitlines()[0]
    if not version.startswith(f'ffmpeg version {VERSION} '):
        raise SystemExit(f'Requires FFmpeg {VERSION}; found {version}')
    inventory = json.loads((ROOT / 'docs/asset-source-hashes.json').read_text())
    indexed = {entry['path']: entry for entry in inventory['assets']}
    output.mkdir(parents=True)
    report = {
        'ffmpeg_version': version,
        'ffmpeg_executable': str(ffmpeg),
        'ffmpeg_executable_sha256': digest(ffmpeg),
        'noise_seed': SEED,
        'script_sha256': digest(pathlib.Path(__file__).resolve()),
        'assets': {},
    }
    for relative, duration in PATHS.items():
        target = output / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        args = shlex.split(indexed[relative]['generation_record']['recipe'])[1:]
        for index, value in enumerate(args):
            if value.startswith('anoisesrc='):
                args[index] = value + f':seed={SEED}'
        if relative.endswith('/error.wav'):
            index = args.index('-filter_complex') + 1
            # Dynamic amix normalization depends on delayed input EOF timing.
            # Fixed half-gain preserves the two-tone level deterministically.
            args[index] = args[index].replace(
                'amix=inputs=2:duration=longest,',
                'amix=inputs=2:duration=longest:normalize=0,volume=0.5,',
            )
        if relative.endswith('.ogg'):
            index = args.index('-af') + 1
            # Preserve the 24-second oscillator loop; compensate limiter delay
            # and smoothly join endpoints without altering its internal rhythm.
            args[index] = args[index].replace('alimiter=limit=0.85', 'alimiter=limit=0.85:latency=1')
            args[index] += ',afade=t=in:st=0:d=0.004,afade=t=out:st=23.996:d=0.004'
        args[-1:] = ['-map_metadata', '-1', '-fflags', '+bitexact', '-flags:a', '+bitexact', str(target)]
        command = [str(ffmpeg), '-cpuflags', '0', '-filter_threads', '1', '-filter_complex_threads', '1', *args]
        subprocess.run(command, check=True, stdout=subprocess.DEVNULL)
        decoded = run(str(ffmpeg), '-v', 'error', '-i', str(target), '-f', 'f32le', '-acodec', 'pcm_f32le', '-')
        samples = array.array('f')
        samples.frombytes(decoded)
        if sys.byteorder != 'little':
            samples.byteswap()
        actual_duration = len(samples) / 44100
        peak = max(abs(sample) for sample in samples)
        if abs(actual_duration - duration) > 1 / 44100 or peak >= 1:
            raise SystemExit(f'Invalid duration/clipping: {relative}: {actual_duration}s peak={peak}')
        probe = json.loads(run('ffprobe', '-v', 'error', '-show_entries', 'stream=sample_rate,channels,codec_name', '-of', 'json', str(target)))['streams'][0]
        if probe['sample_rate'] != '44100' or probe['channels'] != 1:
            raise SystemExit(f'Unexpected audio format: {relative}: {probe}')
        report['assets'][relative] = {
            'command': command,
            'sha256': digest(target),
            'decoded_float_pcm_sha256': hashlib.sha256(decoded).hexdigest(),
            'sample_count': len(samples),
            'duration_seconds': actual_duration,
            'peak_absolute': peak,
            'loop_boundary_jump': abs(samples[0] - samples[-1]),
            'codec': probe['codec_name'],
            'sample_rate': 44100,
            'channels': 1,
        }
    (output / 'reproduction.json').write_text(json.dumps(report, indent=2) + '\n')
    print(f'Validated six generated assets: {output / "reproduction.json"}')


if __name__ == '__main__':
    main()
