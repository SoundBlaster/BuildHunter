#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 REPORT_DIRECTORY" >&2
  exit 2
fi

cd "$(dirname "$0")/../.."
report_dir="$1"
mkdir -p "$report_dir"
report_dir="$(cd "$report_dir" && pwd)"
git rev-parse HEAD > "$report_dir/source-revision.txt"

# Pinned major/minor/patch, report only: no threshold or fail-on-clone option.
npx --yes jscpd@5.2.0 \
  --format swift,rust --min-lines 8 --min-tokens 50 --workers 2 --ignore '**/tests/**' \
  --reporters json,console --output "$report_dir" --summary \
  --summary-by complexity --no-colors --no-tips \
  src crates macos/Sources/BuildHunter > "$report_dir/summary.txt"
