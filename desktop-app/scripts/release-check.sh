#!/bin/bash
set -euo pipefail
source "$(dirname "$0")/environment.sh"
root="$(cd "$DESKTOP_ROOT/.." && pwd)"
export PATH="/Library/TeX/texbin:/opt/homebrew/bin:$PATH"
[[ "${MATHLIDELUX_COMPILER_TESTS:-1}" == 1 ]] || { echo 'Release validation cannot skip real compiler checks.' >&2; exit 1; }
allow_dirty=0
[[ "${1:-}" != --allow-dirty ]] || { allow_dirty=1; shift; }
[[ $# == 0 ]] || { echo 'Usage: release-check.sh [--allow-dirty]' >&2; exit 2; }
dirty=clean
[[ -z "$(git -C "$root" status --porcelain)" ]] || dirty=dirty
[[ "$dirty" == clean || "$allow_dirty" == 1 ]] || { echo 'Commit the intended changes before release validation (or use --allow-dirty for a non-release rehearsal).' >&2; exit 1; }
commit="$(git -C "$root" rev-parse HEAD)"
mkdir -p "$DESKTOP_ROOT/build/release-validation"
output="$(mktemp -d "$DESKTOP_ROOT/build/release-validation/run.XXXXXX")"
if [[ -n "${CARGO_TARGET_DIR:-}" && "$CARGO_TARGET_DIR" != /* ]]; then
  export CARGO_TARGET_DIR="$PWD/$CARGO_TARGET_DIR"
fi
app="${CARGO_TARGET_DIR:-$DESKTOP_ROOT/src-tauri/target}/release/bundle/macos/FerMatter.app"
touch "$output/steps.tsv"
status=failed
finish() { node "$DESKTOP_ROOT/scripts/release-report.mjs" write "$output" "$root" "$app" "$status" "$commit" "$dirty"; echo "Release evidence: $output/report.json"; }
trap finish EXIT
step() {
  local name="$1"; shift
  if "$@" > "$output/$name.log" 2>&1; then
    printf '%s\tpassed\n' "$name" >> "$output/steps.tsv"
  else
    printf '%s\tfailed\n' "$name" >> "$output/steps.tsv"
    tail -40 "$output/$name.log" >&2
    return 1
  fi
}
step check env MATHLIDELUX_COMPILER_TESTS=1 "$DESKTOP_ROOT/scripts/check.sh"
step clippy cargo clippy --manifest-path "$DESKTOP_ROOT/src-tauri/Cargo.toml" --lib -- -D warnings
step build "$DESKTOP_ROOT/scripts/build.sh"
step audit "$DESKTOP_ROOT/scripts/audit.sh" "$output/audit"
step publication node "$root/scripts/check-publication.mjs"
cp "$DESKTOP_ROOT/build/qa-data/verification.json" "$output/compiler-verification.json"
[[ "$(git -C "$root" rev-parse HEAD)" == "$commit" ]] || { echo 'Commit changed during validation.' >&2; exit 1; }
[[ "$dirty" != clean || -z "$(git -C "$root" status --porcelain)" ]] || { echo 'Tracked inputs changed during validation.' >&2; exit 1; }
status=passed
