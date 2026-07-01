#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Guard: WebGPU has no rgba16unorm. bevy loads 16-bit PNGs as TextureFormat::Rgba16Unorm
# and the web build then panics at boot ("Format Rgba16Unorm has no WebGPU equivalent").
# Fail early if any 16-bit PNG snuck into assets so it never reaches a deploy.
# `|| true`: grep exits 1 when there are no 16-bit PNGs (the normal case); without
# it, set -o pipefail + set -e would kill the build on a clean asset tree.
sixteen_bit_pngs="$(find assets -name '*.png' -type f -print0 2>/dev/null |
  xargs -0 -r file | grep -i '16-bit' | cut -d: -f1 || true)"
if [ -n "$sixteen_bit_pngs" ]; then
  echo "ERROR: 16-bit PNG(s) found — WebGPU has no Rgba16Unorm. Downconvert to 8-bit:" >&2
  echo "$sixteen_bit_pngs" | sed 's/^/  /' >&2
  echo "  fix: magick FILE -depth 8 PNG32:FILE" >&2
  exit 1
fi

# The wasm-bindgen CLI MUST match the wasm-bindgen crate version, or the generated
# JS glue and the wasm disagree and boot dies with
#   "WebAssembly.Table.grow(): failed to grow table by 4"
# in __wbindgen_init_externref_table. Pin the CLI to the crate version from
# Cargo.lock (auto-install if the available one doesn't match).
WASM_BINDGEN_VERSION="${WASM_BINDGEN_VERSION:-$(
  awk '/^name = "wasm-bindgen"$/ {getline; print; exit}' Cargo.lock |
    sed -E 's/.*"([0-9.]+)".*/\1/'
)}"
echo ">> wasm-bindgen crate version: ${WASM_BINDGEN_VERSION}"

wasm_bindgen_version_ok() {
  "$1" --version 2>/dev/null | grep -q "wasm-bindgen ${WASM_BINDGEN_VERSION}\$"
}

resolve_wasm_bindgen() {
  local cargo_home="${CARGO_HOME:-$HOME/.cargo}"
  local cargo_bin="$cargo_home/bin/wasm-bindgen"
  if [ -x "$cargo_bin" ] && wasm_bindgen_version_ok "$cargo_bin"; then
    printf '%s\n' "$cargo_bin"
    return
  fi
  if command -v wasm-bindgen >/dev/null 2>&1; then
    local path_bin
    path_bin="$(command -v wasm-bindgen)"
    if wasm_bindgen_version_ok "$path_bin"; then
      printf '%s\n' "$path_bin"
      return
    fi
  fi
  echo ">> installing wasm-bindgen-cli ${WASM_BINDGEN_VERSION}" >&2
  cargo install -q --locked wasm-bindgen-cli --version "${WASM_BINDGEN_VERSION}"
  printf '%s\n' "$cargo_bin"
}

WASM_BINDGEN_BIN="$(resolve_wasm_bindgen)"

# Pin binaryen (wasm-opt): the version Ubuntu/apt ships MISASSIGNS the
# __wbindgen_externrefs export to the (sealed) funcref table, so the JS glue's
# `wasm.__wbindgen_externrefs.grow(4)` grows the wrong, un-growable table and the web
# build boot-fails with "WebAssembly.Table.grow(): failed to grow table by 4". Use a
# pinned recent release (the release binary is self-contained per ldd; copy it into
# $CARGO_HOME/bin so CI's tool cache keeps it). If the download fails, ship
# un-optimized (but valid) wasm rather than abort.
BINARYEN_VERSION="${BINARYEN_VERSION:-version_130}"
BINARYEN_MIN_NUM="${BINARYEN_VERSION#version_}"
wasm_opt_num() { "$1" --version 2>/dev/null | sed -nE 's/.*wasm-opt version ([0-9]+).*/\1/p'; }
resolve_wasm_opt() {
  local cargo_home="${CARGO_HOME:-$HOME/.cargo}"
  local pin_bin="$cargo_home/bin/wasm-opt"
  local cand n
  for cand in "$pin_bin" "$(command -v wasm-opt 2>/dev/null || true)"; do
    [ -n "$cand" ] && [ -x "$cand" ] || continue
    n="$(wasm_opt_num "$cand")"
    if [ -n "$n" ] && [ "$n" -ge "$BINARYEN_MIN_NUM" ]; then
      printf '%s\n' "$cand"
      return 0
    fi
  done
  echo ">> downloading binaryen ${BINARYEN_VERSION} (system wasm-opt missing/too old)" >&2
  local tmp
  tmp="$(mktemp -d)"
  if curl -fsSL "https://github.com/WebAssembly/binaryen/releases/download/${BINARYEN_VERSION}/binaryen-${BINARYEN_VERSION}-x86_64-linux.tar.gz" -o "$tmp/b.tgz" &&
    tar xzf "$tmp/b.tgz" -C "$tmp"; then
    mkdir -p "$cargo_home/bin"
    cp "$tmp/binaryen-${BINARYEN_VERSION}/bin/wasm-opt" "$pin_bin"
    rm -rf "$tmp"
    printf '%s\n' "$pin_bin"
    return 0
  fi
  rm -rf "$tmp"
  return 1
}
WASM_OPT_BIN="$(resolve_wasm_opt || true)"
[ -n "$WASM_OPT_BIN" ] && echo ">> wasm-opt: $("$WASM_OPT_BIN" --version)"

cargo build --release --target wasm32-unknown-unknown --features webgpu

rm -rf web/pkg web/assets
mkdir -p web/pkg
"$WASM_BINDGEN_BIN" \
  --target web \
  --out-dir web/pkg \
  --out-name bevy_open_rts \
  target/wasm32-unknown-unknown/release/bevy-open-rts.wasm

if [ -n "$WASM_OPT_BIN" ]; then
  BG="web/pkg/bevy_open_rts_bg.wasm"
  # Enable exactly the STABLE, browser-shipped post-MVP features rustc emits — most
  # importantly reference-types (the externref table __wbindgen_init_externref_table
  # grows by 4 at boot). Do NOT use --all-features: it turns on experimental
  # proposals (GC, typed-function-refs, ...) and wasm-opt then re-encodes types the
  # browser can't parse ("CompileError: invalid value type 0x0" at instantiate).
  # Retry a few times (wasm-opt occasionally fails transiently) and ship the
  # un-optimized (still valid) wasm rather than abort.
  opt_ok=0
  for attempt in 1 2 3; do
    rm -f "$BG.opt"
    if "$WASM_OPT_BIN" -Oz \
      --enable-reference-types \
      --enable-bulk-memory \
      --enable-nontrapping-float-to-int \
      --enable-sign-ext \
      --enable-mutable-globals \
      --enable-multivalue \
      -o "$BG.opt" "$BG" && [ -f "$BG.opt" ]; then
      mv "$BG.opt" "$BG"
      opt_ok=1
      break
    fi
    echo ">> wasm-opt attempt $attempt failed, retrying..." >&2
    sleep 2
  done
  if [ "$opt_ok" != 1 ]; then
    echo ">> wasm-opt failed 3×; shipping the un-optimized wasm (functional, larger)" >&2
  fi
fi

# Precompressed copy the loader streams with a real progress bar (GitHub Pages
# serves the .gz verbatim, so bytes-received == the actual ~7 MB transfer).
gzip -9 -kf web/pkg/bevy_open_rts_bg.wasm

BUILD_ID="$(scripts/stamp_web_build_id.py \
  web/index.html \
  web/pkg/bevy_open_rts.js \
  web/pkg/bevy_open_rts_bg.wasm)"
echo "Stamped web build id: ${BUILD_ID}"

cp -R assets web/assets
