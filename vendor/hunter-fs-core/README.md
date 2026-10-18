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
Cargo.toml/LICENSE/README.md/src/lib.rs/src/work.rs/src/task.rs/src/children.rs/src/ledger.rs/src/subtree.rs/src/execution.rs/src/backend.rs/src/worker.rs/src/listing.rs/src/measurement.rs/src/session.rs snapshot, recording the upstream revision and hashes.
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
not enforce a wire protocol alone. The DirectoryWorker loop below publishes Listed
before measuring and sends Measured even for a cancelled partial pass. Direct users
of the plan retain those obligations. Result callbacks,
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

## Subtree byte and completion progress

SubtreeProgress holds numeric progress for one host-selected subtree. add_bytes
saturates u64 addition; set_bytes replaces the total for a known leaf. Host chooses
consistent byte units, routes contributions and owns membership/deduplication.
mark_partial is sticky. add_pending_directory/finish_pending_directory announce
and retire descendant work, without queueing anything (invalid counter operations
panic). finish_root records that root processing finished. is_ready requires that
flag, zero pending directories and no prior completion; it proves no worker join.

complete(force_partial) returns current bytes and stored_partial OR force_partial
once, then None. It deliberately does not require readiness: the host checks it
for ordinary directory completion, may finalize a known leaf directly, or may
force a partial prefix on early stop. Pending counts are not cleared and later
host updates are not frozen, but no second completion value is returned. The host
orders dependent completions and publishes status/events; no hierarchy, paths,
IDs, callbacks or owned heap buffers live inside this state.

Tests cover empty/finished roots, early/late directory completion, separate nested
states, saturating sums, explicit leaf totals, partial flags, forced partial prefixes,
one-shot completion and invalid counters. Full tree scheduling, metadata effects,
whole-scan memory and lifetime remain separate from this numeric helper.

## Serial scan execution

drive_scan runs an already initialized coordinator's receive loop. ScanCoordinator
supplies outstanding work, stop checks, checkpoints, receipt accounting and opaque
message dispatch. Zero outstanding work wins at a loop boundary; otherwise stop
ends the loop. Each iteration checkpoints, waits, retires a received response,
checks stop again, then dispatches. Dispatch may add new work. Stop after receipt
returns Stopped even if the count reached zero, and drops the received payload.
Unread responses remain in the host-owned receiver.

None selects blocking receive; a positive optional duration enables idle checkpoint
and stop checks. Zero duration panics. Timeout never retires a response; disconnect
returns Disconnected without changing counts. A stop triggered during checkpoint
is checked after receipt or on the next timeout loop, preserving the original host
boundary. Blocking receive/syscalls/callbacks cannot be interrupted by this helper.
The host must ensure response/disconnect or enable timed waits for idle cancellation.

The host and messages can borrow non-Send/non-static state. Callbacks stay on the
calling thread and their panics propagate. The pump owns no buffers/channels/workers.
Drained/Stopped/Disconnected describe loop exit only, not final statuses or joins.
Host owns root admission, backends/guards, result protocol, partial finalization,
status mapping and terminal publication. Queue/helper shutdown guards must survive
inside the outer thread scope on every exit; actual scope completion joins workers.
This extracts common execution mechanics, not a complete generic filesystem engine.

Ten tests cover empty/pre-stopped work, recursive dispatch, both stop boundaries,
timeout checkpoints, disconnected channels, callback panic, zero timeout rejection
and borrowed non-Send state. Existing worker panic/join tests remain separate gates.

## Native filesystem backend and root observation

FilesystemBackend separates native symlink_metadata/read_dir from host policy.
Its associated Entries yields io::Result<DirEntry>; StdFilesystem delegates directly
to the standard library, preserving native metadata, errors and listing order.
No Send/Sync bound or object allocation is imposed; the host chooses its threading
model and can wrap calls with probes/error decoration and guarded native backends.

observe_root borrows that backend/path and reads metadata exactly once, returning
owned Directory(metadata) or Entry(metadata). Entry includes symlinks and special
files. There is no retry, canonicalization, enumeration or separate type lookup.
Host uses that observation for scheduling/entry admission and can reuse metadata
instead of probing again. No cancellation check is introduced at root observation;
this preserves the existing host behavior even on pre-cancelled initialization.

Only symlink_metadata's final component is not followed. Intermediate components
and read_dir remain path-based and may follow links/replacements. No descriptor
anchoring, atomic tree, content scan, no-download or access guarantee is added.
Host must apply access/cloud admission before calls and retain effective platform
guards through calls and iterator use. Marker-following exceptions, bulk policy,
classification, scheduling, worker startup/join and final status/events stay local.

Tests cover directory/file metadata reuse, one-call/error/raw-path preservation,
borrowed non-Sync backend state, listing paths/open errors and final links to file,
directory and missing targets. Linux additionally covers a non-UTF-8 root name.
Full filesystem engine initialization and live provider/race acceptance remain pending.

## Directory worker protocol

run_directory_worker pops DirectoryTask values from WorkQueue and delegates listing,
opaque-context prefetch selection, measurement and publication to DirectoryWorker.
It constructs the existing measurement plan and publishes Listed before invoking
measure. It then publishes Measured, including a cancelled prefix returned by the
host. The adapter must execute the supplied plan preserving its ID/slot semantics.
The loop does not add cancellation checks or retry a rejected publication. A false
publish result returns immediately without measuring after a rejected listing or
popping the next task. It does not close the shared queue or stop other workers.

Callbacks are synchronous and statically dispatched; borrowed non-Send host state
and opaque contexts are supported. The loop adds no task vectors or trait-object
allocation. IDs are cloned for publication/correlation. Host owns timing, access,
thread setup/guards, backend preparation, stop semantics and error decoration.
Host callbacks/ID cloning can panic; panics propagate. Use spawn_workers for its
close-on-panic guard and keep coordinator shutdown inside the enclosing scope.
An empty open queue blocks until work or closure; callback/syscall cancellation
is not interruptible. Full startup, classification and final status stay outside.

Seven tests cover intent/prefetch and listing-before-measurement ordering, listing
only, rejection at both publications, cancelled results, closed queue, continued
LIFO tasks and callback panic. Existing scope/join and measurement-slot tests remain
separate gates; no whole-scan resource improvement is inferred.

## Owned directory acquisition

collect_directory consumes host-opened entries through visit_directory and returns
(DirectoryListing<Context>, DirectoryVisit). Children/types and errors retain their
original encounter order and raw paths. One synchronous host observer receives
each successfully typed entry before it is retained and may update opaque context.
An observer error is qualified with that child path; it does not remove the child
or roll back context. Iterator/type errors never invoke the entry observer. Context
can borrow non-Send state and need not implement Clone or Default. No second listing
buffer is introduced. A path is cloned only to qualify an observer error.

Cancellation uses the existing visitor boundaries: no extra stop check is added.
An observer-triggered stop retains its current child/context/error before the next
check. Empty input completes; failed opens return their error/OpenFailed even when
already stopped. Callbacks/iterators can panic, and their panics propagate. The host
owns open/guard lifetime, context meaning, observer effects, post-list hooks, final
status and recursive scheduling. Observer I/O may intentionally follow a link;
the helper itself only observes FileType and does not resolve targets or read file
contents. Native pathname races remain; no descriptor snapshot or memory bound.

Seven macOS tests cover order/borrowed context, iteration and observer errors,
observer-triggered cancellation, open failure, empty input, link types, callback
panic and host-supplied iterator paths. Linux adds raw non-UTF-8 names. Existing
visitor, worker protocol and guard/join gates remain separate. No speedup is inferred.

## Typed measurement backend

DirectoryMeasurementPlan::measure_with accepts a mutable DirectoryMeasurementBackend
and a separate stop callback. directory_metadata runs first, then prepare_files
returns owned state implementing FileMeasurementBackend::measure. Preparation runs
even after directory error, pre-cancellation, empty slots or omitted-only slots.
The typed and closure entrypoints share one slot loop, preserving original indices,
None values, error objects, task ID and cancellation prefixes without another buffer.

Prepared state can borrow existing external resources but not the temporary mutable
backend borrow or the transient path/slot arguments. The host copies or moves any
state it must retain and owns native/batch selection, fallbacks, error decoration,
timing and effective guards. There are no Send/Sync, static lifetime, trait-object
or boxing requirements. Prepared state drops before return and on callback/stop
unwind; a panic propagates instead of yielding a completed measurement. The host
must publish Listed first and keep access/platform guards until prepared state drops.
Stop remains separate host policy; active I/O/callbacks cannot be interrupted.
No new retries, status mapping, link semantics or memory budget are introduced.

Five tests cover typed ordering/slot identity/errors with borrowed non-Send state,
pre-cancelled directory failure/preparation, partial cancellation, empty/omitted
slots and prepared-state release on success/cancellation/panic. Existing closure
API and scoped worker tests remain gates; full engine lifecycle stays separate.

## Scoped worker session

run_scoped_workers owns one thread::scope. It retains the queue shutdown guard,
starts host helpers, launches a positive count of cloned workers, then invokes the
coordinator on the calling thread. Host helper startup returns an owned stop guard;
its drop precedes queue closure, and both precede actual join. The coordinator
result is returned only after all workers/helpers join. It can own non-Send state;
helper-start/coordinator callbacks and guards may borrow external resources.

Helper-start, worker and coordinator closures are owned in the scope body. If
startup unwinds, even unused coordinator captures release before helper join. Host
must move receivers into the coordinator, drop unread responses before returning
or unwinding, and avoid returning resources that workers need released to exit.
Callback panics propagate; worker panic closes peers through the existing launch
guard and scope propagates it after join. Low-level spawn_workers remains available.

Host owns root admission, effective platform guards, QoS, worker/message protocol,
setup errors and terminal/status publication after return. Helpers must stop when
their guard drops; workers must react to queue closure or host stop. All destructors
must be well-behaved. This helper cannot interrupt syscalls/callbacks or join code
that never exits; there is no watchdog or whole-scan memory budget. Root setup
failure can remain a host early return before any scope exists.

Five tests cover helper-stop-before-close, actual worker/helper join, caller-thread
non-Send results, opaque setup failure before a terminal boundary, receiver release
on coordinator/startup unwind, and worker panic closing peers. Existing low-level
worker, directory protocol and cancellation tests stay separate gates.

Root admission uses `admit_root(backend, path, preflight)` to return an opaque
`Skipped(reason)`, original `Observed(RootObservation)` or unchanged
`Unavailable(io::Error)`. Host preflight runs once before any backend metadata call;
a skip never observes metadata. Allowed roots delegate to `observe_root` once.
The callback may have its own effects. Guard ownership, reason/status interpretation,
classification, scheduling, warnings and final publication remain host-owned.
No cancellation check or normalization is added; pre-cancelled observation and
final-component no-follow semantics remain. See `specs/root-admission.spec.yaml`
in the upstream repository; this is a draft sequencing boundary, not full startup.

`with_guarded_backend(enter, prepare, run)` owns a synchronous guard/backend scope:
enter guard, prepare backend, run body, drop backend, drop guard. Setup failures
retain original Guard/Backend errors. Neither resource nor result needs Send.
The host must finish scoped work and keep guard-dependent resources inside the body;
externally borrowed backend state and unused callback captures are not owned by the
helper. Restoration is not guaranteed on abort/double panic or faulty guard Drop.

`NonMaterializingIoGuard` now supplies the original Darwin thread-policy RAII in
Hunter (also re-exported by hunter-fs). It cannot move across threads on macOS;
restoration is best effort, and non-macOS uses a no-op. This does not guarantee all
providers refuse downloads. Hosts still choose where to apply guards and own cloud
paths, backend algorithms, warnings/status and terminal events. The default plugin
migration, full engine assembly and whole-process resource budgets remain pending.
See the upstream `specs/backend-lifetime.spec.yaml` draft and fresh validation.
