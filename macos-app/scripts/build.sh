#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd "$(dirname "$0")/.." && pwd)"
if [[ -z "${DEVELOPER_DIR:-}" && -d /Applications/Xcode.app/Contents/Developer ]]; then
    export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi
xcrun swift build --package-path "$project_dir" -c release
binary_dir="$(xcrun swift build --package-path "$project_dir" -c release --show-bin-path)"
app_dir="$project_dir/build/Mathlidelux.app"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
# Compile the native icon stack so macOS 26 preserves the blue enclosure.
# A standalone legacy ICNS is otherwise inset onto a system-generated plate.
xcrun actool "$project_dir/Resources/AppIcon.icon" \
    --compile "$app_dir/Contents/Resources" \
    --app-icon AppIcon --platform macosx --minimum-deployment-target 26.0 \
    --output-partial-info-plist "$project_dir/build/icon-partial.plist" \
    --output-format human-readable-text --warnings --notices
# Remove bytecode left by older development builds before sealing Resources.
rm -rf -- "$app_dir/Contents/Resources/__pycache__"
cp "$binary_dir/Mathlidelux" "$app_dir/Contents/MacOS/Mathlidelux"
cp "$project_dir/Resources/AppIcon.icon/Assets/AppIcon.png" "$app_dir/Contents/Resources/AppMark.png"
cp "$project_dir/Resources/tex_worker.py" "$app_dir/Contents/Resources/tex_worker.py"
cp "$project_dir/Resources/typst_worker.py" "$app_dir/Contents/Resources/typst_worker.py"
cp "$project_dir/Resources/project_outline.py" "$app_dir/Contents/Resources/project_outline.py"
cp "$project_dir/Resources/source_preview.py" "$app_dir/Contents/Resources/source_preview.py"
cp "$project_dir/Resources/document_worker.py" "$app_dir/Contents/Resources/document_worker.py"
# Keep the paired themes and their upstream licenses in the distributable app.
ditto "$binary_dir/Mathlidelux_SlideMath.bundle" "$app_dir/Contents/Resources/Mathlidelux_SlideMath.bundle"
# Apache-2.0 §4 requires the license and NOTICE to travel with every distributed copy.
cp "$project_dir/../LICENSE" "$app_dir/Contents/Resources/LICENSE"
cp "$project_dir/../NOTICE" "$app_dir/Contents/Resources/NOTICE"
cat > "$app_dir/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Mathlidelux</string>
<key>CFBundleDisplayName</key><string>Mathlidelux</string>
<key>CFBundleIdentifier</key><string>local.slidemath.prototype</string>
<key>CFBundleExecutable</key><string>Mathlidelux</string>
<key>CFBundleIconFile</key><string>AppIcon</string>
<key>CFBundleIconName</key><string>AppIcon</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.10.0</string>
<key>CFBundleVersion</key><string>10</string>
<key>CFBundleDevelopmentRegion</key><string>en</string>
<key>LSMinimumSystemVersion</key><string>26.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
# Keep the original bundle identifier above so existing preferences survive branding changes.
codesign --force --sign - "$app_dir"
codesign --verify --deep --strict "$app_dir"
# Notify Finder/Launch Services after rebuilding an existing bundle in place.
touch "$app_dir"
echo "Built: $app_dir"
