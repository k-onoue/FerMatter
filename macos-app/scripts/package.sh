#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

project_dir="$(cd "$(dirname "$0")/.." && pwd)"
app_dir=""
output_dir="$project_dir/build/distribution"
overwrite=0

usage() {
    cat <<'USAGE'
Usage: ./macos-app/scripts/package.sh [--app PATH] [--output DIR] [--overwrite]

Build and verify a prototype DMG and PKG for Apple Silicon, macOS 26+.
Without --app, build.sh first builds the app from the current checkout.
--app PATH     Package an existing ad hoc signed Mathlidelux.app without rebuilding.
--output DIR   Output directory; its parent must exist (default: build/distribution).
--overwrite    Replace existing artifacts with the same version and build number.
--help         Show this help.

No Developer ID signing, notarization, installation, or publishing is performed.
USAGE
}

fail() { printf 'package.sh: %s\n' "$*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --app|--output)
            [[ $# -ge 2 && -n "$2" ]] || fail "$1 requires a value"
            if [[ "$1" = --app ]]; then app_dir="$2"; else output_dir="$2"; fi
            shift 2 ;;
        --overwrite) overwrite=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) usage >&2; fail "Unknown argument: $1" ;;
    esac
done

[[ "$(uname -s)" = Darwin ]] || fail "Packaging requires macOS."
if [[ -z "$app_dir" ]]; then
    "$project_dir/scripts/build.sh"
    app_dir="$project_dir/build/Mathlidelux.app"
fi
[[ -d "$app_dir/Contents" ]] || fail "App bundle not found: $app_dir"
app_dir="$(cd "$app_dir" && pwd -P)"
plist="$app_dir/Contents/Info.plist"
plist_value() { /usr/libexec/PlistBuddy -c "Print :$1" "$plist"; }
[[ "$(plist_value CFBundleIdentifier)" = local.slidemath.prototype ]] || fail "Unexpected app bundle identifier."
[[ "$(plist_value CFBundleExecutable)" = Mathlidelux ]] || fail "Unexpected app executable."
version="$(plist_value CFBundleShortVersionString)"
build="$(plist_value CFBundleVersion)"
minimum_os="$(plist_value LSMinimumSystemVersion)"
[[ "$version" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || fail "Expected a numeric release version, got: $version"
[[ "$build" =~ ^[0-9]+$ ]] || fail "Expected a numeric build number, got: $build"
[[ "$minimum_os" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || fail "Invalid minimum macOS version."
[[ "${minimum_os%%.*}" -ge 26 ]] || fail "This prototype packaging requires macOS 26 or later."
[[ "$(/usr/bin/lipo -archs "$app_dir/Contents/MacOS/Mathlidelux")" = arm64 ]] || fail "This prototype packaging supports an arm64-only app."
/usr/bin/codesign --verify --deep --strict "$app_dir"
signature="$(/usr/bin/codesign --display --verbose=2 "$app_dir" 2>&1)"
[[ "$signature" = *$'\nSignature=adhoc'* ]] || fail "Expected an ad hoc signed development app; this script does not package production releases."
for legal_file in LICENSE NOTICE; do
    [[ -s "$app_dir/Contents/Resources/$legal_file" ]] || fail "The signed app is missing $legal_file; rebuild it first."
done

# Resolve the existing parent before creating anything, so an output path (or
# symlink) inside the signed input cannot accidentally change its contents.
if [[ "$output_dir" = "$project_dir/build/distribution" ]]; then
    mkdir -p "$project_dir/build"
fi
if [[ -d "$output_dir" ]]; then
    output_dir="$(cd "$output_dir" && pwd -P)"
else
    output_parent="$(cd "$(dirname "$output_dir")" && pwd -P)" || fail "The output directory's parent must exist."
    output_dir="$output_parent/$(basename "$output_dir")"
fi
[[ "$output_dir/" != "$app_dir/"* ]] || fail "Output directory must be outside the input app."
mkdir -p "$output_dir"
artifact="Mathlidelux-${version}-build${build}-macOS-arm64-prototype-unnotarized"
for extension in dmg pkg sha256; do
    if [[ -e "$output_dir/$artifact.$extension" || -L "$output_dir/$artifact.$extension" ]]; then
        [[ "$overwrite" = 1 ]] || fail "Output exists: $output_dir/$artifact.$extension (use --overwrite)."
        [[ ! -d "$output_dir/$artifact.$extension" ]] || fail "Output path is a directory: $artifact.$extension"
    fi
done

scratch="$(mktemp -d "$output_dir/.package.XXXXXX")"
mount_dir="$scratch/mount"
mount_pending=0
cleanup() {
    if [[ "$mount_pending" = 1 ]]; then
        if ! /usr/bin/hdiutil detach "$mount_dir" -quiet; then
            printf 'Could not detach %s; leaving temporary files at %s\n' "$mount_dir" "$scratch" >&2
            return
        fi
    fi
    rm -rf -- "$scratch"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir -p "$scratch/dmg" "$scratch/payload/Applications" "$scratch/resources" "$mount_dir"

# Copy the signed bundle without modifying it or its extended attributes.
/usr/bin/ditto "$app_dir" "$scratch/dmg/Mathlidelux.app"
/usr/bin/codesign --verify --deep --strict "$scratch/dmg/Mathlidelux.app"
ln -s /Applications "$scratch/dmg/Applications"
{
    printf 'Mathlidelux %s (build %s) — Apple Silicon, macOS %s+\n\n' "$version" "$build" "$minimum_os"
    cat "$project_dir/packaging/PROTOTYPE-README.txt"
} > "$scratch/dmg/README.txt"
for legal_file in LICENSE NOTICE; do
    cp "$app_dir/Contents/Resources/$legal_file" "$scratch/dmg/$legal_file"
    cp "$scratch/dmg/$legal_file" "$scratch/resources/$legal_file"
done
cp "$scratch/dmg/README.txt" "$scratch/resources/README.txt"
cp "$scratch/dmg/LICENSE" "$scratch/resources/LICENSE.txt"
/usr/bin/ditto "$scratch/dmg/Mathlidelux.app" "$scratch/payload/Applications/Mathlidelux.app"

# Disable relocation: Installer must use /Applications, not another detected copy.
cat > "$scratch/components.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><array><dict>
<key>RootRelativeBundlePath</key><string>Applications/Mathlidelux.app</string>
<key>BundleIsRelocatable</key><false/>
<key>BundleIsVersionChecked</key><true/>
<key>BundleHasStrictIdentifier</key><true/>
<key>BundleOverwriteAction</key><string>upgrade</string>
</dict></array></plist>
PLIST
/usr/bin/pkgbuild --root "$scratch/payload" --component-plist "$scratch/components.plist" \
    --identifier local.slidemath.prototype.pkg --version "$version" \
    --install-location / --ownership recommended "$scratch/Mathlidelux-component.pkg"
cat > "$scratch/Distribution.xml" <<XML
<?xml version="1.0" encoding="UTF-8"?>
<installer-gui-script minSpecVersion="2">
    <title>Mathlidelux ${version} — Unnotarized Prototype</title>
    <readme file="README.txt" mime-type="text/plain"/>
    <license file="LICENSE.txt" mime-type="text/plain"/>
    <options customize="never" require-scripts="false" hostArchitectures="arm64"/>
    <domains enable_anywhere="false" enable_currentUserHome="false" enable_localSystem="true"/>
    <volume-check><allowed-os-versions><os-version min="${minimum_os}"/></allowed-os-versions></volume-check>
    <choices-outline><line choice="app"/></choices-outline>
    <choice id="app" title="Mathlidelux" visible="false"><pkg-ref id="local.slidemath.prototype.pkg"/></choice>
    <pkg-ref id="local.slidemath.prototype.pkg" version="${version}" onConclusion="none">Mathlidelux-component.pkg</pkg-ref>
</installer-gui-script>
XML
/usr/bin/productbuild --distribution "$scratch/Distribution.xml" --package-path "$scratch" \
    --resources "$scratch/resources" "$scratch/$artifact.pkg"
/usr/bin/hdiutil create -volname "Mathlidelux ${version} Prototype" -srcfolder "$scratch/dmg" \
    -fs HFS+ -format UDZO "$scratch/$artifact.dmg"

# Validate both delivery paths without installing or launching the app.
/usr/bin/hdiutil verify "$scratch/$artifact.dmg"
mount_pending=1
/usr/bin/hdiutil attach "$scratch/$artifact.dmg" -readonly -nobrowse -noautoopen -mountpoint "$mount_dir"
[[ -L "$mount_dir/Applications" && "$(readlink "$mount_dir/Applications")" = /Applications ]] || fail "DMG Applications shortcut is invalid."
/usr/bin/codesign --verify --deep --strict "$mount_dir/Mathlidelux.app"
for file in README.txt LICENSE NOTICE; do
    cmp "$scratch/dmg/$file" "$mount_dir/$file"
done
/usr/bin/hdiutil detach "$mount_dir"
mount_pending=0
/usr/sbin/pkgutil --expand-full "$scratch/$artifact.pkg" "$scratch/expanded"
expanded_app="$scratch/expanded/Mathlidelux-component.pkg/Payload/Applications/Mathlidelux.app"
/usr/bin/codesign --verify --deep --strict "$expanded_app"
diff -qr "$scratch/dmg/Mathlidelux.app" "$expanded_app"
pkg_info="$scratch/expanded/Mathlidelux-component.pkg/PackageInfo"
[[ "$(/usr/bin/xmllint --xpath 'string(/pkg-info/@install-location)' "$pkg_info")" = / ]] || fail "Unexpected PKG installation root."
[[ "$(/usr/bin/xmllint --xpath 'string(/pkg-info/@relocatable)' "$pkg_info")" = false ]] || fail "PKG must not be relocatable."
[[ "$(/usr/bin/xmllint --xpath 'count(/pkg-info/relocate/bundle)' "$pkg_info")" = 0 ]] || fail "PKG must not relocate the app."
[[ ! -e "$scratch/expanded/Mathlidelux-component.pkg/Scripts" ]] || fail "Unexpected installer scripts."
if /usr/sbin/pkgutil --check-signature "$scratch/$artifact.pkg" > "$scratch/pkg-signature.txt" 2>&1; then
    fail "Expected an unsigned prototype installer."
fi
cat "$scratch/pkg-signature.txt"
/usr/bin/grep -q 'Status: no signature' "$scratch/pkg-signature.txt" || fail "Could not confirm unsigned PKG status."
(
    cd "$scratch"
    /usr/bin/shasum -a 256 "$artifact.dmg" "$artifact.pkg" > "$artifact.sha256"
)
for extension in dmg pkg sha256; do
    mv -f "$scratch/$artifact.$extension" "$output_dir/$artifact.$extension"
done
printf '\nVerified prototype artifacts (ad hoc app; unsigned, unnotarized DMG/PKG):\n'
printf '%s/%s.%s\n' "$output_dir" "$artifact" dmg "$output_dir" "$artifact" pkg "$output_dir" "$artifact" sha256
