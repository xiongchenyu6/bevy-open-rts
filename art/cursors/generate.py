"""Author original RTS cursor geometry and rasterize a four-frame atlas."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

STROKE = 'stroke="#102830" stroke-width="2" stroke-linejoin="round"'
DESIGNS = {
    "default": f'<path {STROKE} fill="#a5f5e2" d="M2 1 24 17 14 19 10 29 2 1Z"/><path fill="#f4fff8" d="m5 7 12 9-6 1Z"/>',
    "move": f'<path {STROKE} fill="#a5f5e2" d="m16 2 5 7h-3v5h5v-3l7 5-7 5v-3h-5v5h3l-5 7-5-7h3v-5H9v3l-7-5 7-5v3h5V9h-3Z"/>',
    "attack": '<circle cx="16" cy="16" r="9" fill="none" stroke="#102830" stroke-width="5"/><circle cx="16" cy="16" r="9" fill="none" stroke="#ff9b85" stroke-width="2.5"/><path d="M16 2v7m0 14v7M2 16h7m14 0h7" stroke="#102830" stroke-width="5"/><path d="M16 2v7m0 14v7M2 16h7m14 0h7" stroke="#ffb29d" stroke-width="2.5"/><circle cx="16" cy="16" r="2" fill="#ffe4d5"/>',
    "build": f'<path {STROKE} fill="#f3d58b" d="m5 5 5-3 15 12-4 5-6-5-9 15-5-3 10-15Z"/><path fill="#fff0bd" d="m12 6 10 8-1 2-10-8Z"/><path d="M24 23v7m-3-3h7" stroke="#102830" stroke-width="5"/><path d="M24 23v7m-3-3h7" stroke="#f3d58b" stroke-width="2.5"/>',
}


def svg(body, width, height):
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">{body}</svg>\n'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def generate(destination):
    destination.mkdir(parents=True, exist_ok=False)
    frames = []
    for index, (name, body) in enumerate(DESIGNS.items()):
        source = destination / f"{name}.svg"
        source.write_text(svg(body, 32, 32))
        image = source.with_suffix(".png")
        subprocess.run(["resvg", str(source), str(image)], check=True)
        frames.append({"index": index, "state": name, "hotspot": [2, 1] if index == 0 else [16, 16], "svg_sha256": digest(source), "png_sha256": digest(image)})
    atlas = destination / "rts_cursor_atlas.svg"
    atlas.write_text(svg("".join(f'<g transform="translate({index * 32},0)">{body}</g>' for index, body in enumerate(DESIGNS.values())), 128, 32))
    subprocess.run(["resvg", str(atlas), str(atlas.with_suffix(".png"))], check=True)
    review = []
    for row, background in enumerate(["#15252e", "#d7c0a0"]):
        review.append(f'<rect y="{row * 180}" width="640" height="180" fill="{background}"/>')
        for index, body in enumerate(DESIGNS.values()):
            x = index * 160
            review.append(f'<g transform="translate({x + 16},{row * 180 + 16}) scale(3)">{body}</g>')
            review.append(f'<g transform="translate({x + 50},{row * 180 + 128})">{body}</g>')
    sheet = destination / "review-sheet.svg"
    sheet.write_text(svg("".join(review), 640, 360))
    subprocess.run(["resvg", str(sheet), str(sheet.with_suffix(".png"))], check=True)
    manifest = {"scope": "Original geometry candidates; runtime loading and input alignment not yet accepted", "inputs": "repository-authored SVG geometry; no images, fonts or external artwork", "source_license": "repository MIT OR Apache-2.0", "generator_sha256": digest(Path(__file__)), "rasterizer": subprocess.check_output(["resvg", "--version"], text=True).strip(), "layout": {"tile_size": [32, 32], "columns": 4, "rows": 1}, "frames": frames, "atlas_svg_sha256": digest(atlas), "atlas_png_sha256": digest(atlas.with_suffix(".png")), "runtime_replaced": False}
    (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    generate(Path(sys.argv[1]))
