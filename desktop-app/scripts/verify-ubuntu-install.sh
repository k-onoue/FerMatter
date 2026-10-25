#!/bin/bash
# Resolve dependencies and install through APT in a disposable Ubuntu system.
set -euo pipefail
repository="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repository"
fingerprint="$(python3 -c 'import json; print(json.load(open("desktop-app/packaging/ubuntu.json"))["signingFingerprint"])')"
arguments=(--rm -i -e DEBIAN_FRONTEND=noninteractive -e "EXPECTED_FINGERPRINT=$fingerprint")
if [[ "${1:-}" == "--published" ]]; then
  arguments+=(-e APT_MODE=published)
else
  package="${1:?Provide a Debian package or --published}"
  package="$(realpath "$package")"
  temporary="$(mktemp -d)"
  trap 'rm -r "$temporary"' EXIT
  mkdir -m 700 "$temporary/signing"
  gpg --batch --homedir "$temporary/signing" --passphrase '' \
    --quick-gen-key 'FerMatter disposable installation test' ed25519 sign 1d
  fingerprint="$(gpg --batch --homedir "$temporary/signing" --with-colons --list-keys | awk -F: '$1 == "fpr" {print $10; exit}')"
  python3 - "$package" "$temporary" "$fingerprint" <<'PY'
import importlib.util
from pathlib import Path
import sys
spec = importlib.util.spec_from_file_location('apt_builder', 'scripts/apt-repository.py')
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)
builder.build(Path(sys.argv[1]), Path(sys.argv[2]) / 'snapshot', Path(sys.argv[2]) / 'signing', sys.argv[3])
PY
  arguments+=(-e APT_MODE=local -e "EXPECTED_FINGERPRINT=$fingerprint" \
    -v "$temporary/snapshot/apt:/repository:ro")
fi
docker run "${arguments[@]}" ubuntu:24.04 bash -se <<'SH'
apt-get update
apt-get install -y --no-install-recommends ca-certificates curl gnupg python3
install -m 0755 -d /etc/apt/keyrings
if [ "$APT_MODE" = published ]; then
  curl -fsSL https://fermatter.github.io/apt/fermatter.gpg -o /etc/apt/keyrings/fermatter.gpg
  curl -fsSL https://fermatter.github.io/apt/fermatter.sources -o /etc/apt/sources.list.d/fermatter.sources
else
  cp /repository/fermatter.gpg /etc/apt/keyrings/fermatter.gpg
  printf '%s\n' 'Types: deb' 'URIs: file:/repository' 'Suites: experimental' \
    'Components: main' 'Architectures: amd64' 'Signed-By: /etc/apt/keyrings/fermatter.gpg' \
    > /etc/apt/sources.list.d/fermatter.sources
fi
actual="$(gpg --batch --with-colons --show-keys /etc/apt/keyrings/fermatter.gpg | awk -F: '$1 == "fpr" {print $10; exit}')"
test "$actual" = "$EXPECTED_FINGERPRINT"
apt-get update
apt-get install -y --no-install-recommends fermatter
test "$(command -v fermatter)" = /usr/bin/fermatter
dpkg-query -W -f='${Package} ${Version} ${Architecture}\n' fermatter
MATHLIDELUX_DATA_DIR=/tmp/fermatter-test fermatter cli tools > /tmp/tools.json
python3 - <<'PY'
import json
result = json.load(open('/tmp/tools.json'))['result']
assert result['apiVersion'] == 1
assert len(result['tools']) == 8
assert any(tool['name'] == 'list_projects' for tool in result['tools'])
print('Clean Ubuntu APT installation and CLI check passed.')
PY
SH
