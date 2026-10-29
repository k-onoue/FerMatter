#!/bin/bash
# Run only on a disposable CI desktop; never use the normal Library.
set -euo pipefail
temporary="$(mktemp -d)"
trap 'rm -r "$temporary"' EXIT
export MATHLIDELUX_DATA_DIR="$temporary/data"
export FERMATTER_STARTUP_LOG="$temporary/startup.log"
xvfb-run -a dbus-run-session -- bash -se <<'SH'
python3 - <<'PY'
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gio, Gtk

Gtk.init([])
desktop = Gio.DesktopAppInfo.new("FerMatter.desktop")
assert desktop is not None, "Missing installed desktop entry"
icon = desktop.get_icon()
assert icon is not None, "Missing desktop icon reference"
theme = Gtk.IconTheme.get_default()
for size in (32, 64, 128, 256, 512):
    info = theme.lookup_by_gicon(icon, size, Gtk.IconLookupFlags.FORCE_SIZE)
    assert info is not None, f"Desktop icon cannot be resolved at {size} px"
    pixels = info.load_icon()
    assert pixels.get_width() > 0 and pixels.get_height() > 0
print("Installed Ubuntu desktop icon lookup passed.")
PY
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
