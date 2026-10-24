#!/bin/bash
set -euo pipefail
source "$(dirname "$0")/environment.sh"
cd "$DESKTOP_ROOT"
if ! command -v cargo >/dev/null 2>&1; then
  export PATH="$(dirname "$(rustup which cargo)"):$PATH"
fi
exec cargo fmt --manifest-path src-tauri/Cargo.toml --all -- "$@"
