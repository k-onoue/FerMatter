#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd "$(dirname "$0")/.." && pwd)"
repo_dir="$(cd "$project_dir/.." && pwd)"
if [[ -z "${DEVELOPER_DIR:-}" && -d /Applications/Xcode.app/Contents/Developer ]]; then
    export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi
export PYTHONDONTWRITEBYTECODE=1
export SLIDEMATH_COMPILER_TESTS="${SLIDEMATH_COMPILER_TESTS:-1}"
xcrun swift test --package-path "$project_dir"
python3 -B -m unittest discover -s "$project_dir/Tests" -p 'test_*.py'
python3 -B "$repo_dir/scripts/check-publication.py"
