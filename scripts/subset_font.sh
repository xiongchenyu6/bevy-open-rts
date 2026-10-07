#!/usr/bin/env bash
# Rebuild the UI font from the pinned official WenQuanYi archive, not a local
# conversion. Requires uv and Python 3; fontTools is version-pinned by uv.
# Source/download/extraction cache stays under ignored target/font-source.
# The user's wqy-microhei-ui.full.ttf is never read or overwritten.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

uv run --no-project --python "$(command -v python3)" --with 'fonttools==4.61.1' python3 - "$ROOT" <<'PY'
import csv
import hashlib
import io
import json
from pathlib import Path
import re
import sys
import tarfile
import urllib.request

import fontTools
from fontTools import subset
from fontTools.ttLib import TTFont

root = Path(sys.argv[1])
cache = root / "target/font-source"
cache.mkdir(parents=True, exist_ok=True)
url = "https://deb.debian.org/debian/pool/main/f/fonts-wqy-microhei/fonts-wqy-microhei_0.2.0-beta.orig.tar.gz"
expected = "2802ac8023aa36a66ea6e7445854e3a078d377ffff42169341bd237871f7213e"
archive = cache / "wqy-microhei-0.2.0-beta.tar.gz"
if not archive.exists():
    data = urllib.request.urlopen(url, timeout=60).read()
    if hashlib.sha256(data).hexdigest() != expected:
        raise SystemExit("Official font archive checksum mismatch")
    archive.write_bytes(data)
data = archive.read_bytes()
if hashlib.sha256(data).hexdigest() != expected:
    raise SystemExit("Cached font archive checksum mismatch; remove it and rerun")

with tarfile.open(fileobj=io.BytesIO(data)) as source:
    ttc_bytes = source.extractfile("wqy-microhei/wqy-microhei.ttc").read()
    for original, retained in [
        ("LICENSE_Apache2.txt", "WQY-LICENSE-Apache2.txt"),
        ("LICENSE_GPLv3.txt", "WQY-LICENSE-GPLv3.txt"),
        ("README.txt", "WQY-LICENSE-README.txt"),
        ("AUTHORS.txt", "WQY-LICENSE-AUTHORS.txt"),
    ]:
        if source.extractfile("wqy-microhei/" + original).read() != (root / "assets/fonts" / retained).read_bytes():
            raise SystemExit("Retained font notice differs from official archive: " + retained)
ttc = cache / "wqy-microhei.ttc"
ttc.write_bytes(ttc_bytes)

# TTC face 0 is Micro Hei; face 1 is Micro Hei Mono.
font = TTFont(ttc, fontNumber=0, recalcTimestamp=False)
converted = cache / "wqy-microhei-official.ttf"
font.save(converted)
source_cmap = font.getBestCmap()
required = set()
for path in sorted((root / "src").rglob("*.rs")):
    text = path.read_text(encoding="utf-8")
    required.update(ord(char) for char in text if ord(char) > 0x7f)
    required.update(int(value, 16) for value in re.findall(r"\\u\{([0-9a-fA-F]+)\}", text))
# Imported catalogs are runtime content too. Read only the shipped product
# locales; the Polish source column is not a supported product locale.
translation_required = set()
translation_sources = []
for path in sorted((root / "assets/translations").glob("*.csv")):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        rows = csv.DictReader(stream)
        if not {"en", "zh_CN"}.issubset(rows.fieldnames or []):
            raise SystemExit("Translation catalog lacks supported locale columns: " + str(path))
        for row in rows:
            for locale in ("en", "zh_CN"):
                translation_required.update(ord(char) for char in (row.get(locale) or "") if ord(char) > 0x7f)
    translation_sources.append({"path": str(path.relative_to(root)), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
required.update(translation_required)
required.discard(0xfeff)
# Preserve previously shipped CJK glyphs not present in current Rust/catalog
# text, including the negative-value label requested by the release audit.
required.update((0x4e22, 0x951a, 0x8d1f))
# Preserve the old script's Latin and punctuation safety ranges.
for first, last in [(0x20, 0x7e), (0xa0, 0xff), (0x2000, 0x206f), (0x3000, 0x303f), (0xff00, 0xffef)]:
    required.update(range(first, last + 1))
supported = required.intersection(source_cmap)
missing_cjk = sorted(cp for cp in required.difference(source_cmap) if 0x3400 <= cp <= 0x9fff)
if missing_cjk:
    raise SystemExit("Official font lacks required CJK characters: " + str(missing_cjk))

options = subset.Options()
options.layout_features = ["*"]
options.notdef_outline = True
options.recommended_glyphs = True
options.name_IDs = ["*"]
options.name_languages = ["*"]
options.name_legacy = True
options.recalc_timestamp = False
options.harfbuzz_repacker = False
subsetter = subset.Subsetter(options=options)
subsetter.populate(unicodes=supported)
subsetter.subset(font)
output = root / "assets/fonts/wqy-microhei-ui.ttf"
temporary = cache / "wqy-microhei-ui.ttf"
font.save(temporary)
result = TTFont(temporary, recalcTimestamp=False)
cmap = result.getBestCmap()
if not supported.issubset(cmap):
    raise SystemExit("Subset lost required supported glyphs")
if not translation_required.issubset(cmap):
    raise SystemExit("Subset lost characters from supported translation catalogs")
if not set(range(0x20, 0x7f)).issubset(cmap):
    raise SystemExit("Subset lost ASCII glyphs")
output.write_bytes(temporary.read_bytes())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
report = {
    "source_url": url,
    "archive_sha256": expected,
    "ttc_sha256": hashlib.sha256(ttc_bytes).hexdigest(),
    "face": 0,
    "fonttools_version": fontTools.__version__,
    "converted_ttf_sha256": sha(converted),
    "subset_sha256": sha(output),
    "requested_supported_codepoints": len(supported),
    "subset_codepoints": len(cmap),
    "translation_locales": ["en", "zh_CN"],
    "translation_sources": translation_sources,
    "translation_codepoints": len(translation_required),
    "missing_translation_codepoints": [f"U+{cp:04X}" for cp in sorted(translation_required.difference(cmap))],
    "retained_historical_cjk": ["U+4E22", "U+951A", "U+8D1F"],
    "ascii_complete": True,
    "missing_required_cjk": missing_cjk,
    "source_unsupported_requested_symbols": [f"U+{cp:04X}" for cp in sorted(required.difference(source_cmap)) if not any(first <= cp <= last for first, last in [(0xa0, 0xff), (0x2000, 0x206f), (0x3000, 0x303f), (0xff00, 0xffef)])],
}
(cache / "subset-report.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
print(f"Subset written: {output.relative_to(root)} ({output.stat().st_size} bytes)")
PY
