#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "${1:-}" in
  */deps/*) exec "$@" ;;
esac

runtime_has_libx11() {
  local dir
  IFS=: read -ra paths <<< "${LD_LIBRARY_PATH:-}"
  for dir in "${paths[@]}"; do
    [[ -e "$dir/libX11.so.6" || -e "$dir/libX11.so" ]] && return 0
  done
  return 1
}

if [[ "${BEVY_OPEN_RTS_NATIVE_RUNTIME:-0}" != "1" ]] \
  && ! runtime_has_libx11 \
  && command -v nix >/dev/null 2>&1; then
  exec nix develop "$ROOT" --command \
    env BEVY_OPEN_RTS_NATIVE_RUNTIME=1 "$ROOT/scripts/native_runner.sh" "$@"
fi

exec "$@"
