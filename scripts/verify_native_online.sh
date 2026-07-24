#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_URL="${OPEN_BEVY_SIGNALING_URL:-https://signal.101.78.126.6.sslip.io}"
RUN_ID="${OPEN_BEVY_ONLINE_VERIFY_RUN:-native-$(date +%s)-$RANDOM}"
TIMEOUT_SECONDS="${OPEN_BEVY_ONLINE_VERIFY_TIMEOUT:-180}"
OUTPUT_DIR="${OPEN_BEVY_ONLINE_VERIFY_OUTPUT:-/tmp/open-bevy-native-online-$RUN_ID}"

mkdir -p "$OUTPUT_DIR"
cargo build --manifest-path "$ROOT/Cargo.toml" --bin bevy-open-rts

pids=()
cleanup() {
  for pid in "${pids[@]:-}"; do
    kill "$pid" 2>/dev/null || true
  done
  wait "${pids[@]:-}" 2>/dev/null || true
}
trap cleanup EXIT

for role in host player; do
  env \
    OPEN_BEVY_ONLINE_VERIFY_ROLE="$role" \
    OPEN_BEVY_ONLINE_VERIFY_RUN="$RUN_ID" \
    OPEN_BEVY_ONLINE_VERIFY_STATUS="$OUTPUT_DIR/$role.json" \
    OPEN_BEVY_SIGNALING_URL="$SERVICE_URL" \
    BEVY_ASSET_ROOT="$ROOT" \
    "$ROOT/scripts/native_runner.sh" "$ROOT/target/debug/bevy-open-rts" \
    > "$OUTPUT_DIR/$role.log" 2>&1 &
  pids+=("$!")
done

deadline=$((SECONDS + TIMEOUT_SECONDS))
while (( SECONDS < deadline )); do
  if [[ -s "$OUTPUT_DIR/host.json" && -s "$OUTPUT_DIR/player.json" ]]; then
    host_terminal="$(jq -r '.terminal // false' "$OUTPUT_DIR/host.json")"
    player_terminal="$(jq -r '.terminal // false' "$OUTPUT_DIR/player.json")"
    if [[ "$host_terminal" == "true" && "$player_terminal" == "true" ]]; then
      break
    fi
  fi
  sleep 1
done

for role in host player; do
  if [[ ! -s "$OUTPUT_DIR/$role.json" ]]; then
    echo "$role did not publish a verification report" >&2
    tail -100 "$OUTPUT_DIR/$role.log" >&2 || true
    exit 1
  fi
  jq . "$OUTPUT_DIR/$role.json"
  jq -e \
    --arg run_id "$RUN_ID" \
    '.passed == true and .run_id == $run_id and .command_observed == true and .snapshot_tick > 0' \
    "$OUTPUT_DIR/$role.json" >/dev/null
done

host_room="$(jq -r .room_code "$OUTPUT_DIR/host.json")"
player_room="$(jq -r .room_code "$OUTPUT_DIR/player.json")"
if [[ -z "$host_room" || "$host_room" != "$player_room" ]]; then
  echo "clients did not complete the same room: host=$host_room player=$player_room" >&2
  exit 1
fi

echo "native online verification passed: run=$RUN_ID room=$host_room evidence=$OUTPUT_DIR"
