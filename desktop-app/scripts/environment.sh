#!/bin/bash
# Source from scripts in this directory. A local toolchain is optional.
DESKTOP_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -d "$DESKTOP_ROOT/build/rustup/toolchains" ]]; then
  export RUSTUP_HOME="$DESKTOP_ROOT/build/rustup"
fi
export PATH="$HOME/.cargo/bin:$PATH"
