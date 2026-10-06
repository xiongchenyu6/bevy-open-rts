#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FACTION="${1:-human}"
FRAMES="${2:-450}"
RESULT_DIR="${3:-}"

if ! [[ "$FRAMES" =~ ^[0-9]+$ ]] || [[ "$FRAMES" -lt 450 ]]; then
  echo "frames must be an integer of at least 450, got: $FRAMES" >&2
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
  if [[ -n "${CAPTURE_BIN:-}" ]]; then
    BEVY_ASSET_ROOT="$ROOT" RUST_LOG="${RUST_LOG:-warn}" \
      "$CAPTURE_BIN" proof-frames "$RESULT_DIR" "$FRAMES" "$FACTION"
  else
    RUST_LOG="${RUST_LOG:-warn}" cargo run --bin capture -- proof-frames "$RESULT_DIR" "$FRAMES" "$FACTION"
  fi
)

frame_count="$(find "$RESULT_DIR" -maxdepth 1 -name 'frame*.png' | wc -l)"
if [[ "$frame_count" -ne "$FRAMES" ]]; then
  echo "expected $FRAMES frames, wrote $frame_count" >&2
  exit 1
fi
for ((frame=0; frame<FRAMES; frame++)); do
  printf -v frame_path '%s/frame%05d.png' "$RESULT_DIR" "$frame"
  if [[ ! -s "$frame_path" ]]; then
    echo "missing or empty raw frame: $frame_path" >&2
    exit 1
  fi
done

ffmpeg -hide_banner -loglevel error -y -framerate 30 -start_number 0 \
  -i "$RESULT_DIR/frame%05d.png" -frames:v "$FRAMES" \
  -c:v libx264 -pix_fmt yuv420p -r 30 -preset medium -crf 22 -movflags +faststart \
  "$RESULT_DIR/video.mp4"

encoded_frames="$(ffprobe -v error -select_streams v:0 -count_frames \
  -show_entries stream=nb_read_frames -of csv=p=0 "$RESULT_DIR/video.mp4")"
encoded_rate="$(ffprobe -v error -select_streams v:0 \
  -show_entries stream=r_frame_rate -of csv=p=0 "$RESULT_DIR/video.mp4")"
if [[ "$encoded_frames" != "$FRAMES" || "$encoded_rate" != "30/1" ]]; then
  echo "video timing mismatch: frames=$encoded_frames rate=$encoded_rate" >&2
  exit 1
fi

if [[ ! -s "$RESULT_DIR/video.mp4" ]]; then
  echo "video.mp4 was not created" >&2
  exit 1
fi

echo "[capture] proof bundle faction=$FACTION frames=$FRAMES dir=$RESULT_DIR"
