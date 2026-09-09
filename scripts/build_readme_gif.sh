#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="${BEVY_OPEN_RTS_README_GIF_WORKDIR:-$ROOT/target/readme-gif}"
OUTPUT="${1:-$ROOT/docs/media/bevy-open-rts-gameplay.gif}"

if [[ "$OUTPUT" != /* ]]; then
  OUTPUT="$ROOT/$OUTPUT"
fi

command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 1; }
command -v ffmpeg >/dev/null || { echo "ffmpeg is required" >&2; exit 1; }

mkdir -p "$WORK" "$(dirname "$OUTPUT")"
cd "$ROOT"

run_capture() {
  cargo run --quiet --bin capture -- "$@"
}

if [[ "${BEVY_OPEN_RTS_REUSE_README_CAPTURES:-0}" != "1" ]]; then
  run_capture menu "$WORK/menu.png"
  run_capture menu-setup "$WORK/setup.png"
  run_capture base "$WORK/base.png"
  run_capture harvest "$WORK/harvest"
  run_capture play "$WORK/play"
  run_capture assault "$WORK/assault" 180
fi

FRAMES=(
  "$WORK/menu.png"
  "$WORK/setup.png"
  "$WORK/base.png"
  "$WORK/harvest/03_collecting_closeup.png"
  "$WORK/play/04_constructing.png"
  "$WORK/assault/03b_combat_1.png"
  "$WORK/assault/03b_combat_2.png"
  "$WORK/assault/04_result.png"
  "$WORK/menu.png"
)

for frame in "${FRAMES[@]}"; do
  [[ -s "$frame" ]] || { echo "missing capture frame: $frame" >&2; exit 1; }
done

FILTER_COMPLEX="
[0:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.35+0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v0];
[1:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.65-0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v1];
[2:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.35+0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v2];
[3:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.65-0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v3];
[4:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.35+0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v4];
[5:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.65-0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v5];
[6:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.35+0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v6];
[7:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.65-0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v7];
[8:v]scale=1000:563,crop=960:540:x='(in_w-out_w)*(0.35+0.2*t/1.5)':y='(in_h-out_h)/2',fps=10,format=yuv420p,settb=AVTB,setpts=PTS-STARTPTS[v8];
[v0][v1]xfade=transition=fade:duration=0.25:offset=1.25[x1];
[x1][v2]xfade=transition=fade:duration=0.25:offset=2.5[x2];
[x2][v3]xfade=transition=fade:duration=0.25:offset=3.75[x3];
[x3][v4]xfade=transition=fade:duration=0.25:offset=5.0[x4];
[x4][v5]xfade=transition=fade:duration=0.25:offset=6.25[x5];
[x5][v6]xfade=transition=fade:duration=0.25:offset=7.5[x6];
[x6][v7]xfade=transition=fade:duration=0.25:offset=8.75[x7];
[x7][v8]xfade=transition=fade:duration=0.25:offset=10.0[joined];
[joined]split[palette_source][gif_source];
[palette_source]palettegen=max_colors=160:stats_mode=diff[palette];
[gif_source][palette]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle[out]
"

ffmpeg -hide_banner -loglevel error -y \
  -loop 1 -t 1.5 -i "${FRAMES[0]}" \
  -loop 1 -t 1.5 -i "${FRAMES[1]}" \
  -loop 1 -t 1.5 -i "${FRAMES[2]}" \
  -loop 1 -t 1.5 -i "${FRAMES[3]}" \
  -loop 1 -t 1.5 -i "${FRAMES[4]}" \
  -loop 1 -t 1.5 -i "${FRAMES[5]}" \
  -loop 1 -t 1.5 -i "${FRAMES[6]}" \
  -loop 1 -t 1.5 -i "${FRAMES[7]}" \
  -loop 1 -t 1.5 -i "${FRAMES[8]}" \
  -filter_complex "$FILTER_COMPLEX" \
  -map "[out]" -t 11.25 -an "$OUTPUT"

echo "wrote $OUTPUT ($(du -h "$OUTPUT" | cut -f1), 960x540, 10 fps)"
