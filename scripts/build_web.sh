#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --release --target wasm32-unknown-unknown --features webgpu

rm -rf web/pkg web/assets
mkdir -p web/pkg
wasm-bindgen \
  --target web \
  --out-dir web/pkg \
  --out-name bevy_open_rts \
  target/wasm32-unknown-unknown/release/bevy-open-rts.wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz -o web/pkg/bevy_open_rts_bg.wasm web/pkg/bevy_open_rts_bg.wasm
fi

BUILD_ID="$(scripts/stamp_web_build_id.py \
  web/index.html \
  web/pkg/bevy_open_rts.js \
  web/pkg/bevy_open_rts_bg.wasm)"
echo "Stamped web build id: ${BUILD_ID}"

cp -R assets web/assets
