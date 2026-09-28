#!/bin/bash
set -euo pipefail
source "$(dirname "$0")/environment.sh"
cd "$DESKTOP_ROOT"
signing_identity="${APPLE_SIGNING_IDENTITY:-}"
signing_file="$DESKTOP_ROOT/build/signing-identity.txt"
if [[ -z "$signing_identity" && -f "$signing_file" ]]; then
  signing_identity="$(cat "$signing_file")"
  [[ -n "$signing_identity" ]] || { echo "Signing identity file is empty: $signing_file" >&2; exit 1; }
fi
build_args=(--bundles app)
if [[ -n "$signing_identity" ]]; then
  [[ "$signing_identity" != *$'\n'* ]] || { echo "Use one signing identity." >&2; exit 1; }
  if [[ "$signing_identity" != "-" ]]; then
    # Fail instead of silently falling back to an ad hoc signature: changing
    # the app's designated requirement invalidates remembered macOS consent.
    signing_match="\"$signing_identity\""
    if [[ "$signing_identity" =~ ^[[:xdigit:]]{40}$ ]]; then
      signing_identity="$(printf '%s' "$signing_identity" | tr '[:lower:]' '[:upper:]')"
      signing_match=" $signing_identity \""
    fi
    signing_identities="$(/usr/bin/security find-identity -v -p codesigning)"
    if ! printf '%s\n' "$signing_identities" | /usr/bin/grep -Fq -- "$signing_match"; then
      echo "Configured signing certificate is unavailable. See dev-docs/reference/signing.md; no ad hoc fallback was used." >&2
      exit 1
    fi
  fi
  signing_config="$(node -e 'console.log(JSON.stringify({bundle:{macOS:{signingIdentity:process.argv[1]}}}))' -- "$signing_identity")"
  build_args+=(--config "$signing_config")
else
  echo "Ad hoc build: macOS may ask for folder access again after rebuilding. See dev-docs/reference/signing.md."
fi
[[ -d node_modules ]] || npm ci
npm run tauri build -- "${build_args[@]}" "$@"
