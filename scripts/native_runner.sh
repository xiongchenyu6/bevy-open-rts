#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "${1:-}" in
  */deps/*) exec "$@" ;;
esac

runtime_has_wayland() {
  local dir
  IFS=: read -ra paths <<< "${LD_LIBRARY_PATH:-}"
  for dir in "${paths[@]}"; do
    [[ -e "$dir/libwayland-client.so.0" || -e "$dir/libwayland-client.so" ]] && return 0
  done
  return 1
}

if [[ "${BEVY_OPEN_RTS_NATIVE_RUNTIME:-0}" != "1" ]] \
  && ! runtime_has_wayland \
  && command -v nix >/dev/null 2>&1; then
  exec nix develop "$ROOT" --command \
    env BEVY_OPEN_RTS_NATIVE_RUNTIME=1 "$ROOT/scripts/native_runner.sh" "$@"
fi

exec env WINIT_UNIX_BACKEND="${WINIT_UNIX_BACKEND:-wayland}" "$@"
