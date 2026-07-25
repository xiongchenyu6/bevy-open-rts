#!/usr/bin/env bash
set -euo pipefail

WORKER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$WORKER_DIR/../../.." && pwd)"
PORT="${OPEN_BEVY_WORKER_PORT:-8790}"
WRANGLER_VERSION="${WRANGLER_VERSION:-4.114.0}"
SERVICE_URL="http://127.0.0.1:${PORT}"
LOG="${OPEN_BEVY_WORKER_LOG:-/tmp/open-bevy-signaling-worker.log}"

cd "$WORKER_DIR"
npx --yes "wrangler@${WRANGLER_VERSION}" dev --local --port "$PORT" \
  --var "PUBLIC_BASE_URL:${SERVICE_URL}" >"$LOG" 2>&1 &
worker_pid=$!

cleanup() {
  kill "$worker_pid" 2>/dev/null || true
  wait "$worker_pid" 2>/dev/null || true
}
trap cleanup EXIT

for attempt in {1..60}; do
  if curl -fsS "$SERVICE_URL/readyz" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$worker_pid" 2>/dev/null; then
    cat "$LOG" >&2
    exit 1
  fi
  if [[ "$attempt" == 60 ]]; then
    cat "$LOG" >&2
    exit 1
  fi
  sleep 1
done

OPEN_BEVY_SIGNALING_URL="$SERVICE_URL" \
  "$ROOT/scripts/verify_signaling_backend.sh"
