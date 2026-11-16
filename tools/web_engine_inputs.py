#!/usr/bin/env python3
"""Fingerprint compiled inputs while excluding independent runtime content.

Literal include_bytes!/include_str! paths support quoted and raw Rust strings.
Explicit #[cfg(test)] items are skipped only for external-include discovery; all
source bytes remain fingerprinted. Nonliteral macros require audited EXTRA_INPUTS
entries. This is deliberately conservative, not a general Rust dependency parser.
"""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path

TREES = ('src', 'vendor', 'crates', '.cargo')
FILES = ('Cargo.toml', 'Cargo.lock', 'build.rs', 'rust-toolchain', 'rust-toolchain.toml',
         'scripts/build_web.sh', 'tools/web_engine_inputs.py')
RAW_STRING = re.compile(r'(?:b|c)?r(\#*)"')
CHAR_LITERAL = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x..|.)|[^'\\])'")
INCLUDE_CALL = re.compile(r'\binclude_(?:bytes|str)!\s*\(')
# Nonliteral include paths require an audited source -> root-relative files entry.
EXTRA_INPUTS = {}
INCLUDE = re.compile(
    r'\binclude_(?:bytes|str)!\s*\(\s*(?:"(?P<quoted>(?:[^"\\]|\\.)*)"|'
    r'r(?P<hashes>\#*)"(?P<raw>.*?)"(?P=hashes))\s*[,)]', re.DOTALL)


def release_source(source: str):
    """Exclude explicitly test-only items when discovering embedded external files.

    Keep character positions while hiding comments/literals for brace matching;
    nested block comments, raw strings and quoted braces must not end an item.
    The source bytes themselves are still part of the conservative fingerprint.
    """
    mask = list(source)
    index = 0
    while index < len(source):
        end = index
        if source.startswith('//', index):
            end = source.find('\n', index)
            if end < 0:
                end = len(source)
        elif source.startswith('/*', index):
            end, depth = index + 2, 1
            while end < len(source) and depth:
                if source.startswith('/*', end):
                    depth += 1
                    end += 2
                elif source.startswith('*/', end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
        else:
            raw = RAW_STRING.match(source, index)
            char = CHAR_LITERAL.match(source, index)
            if raw:
                terminator = '"' + raw[1]
                found = source.find(terminator, raw.end())
                end = len(source) if found < 0 else found + len(terminator)
            elif char:
                end = char.end()
            elif source[index] == '"':
                end = index + 1
                while end < len(source):
                    if source[end] == '\\':
                        end += 2
                    elif source[end] == '"':
                        end += 1
                        break
                    else:
                        end += 1
        if end > index:
            mask[index:end] = ' ' * (end - index)
            index = end
        else:
            index += 1
    code = ''.join(mask)
    result = list(source)
    for match in re.finditer(r'\#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]', code):
        end = match.end()
        depth = 0
        while end < len(code):
            if code[end] == '{':
                depth += 1
            elif code[end] == '}':
                depth -= 1
                if depth == 0:
                    end += 1
                    break
            elif code[end] == ';' and depth == 0:
                end += 1
                break
            end += 1
        result[match.start():end] = ' ' * (end - match.start())
    return ''.join(result)


def input_files(root: Path):
    root = root.resolve()
    paths = {path.resolve() for tree in TREES for path in (root / tree).rglob('*') if path.is_file()}
    paths.update((root / name).resolve() for name in FILES if (root / name).is_file())
    pending = list(paths)
    scanned = set()
    while pending:
        source = pending.pop()
        if source in scanned or source.suffix != '.rs':
            continue
        scanned.add(source)
        text = release_source(source.read_text(errors='replace'))
        matches = list(INCLUDE.finditer(text))
        starts = {match.start() for match in matches}
        source_name = source.relative_to(root).as_posix()
        if any(call.start() not in starts for call in INCLUDE_CALL.finditer(text)) and source_name not in EXTRA_INPUTS:
            raise ValueError(f'Nonliteral include in {source_name}; list its runtime files in EXTRA_INPUTS before reusing engines')
        for name in EXTRA_INPUTS.get(source_name, []):
            included = (root / name).resolve()
            if not included.is_file():
                raise ValueError('Missing explicit engine input: ' + name)
            if included not in paths:
                paths.add(included)
                pending.append(included)
        for match in matches:
            if match['quoted'] is not None:
                # Literal paths in Rust use the same usual quote/slash escapes.
                # Unsupported escapes fail closed instead of producing a wrong key.
                relative = json.loads('"' + match['quoted'] + '"')
            else:
                relative = match['raw']
            included = (source.parent / relative).resolve()
            if included.is_file() and included not in paths:
                paths.add(included)
                pending.append(included)
    return sorted(paths, key=lambda path: str(path.relative_to(root)))


def fingerprint(root: Path):
    root = root.resolve()
    files = [{'path': path.relative_to(root).as_posix(),
              'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
             for path in input_files(root)]
    if not any(file['path'] == 'Cargo.toml' for file in files):
        raise ValueError('Cargo.toml is required to fingerprint a web engine')
    canonical = json.dumps({'format': 1, 'files': files, 'signaling': os.environ.get('OPEN_BEVY_SIGNALING_URL', '')}, sort_keys=True, separators=(',', ':'))
    return hashlib.sha256(canonical.encode()).hexdigest(), files


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--json', action='store_true', help='Include the audited input file list')
    args = parser.parse_args()
    digest, files = fingerprint(args.root)
    print(json.dumps({'engine_inputs_sha256': digest, 'files': files}, indent=2) if args.json else digest)
