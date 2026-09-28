#!/bin/bash
set -euo pipefail
source "$(dirname "$0")/environment.sh"
cd "$DESKTOP_ROOT"
export PATH="$DESKTOP_ROOT/build/audit-tools/bin:$PATH"
output="${1:-$DESKTOP_ROOT/build/audit}"
mkdir -p "$output"
[[ -d node_modules ]] || npm ci
failed=0
npm audit --omit=dev --json > "$output/npm.json" || failed=1
if cargo audit --version > "$output/cargo-audit-version.txt" 2>&1; then
  cargo audit --file src-tauri/Cargo.lock --json > "$output/rust.json" || failed=1
else
  echo 'Missing cargo-audit. Install with: cargo install --locked cargo-audit --version 0.22.2 --root desktop-app/build/audit-tools' >&2
  failed=1
fi
# Reports include advisory details; audit never silently adds ignores or updates dependencies.
exit "$failed"
