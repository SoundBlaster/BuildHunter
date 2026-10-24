# R5 performance popover rendering — 2026-10-24

Only per-tick numericText transitions on elapsed time and metric values are removed.
Digits stay monospaced and accessible; unit/column layout, live line-chart/sparkline
and sunburst animations remain. This is a narrow native font-cache workaround.

The Release GUI was operated through Computer Use. The earlier variant reached
650.05 MiB sampled process RSS within a three-minute monitor window, then remained near 640–650 MiB for the
rest of that window. No unbounded-growth claim follows from this finite record. The immediate-label variant's four-minute monitor
peaked at 290.84 MiB and ended at 247.23 MiB; per-minute maxima decreased. The
popover remained open for more than five minutes without renewed growth.

Both home scans reached 18,204 findings, 659 warnings and two pending workers
inside read_dir/opendir/open$NOCANCEL. They did not complete. No queue-deadlock or
completed-scan latency claim follows. UI Stop was responsive; process termination
cleans up the diagnostic instance. Callbacks/syscalls are not interruptible promises.

Before: heap had about 304 MiB non-object payload, two ScanRow arrays (~9.4 MiB)
and three small point arrays (~100 KiB). After: about 21.4 MiB non-object payload.
Two sampled 2,560-byte buffers had reverse references through CoreGraphics font
cache. Leaks found only 23,584 orphaned bytes; the dominant issue was retained
memory. xctrace attach timed out and produced an invalid trace; no allocation
trace success is claimed. Raw traces/memgraphs and home paths stay local.

JSON time series are numeric-only with exact executable hashes. Plot axes are
monitor time, including the folder picker; opening times differ across runs.
Sampling is nominally 200 ms and can miss brief peaks. RSS, physical footprint,
live heap payload and allocator residency are separate quantities. This result is
local macOS behavior, not a process-global budget or owner acceptance of C5.

The existing unit/UI suite verifies labels and profile behavior; Debug test builds
and Release app execution are recorded separately from final native CI execution.
No brittle source-mirroring test or extra long hosted UI-memory job is added.
A manual passive PID monitor makes future reproduction possible.
