#!/bin/bash
# Run only on a disposable CI desktop; never use the normal Library.
set -euo pipefail
temporary="$(mktemp -d)"
trap 'rm -r "$temporary"' EXIT
export MATHLIDELUX_DATA_DIR="$temporary/data"
export FERMATTER_STARTUP_LOG="$temporary/startup.log"
xvfb-run -a dbus-run-session -- bash -se <<'SH'
fermatter > "$FERMATTER_STARTUP_LOG" 2>&1 &
process=$!
trap 'kill "$process" 2>/dev/null || true' EXIT
sleep 8
if ! kill -0 "$process" 2>/dev/null; then
  cat "$FERMATTER_STARTUP_LOG"
  echo 'FerMatter exited during headless startup.' >&2
  exit 1
fi
echo 'Ubuntu headless startup smoke check passed.'
SH
