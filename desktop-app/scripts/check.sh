#!/bin/bash
set -euo pipefail
source "$(dirname "$0")/environment.sh"
cd "$DESKTOP_ROOT"
[[ -d node_modules ]] || npm ci
npm run build
npm test
node --test scripts/*.check.mjs
cargo test --manifest-path src-tauri/Cargo.toml --lib
if [[ "${MATHLIDELUX_COMPILER_TESTS:-1}" == "1" ]]; then
  cargo run --manifest-path src-tauri/Cargo.toml --example verify -- "$DESKTOP_ROOT/.." "$DESKTOP_ROOT/build/qa-data"
else
  echo "SKIPPED: real LaTeX, Typst, TikZ and CeTZ compiler checks."
fi
