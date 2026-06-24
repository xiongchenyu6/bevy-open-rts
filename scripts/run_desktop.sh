#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ "${BEVY_OPEN_RTS_SKIP_NIX:-0}" != "1" ]] \
  && [[ "${BEVY_OPEN_RTS_IN_NIX_RUNNER:-0}" != "1" ]] \
  && command -v nix >/dev/null 2>&1; then
  exec nix develop "$ROOT" --command env BEVY_OPEN_RTS_IN_NIX_RUNNER=1 "$ROOT/scripts/run_desktop.sh" "$@"
fi

exec cargo run "$@"
