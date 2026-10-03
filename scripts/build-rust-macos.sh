#!/bin/bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"
# Xcode launched from Finder does not inherit the user's shell PATH.
if command -v cargo >/dev/null 2>&1; then
  cargo_command="$(command -v cargo)"
elif [[ -x "${CARGO_HOME:-$HOME/.cargo}/bin/cargo" ]]; then
  cargo_command="${CARGO_HOME:-$HOME/.cargo}/bin/cargo"
else
  echo 'Cargo was not found. Install Rust with rustup or add Cargo to PATH.' >&2
  exit 1
fi
"$cargo_command" build --locked --release --lib --target aarch64-apple-darwin
