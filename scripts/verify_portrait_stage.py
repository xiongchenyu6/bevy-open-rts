#!/usr/bin/env python3
"""Verify complete paired model exports without modifying their images."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify(inventory_path, directory):
    inventory = json.loads(inventory_path.read_text())
    expected = {row["entity_id"]: row for row in inventory["entities"]}
    records = json.loads((directory / "manifest.json").read_text())
    pairs = {(row["id"], row["run"]): row for row in records}
    if len(pairs) != len(records) or set(pairs) != {
        (entity, run) for entity in expected for run in (0, 1)
    }:
        raise ValueError("Missing, duplicate or unexpected portrait records")
    verified = []
    for entity, source in expected.items():
        images = [directory / f"{entity}-{run}.png" for run in (0, 1)]
        hashes = [digest(path) for path in images]
        if hashes[0] != hashes[1]:
            raise ValueError(f"Independent renders differ: {entity}")
        for run in (0, 1):
            row = pairs[entity, run]
            if row["runtime_path"] != source["runtime_path"] or row["faction"] != source["portrait_faction"]:
                raise ValueError(f"Wrong portrait identity/faction: {entity}")
            marker_count = {"alliance": 2, "demon": 3, "chaos": 2}[source["portrait_faction"]]
            if (row.get("reference_scope") != "model-only parity; not whole-scene parity"
                    or row.get("excluded_contextual_marker_count") != marker_count
                    or row["geometry"].get("excluded_contextual_marker_count") != marker_count):
                raise ValueError(f"Missing model-only scope or contextual marker exclusion: {entity}")
            if not row["asset_only_full_game_reference_verified"] or row["live_gameplay_reference_verified"]:
                raise ValueError(f"Incorrect reference scope: {entity}")
        header = images[0].read_bytes()[:33]
        if header[:8] != b"\x89PNG\r\n\x1a\n" or struct.unpack(">II", header[16:24]) != (512, 512) or header[24:26] != bytes([8, 6]):
            raise ValueError(f"Expected512px RGBA8 PNG: {entity}")
        rgba = subprocess.check_output([
            "ffmpeg", "-v", "error", "-threads", "1", "-i", str(images[0]),
            "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1",
        ])
        if len(rgba) != 512 * 512 * 4:
            raise ValueError(f"Wrong decoded image size: {entity}")
        alpha = rgba[3::4]
        if min(alpha) != 0 or max(alpha) < 250:
            raise ValueError(f"Missing transparency or opaque silhouette: {entity}")
        pixels = [index for index, value in enumerate(alpha) if value]
        bounds = [min(i % 512 for i in pixels), min(i // 512 for i in pixels),
                  max(i % 512 for i in pixels), max(i // 512 for i in pixels)]
        if bounds[0] == 0 or bounds[1] == 0 or bounds[2] >= 511 or bounds[3] >= 511:
            raise ValueError(f"Clipped silhouette: {entity}")
        verified.append({"id": entity, "runtime_path": source["runtime_path"],
                         "faction": source["portrait_faction"], "sha256": hashes[0],
                         "alpha_bounds": bounds, "independent_render_bytes_equal": True})
    return {"scope": "Paired asset-only exports; visual/readability and paid-gameplay reference remain pending",
            "inventory_sha256": digest(inventory_path), "manifest_sha256": digest(directory / "manifest.json"),
            "entity_count": len(verified), "records": verified, "runtime_replaced": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inventory", type=Path)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    report = verify(arguments.inventory, arguments.directory)
    with arguments.output.open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")
    print(f"Verified {report['entity_count']} paired RGBA model exports; visual review pending.")
