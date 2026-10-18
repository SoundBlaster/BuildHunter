# Directory worker protocol — 2026-10-18

Hunter `run_directory_worker` consumes an existing WorkQueue of DirectoryTask values
through the synchronous `DirectoryWorker` adapter. It lists a task, requests the
host prefetch decision, constructs the measurement plan, publishes Listed, measures
only after accepted publication, then publishes Measured. A cancelled prefix is
still delivered. Rejected publication returns immediately, before more measurement
or another task. It does not close the shared queue or stop peers.

BuildHunter `WorkerAdapter` retains all marker/prefetch interpretation, list timing,
measurement timing, Probes, bulk metadata setup/fallback and materialization error
decoration. `run_worker` enters the platform guard once before the helper and holds
it until return/unwind. Setup failure still sends SetupFailed. There is no added
stop check: listing and measurement keep the existing cancellation boundaries,
including backend preparation after a pre-cancelled listing. Static dispatch adds
no task-vector/trait-object allocation; queue/results remain independently unbounded.

Host retains queue shutdown inside the outer thread scope, worker panic closure,
channel/receiver lifetime and final status. Only scope completion proves worker join.
Seven upstream tests exercise order/intent/prefetch, rejected Listed/Measured, empty
closed queue, continuation, cancelled result and panic using non-Copy IDs and
borrowed non-Send context/state. The new application test disconnects the receiver,
verifies zero metadata/bulk preparation and the untouched next task, and on macOS
checks active guard policy plus restoration. Existing cancellation/publication,
worker/callback panic, opt-in timing, bulk, marker and plugin scanner tests are gates.

The exact twelve-file std-only distribution pins Hunter `a1b30a7967ed40f146bf4540af7fcdda4e3809b4`. Offline CLI remains
supported; the independent scoped-metadata Git pin is unchanged. Runtime bindings
identify immutable BuildHunter commit `a2985eeb8e204ded3e9037b4e6d719d637c5d524`. Frozen maps/upstream draft and
source/tool/command receipts live under specs/evidence/hunter-worker-20261018 and
specs/validation/hunter-worker-20261018. Contracts remain draft; no local UI/home or
live-cloud acceptance and no speedup/RAM improvement is claimed. Merge Hunter with
its pinned ancestry preserved before this consumer.
