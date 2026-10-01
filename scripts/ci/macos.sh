#!/bin/bash
# Unsigned compilation/tests, not sandbox runtime or App Store validation.
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"
mode="${1:-test}"
case "$mode" in test|release) ;; *) echo 'Usage: macos.sh test|release' >&2; exit 2 ;; esac
output_dir="$repo_root/macos/.build/ci"
mkdir -p "$output_dir"
xcodebuild -version
swift --version
if [[ "$(uname -m)" != arm64 ]]; then
  echo 'This app requires an Apple Silicon runner.' >&2
  exit 1
fi
# User-authorized, invocation-scoped bypass for the pinned macro package.
# This skips ALL macro validation in this invocation; no global trust is changed.
common=(-project macos/BuildHunter.xcodeproj -scheme BuildHunter
  -destination 'platform=macOS,arch=arm64'
  -skipMacroValidation -disableAutomaticPackageResolution CODE_SIGNING_ALLOWED=NO ONLY_ACTIVE_ARCH=YES)
if [[ "$mode" == test ]]; then
  if [[ -e "$output_dir/test.xcresult" ]]; then
    echo 'test.xcresult already exists. Use a clean CI output directory.' >&2
    exit 2
  fi
  xcodebuild test "${common[@]}" -resultBundlePath "$output_dir/test.xcresult" \
    -enableCodeCoverage YES 2>&1 | tee "$output_dir/test.log"
  xcrun xcresulttool get test-results summary --path "$output_dir/test.xcresult" \
    --format json > "$output_dir/test-summary.json"
  python3 - "$output_dir/test-summary.json" <<'PY'
import json
import sys
from pathlib import Path
summary = json.loads(Path(sys.argv[1]).read_text())
if summary.get("result") != "Passed" or summary.get("failedTests", 0) or summary.get("passedTests", 0) < 1:
    raise SystemExit("Expected a passing test run with at least one executed test")
print(f"Verified {summary['passedTests']} passing tests")
PY
else
  xcodebuild build "${common[@]}" -configuration Release 2>&1 | tee "$output_dir/release.log"
fi
