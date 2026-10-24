#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

project_dir="$(cd "$(dirname "$0")/.." && pwd)"
app_dir=""
output_dir="$project_dir/build/distribution"
overwrite=0
public=0
preview=0
notary_profile=""
installer_identity=""
signing_identity=""
validation=""

usage() {
    cat <<'USAGE'
Usage: ./desktop-app/scripts/package.sh [--app PATH] [--output DIR] [--overwrite] [--preview]

Build and verify a prototype DMG and PKG for Apple Silicon, macOS 14+.
Without --app, build.sh first builds the app from the current checkout.
--app PATH     Package an existing ad hoc signed FerMatter.app without rebuilding.
--output DIR   Output directory; its parent must exist (default: build/distribution).
--overwrite    Replace existing artifacts with the same version and build number.
--preview      Label an existing ad hoc --app as a public, unnotarized preview.
--help         Show this help.

Public preparation: add --public --notary-profile NAME --signing-identity NAME
  --installer-identity NAME --validation REPORT. Requires an existing --app,
  Developer ID signatures and a passing release-check.sh report for that bundle.
  Public mode submits app, DMG and PKG to Apple, staples accepted tickets, and
  verifies Gatekeeper. It never installs or publishes a release.
Default prototype mode performs no notarization or production signing.
USAGE
}

fail() { printf 'package.sh: %s\n' "$*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --public) public=1; shift ;;
        --preview) preview=1; shift ;;
        --notary-profile|--installer-identity|--signing-identity|--validation)
            [[ $# -ge 2 && -n "$2" ]] || fail "$1 requires a value"
            case "$1" in
                --notary-profile) notary_profile="$2" ;;
                --installer-identity) installer_identity="$2" ;;
                --signing-identity) signing_identity="$2" ;;
                --validation) validation="$2" ;;
            esac
            shift 2 ;;
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
[[ "$public$preview" != 11 ]] || fail "Choose --public or --preview."
[[ "$preview" != 1 || -n "$app_dir" ]] || fail "Preview mode requires an explicit --app."
if [[ "$public" = 1 ]]; then
    [[ -n "$app_dir" && -n "$notary_profile" && -n "$validation" ]] || fail "Public mode requires --app, --notary-profile and --validation."
    [[ "$signing_identity" = "Developer ID Application: "* ]] || fail "Use the exact Developer ID Application identity name."
    [[ "$installer_identity" = "Developer ID Installer: "* ]] || fail "Use the exact Developer ID Installer identity name."
else
    [[ -z "$notary_profile$installer_identity$signing_identity$validation" ]] || fail "Production options require --public."
fi
if [[ -z "$app_dir" ]]; then
    "$project_dir/scripts/build.sh"
    app_dir="$project_dir/src-tauri/target/release/bundle/macos/FerMatter.app"
fi
[[ -d "$app_dir/Contents" ]] || fail "App bundle not found: $app_dir"
app_dir="$(cd "$app_dir" && pwd -P)"
plist="$app_dir/Contents/Info.plist"
plist_value() { /usr/libexec/PlistBuddy -c "Print :$1" "$plist"; }
[[ "$(plist_value CFBundleIdentifier)" = local.slidemath.prototype ]] || fail "Unexpected app bundle identifier."
[[ "$(plist_value CFBundleExecutable)" = FerMatter ]] || fail "Unexpected app executable."
version="$(plist_value CFBundleShortVersionString)"
build="$(plist_value CFBundleVersion)"
minimum_os="$(plist_value LSMinimumSystemVersion)"
[[ "$version" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || fail "Expected a numeric release version, got: $version"
[[ "$build" =~ ^[0-9]+$ ]] || fail "Expected a numeric build number, got: $build"
[[ "$minimum_os" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || fail "Invalid minimum macOS version."
[[ "${minimum_os%%.*}" -ge 14 ]] || fail "This prototype packaging requires macOS 14 or later."
[[ "$(/usr/bin/lipo -archs "$app_dir/Contents/MacOS/FerMatter")" = arm64 ]] || fail "This prototype packaging supports an arm64-only app."
/usr/bin/codesign --verify --deep --strict "$app_dir"
signature="$(/usr/bin/codesign --display --verbose=2 "$app_dir" 2>&1)"
if [[ "$public" = 1 ]]; then
    [[ "$signature" = *"Authority=$signing_identity"* && "$signature" = *"runtime"* && "$signature" = *"Timestamp="* ]] || fail "App must have the selected Developer ID signature, hardened runtime and secure timestamp."
    [[ -z "$(git -C "$project_dir/.." status --porcelain)" ]] || fail "Public packaging requires a clean checkout."
    node "$project_dir/scripts/release-report.mjs" verify "$validation" "$(git -C "$project_dir/.." rev-parse HEAD)" "$app_dir"
    for legal_file in JAVASCRIPT-LICENSES.txt RUST-LICENSES.txt; do
        [[ -s "$app_dir/Contents/Resources/licenses/$legal_file" ]] || fail "Missing bundled dependency notices: $legal_file"
    done
else
    [[ "$signature" = *$'\nSignature=adhoc'* ]] || fail "Expected an ad hoc signed development app; use --public for Developer ID packages."
fi
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
label="Prototype"
suffix="prototype-unnotarized"
readme="$project_dir/packaging/PROTOTYPE-README.txt"
if [[ "$public" = 1 ]]; then
    label="Release Candidate"
    suffix="notarized"
    readme="$project_dir/packaging/RELEASE-README.txt"
elif [[ "$preview" = 1 ]]; then
    label="Unnotarized Preview"
    suffix="preview-unnotarized"
    readme="$project_dir/packaging/PREVIEW-README.txt"
fi
artifact="FerMatter-${version}-build${build}-macOS-arm64-${suffix}"
for extension in dmg pkg sha256 validation.json notarization.json; do
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
/usr/bin/ditto "$app_dir" "$scratch/dmg/FerMatter.app"
/usr/bin/codesign --verify --deep --strict "$scratch/dmg/FerMatter.app"
notarize() {
    local target="$1" name="$2"
    /usr/bin/xcrun notarytool submit "$target" --keychain-profile "$notary_profile" --wait --output-format json > "$scratch/$name-notary.json"
    node -e 'const r=require(process.argv[1]);if(r.status!=="Accepted")throw new Error("Notarization was not accepted: "+r.status)' "$scratch/$name-notary.json"
}
if [[ "$public" = 1 ]]; then
    # Staple only the staging copy; the exact input bundle remains unchanged.
    /usr/bin/ditto -c -k --keepParent "$scratch/dmg/FerMatter.app" "$scratch/FerMatter.zip"
    notarize "$scratch/FerMatter.zip" app
    /usr/bin/xcrun stapler staple "$scratch/dmg/FerMatter.app"
    /usr/bin/xcrun stapler validate "$scratch/dmg/FerMatter.app"
    /usr/sbin/spctl --assess --type execute --verbose=2 "$scratch/dmg/FerMatter.app"
fi
ln -s /Applications "$scratch/dmg/Applications"
{
    printf 'FerMatter %s (build %s) — Apple Silicon, macOS %s+\n\n' "$version" "$build" "$minimum_os"
    cat "$readme"
} > "$scratch/dmg/README.txt"
for legal_file in LICENSE NOTICE; do
    cp "$app_dir/Contents/Resources/$legal_file" "$scratch/dmg/$legal_file"
    cp "$scratch/dmg/$legal_file" "$scratch/resources/$legal_file"
done
cp "$scratch/dmg/README.txt" "$scratch/resources/README.txt"
cp "$scratch/dmg/LICENSE" "$scratch/resources/LICENSE.txt"
/usr/bin/ditto "$scratch/dmg/FerMatter.app" "$scratch/payload/Applications/FerMatter.app"

# Disable relocation: Installer must use /Applications, not another detected copy.
cat > "$scratch/components.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><array><dict>
<key>RootRelativeBundlePath</key><string>Applications/FerMatter.app</string>
<key>BundleIsRelocatable</key><false/>
<key>BundleIsVersionChecked</key><true/>
<key>BundleHasStrictIdentifier</key><true/>
<key>BundleOverwriteAction</key><string>upgrade</string>
</dict></array></plist>
PLIST
/usr/bin/pkgbuild --root "$scratch/payload" --component-plist "$scratch/components.plist" \
    --identifier local.slidemath.prototype.pkg --version "$version" \
    --install-location / --ownership recommended "$scratch/FerMatter-component.pkg"
cat > "$scratch/Distribution.xml" <<XML
<?xml version="1.0" encoding="UTF-8"?>
<installer-gui-script minSpecVersion="2">
    <title>FerMatter ${version} — ${label}</title>
    <readme file="README.txt" mime-type="text/plain"/>
    <license file="LICENSE.txt" mime-type="text/plain"/>
    <options customize="never" require-scripts="false" hostArchitectures="arm64"/>
    <domains enable_anywhere="false" enable_currentUserHome="false" enable_localSystem="true"/>
    <volume-check><allowed-os-versions><os-version min="${minimum_os}"/></allowed-os-versions></volume-check>
    <choices-outline><line choice="app"/></choices-outline>
    <choice id="app" title="FerMatter" visible="false"><pkg-ref id="local.slidemath.prototype.pkg"/></choice>
    <pkg-ref id="local.slidemath.prototype.pkg" version="${version}" onConclusion="none">FerMatter-component.pkg</pkg-ref>
</installer-gui-script>
XML
product_args=(--distribution "$scratch/Distribution.xml" --package-path "$scratch" --resources "$scratch/resources")
if [[ "$public" = 1 ]]; then product_args+=(--sign "$installer_identity" --timestamp); fi
/usr/bin/productbuild "${product_args[@]}" "$scratch/$artifact.pkg"
# Keep packaging dependencies isolated from both the app and the system Python.
dmg_python="$project_dir/build/dmg-tools/bin/python3"
if [[ ! -x "$dmg_python" ]]; then
    python3 -m venv "$project_dir/build/dmg-tools"
fi
"$dmg_python" -m pip install --disable-pip-version-check --only-binary=:all: \
    --require-hashes -r "$project_dir/packaging/dmg-requirements.txt"
/usr/bin/xcrun swift "$project_dir/packaging/dmg-background.swift" "$scratch/arrow"
/usr/bin/tiffutil -cat "$scratch/arrow.png" "$scratch/arrow@2x.png" -out "$scratch/arrow.tiff"
"$dmg_python" "$project_dir/packaging/create-dmg.py" build \
    "$scratch/dmg" "$scratch/$artifact.dmg" "FerMatter ${version} ${label}" "$scratch/arrow.tiff"

if [[ "$public" = 1 ]]; then
    /usr/bin/codesign --sign "$signing_identity" --timestamp "$scratch/$artifact.dmg"
    for extension in dmg pkg; do
        notarize "$scratch/$artifact.$extension" "$extension"
        /usr/bin/xcrun stapler staple "$scratch/$artifact.$extension"
        /usr/bin/xcrun stapler validate "$scratch/$artifact.$extension"
    done
    /usr/sbin/spctl --assess --type install --verbose=2 "$scratch/$artifact.pkg"
    /usr/sbin/spctl --assess --type open --context context:primary-signature --verbose=2 "$scratch/$artifact.dmg"
fi
# Validate both delivery paths without installing or launching the app.
/usr/bin/hdiutil verify "$scratch/$artifact.dmg"
mount_pending=1
/usr/bin/hdiutil attach "$scratch/$artifact.dmg" -readonly -nobrowse -noautoopen -mountpoint "$mount_dir"
[[ -L "$mount_dir/Applications" && "$(readlink "$mount_dir/Applications")" = /Applications ]] || fail "DMG Applications shortcut is invalid."
"$dmg_python" "$project_dir/packaging/create-dmg.py" verify "$mount_dir" "$scratch/arrow.tiff"
/usr/bin/codesign --verify --deep --strict "$mount_dir/FerMatter.app"
for file in README.txt LICENSE NOTICE; do
    cmp "$scratch/dmg/$file" "$mount_dir/$file"
done
/usr/bin/hdiutil detach "$mount_dir"
mount_pending=0
/usr/sbin/pkgutil --expand-full "$scratch/$artifact.pkg" "$scratch/expanded"
expanded_app="$scratch/expanded/FerMatter-component.pkg/Payload/Applications/FerMatter.app"
/usr/bin/codesign --verify --deep --strict "$expanded_app"
diff -qr "$scratch/dmg/FerMatter.app" "$expanded_app"
pkg_info="$scratch/expanded/FerMatter-component.pkg/PackageInfo"
[[ "$(/usr/bin/xmllint --xpath 'string(/pkg-info/@install-location)' "$pkg_info")" = / ]] || fail "Unexpected PKG installation root."
[[ "$(/usr/bin/xmllint --xpath 'string(/pkg-info/@relocatable)' "$pkg_info")" = false ]] || fail "PKG must not be relocatable."
[[ "$(/usr/bin/xmllint --xpath 'count(/pkg-info/relocate/bundle)' "$pkg_info")" = 0 ]] || fail "PKG must not relocate the app."
[[ ! -e "$scratch/expanded/FerMatter-component.pkg/Scripts" ]] || fail "Unexpected installer scripts."
if [[ "$public" = 1 ]]; then
    /usr/sbin/pkgutil --check-signature "$scratch/$artifact.pkg" > "$scratch/pkg-signature.txt"
    /usr/bin/grep -Fq "$installer_identity" "$scratch/pkg-signature.txt" || fail "Unexpected installer identity."
    cp "$validation" "$scratch/$artifact.validation.json"
    node -e 'const fs=require("node:fs"),p=process.argv[1];fs.writeFileSync(process.argv[2],JSON.stringify(Object.fromEntries(["app","dmg","pkg"].map(k=>[k,JSON.parse(fs.readFileSync(p+"/"+k+"-notary.json"))])),null,2)+"\n")' "$scratch" "$scratch/$artifact.notarization.json"
else
    if /usr/sbin/pkgutil --check-signature "$scratch/$artifact.pkg" > "$scratch/pkg-signature.txt" 2>&1; then fail "Expected an unsigned prototype installer."; fi
    /usr/bin/grep -q 'Status: no signature' "$scratch/pkg-signature.txt" || fail "Could not confirm unsigned PKG status."
    printf '{"status":"%s"}\n' "$suffix" > "$scratch/$artifact.notarization.json"
    printf '{"status":"not-a-release-validation"}\n' > "$scratch/$artifact.validation.json"
fi
cat "$scratch/pkg-signature.txt"
(
    cd "$scratch"
    /usr/bin/shasum -a 256 "$artifact.dmg" "$artifact.pkg" "$artifact.validation.json" "$artifact.notarization.json" > "$artifact.sha256"
)
for extension in dmg pkg sha256 validation.json notarization.json; do
    mv -f "$scratch/$artifact.$extension" "$output_dir/$artifact.$extension"
done
printf '\nVerified %s artifacts (installation and manual validation are separate):\n' "$suffix"
printf '%s/%s.%s\n' "$output_dir" "$artifact" dmg "$output_dir" "$artifact" pkg "$output_dir" "$artifact" sha256
