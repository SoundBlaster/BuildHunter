# Scan retention budgets (R5 draft)

Each new ScanControl selects immutable per-scan limits. Rust consumers can use
`ScanControl::with_limits` before scanning; the CLI/GUI currently use defaults.
These limit retained payloads, not exact allocator overhead or whole-process RSS.

| Retention | Default | Exhaustion |
| --- | --- | --- |
| Queued directories | 65,536 tasks / 32 MiB estimated task+node payload | Nonblocking rejection, stop |
| Worker responses | 8 queued messages, at most 8 active workers | Producer backpressure, receiver drops before join |
| Each directory listing | 65,536 entries / 4,096 errors / 8 MiB estimated payload | Explicit prefix limit, stop |
| Optional macOS batch map | 65,536 size/error observations / 8 MiB estimated payload | Explicit prefix limit, stop |
| Findings + progress | 100,000 / 32 MiB estimated payload | Stop before discovery/event |
| Warnings | 4,096 / 2 MiB estimated payload | Stop; one reserved summary remains |
| Plugin diagnostics | 4,096 / 4 MiB estimated payload | Stop before retained clone/event |

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
unchanged. Independent scan windows, UI/table/diagram/export copies and profile
backlog need separate accounting. R5's next layer records those boundaries and
fresh CPU/RSS/latency measurements; this engine slice does not close C5.
