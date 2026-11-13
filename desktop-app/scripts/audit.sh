#!/bin/bash
set -euo pipefail
source "$(dirname "$0")/environment.sh"
export PATH="$DESKTOP_ROOT/build/audit-tools/bin:$PATH"
output="${1:-$DESKTOP_ROOT/build/audit}"
mkdir -p "$output"
output="$(cd "$output" && pwd)"
cd "$DESKTOP_ROOT"
# npm audit reads the lockfile; installing/building dependencies is unnecessary.
failed=0
npm audit --omit=dev --json > "$output/npm.json" || failed=1
if cargo audit --version > "$output/cargo-audit-version.txt" 2>&1; then
  cargo audit --file src-tauri/Cargo.lock --json > "$output/rust.json" || failed=1
else
  echo 'Missing cargo-audit. Run: bash desktop-app/scripts/install-auditor.sh' >&2
  failed=1
fi
# Reports include advisory details; audit never silently adds ignores or updates dependencies.
if [[ "$failed" != 0 ]]; then
  echo "Dependency audit failed. Reports: $output" >&2
  [[ ! -f "$output/rust.json" ]] || cat "$output/rust.json" >&2
fi
exit "$failed"
