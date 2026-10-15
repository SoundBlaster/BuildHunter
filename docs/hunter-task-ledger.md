# Directory task ledger — 2026-10-15

DirectoryTaskLedger is a serial in-memory task registry. It now owns the former
Scanner.nodes / awaiting / outstanding fields: non-reused directory IDs, opaque
DirNode state before Listed, optional PendingMeasure state before Measured and
an expected-message count. It does not send tasks or execute filesystem work.

BuildHunter registers each request before queue.push and mirrors the ledger count
to the existing pending_tasks profile counter. Queue rejection still fails the scan.
On each received worker message it retires one response and updates that counter
before checking stop or dispatching the message. SetupFailed can replace a listing
response and triggers the same host failure path. take_listing moves state once
and reserves a second response when the worker announced measurement.

After policy/child processing, the host defers Some(PendingMeasure), or None for
unneeded speculative measurement. Measured moves that state back by ID; missing,
ignored or duplicate measurement IDs return None as before. Lookup itself does
not retire a response: the receive step already did. The host must defer at most
once before consuming Measured and must provide valid identity/order. This is
not a wire-protocol validator. Unknown/repeated listing IDs and retirement at zero
panic as caller protocol violations; no new public scan status or ABI is introduced.

DirNode/PendingMeasure remain product-owned types. Hunter never interprets marker
bits, ancestor chains, artifacts or outcomes. BuildHunter still owns policy calls,
cloud/QoS/permission guards, completion/partial/error precedence, byte aggregation,
result channels and queue/shutdown/sampler lifetime. Ledger drop releases unclaimed
opaque payloads; worker completion remains the outer thread Scope join boundary.
A counter of zero does not prove that unrelated workers/resources have ended.

Storage strategy is preserved: Vec<Option<State>> retains consumed-ID slots and
capacity; HashMap retains capacity after deferred-state removal. Payloads move
without clones. This extraction does not establish a fixed RAM budget or a memory
reduction. The root CLI keeps an exact eight-file std-only snapshot and offline
check/install/run; the opt-in plugin scanner retains its separate metadata Git pin.

Seven upstream tests cover ID reuse prevention, out-of-order listing/measurement
correlation, ignored response accounting, stopped-state payload release, setup
failure and invalid host operations. A deterministic application coordinator test
scripts Listed then an ignored Measured error, requiring both to be consumed with
zero outstanding responses and no artifact/status warning. Existing cancellation,
worker/observer panic, nested sizing, cloud and profiling gates remain in place.
Fresh source/runtime/performance/map receipts: specs/validation/hunter-ledger-20261015.
Upstream draft snapshot/frozen maps: specs/evidence/hunter-ledger-20261015.

This completes the task registry/result-correlation slice. Full generic scan
execution and subtree completion/aggregation are still separate boundaries.
Merge the upstream Hunter PR first with its pinned source commit preserved in main
ancestry, then the dependent BuildHunter PR. Contracts remain descriptive drafts.
