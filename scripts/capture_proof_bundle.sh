#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FACTION="${1:-human}"
FRAMES="${2:-900}"
RESULT_DIR="${3:-}"

if ! [[ "$FRAMES" =~ ^[0-9]+$ ]] || [[ "$FRAMES" -le 0 ]]; then
  echo "frames must be a positive integer, got: $FRAMES" >&2
  exit 2
fi

if [[ -z "$RESULT_DIR" ]]; then
  next=1
  while [[ -e "$ROOT/screenshots/result/$next" ]]; do
    next=$((next + 1))
  done
  RESULT_DIR="$ROOT/screenshots/result/$next"
elif [[ "$RESULT_DIR" != /* ]]; then
  RESULT_DIR="$ROOT/$RESULT_DIR"
fi

if [[ -e "$RESULT_DIR" ]]; then
  echo "result directory already exists: $RESULT_DIR" >&2
  exit 2
fi

mkdir -p "$RESULT_DIR"

(
  cd "$ROOT"
  RUST_LOG="${RUST_LOG:-warn}" cargo run --bin capture -- proof-frames "$RESULT_DIR" "$FRAMES" "$FACTION"
)

ffmpeg -hide_banner -loglevel error -y -framerate 30 -i "$RESULT_DIR/frame%05d.png" \
  -c:v libx264 -pix_fmt yuv420p -preset medium -crf 22 -movflags +faststart \
  "$RESULT_DIR/video.mp4"

frame_count="$(find "$RESULT_DIR" -maxdepth 1 -name 'frame*.png' | wc -l)"
if [[ "$frame_count" -ne "$FRAMES" ]]; then
  echo "expected $FRAMES frames, wrote $frame_count" >&2
  exit 1
fi

if [[ ! -s "$RESULT_DIR/video.mp4" ]]; then
  echo "video.mp4 was not created" >&2
  exit 1
fi

echo "[capture] proof bundle faction=$FACTION frames=$FRAMES dir=$RESULT_DIR"
