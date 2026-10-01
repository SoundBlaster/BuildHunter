#!/bin/bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"
cargo build --locked --release --lib --target aarch64-apple-darwin
