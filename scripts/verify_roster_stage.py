#!/usr/bin/env python3
"""Check paired asset candidates without claiming UI or gameplay acceptance."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess


def require(condition, message):
    if not condition:
        raise ValueError(message)


def member_identity(member):
    position = member.get("ground_position")
    require(
        isinstance(position, list) and len(position) == 2
        and all(isinstance(v, (int, float)) and not isinstance(v, bool)
                and math.isfinite(v) and abs(v) <= 20 for v in position),
        "Invalid member ground position",
    )
    # The renderer stores layout in Vec3/f32; compare that exact representation.
    normalized = [struct.unpack("!f", struct.pack("!f", v))[0] for v in position]
    return member.get("entity_id"), member.get("faction"), normalized


def verify(directory, configuration=None):
    manifest = json.loads((directory / "manifest.json").read_text())
    recorded_configuration = manifest["configuration"]
    if configuration is not None:
        require(recorded_configuration == configuration, "Recorded configuration differs from source")
    expected = [member_identity(member) for member in recorded_configuration["members"]]
    runs = manifest["runs"]
    require(
        len(runs) == 2
        and all(type(run.get("run")) is int for run in runs)
        and {run["run"] for run in runs} == {0, 1},
        "Independent run IDs must be exactly {0, 1}",
    )
    decoded = []
    results = []
    for run in sorted(runs, key=lambda row: row["run"]):
        members = run["members"]
        require([member_identity(member) for member in members] == expected,
                f"Run {run['run']} member identity/faction/position differs from configuration")
        require(all(member.get("model_reference_verified") is True
                    and member.get("paid_gameplay_verified") is False for member in members),
                "Missing model reference or incorrect gameplay scope")
        filename = f"RosterPreview-{run['run']}.png"
        require(Path(run["image"]).name == filename, "Recorded image has incorrect run identity")
        path = directory / filename
        encoded = path.read_bytes()
        png_sha = hashlib.sha256(encoded).hexdigest()
        for field in ("png_sha256", "image_sha256"):
            if field in run:
                require(run[field] == png_sha, f"Run {run['run']} recorded PNG hash differs")
        require(len(encoded) >= 26 and encoded[:8] == b"\x89PNG\r\n\x1a\n"
                and struct.unpack(">II", encoded[16:24]) == (512, 512)
                and encoded[24:26] == bytes([8, 6]), "Expected 512px RGBA8 PNG")
        rgba = subprocess.check_output([
            "ffmpeg", "-v", "error", "-threads", "1", "-i", str(path),
            "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1",
        ])
        require(len(rgba) == 512 * 512 * 4, "Incorrect decoded image dimensions")
        alpha = rgba[3::4]
        occupied = [index for index, value in enumerate(alpha) if value]
        require(bool(occupied), "Empty composition")
        bounds = [min(i % 512 for i in occupied), min(i // 512 for i in occupied),
                  max(i % 512 for i in occupied), max(i // 512 for i in occupied)]
        require(bounds[0] > 0 and bounds[1] > 0 and bounds[2] < 511 and bounds[3] < 511,
                "Clipped composition")
        require(sum(v == 0 for v in alpha) > 1000 and sum(v >= 250 for v in alpha) > 1000,
                "Missing transparent background or opaque models")
        decoded.append(rgba)
        results.append({"run": run["run"], "png_sha256": png_sha,
                        "decoded_rgba_sha256": hashlib.sha256(rgba).hexdigest(),
                        "alpha_bounds": bounds})
    require(decoded[0] == decoded[1], "Independent composition pixels differ")
    return {"scope": "Original asset-only group; pixel/model checks, no UI/gameplay acceptance",
            "runs": results, "decoded_rgba_reproduced": True,
            "normal_ui_verified": False, "runtime_replaced": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--configuration", type=Path,
                        default=Path(__file__).resolve().parents[1] / "art/portraits/roster.json")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    configuration = json.loads(args.configuration.read_text())
    report = verify(args.directory, configuration)
    output = args.output or args.directory / "independent-review.json"
    with output.open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
