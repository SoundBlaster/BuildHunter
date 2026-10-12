# Table projection retention — 2026-10-13

## Evidence

A four-minute Allocations + VM Tracker recording attached to a verified Release
BuildHunter process during a home-directory scan found 1,049,912,456 live heap +
anonymous VM bytes. Of those, 815,818,368 bytes belonged to 1,449
`_ContiguousArrayStorage<ScanRow>` buffers:

- 1,448 sorted projections: 814,245,504 bytes.
- One scanner-source array: 1,572,864 bytes.

Allocation backtrace: `ScanTableModel.refresh` → detached worker →
`ScanTableProjection.init` → `Collection.map` → Swift array allocation.
This establishes the allocation site, not the owner.

`leaks --trace=<allocation-address> <pid>` and a locally saved Memory Graph
established the strong reference:

```text
AttributeGraph Owned
  → SwiftUI.TableRowForEachState<...>.ChildTraits
  → value.forEach.data
  → _ContiguousArrayStorage<ScanRow>
```

The old projection was still reachable. A separate weak-reference experiment
using the same detached-task/cancellation-handler pattern released every
completed task result. The framework's row traits, rather than that task pattern,
were the confirmed owner in the app graph.

A separate 109,051,904-byte allocation came from Rust
`RawVec<Option<DirNode>>::grow_one` → `Scanner.enqueue`. That arena is not changed
in this fix.

## Fix

The main report hosts an AppKit `NSOutlineView` through `NSViewRepresentable`.
Its data source owns only the current row objects, reuses them by artifact UUID,
and supplies SwiftUI cell content for visible cells. A row never owns the full
projection. Sorting still runs through `ScanTableModel` and its existing
comparators; column headers, language icons, hover, folder context menus and
Finder actions remain available. The diagram is unchanged.

## Regression

`ScanPerformanceTests.testRenderedTableDoesNotRetainPreviousProjections` mounts
and renders the actual report table in a window. It starts with 10,000 rows,
appends 800 rows over 80 refreshes, and checks live malloc-byte growth (not RSS,
reserved pages or peak allocations). It waits for the rendered native table to
publish each row count before measuring.

| Implementation | Live malloc growth |
|---|---:|
| SwiftUI Table, before fix | 81,465,296 bytes |
| AppKit data source, after fix | 6,305,664 bytes |

The unchanged acceptance limit is 24 MiB. The old implementation fails and the
new one passes. Exact byte counts vary with allocator/framework caches; the test
is a broad retention guard, not a portable memory budget for the whole app.
Existing CI's **macOS performance tests** job runs this regression in Release
and publishes its JSON attachment.

## Local trace analysis

Export the Allocations detail tables, whose XPath is under `tracks`, not `data`:

```sh
xcrun xctrace export --input capture.trace \
  --xpath '/trace-toc/run[@number="1"]/tracks/track[@name="Allocations"]/details/detail[@name="Statistics"]' \
  --output statistics.xml
xcrun xctrace export --input capture.trace \
  --xpath '/trace-toc/run[@number="1"]/tracks/track[@name="Allocations"]/details/detail[@name="Allocations List"]' \
  --output allocations.xml
python3 scripts/analyze_allocations.py allocations.xml \
  --statistics statistics.xml --output analysis.json
```

The analyzer streams the XML, resolves individual frame and whole-backtrace
references and optionally checks
that summed live bytes match Statistics exactly. Raw traces and Memory Graphs
stay local: they may contain user paths, object contents and process environment.
The capture discarded freed events, so its exported total bytes do not represent
allocation churn. The baseline scan was still running at the end of capture.

## Repeat Release home scan

A fresh production Release process was launched through Computer Use, verified by
PID and executable path, then attached to the same Instruments 27.1 Allocations +
VM Tracker configuration. The target was selected through the system Open panel.
The second recording lasted 241.34 seconds, including time after traversal ended.
Both runs used the same MacBook Pro / macOS 27.0 and source base `507da84` (the
second includes this table fix). The regression uses a Release testable build;
the home scan uses the production Release configuration.

| Metric | Before | After |
|---|---:|---:|
| Live heap + anonymous VM at recording end | 1,049,912,456 bytes | 136,530,552 bytes |
| Live heap | 1,012,376,712 bytes | 84,626,040 bytes |
| Live `ScanRow` array buffers | 1,449 | 3 |
| Bytes in those buffers | 815,818,368 | 4,620,288 |
| Post-traversal physical footprint (`vmmap` display) | 3.9G | 485.9M |
| Physical footprint peak (`vmmap` display) | 4.2G | 665.8M |

The fixed scan reached 2,275,146 entries, 325,963 folders and 17,481 artifacts in
123.8 seconds. Its profile showed a sampled RSS peak of 631.8 MB, average 448.3 MB,
and 627 final warnings. Traversal ended with `Scan incomplete · review warnings`;
this is not a claim of access to all protected home-directory content. The old
scan reached 2,263,135 entries and 17,478 artifacts in 513.9 seconds. Timing is
observational: these were sequential scans of a changing tree, with different
filesystem cache state and warning counts, under allocation instrumentation.
It is not a controlled scanner speed benchmark or a comparison with DaisyDisk.

The remaining buffers comprise two projection allocations (3,080,192 bytes)
and one scanner-source allocation (1,540,096 bytes); the allocation-list sums
match Statistics exactly.

The buffer count/bytes and red/green rendered-table test provide the direct
retention evidence. Instrumentation overhead is included in process footprint;
RSS alone can fall while retained objects are compressed or paged out.

## Validation

- Production Release build succeeded with normal signing and optimization.
- All 14 performance tests passed, including the rendered-table memory guard.
- All 133 Swift Testing unit tests passed.
- UI column sorting passed for Path, Size and Language, including disabled
  Finder actions on mock rows.
- Three analyzer regression tests cover live/freed rows, frame references and
  whole-backtrace references, including large-allocation attribution. The Rust CI
  helper-test step runs them on all three platforms.
- A final repeat of the rendered-table memory guard passed (3.786 seconds).
- Computer Use: a real 4 KiB `.build` fixture completed scanning; the row's
  context menu opened Finder with the correct `.build` folder selected.
- Local automated real-folder UI test did not pass the system Open panel. It
  failed waiting for scan completion before table assertions; an attempted
  `PathTextField` wait did not resolve the dialog issue and was reverted. That
  test is not reported as passing. A later retry also hit temporary disk
  exhaustion while raw XML was being exported; the exports were compressed.
