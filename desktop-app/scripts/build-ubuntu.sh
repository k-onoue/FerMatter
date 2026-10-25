#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/environment.sh"
[[ "$(uname -s)" == Linux ]] || { echo 'Build Ubuntu packages on Linux.' >&2; exit 1; }
cd "$DESKTOP_ROOT"
[[ -d node_modules ]] || npm ci
npm run tauri build -- --bundles deb
python3 scripts/package-ubuntu.py
