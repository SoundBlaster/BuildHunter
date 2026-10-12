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
- Clock and physical-memory metrics for classifying 10,000 Swift/Rust directory and
  Python bytecode file candidates through the reused SpecificationCore decision policy.
- Clock and physical-memory metrics for preparing a sunburst snapshot and bounded
  layout from 10,000 artifact rows. Fixture generation is excluded; the resulting
  count, byte total, and sector bound are checked after measurement.
- The same comparison for one streaming diagram refresh (25 new and 25 measured rows on
  10,000) against rebuilding every folder node; the update must be at least 3x faster.
- An interleaved comparison of one streaming table refresh (25 new rows merged into
  10,000 sorted rows) with a full localized re-sort; the merge must be at least 5x faster.
- A five-sample, interleaved scaling check: 10,000 rows must take no more than
  `20 × median(1,000 rows) + 25 ms`. This allows a tenfold input increase plus a
  coarse noise budget and rejects the observed quadratic ID lookup behavior.
- The same interleaved scaling gate for deduplicating 1,000 and 10,000 distinct warnings;
  a linear scan of the warning list grows a hundredfold and fails it.

XCTest clock/memory metrics are recorded for inspection; no machine-specific
Xcode baseline is committed for them. The explicit scaling assertion is the CI gate.
The index uses additional O(n) memory and keeps discovery/completion lookup O(1)
on average. Rescan, target replacement and mock changes clear it with the report.

The macOS performance job publishes `performance.xcresult`, a summary, native metrics
as JSON/CSV, the `event-scaling.json` attachment and the log under `macos/.build/ci/`.
The script requires every discovered performance test to pass (at least the original five). Unit tests protect duplicate,
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

## Opt-in macOS batch metadata experiment (#51)

```sh
python3 scripts/compare_backends.py
python3 scripts/test_backend_comparison.py
cargo test --locked --all-features
cargo run --locked --release --example profile_tree -- /path/to/local/projects
```

`macos-bulk-metadata` is an experimental Cargo feature, disabled in ordinary CLI
and GUI builds. It batches file sizes inside measured directories with
`getattrlistbulk`, using `ATTR_FILE_ALLOCSIZE` (all forks) and `ATTR_FILE_DATALENGTH`
(apparent bytes). Directory own sizes and discovery listings retain the portable
implementation. This first prototype deliberately reads a measured directory a
second time; it does not replace discovery with a new full traversal engine.
Missing/unsupported per-file attributes fall back to `symlink_metadata`, as does
an unsupported filesystem syscall. Other syscall and per-entry errors propagate;
the parser checks returned attribute masks, record lengths, name references and
nonnegative sizes. Symlinks are not followed and hard links count as before.

The comparison builds both backends from the same revision, warms them, then
interleaves seven paired scans of owned fixtures in allocated/apparent modes:
8,192 files in one artifact, 128 artifact subdirectories, a Swift/Rust/Python
mixture with nesting, hard links, a sparse file, resource fork and symlink, plus
cancellation at discovery. It requires exact equality of canonical report paths,
languages, kinds, bytes, nesting, partial flags, warnings and terminal status.
Non-cancelled bulk samples must actually use batch calls without fallback.

Raw samples include scan time, first discovery, cancellation-to-return latency,
peak process RSS (`/usr/bin/time -l`, bytes on macOS), and aggregate worker wall
times for listing and measurement. Worker times overlap and must not be added to
or interpreted as a partition of scan wall time. Normal scans read no phase
clocks unless profiling is enabled. Fixture preparation, compilation and report
serialization are outside scanner timing; process peak RSS includes the driver.

The macOS **Scanner comparison** job uploads `bulk/report.json` and
`bulk/report.md`. Equality, fixture counts and actual batch usage are gates;
timing ratios remain diagnostic while the feature is disabled by default.
The existing default-backend performance regression budget remains enforced.

An initial read-only profile of the user's real development tree on 2026-10-12
observed 686,319 entries and 4,512 artifacts, `Completed` with no warnings, in
3.070 s. Aggregate worker listing/measurement times were 17.865/6.234 s, so about
26% of observed worker time was measurement. A subsequent experimental scan took
2.578 s with 16.543/3.669 s worker times. This was an exploratory, uncommitted
development build and sequential warm scans, not a controlled speed claim.
The live tree changed between scans (entry and byte totals differ), so these
real-tree samples do not establish report parity. Owned paired fixtures provide
that check. The source-tree paths and contents are not stored in this document.

The controlled seven-pair result and raw evidence are saved in
[diagnostics/bulk-metadata-20261012.md](diagnostics/bulk-metadata-20261012.md).
