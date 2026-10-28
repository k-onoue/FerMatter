#!/usr/bin/env python3
"""Verify WKWebView LocalStorage across the bundle-ID alias on a synthetic profile."""
import json
import pathlib
import plistlib
import shutil
import subprocess
import sys
import tempfile
import uuid

if sys.platform != "darwin":
    raise SystemExit("Run this check on macOS.")
script = pathlib.Path(__file__).resolve().parent
output = script.parent / "build" / "webview-migration"
output.mkdir(parents=True, exist_ok=True)
identifier = "io.github.fermatter.migrationprobe." + uuid.uuid4().hex
profiles = [pathlib.Path.home() / "Library/WebKit" / (identifier + suffix) for suffix in (".old", ".new")]
try:
    with tempfile.TemporaryDirectory(prefix="probe-", dir=output) as temporary:
        root = pathlib.Path(temporary)
        subprocess.run(["xcrun", "swiftc", str(script / "webview-migration-probe.swift"), "-o", str(root / "probe")], check=True)
        for role, profile in zip(("old", "new"), profiles):
            app = root / (role + ".app")
            (app / "Contents/MacOS").mkdir(parents=True)
            shutil.copy2(root / "probe", app / "Contents/MacOS/probe")
            (app / "Contents/Info.plist").write_bytes(plistlib.dumps({"CFBundleIdentifier": profile.name, "CFBundleExecutable": "probe", "CFBundlePackageType": "APPL", "CFBundleName": "FerMatter migration probe"}))
            subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
        subprocess.run([str(root / "old.app/Contents/MacOS/probe"), "write"], check=True, timeout=40)
        if not profiles[0].is_dir():
            raise RuntimeError("The legacy WKWebView did not create its expected data store.")
        # Production path creation/conflict handling is covered by identity.rs unit tests.
        profiles[1].symlink_to(profiles[0], target_is_directory=True)
        subprocess.run([str(root / "new.app/Contents/MacOS/probe"), "read"], check=True, timeout=40)
        subprocess.run([str(root / "old.app/Contents/MacOS/probe"), "read"], check=True, timeout=40)
        (output / "verification.json").write_text(json.dumps({"status": "passed", "scope": "Synthetic WKWebView LocalStorage survives new bundle ID and rollback through the legacy alias", "nativeInstall": "not tested by this probe"}, indent=2) + "\n")
finally:
    # Only the two randomly named synthetic profiles created by this invocation.
    for profile in reversed(profiles):
        if profile.is_symlink():
            profile.unlink()
        elif profile.is_dir():
            shutil.rmtree(profile)
print("PASS: synthetic WKWebView pending data survived bundle-ID migration and rollback.")
