#!/bin/bash
# Install a hash-pinned official binary into ignored build tools, not the system.
set -euo pipefail
source "$(dirname "$0")/environment.sh"
version=0.22.2
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    target=x86_64-unknown-linux-gnu
    checksum=ab28a1bdb54db4d5d8ad5981cf1f959410370b3d28250dbd35f6a44248620e39
    ;;
  Darwin-arm64)
    target=aarch64-apple-darwin
    checksum=ec7ca4263769593df4d909be85b94a6b79efa2897be5d2bb8ebd516e823175af
    ;;
  *) echo 'No pinned auditor for this platform; install cargo-audit 0.22.2 with cargo.' >&2; exit 1 ;;
esac
tools="$DESKTOP_ROOT/build/audit-tools"
mkdir -p "$tools/bin"
stage="$(mktemp -d "$tools/download.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
archive="cargo-audit-$target-v$version"
curl -fL --retry 3 -o "$stage/audit.tgz" "https://github.com/rustsec/rustsec/releases/download/cargo-audit/v$version/$archive.tgz"
if [[ "$(uname -s)" == Darwin ]]; then
  echo "$checksum  $stage/audit.tgz" | shasum -a 256 --check
else
  echo "$checksum  $stage/audit.tgz" | sha256sum --check
fi
tar -xzf "$stage/audit.tgz" -C "$stage"
install -m 755 "$stage/$archive/cargo-audit" "$tools/bin/cargo-audit"
"$tools/bin/cargo-audit" --version
