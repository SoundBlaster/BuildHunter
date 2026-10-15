# Scoped work coordination extraction — 2026-10-15

Hunter #3 at 836d764328212fcccb944e0f0a69ba79421b3dcd now owns the generic
WorkQueue<T>, close-on-exit/worker-panic guards, positive capped worker count and
scoped worker launch. Root BuildHunter adopts the exact std-only snapshot.

Opaque ListTask remains BuildHunter data: path/node, parent markers and measurement
intent. BuildHunter owns recursive enqueue decisions, task bodies, guarded listing,
legacy artifact prefetch, sizing strategy, sampler/QoS, result channels, arena,
profile/status/nesting and terminal event. The helper invokes each host worker clone
in caller-owned std::thread::Scope. spawn_workers return is only launch completion;
outer scope completion remains the join boundary before host access is released.
Coordinator retains close_on_drop inside that scope. Numeric FFI/Swift code unchanged.

Shutdown now rejects tasks and releases queued payloads outside the mutex, wakes
waiters and lets in-flight work finish/observe host cancellation. Worker panic closes
peers before unwind; standard scope propagates panic after joining. Host detects a
rejected enqueue as failure rather than silently accepting a lost job. Generic
payload destructors/closure cloning must be well-behaved, especially during unwind.
The queue is unbounded. LIFO is not a queue-memory bound; blocking filesystem calls
remain uninterruptible. No whole-process RAM or live-tree race guarantee is claimed.

The canonical package snapshot now includes src/work.rs as a fifth pinned file.
The verifier exact inventory is updated accordingly; extra build scripts/hidden
inputs/symlinks still fail. Registry/Git-free default CLI graph remains accepted by
the empty-Cargo-home offline check/install/run. Scoped marker Git dependency stays
outside that graph. Source/contract/map receipts: specs/validation/hunter-work-20261015.

New root regressions cover policy/observer unwind and worker filesystem-probe panic,
then a successful follow-up scan. Hunter tests cover recursive task expansion, LIFO,
idle peers, release of pending payloads and peer exit before coordinator cleanup.
Paired Release fixtures and hosted CI are separate from live-home/UI/cloud acceptance.

Merge Hunter #3 first, then this consumer. Next boundary: a generic directory-task
result/planning interface separating recursive discovery/measurement from artifact
ancestor accounting and prefetch; the filesystem recursion coordinator is not moved
by this generic work slice alone.

Follow-up: hunter-directory-tasks.md extracts typed directory tasks/results and
positional measurement planning; host still owns prefetch, I/O callbacks, recursive
discovery and final aggregation.
