# Hunter

Reusable filesystem acquisition for in-process search engines. `hunter-fs` is an
Unix scoped-metadata and macOS/Linux descriptor-acquisition library, alongside the
portable scan engine in hunter-fs-core. Product policy remains outside Hunter.
It has no BuildHunter/Rifle dependency or artifact vocabulary.

```rust
# #[cfg(unix)] {
use hunter_fs::{ScopedMetadata, Location};
use std::{path::Path, ffi::OsStr};
let root = Path::new("/selected/tree");
let scope = ScopedMetadata::open(root)?;
let observation = scope.inspect_regular_file(root, Location::Own, OsStr::new("config"));
# }
# Ok::<(), std::io::Error>(())
```

The root descriptor remains anchored after path replacement. Intermediate opens
and final metadata observations do not follow symlinks. Regular files return true;
missing entries, directories and links return false. Access/scope/cloud/I/O failures
stay distinct unavailable outcomes. `metadata_attempted` counts the final metadata
syscall only, not descriptor opens or `fstat`. Invalid names return without I/O.
Paths/names retain OS bytes. Names must be one nonempty component, without `/`, NUL,
`.` or `..`. Own/Parent select the candidate directory or its parent; root Parent
cannot escape the selected scope. Candidates containing parent components are rejected
before deriving Parent, with no final metadata attempt.

The host must hold effective permissions and set a non-materializing I/O policy on
the calling thread before opening/using this object on macOS. Dataless detection
alone does not establish a no-download guarantee. Calls are synchronous; transient
file changes are not an atomic snapshot. A selected regular-file root has no directory
handle and observations return unavailable. `Drop` releases owned descriptors.

Windows currently compiles without this Unix API. ScopedMetadata alone provides no traversal, size accounting,
worker pool, cancellation, indexing, full memory budget or performance gain is claimed.

Tests preserve the prior marker race cases and cover public name validation,
regular/directory/link distinctions and selected-root parent scope. Linux CI covers
non-UTF-8 names; APFS rejects creation of that fixture. Cloud evidence is synthetic,
not a live provider test.

Run `cargo test --locked`, `cargo test --locked --release`, and
`cargo clippy --locked --all-targets -- -D warnings`. CI runs these small crate checks
on Linux/macOS/Windows; no app builds or whole-disk benchmarks.

Derived from BuildHunter `crates/buildhunter-plugin-scanner/src/scope.rs` at
`96780d7f9a9a9e5a6a19a53e2eea2f4f4ff1a2aa`, under the included MIT license.
[Draft boundary](specs/scoped-metadata.spec.yaml), [map](specs/hypercode/current.hc).

## Portable primitives

`crates/hunter-fs-core` now owns portable one-directory enumeration and metadata
byte calculation, without libc or any external dependencies. See its README and
`specs/portable.spec.yaml`. This is an additional partial extraction; recursion,
worker orchestration and complete scan lifetime remain host-owned. Workspace CI
checks both packages. Prior scoped-metadata test records remain historical.

The std-only core also contains generic WorkQueue and scoped worker launch/shutdown.
It supports recursive task expansion while selection, recursion policy, setup/QoS
and report lifecycle remain consumer-owned; see the core README and worker spec.

DirectoryTask/Listing/Result and measurement planning now provide typed opaque
job/result correlation and aligned partial measurements. Host keeps recursive
selection, backends and protocol publication; see the core README and task spec.

ChildTraversal now emits host-selected recursive directory requests or indexed leaf
observations from an owned listing. Host keeps guard/selection effects, assigns
IDs, schedules requests and owns recursive accounting/status. See the core README
and specs/child-traversal.spec.yaml; this is not a complete recursive scanner.

DirectoryTaskLedger now centralizes serial task IDs, opaque listing/deferred state
and expected-response bookkeeping. Caller still owns the result protocol and
actual worker/queue lifecycle; see specs/task-ledger.spec.yaml and the core README.

SubtreeProgress now supplies saturating byte totals, pending-directory/root progress
and one-shot partial completion values. Host routes contributions and owns dependent
completion order; see specs/subtree-progress.spec.yaml and the core README.

drive_scan and ScanCoordinator now provide the serial result execution loop with
optional idle checkpoints and receipt-before-stop dispatch ordering. Host keeps
initialization, backends, finalization/status and scoped worker lifetime; see
specs/scan-execution.spec.yaml and the core README. Full engine extraction and
whole-scan memory budgets remain outstanding; no performance gain is inferred.

FilesystemBackend/StdFilesystem now provide the native metadata/listing seam;
observe_root returns one owned Directory/Entry observation for host initialization.
Host still applies access/cloud admission, guards, scheduling and final status.
See specs/filesystem-backend.spec.yaml and the core README for path/link/race limits.

DirectoryWorker/run_directory_worker now own the directory-task protocol loop:
list, request host prefetch, publish Listed, measure and publish Measured. Host
keeps backend/guard/timing policy and actual worker scope lifetime. Rejection
stops this worker without taking another task; see specs/directory-worker.spec.yaml.

collect_directory now collects an owned listing plus visit outcome with an ordered
host entry/context observer. It keeps collection independent of context meaning;
observer errors retain children, and cancellation preserves the observed prefix.
Host keeps opening, guards, observer I/O and post-list behavior. See
specs/directory-listing.spec.yaml and the core README for callback/link/race limits.

DirectoryMeasurementBackend/FileMeasurementBackend and plan.measure_with now supply
a typed preparation seam while retaining the closure API and shared slot loop.
Host owns native/batch effects and guards; prepared state is owned and released
before completion/unwind. See specs/measurement-backend.spec.yaml and the core README.

run_scoped_workers now assembles helper startup, scoped worker launch, caller-thread
coordination, helper stop/queue closure and actual join. Host retains root admission,
guards, QoS and terminal interpretation; see specs/scoped-session.spec.yaml. The
helper does not guarantee arbitrary callback termination or full scan acceptance.

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

Guarded scan assembly now lives in the std-only core: `with_scan_session` owns the
empty queue/count inside backend/guard lifetime; consuming `ScanSession::run` joins
before caller finalization. Root admission policy stays in the host coordinator.
See specs/scan-assembly.spec.yaml and specs/validation/scan-assembly-20261019;
full engine assembly, classification and resource budgets remain outside this slice.

## Serial traversal coordinator

`TraversalCoordinator` now owns root admission sequencing and per-scan task/result
accounting. Its `run` performs host preflight and one root observation, then gives
that outcome to `TraversalHandler::initialize`. A false result skips both the pump
and finish callback. Otherwise `drive_scan` keeps the existing checkpoint, receive,
retire, stop, dispatch order and invokes host finish before releasing owned task state.
`drive` also accepts already initialized tasks without repeating root admission.

`TraversalTasks` owns the ledger and borrows the scoped queue. Enqueue reserves the
ID/response before a host telemetry callback, then publishes; rejected publication
retains its reservation and returns the unchanged task. Hosts must stop/interpret
rejection. Listing and speculative measurement accounting keep the original ledger
protocol; state, pending payloads, context, root skip reasons and message meaning
remain opaque. `TraversalOutcome` distinguishes declined initialization from Drained,
Stopped and Disconnected and retains outstanding counts. It is not a product scan
status or evidence of worker join.

Host callbacks are serial caller-thread operations and may borrow non-Send state.
The coordinator consumes itself and releases its ledger after host finish, or on
unwind without finish. Queue closure/helpers/worker join and unread receiver lifetime
remain the enclosing ScanSession's host obligations. Call run inside that guarded
scope to retain root-observation timing. Classification, markers, child-routing
policy, selected-subtree membership, report events and final status stay in the host.
This remains partial engine extraction: see specs/traversal-coordinator.spec.yaml
(draft); full domain-independent recursion and resource budgets are not promised.

Child routing and selected metadata reconciliation now live in the std-only core.
ChildTraversal::route retains host admission/selection; SelectedMeasurementPlan
preserves root/worker/fallback and original-slot semantics, while host callbacks
retain interpretation/aggregation. See specs/entry-processing.spec.yaml; the serial composition facade is described below.
Autonomous backend/worker factories and resource budgets remain pending.

## Composed scan entrypoint

`ScanEngine::run` now composes root/task coordination, typed response dispatch,
child routing and selected metadata resolution around `ScanHandler`. Branch state
and selection payload stay opaque; the engine owns deferred plan assembly and the
immediate/deferred/ignored measurement branches. `drive` accepts already initialized
work. Lower-level primitives remain available; no new dependency or thread is added.

Host callbacks retain policy, context meaning, scheduling decisions, warnings/units/
aggregation and product reporting. The facade is called inside the existing guarded
ScanSession worker scope, with unread receiver release before join and final events
afterward. It is not an autonomous worker/backend factory or a resource-budget claim.
See the core README, specs/scan-engine.spec.yaml (draft), and fresh source-bound
validation at specs/validation/scan-engine-20261019. Earlier receipts stay historical.

`ScanSession::run_engine` now composes launch wiring with ScanEngine: ScanChannel
retains original producer/receiver ownership, EngineLaunch groups borrowed settings,
make_host runs on the caller, and collect extracts report inputs/releases host state
before helper shutdown and join. The result returns after join; product terminal
publication remains outside. See the expanded draft scan-assembly boundary and
specs/validation/engine-launch-20261020. No extra stop probe, I/O or thread is added.

## Opt-in Darwin batch metadata — 2026-10-21

`darwin_bulk::read_directory_sizes(path, should_stop)` is available on macOS with
`macos-bulk-metadata` (default off). It owns the original getattrlistbulk acquisition
and checked decoder, preserving raw basename bytes, apparent/allocated sizes and
native errors. Open precedes the first stop check; stop is checked before each batch
and returns an owned prefix. The caller holds permissions and thread materialization
policy. Only the final directory component is opened no-follow; this is not whole-tree
descriptor anchoring, a snapshot, constant memory or an interrupted syscall.

Source: BuildHunter `src/bulk_metadata.rs` at 7ab00f16ae437fe88427c2acbe1b651b30b044f1
(MIT). Product counters, error decoration, fallback and backend choice remain host-owned.
See `specs/darwin-bulk.spec.yaml` in Hunter and fresh validation
`specs/validation/darwin-bulk-20261021`; historical receipts remain historical.

## Prepared directory sizes — 2026-10-21

`PreparedDirectorySizes::new(result).take_file(path)` owns a batch and consumes each
basename value/error once. Missing/consumed names and Unsupported acquisition return
None for host fallback. Other whole-batch errors repeat without reacquisition, with
errno or kind/message preserved. No I/O, stop probe or profiling is added. The host
supplies original paths from the same directory, holds guards and applies error
decoration. Keys use the same process/platform OsStr encoding, not a serialized path.
This portable helper is separate from opt-in Darwin acquisition. See the draft
`specs/prepared-batch.spec.yaml` and `specs/validation/prepared-batch-20261021`.

## Process platform primitives

`hunter_fs_core::thread_qos::{capture, apply}` explicitly propagates requested
macOS pthread QoS (class and relative priority), best-effort; other platforms
are no-ops. This is neither effective QoS nor a restoration guard. The consumer
chooses which workers/helpers receive the request and when.

`hunter_fs_core::process_resources::read()` reads current-process cumulative
user/system CPU nanoseconds and current resident bytes (Windows working set).
Each counter is independently optional; unsupported platforms return no values.
The host owns timing, deltas, percentages, averages, peaks and scan attribution.
No child processes, scan-tree payloads or sampling tasks are introduced.
See the draft [process-platform contract](specs/process-platform.spec.yaml).

## Descriptor scan acquisition — R3 (2026-10-23)

`ScopedFilesystem` (macOS/Linux) implements core `ScanAccess` for a selected root.
The host installs its non-materializing guard before construction and on every
worker. One directory root handle anchors listing, sizing and `marker_scope()`.
Component-by-component `openat(O_DIRECTORY | O_NOFOLLOW)` prevents intermediate
links from redirecting access; child metadata uses `fstatat(AT_SYMLINK_NOFOLLOW)`.
No file contents are opened, including unreadable files and FIFOs. Initial root
ancestors are caller-selected; a selected non-directory retains its initial
no-follow metadata observation and cannot become a directory traversal root.

Every listing opens an independent directory stream; concurrent passes do not
share offsets. `DirectoryAccess` retains its directory across metadata/size reads,
even if its pathname is replaced. Coordinator missing-result fallback acquires
one directory lazily per selected pass; fully cached passes do not acquire it,
and scoped failures never retry against ordinary paths. Native names stay raw.
`EntryType` is the public four-category observation used by core routing instead
of the private-layout std `FileType`; `EntryMetadata` carries sizes without
constructing std metadata from native stat layouts. Existing std visitors convert
observations losslessly for their directory/file/link/other routing decisions.
The facade uses its existing libc dependency; the core stays std-only/offline.

This is no-follow acquisition, not a filesystem snapshot or stable inode ledger
between separate passes. Directory replacement with another ordinary directory
can change later observations; an opened pass remains on its original directory.
Paths shown to a user are identities for reporting, not durable filesystem action
capabilities. No open descriptor per queued path is introduced; full budgets,
Windows/other Unix implementation, default client adoption and live cloud
acceptance remain separate work. Root/child dataless refusal supplements the
calling-thread policy; it is not independent proof that a provider never downloads.

Tests cover selected-root and intermediate-link replacement, regular directory
replacement during a prepared pass, final links, raw Linux names, cancellation,
permission-independent metadata and parallel listing offsets. Tests and source
identities: `specs/validation/scoped-filesystem-20261023`. Contracts remain draft.

Relative-root review correction (2026-10-23): ScopedFilesystem keeps the caller's
original root identity (including `.` and relative selections) for all report/
scope operations. A separate absolute path is used only for initial acquisition.
The relative-root regression covers root metadata, listings, prepared metadata
and markers without a process-wide chdir. Fresh evidence is in
`specs/validation/scoped-relative-root-20261023`; the first R3 receipts remain
historical evidence for eafb6ff.
