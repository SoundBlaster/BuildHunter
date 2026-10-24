# R5 local resource measurements — 2026-10-24

Baseline: Release CLI from main `d6b113a4aa9bbc8f56e4518021a3cbaa553c9f5f`.
Candidate: source/binary hashes in
`specs/validation/r5-profile-budgets-20261024/candidate-source-identity.json`.
Both use the default catalog CLI profile, allocated bytes and profiling. Each
input ran three times; values below are medians, not a cold-cache guarantee or
confidence interval. The Mac had normal background activity.

The runner uses Unix wait4 for each scanner process's CPU time and kernel peak
RSS. CPU means user + system seconds, not the percent of one core; RSS is not
physical footprint, allocator live bytes or the entire GUI application. The final
response capacity is 16; an earlier 8-message trial had higher CPU cost on home.

| Input | R4 peak RSS | R5 peak RSS | R4 wall | R5 wall | R4 CPU | R5 CPU | Result |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| traversal | 10.94 MiB | 6.59 MiB | 0.327 s | 0.222 s | 2.292 s | 1.426 s | same semantic digest |
| burst | 5.66 MiB | 5.83 MiB | 0.057 s | 0.057 s | 0.370 s | 0.366 s | same semantic digest |
| wide | 29.59 MiB | 12.97 MiB | 0.274 s | 0.057 s | 0.261 s | 0.019 s | explicit listing payload limit; truncated, not a speedup |
| home | 119.86 MiB | 45.56 MiB | 9.808 s | 11.464 s | 33.079 s | 32.392 s | same 44,451 findings, 658 warnings and digest; no quota hit |

- Traversal fixture: 20,201 directories; no selected findings. Burst fixture:
  65,536 files under a selected `.build` root. Both retain exact size/selection
  parity. Repeated short runs are quantized by the runner's 50 ms wait interval.
- Wide fixture: 50,000 `.pyc` files and long absolute fixture paths exceed the
  8 MiB estimated listing quota. R5 explicitly returns incomplete and one limit
  warning before discovery; compare semantic behavior, not its shortened time.
- Home is a live mutable tree. All six runs happened to produce equal semantic
  digests, finding/warning counts and terminal status. Both are incomplete due to
  existing access/cloud warnings; the R5 limit is null. This is observational
  parity, not a filesystem snapshot guarantee. Home peak RSS dropped about 62%;
  wall latency regressed in this small sample while total CPU was comparable.
  Do not generalize these timings to other machines or call this a throughput win.
- R5 records first-finding latency in CLI profiles; R4 did not, so no before/after
  latency claim is made for that metric. No finding means null.

Raw reports/profile/error paths stay on local TB5. Committed receipts contain
aggregate counters, sanitized commands, source/binary hashes and all three runs.
The helpers are manual; no extra hosted CI job was introduced.

## GUI evidence remains separate

A fresh Release GUI scan was started through the native folder picker. It reached
2,713,544 entries / 416,877 folders / 18,204 findings, then two workers remained in
`read_dir → opendir → open$NOCANCEL`. A queue deadlock is not established: other
workers were idle and the coordinator continued sampling. Active system calls
are outside the cancellation/retention guarantee. Do not label this run completed.

Opening the performance popover caused further RSS growth despite a bounded point
history. Closing it stopped sustained growth. A saved local memgraph shows two
ScanRow arrays (about 9.4 MiB) and three small profile-point arrays (about 100 KiB);
these do not account for hundreds of MiB. Two sampled large non-object buffers
have reverse references through CoreGraphics font cache. This is evidence for a
separate rendering/cache hypothesis, not proof that every retained byte belongs
to Charts. Leaks found 23,584 orphaned bytes, not the hundreds of MiB retained.

The attempted xctrace Allocations attach did not finish; its trace is invalid and
is not a successful allocation profile. Sample/vmmap/heap/memgraph evidence is
saved locally. The next stack layer checks a workaround for continuous numeric
text transitions while retaining line and sunburst animations. C5 whole-app
acceptance remains open until that GUI behavior and final CI have been assessed.
