# Process resource profile — 2026-10-12

Source: `b4ff23cdca30b91232926757efe9013cbb712b79`, clean worktree. Host:
macOS 27, Apple Silicon arm64. Compiler and harness hash, all seven paired
runs and their timelines are in [the raw report](resource-profile-20261012.json).

The same Release binary scanned an owned fixture with 128 folders and 131,072
896-byte files, once warmed, with profiling enabled/disabled in alternating
order. All seven complete artifact reports matched exactly. Each enabled run
captured initial, one periodic and terminal CPU/RSS samples. These measurements
apply to this short warm fixture, not a full home-directory scan or GUI process.

| Metric | Median over seven runs |
|---|---:|
| Elapsed, profiling enabled | 144.87 ms |
| Elapsed, profiling disabled | 145.31 ms |
| Paired profiling overhead | +0.25% |
| Paired overhead MAD | 1.46 percentage points |
| Process user + system CPU time | 1115.47 ms |
| Mean CPU | 762.6% (7.63 logical cores) |
| Largest interval CPU | 774.8% |
| Time-weighted resident memory | 6.02 MiB |
| Sampled peak resident memory | 8.38 MiB |

The overhead estimate is smaller than the observed variability; this run does
not establish a measurable slowdown. The paired ratio and ratio of separate
medians differ, so use the paired statistic above for overhead.

CPU 100% means one logical core, not the whole machine. RSS includes the runtime
baseline; the GUI's Swift runtime, chart, rows and other windows will add memory.
The 100 ms sampling cadence can miss brief peaks. The maximum sampled RSS across
all seven runs was 8.78 MiB. Missing OS readings remain null/gaps. These metrics
do not measure energy or disk cache. Compare future candidates on the same host
and fixture rather than against another CI runner's absolute values.
