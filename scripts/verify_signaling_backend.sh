#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_URL="${OPEN_BEVY_SIGNALING_URL:?set OPEN_BEVY_SIGNALING_URL to the signaling HTTP(S) origin}"

curl --retry 5 --retry-all-errors --retry-delay 1 -fsS "$SERVICE_URL/healthz" \
  | jq -e '.status == "ok" and .service == "open-bevy-signaling"' >/dev/null
curl --retry 5 --retry-all-errors --retry-delay 1 -fsS "$SERVICE_URL/readyz" \
  | jq -e '.status == "ok" and .service == "open-bevy-signaling"' >/dev/null
curl --retry 5 --retry-all-errors --retry-delay 1 -fsS "$SERVICE_URL/v1/config" \
  | jq -e '.service == "open-bevy-signaling" and (.websocket_base_url | test("^wss?://"))' >/dev/null

cd "$ROOT"
OPEN_BEVY_SIGNALING_URL="$SERVICE_URL" \
OPEN_BEVY_REQUIRE_TURN="${OPEN_BEVY_REQUIRE_TURN:-0}" \
cargo test -p open-bevy-net --test transport \
  deployed_service_exchanges_reliable_and_snapshot_channels \
  -- --ignored --exact --nocapture

echo "signaling backend verification passed: $SERVICE_URL"
