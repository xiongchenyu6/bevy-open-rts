#!/usr/bin/env python3
"""Verify an extracted desktop package against its recorded runtime asset hashes."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath


def verify(package):
    recorded = json.loads((package / "BUILD_INFO.json").read_text())["runtime_asset_sha256"]
    if not isinstance(recorded, dict) or not recorded:
        raise ValueError("Runtime asset inventory is empty or invalid")
    for name, digest in recorded.items():
        path = PurePosixPath(name)
        if (path.is_absolute() or ".." in path.parts or len(path.parts) < 2
                or path.parts[0] != "assets" or "\\" in name
                or not isinstance(digest, str) or len(digest) != 64
                or any(c not in "0123456789abcdef" for c in digest)):
            raise ValueError(f"Invalid asset inventory entry: {name}")
    actual = {p.relative_to(package).as_posix(): p
              for p in (package / "assets").rglob("*") if p.is_file()}
    missing = sorted(recorded.keys() - actual.keys())
    extra = sorted(actual.keys() - recorded.keys())
    changed = []
    for name in sorted(recorded.keys() & actual.keys()):
        with actual[name].open("rb") as stream:
            if hashlib.file_digest(stream, "sha256").hexdigest() != recorded[name]:
                changed.append(name)
    result = {"passed": not (missing or extra or changed),
              "recorded_assets": len(recorded), "actual_assets": len(actual),
              "missing": missing, "extra": extra, "changed": changed}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path, help="Extracted directory containing BUILD_INFO.json")
    args = parser.parse_args()
    try:
        result = verify(args.package.resolve(strict=True))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({"passed": False, "error": str(error)}))
        return 1
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
