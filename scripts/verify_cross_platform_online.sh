#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_URL="${OPEN_BEVY_SIGNALING_URL:-https://signal.101.78.126.6.sslip.io}"
GAME_URL="${OPEN_BEVY_GAME_URL:-https://xiongchenyu6.github.io/bevy-open-rts/}"
RUN_ID="${OPEN_BEVY_ONLINE_VERIFY_RUN:-cross-$(date +%s)-$RANDOM}"
TIMEOUT_SECONDS="${OPEN_BEVY_ONLINE_VERIFY_TIMEOUT:-240}"
OUTPUT_DIR="${OPEN_BEVY_ONLINE_VERIFY_OUTPUT:-/tmp/open-bevy-cross-online-$RUN_ID}"

mkdir -p "$OUTPUT_DIR/browser"
cargo build --manifest-path "$ROOT/Cargo.toml" --bin bevy-open-rts
npm ci --prefix "$ROOT/scripts/browser-smoke"

host_pid=""
cleanup() {
  if [[ -n "$host_pid" ]]; then
    kill "$host_pid" 2>/dev/null || true
    wait "$host_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

env \
  OPEN_BEVY_ONLINE_VERIFY_ROLE=host \
  OPEN_BEVY_ONLINE_VERIFY_RUN="$RUN_ID" \
  OPEN_BEVY_ONLINE_VERIFY_STATUS="$OUTPUT_DIR/host.json" \
  OPEN_BEVY_SIGNALING_URL="$SERVICE_URL" \
  BEVY_ASSET_ROOT="$ROOT" \
  "$ROOT/scripts/native_runner.sh" "$ROOT/target/debug/bevy-open-rts" \
  > "$OUTPUT_DIR/host.log" 2>&1 &
host_pid="$!"

if ! env \
  OPEN_BEVY_BROWSER_ROLES=player \
  OPEN_BEVY_GAME_URL="$GAME_URL" \
  OPEN_BEVY_SIGNALING_URL="$SERVICE_URL" \
  OPEN_BEVY_ONLINE_VERIFY_RUN="$RUN_ID" \
  OPEN_BEVY_BROWSER_OUTPUT="$OUTPUT_DIR/browser" \
  OPEN_BEVY_BROWSER_REPORT="$OUTPUT_DIR/browser.json" \
  OPEN_BEVY_MULTIPLAYER_TIMEOUT_MS="$((TIMEOUT_SECONDS * 1000))" \
  node "$ROOT/scripts/browser-smoke/multiplayer_smoke.mjs" \
  > "$OUTPUT_DIR/browser.log" 2>&1; then
  echo "browser player verification failed" >&2
  tail -160 "$OUTPUT_DIR/browser.log" >&2 || true
  exit 1
fi

deadline=$((SECONDS + TIMEOUT_SECONDS))
while (( SECONDS < deadline )); do
  if [[ -s "$OUTPUT_DIR/host.json" ]] \
    && [[ "$(jq -r '.terminal // false' "$OUTPUT_DIR/host.json")" == "true" ]]; then
    break
  fi
  sleep 1
done

if [[ ! -s "$OUTPUT_DIR/host.json" ]]; then
  echo "native host did not publish a verification report" >&2
  tail -100 "$OUTPUT_DIR/host.log" >&2 || true
  exit 1
fi
if [[ ! -s "$OUTPUT_DIR/browser.json" ]]; then
  echo "browser player did not publish a verification report" >&2
  tail -100 "$OUTPUT_DIR/browser.log" >&2 || true
  exit 1
fi

jq . "$OUTPUT_DIR/host.json"
jq . "$OUTPUT_DIR/browser.json"
jq -e \
  --arg run_id "$RUN_ID" \
  '.passed == true and .terminal == true and .run_id == $run_id
    and .role == "host" and .result == "victory"
    and .connected_humans == 2 and .snapshot_tick > 0
    and .command_observed == true' \
  "$OUTPUT_DIR/host.json" >/dev/null
jq -e \
  --arg run_id "$RUN_ID" \
  '.passed == true and .runId == $run_id and .roles == ["player"]
    and .player.result == "defeat" and .player.command_sent == true
    and .player.command_observed == true and .player.snapshot_tick > 0
    and .player.connected_humans == 2' \
  "$OUTPUT_DIR/browser.json" >/dev/null

host_room="$(jq -r .room_code "$OUTPUT_DIR/host.json")"
player_room="$(jq -r .player.room_code "$OUTPUT_DIR/browser.json")"
if [[ -z "$host_room" || "$host_room" != "$player_room" ]]; then
  echo "native/browser clients did not complete the same room: host=$host_room player=$player_room" >&2
  exit 1
fi

echo "cross-platform online verification passed: run=$RUN_ID room=$host_room evidence=$OUTPUT_DIR"
