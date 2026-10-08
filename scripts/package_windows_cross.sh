#!/usr/bin/env bash
# Preliminary Windows GNU build/package from Linux. Native MSVC CI is separate.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${1:-debug}"
PACKAGE_TAG="${2:-}"
[[ "$PACKAGE_TAG" =~ ^[a-zA-Z0-9_-]*$ ]] || { echo 'Package tag must contain letters, numbers, underscores or hyphens' >&2; exit 1; }
[[ "$PROFILE" == debug || "$PROFILE" == release ]] || { echo 'Profile must be debug or release' >&2; exit 1; }
command -v docker >/dev/null
mkdir -p "$ROOT/logs" "$ROOT/dist"
STAGE="$ROOT/dist/bevy-open-rts-windows-gnu-x86_64-$PROFILE${PACKAGE_TAG:+-$PACKAGE_TAG}"
[[ ! -e "$STAGE" && ! -e "$STAGE.zip" ]] || { echo "Refusing to replace $STAGE" >&2; exit 1; }
# Downloads only the required Windows std and compiler. Separate target directory
# avoids invalidating or locking the normal Linux build. Limit jobs for Bevy RAM.
docker run --rm -v "$ROOT:/work" -v "${CARGO_HOME:-$HOME/.cargo}/registry:/usr/local/cargo/registry" -w /work -e PROFILE="$PROFILE" -e PACKAGE_TAG="$PACKAGE_TAG" rust:1.96.1-bookworm sh -ec '
  apt-get update
  apt-get install -y --no-install-recommends gcc-mingw-w64-x86-64-posix python3
  rustup target add x86_64-pc-windows-gnu
  export CARGO_TARGET_DIR=/work/target/windows-cross
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
  export CARGO_BUILD_JOBS=1
  if [ "$PROFILE" = release ]; then
    cargo build --locked --release --target x86_64-pc-windows-gnu --bin bevy-open-rts
  else
    cargo build --locked --target x86_64-pc-windows-gnu --bin bevy-open-rts --bin capture
  fi
  python3 scripts/package_windows_cross.py "$PROFILE" "$PACKAGE_TAG"
' > "$ROOT/logs/windows-cross-$PROFILE${PACKAGE_TAG:+-$PACKAGE_TAG}.log" 2>&1
printf 'Package: %s.zip\nLog: %s\n' "$STAGE" "$ROOT/logs/windows-cross-$PROFILE${PACKAGE_TAG:+-$PACKAGE_TAG}.log"
