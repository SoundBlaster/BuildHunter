# Profiling individual rules

Rule profiling is free and available for built-in and user templates on the same
runtime. It is **off by default**, independent of the regular CPU/RAM/throughput
profile. Enable it only for investigations: every evaluated rule reads a timer.

## GUI

1. Open **BuildHunter → Settings… → Search → Profile individual rules**.
2. Save/Check your template files and start a new scan or Rescan. The setting is
   captured at scan start; changing it does not instrument a running scan.
3. After the scan ends, open the performance graph in the status bar and choose
   **Rule profile…**. A separate resizable window keeps a snapshot of this scan. Rows start with the highest total time and show rule/target,
   phase, checks/skips, true/false/unknown and total/mean/maximum wall time.
4. Export the finished report as **JSON**. Its `rule_profile` contains all rows;
   `catalog` identifies the exact packages/versions, digest, profile and exclusions.
   CSV still contains artifact rows, not rule metrics. No account/purchase needed.

Only one aggregate is sent before the terminal event. Cancellation, incomplete
or failed scans can carry partial metrics; report status remains significant.
A setup failure before classification has no measured work. No partial run should
be compared with a successful full scan as if their workloads were identical.

## CLI

```sh
build-hunter ./fixtures --apparent --json --profile-rules rules-before.json
build-hunter ./fixtures --apparent --json --profile-rules rules-after.json
```

The output file must not exist and is created after traversal, so it does not
inflate artifact sizes in that scan. `--profile FILE` can be used simultaneously
with a **different** output path for CPU/RAM and traversal rates. The standalone
rule report has `format: buildhunter.rule-profile-report/1`, `catalog`, `root`,
`status` and `rule_profile`. CLI template import is still separate work; custom
files are currently connected through GUI Templates.

## Fields and units

The nested profile format is `buildhunter.rule-profile/1`; its
[schema](contracts/rule-profile-v1.schema.json) documents the serializable shape.
Rows are keyed by **package_id + target_id + rule_id + phase**. Rule IDs are local
to their package; do not compare bare `output` IDs across packages.

| Field | Meaning |
|---|---|
| `catalog_digest`, `excluded_target_ids` | Immutable effective selection identity (digest covers full package data; exclusions are separate) |
| `preliminary_calls`, `resolved_calls` | Actual classification calls in each phase, after host pruning |
| `classification_errors` | Integration failures, not false matches |
| `scan_status` | Client scan status, retained in the snapshot window; absent only for standalone pure-policy accumulators |
| `evaluations` | Actual executions of this rule, once per eligible classification call |
| `skipped_by_dispatch` | Phase calls minus executions; normally excluded by the conservative name index |
| `true`, `false`, `unknown` | Results of rule evaluations, not counts of artifact roots |
| `total_ns` | Sum of instrumented rule wall durations, nanoseconds |
| `average_ns` | `total_ns / evaluations`, or null if never executed |
| `max_ns` | Largest observed wall duration; zero if never executed |

On classification errors, unevaluated rows can also reflect an aborted call;
inspect `classification_errors` before interpreting skips as index effectiveness.
Disabled targets have no rows. Enabled rules with no checks still have zero rows
of counters (one row per phase), so absence is not confused with zero cost.
Counters/totals saturate at UInt64 maximum. Accumulators are per scan, with at
most 8192 rows (2 × 4096 effective rules); there is no per-path event history.
The report can include file paths elsewhere; profile rows themselves do not.

**Preliminary** runs before needed markers are read. A `not_observed` marker is
intentionally unknown; a decisive name/kind condition can avoid reading it.
**Resolved** repeats classification only when the preliminary target result needs
real marker observations. A rule can therefore run twice for one candidate, or
only in preliminary, or not at all. Multiple rules in a target can be true, and
GUI outer-root pruning differs from CLI nested scanning. Neither `true` nor the
sum of phase evaluations equals the number of displayed folders.

## What timing includes

`timing_scope` is `rule_evaluation_and_outcome_wall_time`: Rifle expression work
and construction of that rule's owned outcome/unknown diagnostics. It excludes
JSON parsing/compilation, input type checks, dispatch, bindings, marker reads,
traversal, directory sizing, host aggregation, UI and report serialization.
It does not measure bytes read or per-rule CPU/RAM. Shared marker I/O cannot be
attributed independently to every rule without double counting.

Timers, allocation and scheduling can dominate very small rules. These are
**instrumented wall observations**, not calibrated pure predicate timings.
No percentile, CPU attribution, whole-process memory reduction or speedup is
inferred from them. Use the ordinary profile and Instruments for costs outside
Rifle. The ordinary evaluation path reads no per-rule clocks.

## Reproducible comparison

Use a Release build, the same app/runtime version, root fixture, filesystem,
marker policy, filters and machine; record OS/toolchain, package versions and
both catalog digests. The digest normally changes when the template changes.
Compare the same package/target/rule/phase IDs, not an expected identical digest.
Keep cold/warm-cache conditions explicit and repeat several times. Retain JSON
reports beside template source in your own Git repo if appropriate.

1. Confirm artifact target IDs, bytes, status and unknown/ambiguity diagnostics
   are equivalent before judging speed. A faster incomplete scan is not success.
2. Sort by total time to find overall rule cost; average time identifies expensive
   checks, while evaluations/skips show how often the index admits the rule.
3. Optimize narrow name conditions and expression order; keep semantic fixtures.
4. Repeat the profiled runs and an uninstrumented run. The difference includes
   profiling overhead, not just a rule improvement. One maximum is noisy.

The first implementation reports aggregates only. Expression-level tracing,
per-marker acquisition timing, percentiles, an automated benchmark suite and
historical UI comparison are separate extensions, not current measurements.
