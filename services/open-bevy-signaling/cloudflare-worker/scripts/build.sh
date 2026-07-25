#!/usr/bin/env bash
set -euo pipefail

if command -v worker-build >/dev/null 2>&1; then
  exec worker-build --release
fi

if [[ -x "${HOME}/.cargo/bin/worker-build" ]]; then
  exec "${HOME}/.cargo/bin/worker-build" --release
fi

echo "worker-build is required; install it with: cargo install worker-build --version 0.8.5 --locked" >&2
exit 127
