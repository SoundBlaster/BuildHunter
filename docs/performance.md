# Performance tests

Performance checks run separately from ordinary unit/UI tests. They use owned
temporary fixtures, Release code, monotonic clocks and validation of the scan
result. Fixture creation, compilation, UUID generation and assertions are outside
the timed operations. No user project is used as test data.

## Rust scanner

```sh
cargo bench --locked --bench scanner
python3 scripts/compare_performance.py --baseline-ref origin/main
python3 scripts/test_performance_report.py
```

The dependency-free Cargo benchmark emits CSV. The comparison script compiles the
same benchmark source against both revisions' Rust libraries, prepares one shared
fixture, warms both revisions, then alternates their execution order for seven
samples. It removes its own temporary worktree and fixture afterwards. Both
revisions must support the current scanner library API; this is not a benchmark
against the older pre-library CLI.

| Scenario | Fixture | Measurements |
|---|---|---|
| `source-tree` | 16 projects, 4,096 source files and 1,024 files in `.build` roots | Scan time, first discovery |
| `many-roots` | 4,096 `.pytest_cache` roots with one 1 KiB file each | Scan time, first discovery |
| `pruned-git` | 4,096 files under `.git`, none reported | Scan time averaged over 20 scans |
| `cancel` | `.build` with 4,096 files, cancelled at discovery | Scan time, first discovery, cancellation-to-return latency averaged over 20 scans |

Each scan validates artifact counts, warning/status/partial state and, for complete
artifact scans, minimum measured payload bytes. This prevents skipping work from
appearing as a speed improvement. Times cover traversal, Rust CLI policy callbacks,
event emission and report collection in process. CLI process startup, JSON formatting,
Swift callbacks, SwiftUI rendering and fixture setup are excluded.

`target/performance/report.json` records raw samples, median, nearest-rank p95, MAD,
commits, harness digest, dirty-checkout flag, host and Rust version. The short
`report.md` is suitable for a GitHub job summary. With seven samples, p95 is the
largest sample; it is diagnostic rather than a precise latency-distribution estimate.

A comparison fails if the candidate median exceeds:

```text
baseline median × 1.5 + 5 ms + 6 × (baseline MAD + candidate MAD)
```

This is a deliberately coarse regression budget with a noise allowance, not a
statistical significance test. It catches large stable regressions while avoiding
failures for tiny timing fluctuations. These are warm filesystem-cache measurements;
the scripts do not flush caches or establish cold-disk performance. CI runs the
same-host comparison on Ubuntu and macOS, uploads both reports and writes the table
to the job summary.

## Swift model and policy

```sh
bash scripts/ci/macos.sh performance
```

The `BuildHunterPerformance` scheme has a separate XCTest performance target and
runs in Release with code coverage and test parallelism disabled. The script enables
testability only for this invocation; normal Release builds retain their own settings.

- Clock and physical-memory metrics for applying discovery/completion events for
  1,000 and 10,000 rows, including the terminal transition.
- Clock and physical-memory metrics for classifying 10,000 Swift/Rust candidates
  through the reused SpecificationCore decision policy.
- A five-sample, interleaved scaling check: 10,000 rows must take no more than
  `20 × median(1,000 rows) + 25 ms`. This allows a tenfold input increase plus a
  coarse noise budget and rejects the observed quadratic ID lookup behavior.

XCTest clock/memory metrics are recorded for inspection; no machine-specific
Xcode baseline is committed for them. The explicit scaling assertion is the CI gate.
The index uses additional O(n) memory and keeps discovery/completion lookup O(1)
on average. Rescan, target replacement and mock changes clear it with the report.

The macOS performance job publishes `performance.xcresult`, a summary, native metrics
as JSON/CSV, the `event-scaling.json` attachment and the log under `macos/.build/ci/`.
The script requires all four performance tests to pass. Unit tests protect duplicate,
out-of-order, stale events and reuse of an artifact ID after restart.

Local TDD evidence on 2026-10-02: the original model failed the scaling check
(median approximately 14 ms for 1,000 rows and 1.38 s for 10,000). After adding the ID
index, a hostless Release harness passed all four performance tests and 24 unit tests;
10,000-row clock samples were about 3–5 ms, with one 6.7 ms sample. This harness used
the existing Xcode Debug SpecificationCore binary; CI uses the full Release scheme.
The Rust comparison against `origin/main` passed on the local Mac. These measurements
describe model/engine work and do not prove table frame rate or signed sandbox behavior.

Native macOS CI at `ccd234b` passed all four Release performance tests. Its scaling
medians were 1.18 ms for 1,000 rows and 12.39 ms for 10,000 rows on an Apple Silicon
runner. The same-host Rust comparisons passed on both Ubuntu and macOS. These are
recorded observations for that run, not portable absolute timing limits.
