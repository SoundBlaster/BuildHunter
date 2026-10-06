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
# A failing `xcodebuild test` must not skip the evidence: run it with errexit off,
# export what the result bundle holds, then fail with the test's own status.
# xcresulttool also writes the bundle's database.sqlite3 on first read.
run_tests() {
  local log="$1"
  shift
  set +e
  xcodebuild test "$@" 2>&1 | tee "$log"
  local status="${PIPESTATUS[0]}"
  set -e
  return "$status"
}
# After a failed test run, evidence extraction is best-effort: a bundle from a build or
# test-planning failure may hold no test results, and xcresulttool's error must not
# replace xcodebuild's status. After a passing run, extraction failures still fail.
evidence() {
  local status=0
  "$@" || status=$?
  if [[ "$status" -ne 0 && "$test_status" -ne 0 ]]; then
    echo "warning: '$1 ${2:-} ${3:-}' failed with status $status after the failed test run" >&2
    return 0
  fi
  return "$status"
}
if [[ "$mode" == test ]]; then
  if [[ -e "$output_dir/test.xcresult" ]]; then
    echo 'test.xcresult already exists. Use a clean CI output directory.' >&2
    exit 2
  fi
  screenshot_dir="$output_dir/ui-screenshots"
  if [[ -e "$screenshot_dir" ]]; then
    echo 'ui-screenshots already exists. Use a clean CI output directory.' >&2
    exit 2
  fi
  test_status=0
  run_tests "$output_dir/test.log" "${common[@]}" -resultBundlePath "$output_dir/test.xcresult" \
    -enableCodeCoverage YES || test_status=$?
  if [[ ! -d "$output_dir/test.xcresult" ]]; then
    echo "xcodebuild test exited $test_status without a result bundle" >&2
    exit "$(( test_status == 0 ? 1 : test_status ))"
  fi
  evidence xcrun xcresulttool get test-results summary --path "$output_dir/test.xcresult" \
    --format json > "$output_dir/test-summary.json"
  evidence xcrun xcresulttool export attachments --path "$output_dir/test.xcresult" \
    --output-path "$screenshot_dir" --filter '*.png'
  if [[ "$test_status" -ne 0 ]]; then
    echo "xcodebuild test failed with status $test_status; exported what the result bundle holds" >&2
    exit "$test_status"
  fi
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
  test_status=0
  run_tests "$output_dir/performance.log" "${common[@]}" -configuration Release ENABLE_TESTABILITY=YES \
    -parallel-testing-enabled NO -enableCodeCoverage NO \
    -resultBundlePath "$output_dir/performance.xcresult" || test_status=$?
  if [[ ! -d "$output_dir/performance.xcresult" ]]; then
    echo "xcodebuild test exited $test_status without a result bundle" >&2
    exit "$(( test_status == 0 ? 1 : test_status ))"
  fi
  evidence xcrun xcresulttool get test-results summary --path "$output_dir/performance.xcresult" \
    --format json > "$output_dir/performance-summary.json"
  evidence xcrun xcresulttool get test-results metrics --path "$output_dir/performance.xcresult" \
    > "$output_dir/performance-metrics.json"
  evidence xcrun xcresulttool export metrics --path "$output_dir/performance.xcresult" \
    --output-path "$output_dir/performance-metrics"
  evidence xcrun xcresulttool export attachments --path "$output_dir/performance.xcresult" \
    --output-path "$output_dir/performance-attachments" --filter '*.json'
  if [[ "$test_status" -ne 0 ]]; then
    echo "xcodebuild test failed with status $test_status; exported what the result bundle holds" >&2
    exit "$test_status"
  fi
  python3 - "$output_dir/performance-summary.json" <<'PY'
import json
import sys
from pathlib import Path
summary = json.loads(Path(sys.argv[1]).read_text())
passed = summary.get("passedTests", 0)
# Every discovered performance test must run and pass; at least the original five exist.
if (summary.get("result") != "Passed" or summary.get("failedTests", 0) or summary.get("skippedTests", 0)
        or passed != summary.get("totalTestCount", passed) or passed < 5):
    raise SystemExit("Expected every XCTest performance test to pass")
print(f"Verified {passed} passing Release performance tests")
PY
else
  xcodebuild build "${common[@]}" -configuration Release 2>&1 | tee "$output_dir/release.log"
fi
