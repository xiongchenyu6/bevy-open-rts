#!/usr/bin/env bash
# Generate the Chinese voice pack with Microsoft edge-tts neural voices
# (the English pack came from ttsmaker, same online-TTS approach).
#
#   nix-shell -p python3Packages.edge-tts --run scripts/gen_zh_voice.sh
#
# Unit acknowledgments use 云希 (male); the EVA-style announcer uses 晓晓
# (female). Output: assets/voice/chinese/<voice>/<name>.ogg (mono 24 kHz).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

UNIT_VOICE="zh-CN-YunxiNeural"
ANNOUNCER_VOICE="zh-CN-XiaoxiaoNeural"
UNIT_DIR="assets/voice/chinese/edge-yunxi"
ANN_DIR="assets/voice/chinese/edge-xiaoxiao"
mkdir -p "$UNIT_DIR" "$ANN_DIR"

gen() { # voice text out_ogg
  local tmp
  tmp="$(mktemp --suffix=.mp3)"
  edge-tts --voice "$1" --rate '+8%' --text "$2" --write-media "$tmp" >/dev/null
  ffmpeg -y -loglevel error -i "$tmp" -ac 1 -ar 24000 -c:a libvorbis -qscale:a 3 "$3"
  rm -f "$tmp"
  echo "  $3"
}

echo ">> unit acknowledgments ($UNIT_VOICE)"
gen "$UNIT_VOICE" "长官?"   "$UNIT_DIR/sir.ogg"
gen "$UNIT_VOICE" "遵命!"   "$UNIT_DIR/yes_sir.ogg"
gen "$UNIT_VOICE" "收到。"   "$UNIT_DIR/acknowledged.ogg"

echo ">> announcer ($ANNOUNCER_VOICE)"
gen "$ANNOUNCER_VOICE" "训练中"           "$ANN_DIR/training.ogg"
gen "$ANNOUNCER_VOICE" "单位就绪"         "$ANN_DIR/unit_ready.ogg"
gen "$ANNOUNCER_VOICE" "建造完成"         "$ANN_DIR/construction_complete.ogg"
gen "$ANNOUNCER_VOICE" "资源不足"         "$ANN_DIR/not_enough_resources.ogg"
gen "$ANNOUNCER_VOICE" "支援技能就绪"     "$ANN_DIR/support_power_ready.ogg"
gen "$ANNOUNCER_VOICE" "敌方发动支援技能" "$ANN_DIR/enemy_support_power.ogg"
gen "$ANNOUNCER_VOICE" "警告,敌方超级武器就绪" "$ANN_DIR/enemy_superweapon_ready.ogg"
gen "$ANNOUNCER_VOICE" "警告,敌方超级武器已发射" "$ANN_DIR/enemy_superweapon_launched.ogg"
gen "$ANNOUNCER_VOICE" "你已获得胜利"     "$ANN_DIR/victory.ogg"
gen "$ANNOUNCER_VOICE" "你已战败"         "$ANN_DIR/defeat.ogg"
gen "$ANNOUNCER_VOICE" "基地遇袭"         "$ANN_DIR/base_under_attack.ogg"
gen "$ANNOUNCER_VOICE" "部队遇袭"         "$ANN_DIR/unit_under_attack.ogg"
gen "$ANNOUNCER_VOICE" "单位损失"         "$ANN_DIR/unit_lost.ogg"
echo ">> done"
