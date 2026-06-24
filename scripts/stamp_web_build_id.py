#!/usr/bin/env python3
"""Stamp web/index.html with a cache-busting build id from built artifacts."""

from __future__ import annotations

import argparse
import hashlib
import re
from pathlib import Path


BUILD_ID_RE = re.compile(r'const buildId = "[^"]+";')


def artifact_hash(paths: list[Path]) -> str:
    digest = hashlib.sha256()
    for path in paths:
        digest.update(path.name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()[:16]


def stamp_index(index: Path, artifacts: list[Path]) -> str:
    missing = [path for path in artifacts if not path.is_file()]
    if missing:
        missing_text = ", ".join(str(path) for path in missing)
        raise FileNotFoundError(f"missing web artifacts: {missing_text}")

    build_id = artifact_hash(artifacts)
    text = index.read_text(encoding="utf-8")
    replacement = f'const buildId = "{build_id}";'
    updated, count = BUILD_ID_RE.subn(replacement, text, count=1)
    if count != 1:
        raise ValueError(f"{index} does not contain exactly one buildId declaration")
    index.write_text(updated, encoding="utf-8")
    return build_id


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("index", type=Path)
    parser.add_argument("artifacts", nargs="+", type=Path)
    args = parser.parse_args()

    build_id = stamp_index(args.index, args.artifacts)
    print(build_id)


if __name__ == "__main__":
    main()
