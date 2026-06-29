#!/usr/bin/env bash
# Regenerate the subsetted UI font from the full font.
#
# assets/fonts/wqy-microhei-ui.ttf is a SUBSET (only the glyphs the UI actually
# uses) — ~128 KB instead of the 3.2 MB full WenQuanYi Micro Hei. Re-run this after
# adding/changing any displayed Chinese text so new glyphs are included; otherwise
# they render as blank boxes (tofu).
#
# Needs: pyftsubset (pip install fonttools) and the full font at
# assets/fonts/wqy-microhei-ui.full.ttf (gitignored — keep a local copy).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FULL="assets/fonts/wqy-microhei-ui.full.ttf"
OUT="assets/fonts/wqy-microhei-ui.ttf"

if [[ ! -f "$FULL" ]]; then
  echo "ERROR: full font not found at $FULL" >&2
  echo "Keep the full WenQuanYi Micro Hei there (it's gitignored) to re-subset." >&2
  exit 1
fi

# Collect every non-ASCII glyph the UI can display: literal CJK/punctuation in the
# Rust sources plus \u{XXXX} escapes (e.g. the ▾ dropdown arrow).
CHARS="$(mktemp)"
trap 'rm -f "$CHARS"' EXIT
python3 - "$CHARS" <<'PY'
import glob, re, sys
chars = set()
for path in glob.glob("src/**/*.rs", recursive=True):
    txt = open(path, encoding="utf-8").read()
    chars.update(ch for ch in txt if ord(ch) > 0x7F)
    for m in re.findall(r'\\u\{([0-9A-Fa-f]+)\}', txt):
        cp = int(m, 16)
        if cp > 0x7F:
            chars.add(chr(cp))
chars.discard('﻿')
open(sys.argv[1], "w", encoding="utf-8").write("".join(sorted(chars)))
print(f"glyphs from source: {len(chars)}")
PY

# Keep full Latin/digits/punctuation + CJK punctuation ranges as a safety net.
pyftsubset "$FULL" \
  --text-file="$CHARS" \
  --unicodes="U+0020-007E,U+00A0-00FF,U+2000-206F,U+3000-303F,U+FF00-FFEF" \
  --layout-features='*' \
  --notdef-outline --recommended-glyphs \
  --output-file="$OUT"

echo "Subset written: $OUT ($(du -h "$OUT" | cut -f1), from $(du -h "$FULL" | cut -f1))"
