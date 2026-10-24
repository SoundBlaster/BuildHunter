# Scan retention budgets (R5 draft)

Each new ScanControl selects immutable per-scan limits. Rust consumers can use
`ScanControl::with_limits` before scanning; the CLI/GUI currently use defaults.
These limit retained payloads, not exact allocator overhead or whole-process RSS.

| Retention | Default | Exhaustion |
| --- | --- | --- |
| Queued directories | 65,536 tasks / 32 MiB estimated task+node payload | Nonblocking rejection, stop |
| Worker responses | 16 queued messages, at most 8 active workers | Producer backpressure, receiver drops before join |
| Each directory listing | 65,536 entries / 4,096 errors / 8 MiB estimated payload | Explicit prefix limit, stop |
| Optional macOS batch map | 65,536 size/error observations / 8 MiB estimated payload | Explicit prefix limit, stop |
| Findings + progress | 100,000 / 32 MiB estimated payload | Stop before discovery/event |
| Warnings | 4,096 / 2 MiB estimated payload | Stop; one reserved summary remains |
| Plugin diagnostics | 4,096 / 4 MiB estimated payload | Stop before retained clone/event |
| Pending profile samples | 4,096 × 88-byte numeric samples | Retain newest suffix; count every omission |
| CLI profile history | 4,096 received samples | Separate rolling eviction count |
| GUI profile chart | 600 received points | Rolling display; aggregates kept separately |

The first exhausted limit is stable across workers. After join a reserved small
warning explains why scanning stopped. Status is incomplete unless cancellation
or failure takes precedence. Already discovered unfinished findings complete once
as partial; Finished is last. Within-budget policy/bytes/events retain R4 semantics.
The selected tree is never written.

Completed ledger slots release immediately; hash-map capacity follows peak concurrent
work instead of all visited directories. Queue weight includes both owned task and
coordinator node/path/ancestry. Bounded worker responses plus per-listing limits
prevent a stalled coordinator retaining arbitrarily many large directory replies.
Workers may hold their current listing and prepared measurement simultaneously.
The per-response cap is not a shared global byte semaphore.

Weights count path/string capacities and inline payload slots; vectors, hash tables,
allocator slack, current events, opaque I/O error/provider allocations and callback
effects are not exact RSS accounting. Native syscalls and callbacks cannot be
interrupted. No-follow/default marker compatibility and live-tree race limits stay
unchanged. Independent scan windows, UI/table/diagram/export copies and still-stopping cancelled
generations can multiply per-scan retention. The GUI event ring has 4,096 slots;
rows, row indices, unsafe prefixes, diagnostics and warning deduplication follow
admitted source sets. Table and diagram workers coalesce to the current report,
while explicit exports hold a frozen source and serialized output. These are
finite per-scan inputs, not a shared process-global memory semaphore.

Pending profile omissions do not stop filesystem traversal. A reserved final
warning discloses loss (separately from a resource-limit warning); exact pending
loss is in optional ABI-1 metadata. CLI `truncated_samples` sums pending loss and
its own rolling eviction. GUI breaks CPU/RAM series at disclosed gaps and skips
missing RAM intervals in its integral. Sampled peaks may miss omitted spikes.
JSON reports/exports preserve `scan_budgets`; CSV remains the table-only format.
The numeric 88-byte C profile layout remains unchanged.

Fresh measurements and their limits are in `docs/r5-resource-measurements-20261024.md`.
They do not establish a process-wide RSS ceiling or owner acceptance of C5.


Reproduce a local comparison (outputs must be outside the scanned tree):

```sh
python3 scripts/create_resource_fixtures.py <new-fixture-directory>
python3 scripts/measure_scan_resources.py --binary <pinned-release-cli> \
  --root <fixture-or-home> --output <new-receipt-directory> --revision <source-identity>
```

The helper uses Unix `wait4` for one child's CPU and kernel peak RSS and preserves
raw local report/profile/stderr files. Do not publish raw home paths/errors; the
repository receipt contains aggregate counts, sanitized commands and log hashes.
It is a manual measurement tool and does not add hosted CI jobs.

## Adapter parity

Both `scan_client` and the experimental scoped plugin scanner use one per-scan
`DiagnosticRetention` from `src/scan_budget.rs`. Admission precedes report/event
cloning; count or estimated-payload rejection sets the same first resource limit.
The engine then joins workers and emits its reserved explanatory warning and
terminal status. This does not change diagnostic-only pilot scans that stay within
quota: their pre-existing status remains `Completed`, with typed diagnostics.

Both adapters share immutable target descriptors rather than multiplying their
strings by finding count. The experimental `PluginArtifact.target` is now
`Arc<TargetDescriptor>`: borrow with `.as_ref()`, and clone the inner descriptor only
when an independently owned mutable copy is required. Descriptors survive dropping
the input catalog. Catalog/package loading and temporary classification payloads
are not included in report retention estimates.
