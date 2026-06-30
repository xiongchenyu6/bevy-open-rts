#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

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

cargo build --release --target wasm32-unknown-unknown --features webgpu

rm -rf web/pkg web/assets
mkdir -p web/pkg
"$WASM_BINDGEN_BIN" \
  --target web \
  --out-dir web/pkg \
  --out-name bevy_open_rts \
  target/wasm32-unknown-unknown/release/bevy-open-rts.wasm

if command -v wasm-opt >/dev/null 2>&1; then
  BG="web/pkg/bevy_open_rts_bg.wasm"
  # --all-features keeps reference-types (the externref table that
  # __wbindgen_init_externref_table grows by 4 at boot) AND every other post-MVP op
  # modern rustc emits (bulk-memory, sign-ext, nontrapping-fptoint, ...), so wasm-opt
  # never strips the growable table or rejects the input. Retry a few times (wasm-opt
  # occasionally fails transiently) and ship un-optimized rather than abort.
  opt_ok=0
  for attempt in 1 2 3; do
    rm -f "$BG.opt"
    if wasm-opt -Oz --all-features -o "$BG.opt" "$BG" && [ -f "$BG.opt" ]; then
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
