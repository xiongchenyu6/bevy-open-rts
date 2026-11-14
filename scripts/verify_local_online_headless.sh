#!/usr/bin/env bash
# Two headless peers (host + player) play a real online match against a local
# signaling server over the real WebRTC transport. No window, no public service.
#
#   OPEN_BEVY_ONLINE_VERIFY_MODE=combat  also requires a support strike, unit
#   deaths, a building destruction and weapon fire to reach the client, and
#   compares per-tick snapshot digests between the two peers.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${OPEN_BEVY_LOCAL_SIGNALING_PORT:-3546}"
RUN_ID="${OPEN_BEVY_ONLINE_VERIFY_RUN:-local-$(date +%s)-$RANDOM}"
TIMEOUT_SECONDS="${OPEN_BEVY_ONLINE_VERIFY_TIMEOUT:-240}"
OUTPUT_DIR="${OPEN_BEVY_ONLINE_VERIFY_OUTPUT:-/tmp/open-bevy-local-online-$RUN_ID}"
MODE="${OPEN_BEVY_ONLINE_VERIFY_MODE:-}"
SERVICE_URL="http://127.0.0.1:$PORT"
SIGNALING="$ROOT/target/debug/open-bevy-signaling"
CAPTURE="$ROOT/target/debug/capture"

for binary in "$SIGNALING" "$CAPTURE"; do
  if [[ ! -x "$binary" ]]; then
    echo "missing $binary; build it first (cargo build --bin capture -p bevy-open-rts; cargo build -p open-bevy-signaling)" >&2
    exit 1
  fi
done
mkdir -p "$OUTPUT_DIR"

pids=()
cleanup() {
  for pid in "${pids[@]:-}"; do
    kill "$pid" 2>/dev/null || true
  done
  wait "${pids[@]:-}" 2>/dev/null || true
}
trap cleanup EXIT

"$SIGNALING" --bind "127.0.0.1:$PORT" --public-ws-base "ws://127.0.0.1:$PORT" \
  > "$OUTPUT_DIR/signaling.log" 2>&1 &
pids+=("$!")
curl --retry 20 --retry-all-errors --retry-delay 1 -fsS "$SERVICE_URL/healthz" >/dev/null

for role in host player; do
  env \
    OPEN_BEVY_ONLINE_VERIFY_ROLE="$role" \
    OPEN_BEVY_ONLINE_VERIFY_RUN="$RUN_ID" \
    OPEN_BEVY_ONLINE_VERIFY_MODE="$MODE" \
    OPEN_BEVY_ONLINE_VERIFY_STATUS="$OUTPUT_DIR/$role.json" \
    OPEN_BEVY_SIGNALING_URL="$SERVICE_URL" \
    BEVY_ASSET_ROOT="$ROOT" \
    "$ROOT/scripts/native_runner.sh" "$CAPTURE" online-peer "$TIMEOUT_SECONDS" \
    > "$OUTPUT_DIR/$role.log" 2>&1 &
  pids+=("$!")
done

deadline=$((SECONDS + TIMEOUT_SECONDS + 20))
while (( SECONDS < deadline )); do
  if [[ -s "$OUTPUT_DIR/host.json" && -s "$OUTPUT_DIR/player.json" ]] \
    && [[ "$(jq -r '.terminal // false' "$OUTPUT_DIR/host.json")" == "true" ]] \
    && [[ "$(jq -r '.terminal // false' "$OUTPUT_DIR/player.json")" == "true" ]]; then
    break
  fi
  sleep 1
done

status=0
for role in host player; do
  if [[ ! -s "$OUTPUT_DIR/$role.json" ]]; then
    echo "$role did not publish a verification report" >&2
    tail -40 "$OUTPUT_DIR/$role.log" >&2 || true
    exit 1
  fi
  jq -c '{role, passed, stage, result, error, snapshot_tick, command_observed, connected_humans, online_phase, match_phase, combat: (.combat | if . then del(.digests) else . end)}' \
    "$OUTPUT_DIR/$role.json"
  jq -e --arg run_id "$RUN_ID" \
    '.passed == true and .run_id == $run_id and .command_observed == true and .snapshot_tick > 0' \
    "$OUTPUT_DIR/$role.json" >/dev/null || status=1
done

if [[ "$MODE" == "combat" ]]; then
  # Every tick both peers digested must agree: the client's reconstructed
  # world (keyframe + delta) has to equal what the host sent.
  jq -n --slurpfile host "$OUTPUT_DIR/host.json" --slurpfile player "$OUTPUT_DIR/player.json" '
    ($host[0].combat.digests // {}) as $h | ($player[0].combat.digests // {}) as $p
    | [$p | to_entries[] | select($h[.key] != null)] as $common
    | {host_ticks: ($h | length), player_ticks: ($p | length), compared: ($common | length),
       mismatched: [$common[] | select($h[.key] != .value) | .key]}' \
    | tee "$OUTPUT_DIR/sync.json"
  jq -e '.compared >= 20 and (.mismatched | length) == 0' "$OUTPUT_DIR/sync.json" >/dev/null || status=1
fi

if (( status != 0 )); then
  echo "local online verification FAILED: run=$RUN_ID evidence=$OUTPUT_DIR" >&2
  exit 1
fi
echo "local online verification passed: run=$RUN_ID mode=${MODE:-basic} evidence=$OUTPUT_DIR"
