#!/bin/bash
set -euo pipefail
source "$(dirname "$0")/environment.sh"
cd "$DESKTOP_ROOT"
[[ -d node_modules ]] || npm ci
npm run tauri dev
