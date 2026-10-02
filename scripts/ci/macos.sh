#!/bin/bash
# Unsigned compilation/tests, not sandbox runtime or App Store validation.
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"
mode="${1:-test}"
case "$mode" in test|release|performance) ;; *) echo 'Usage: macos.sh test|release|performance' >&2; exit 2 ;; esac
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
scheme=BuildHunter
if [[ "$mode" == performance ]]; then scheme=BuildHunterPerformance; fi
common=(-project macos/BuildHunter.xcodeproj -scheme "$scheme"
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
  screenshot_dir="$output_dir/ui-screenshots"
  if [[ -e "$screenshot_dir" ]]; then
    echo 'ui-screenshots already exists. Use a clean CI output directory.' >&2
    exit 2
  fi
  xcrun xcresulttool export attachments --path "$output_dir/test.xcresult" \
    --output-path "$screenshot_dir" --filter '*.png'
  screenshot_count="$(find "$screenshot_dir" -type f -name '*.png' | wc -l | tr -d ' ')"
  if [[ "$screenshot_count" -lt 14 ]]; then
    echo "Expected 14 UI screenshots, found $screenshot_count" >&2
    exit 1
  fi
  python3 - "$output_dir/test-summary.json" <<'PY'
import json
import sys
from pathlib import Path
summary = json.loads(Path(sys.argv[1]).read_text())
if summary.get("result") != "Passed" or summary.get("failedTests", 0) or summary.get("passedTests", 0) < 1:
    raise SystemExit("Expected a passing test run with at least one executed test")
print(f"Verified {summary['passedTests']} passing unit/UI tests")
PY
elif [[ "$mode" == performance ]]; then
  if [[ -e "$output_dir/performance.xcresult" ]]; then
    echo 'performance.xcresult already exists. Use a clean CI output directory.' >&2
    exit 2
  fi
  xcodebuild test "${common[@]}" -configuration Release ENABLE_TESTABILITY=YES \
    -parallel-testing-enabled NO -enableCodeCoverage NO \
    -resultBundlePath "$output_dir/performance.xcresult" \
    2>&1 | tee "$output_dir/performance.log"
  xcrun xcresulttool get test-results summary --path "$output_dir/performance.xcresult" \
    --format json > "$output_dir/performance-summary.json"
  xcrun xcresulttool get test-results metrics --path "$output_dir/performance.xcresult" \
    > "$output_dir/performance-metrics.json"
  xcrun xcresulttool export metrics --path "$output_dir/performance.xcresult" \
    --output-path "$output_dir/performance-metrics"
  xcrun xcresulttool export attachments --path "$output_dir/performance.xcresult" \
    --output-path "$output_dir/performance-attachments" --filter '*.json'
  python3 - "$output_dir/performance-summary.json" <<'PY'
import json
import sys
from pathlib import Path
summary = json.loads(Path(sys.argv[1]).read_text())
if summary.get("result") != "Passed" or summary.get("failedTests", 0) or summary.get("passedTests", 0) != 5:
    raise SystemExit("Expected all five XCTest performance tests to pass")
print("Verified five passing Release performance tests")
PY
else
  xcodebuild build "${common[@]}" -configuration Release 2>&1 | tee "$output_dir/release.log"
fi
