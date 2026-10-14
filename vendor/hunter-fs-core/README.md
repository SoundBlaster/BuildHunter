# hunter-fs-core

Std-only primitives extracted from BuildHunter `src/lib.rs` at 13aece6.
`visit_directory` consumes a host-opened directory iterator, observes file types
without following targets, emits owned Entry/Error values and checks cancellation
before type lookup. The host owns open/access/cloud policy, marker decisions,
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
Cargo.toml/LICENSE/README.md/src/lib.rs/src/work.rs snapshot, recording the upstream revision and hashes.
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
