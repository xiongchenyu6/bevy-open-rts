"""Original geometric OPEN RTS wordmark; no external fonts or artwork."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

GLYPHS = {
    "O": "M16 10H48L56 18V62L48 70H16L8 62V18Z",
    "P": "M10 70V10H45L55 20V32L45 42H10",
    "E": "M54 10H10V70H54M10 40H46",
    "N": "M10 70V10L54 70V10",
    "R": "M10 70V10H44L54 20V31L44 41H10M36 41L56 70",
    "T": "M5 10H59M32 10V70",
    "S": "M56 10H18L8 20V29L18 39H46L56 49V60L46 70H8",
}


def wordmark():
    pieces = []
    x = 24
    for character in "OPEN RTS":
        if character == " ":
            x += 42
            continue
        color = "#e1f5ed" if x < 336 else "#f1d37d"
        common = f'd="{GLYPHS[character]}" fill="none" stroke-linejoin="bevel" stroke-linecap="square"'
        pieces.append(f'<g transform="translate({x},14)"><path {common} stroke="#142a32" stroke-width="22"/><path {common} stroke="{color}" stroke-width="12"/></g>')
        x += 78
    return "".join(pieces)


def svg(width, height, body):
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">{body}</svg>\n'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def generate(output):
    output.mkdir(parents=True, exist_ok=False)
    mark = wordmark()
    records = []
    for name, width, height, body in [
        ("wordmark", 630, 110, mark),
        ("library-logo", 1280, 230, f'<g transform="translate(36,10) scale(1.9)">{mark}</g>'),
    ]:
        source = output / f"{name}.svg"
        source.write_text(svg(width, height, body))
        image = source.with_suffix(".png")
        subprocess.run(["resvg", str(source), str(image)], check=True)
        records.append({"name": name, "width": width, "height": height, "svg_sha256": digest(source), "png_sha256": digest(image)})
    review = []
    for y, color in [(0, "#15242d"), (160, "#ccb79e")]:
        review.append(f'<rect y="{y}" width="630" height="160" fill="{color}"/><g transform="translate(0,{y})">{mark}</g>')
        review.append(f'<g transform="translate(30,{y+118}) scale(.2)">{mark}</g>')
        review.append(f'<g transform="translate(225,{y+115}) scale(.3)">{mark}</g>')
    source = output / "review-sheet.svg"
    source.write_text(svg(630, 320, "".join(review)))
    subprocess.run(["resvg", str(source), str(source.with_suffix(".png"))], check=True)
    (output / "manifest.json").write_text(json.dumps({"scope": "Original brand candidates, not final Steam media or runtime replacements", "display_title": "Open RTS", "inputs": "repository-authored geometric letter paths; no font or external image", "source_license": "repository MIT OR Apache-2.0", "generator_sha256": digest(Path(__file__)), "rasterizer": subprocess.check_output(["resvg", "--version"], text=True).strip(), "records": records}, indent=2) + "\n")


if __name__ == "__main__":
    generate(Path(sys.argv[1]))
