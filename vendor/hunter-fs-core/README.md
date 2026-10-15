# hunter-fs-core

Std-only primitives extracted from BuildHunter `src/lib.rs` at 13aece6.
`visit_directory` consumes a host-opened directory iterator, observes file types
without following targets, emits owned Entry/Error values and checks cancellation
before type lookup. The host owns open/access/cloud policy, entry-selection decisions,
recursion, worker scheduling and report/status aggregation. Failed open emits one
Error then OpenFailed; other iteration/type errors emit Error and continue.
A cancelled pass keeps its emitted prefix. Empty iteration completes even if the
stop predicate would be true, preserving the previous loop behavior. No blocking
syscall interruption or descriptor-bound/atomic tree guarantee is claimed.

`metadata_bytes` and `MeasuredSize` preserve apparent `len`, Unix allocated
`blocks * 512` (saturating) and non-Unix `len` fallback. They do not perform I/O,
filter file types, deduplicate hard links or estimate reclaimable disk space.
The boolean selects apparent bytes when true and allocated bytes when false.

This package has no external dependencies. It is separate from Unix `hunter-fs`,
which uses libc. Consumers needing fresh-cache offline builds may vendor the exact
Cargo.toml/LICENSE/README.md/src/lib.rs/src/work.rs/src/task.rs/src/children.rs/src/ledger.rs snapshot, recording the upstream revision and hashes.
That vendored copy is distribution of Hunter source, not a second implementation.

Tests cover cancellation prefix, open/iteration error ordering, empty directories,
file/directory/link types, sparse sizes, hard links and Linux raw names.
No live cloud, recursive race, memory or throughput acceptance is inferred.

## Recursive work coordination

WorkQueue<T> owns an unbounded LIFO stack of opaque jobs. push returns the job
when closed; pop blocks until work or shutdown. close wakes all waiters and drops
pending jobs outside the mutex; it does not interrupt in-flight operations.
Queue length/memory is not bounded by LIFO order or worker count. Job destructors
and worker closure cloning must be well-behaved (especially non-panicking during
unwind); this primitive cannot enforce those consumer obligations.

spawn_workers launches exactly a positive caller-specified count in a caller-owned
std::thread::Scope. Each worker clone closes the queue if it panics. The coordinator
MUST retain queue.close_on_drop() inside the scope so every normal/exceptional exit
closes before scope join. worker_count applies available-parallelism + caller cap.
The helper returns after launching; only completion of the outer scope establishes
that workers/callbacks ended. No detached worker escapes the borrowed scope.

Workers may expand recursive jobs through the queue. The host still decides what
to traverse, performs setup/QoS/cloud/cancellation checks, closes its result channels
and owns aggregation/terminal events. Tests check idle-peer shutdown, recursive
expansion, payload release, coordinator unwind and worker panic with peers exiting
while the coordinator guard is still alive. No full filesystem engine is claimed.

## Directory task/result and measurement plan

DirectoryTask<Id, Context> owns an ID, path, measurement intent and opaque host
context. DirectoryListing<Context> owns children/errors and opaque listing context.
Hunter never interprets context or assigns consumer-specific meaning to it.
measurement_plan combines explicit intent with a host prefetch boolean, creates
original-position slots for non-directory/non-symlink entries, and borrows the
job path (no extra directory-path allocation). Initial file types are observations,
not a snapshot against later replacement; special non-directory types keep the
legacy slot behavior. No extra filesystem reads are used for plan construction.

The consuming plan calls directory metadata, then backend preparation, then checks
cancellation before each slot. This deliberately preserves legacy behavior: even
a pre-cancelled plan or directory metadata error still invokes preparation. A host
backend can therefore perform I/O there. Slots retain None positions for omitted
directories/links; cancellation returns a prefix rather than compacted/reindexed
files. Errors are owned and stay at the same slot. Host may fill missing results
later, decide statuses/partial outcomes and own platform/cloud guard lifetime.

DirectoryResult provides Listed/Measured envelopes with matching host IDs. It does
not enforce a wire protocol: the host must publish Listed before measuring and
send the Measured outcome even for a cancelled partial pass. Result callbacks,
root permissions, timing, bulk acquisition and recursive decisions remain host-owned.
Tests cover intent/prefetch, slot alignment, callback order/errors, cancellation
prefix/preparation and Unix symlink omission.

## Host-selected recursive child requests

ChildTraversal consumes an existing listing in order and calls a synchronous host
selector for each original slot. Visit routes observed directories to DirectoryRequest;
all other types (including observed directory symlinks) remain indexed Entry values.
Skip omits that slot without compacting later indices. Stop is terminal and visible
through is_stopped(); empty/exhausted input never invokes selection. No filesystem
reads or refreshed type observations occur, so replacement races remain host-owned.

The listing is consumed without another child/task vector; context is opaque and
cloned only for admitted directories. The original listing buffer and any
unconsumed suffix stay owned until the plan is dropped. Requests inherit the host measurement flag,
move paths/context into a DirectoryTask through into_task(id), and never allocate
IDs themselves. Host selectors/context cloning may have effects or panic. The host
retains guards, schedules requests, accounts for descendants/results and maps stop
into final status. Full recursion/lifecycle/reporting and memory budgets remain
outside this helper. Tests cover filtering/order/indices/context cloning, terminal
stop, empty input, stale observations without I/O and Unix raw paths/symlinks.

## Serial task ledger

DirectoryTaskLedger is an in-memory registry for a serial host coordinator, not a
scheduler or protocol validator. register(request,state) assigns a zero-based,
non-reused usize ID, moves request values into a task, stores opaque state and
reserves one expected listing response. The host then enqueues the returned task;
queue rejection remains a host failure decision. outstanding counts messages,
not running jobs or bytes. take_listing moves state once and reserves an additional
response when measuring is true. Invalid/repeated listing IDs panic.

The host calls response_received once for each received message, before its stop
check or result dispatch. It retires one expected message (panic at zero), including
setup failure followed by host shutdown. After a measuring listing the host defers
Some pending state or None for speculative work it will ignore, at most once per
ID, before consuming Measured. take_measurement moves/removes the state once;
ignored/unknown/repeated measurement IDs yield None. Neither deferral nor lookup
changes counters or validates wire order/identity. The host owns those obligations,
channels, cancellation, queue lifetime, recursive accounting and final status.

Payloads are opaque and never cloned. Unclaimed listing/deferred values release
on ledger drop, independently of worker completion; retain guards until actual
scope join. Vector/map capacities remain until drop, so the registry has no fixed
memory bound. Tests cover out-of-order IDs/results, ignored responses, setup
failure, stopped-state release and invalid host operations. No full scan engine
or runtime resource improvement follows from this extraction.
