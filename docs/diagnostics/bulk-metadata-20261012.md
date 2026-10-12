# macOS batch metadata experiment — 2026-10-12

Source: `b80438fb6b09dda580450a21b87f357aeba9e1f4` (clean checkout at measurement).
The [raw artifact](bulk-metadata-20261012.json) records host/toolchain identity,
driver hash, seven interleaved samples, median/p95/MAD for every metric, and exact
canonical fixture reports. Both variants were Release builds of the same source;
only the experimental Cargo feature differed. Filesystem caches were warm.

| Scenario / size mode | Portable median ms | Bulk median ms | Bulk / portable | Portable peak MiB | Bulk peak MiB |
|---|---:|---:|---:|---:|---:|
| wide / allocated | 19.732 | 12.216 | 0.62 | 5.78 | 7.62 |
| wide / apparent | 19.069 | 11.708 | 0.61 | 5.75 | 7.62 |
| deep / allocated | 5.365 | 4.231 | 0.79 | 2.92 | 3.25 |
| deep / apparent | 5.309 | 4.240 | 0.80 | 2.91 | 3.28 |
| mixed / allocated | 0.555 | 0.492 | 0.89 | 2.41 | 2.59 |
| mixed / apparent | 0.541 | 0.473 | 0.87 | 2.41 | 2.61 |
| cancel / allocated | 3.901 | 3.867 | 0.99 | 5.16 | 5.17 |

Every paired report matched exactly, with actual batch calls and no fallback for
complete scans. Sparse-file, hard-link, resource-fork and symlink fixtures were
included. First discovery, cancellation-to-return latency and aggregate worker
listing/measurement times are in the raw artifact. Peak RSS includes the driver
process, not only scanner allocations.

These owned fixtures support retaining the prototype for further evaluation:
wide measurement improves by about 38–39%, deep measurement by about 20–21%, with
about 1.9 MiB additional peak RSS in the wide fixture. They do not establish
cold-disk performance, universal speedup, sandbox acceptance or live iCloud
behavior. Ordinary CLI and GUI builds keep the portable backend.

Reproduce with `python3 scripts/compare_backends.py`. The macOS Scanner comparison
job repeats the test and uploads a fresh report; timings are diagnostic while
report equality and actual batch usage are required.
