# Owned traversal coordinator

Hunter TraversalCoordinator owns root admission and per-scan task accounting. Its
TraversalTasks borrows the ScanSession queue and owns the ledger. BuildHunter's
former Scanner is ArtifactHandler, implementing the upstream TraversalHandler.
It retains product decisions/results and no longer owns queue/ledger fields.

The consuming run path observes the original root through admit_root after host
preflight, then invokes host initialize. False skips pump and finish. Otherwise the
existing drive_scan order remains: outstanding/stop, checkpoint, receive, retire,
host pending-counter update, stop check, dispatch. A timeout adds no receipt. The
consuming drive path supports already initialized tasks without repeating admission.

Enqueue reserves ID/state/response, invokes before_publish for host telemetry, then
pushes. Rejection returns the unchanged task and retains the reservation; the host
must stop/interpret it. Listings retain their measurement response reservation and
ignored speculative results still retire. Normal pump exit invokes host finish once
before ledger release; panic skips finish and unwinds state. Loop outcome includes
outstanding count and does not establish product status or worker join.

Ledger release now occurs before coordinator returns; BuildHunter releases unread
receiver next, before scope join. Guards, worker/sampler QoS, shutdown/join and final
profiles/Finished remain in scan_session. Root/profile/cancellation timing and status
precedence remain. Marker facts, classification, child-selection policy, artifact
ancestry, subtree membership and report events stay product-owned. The separate
plugin pipeline is not enabled by this refactor. No new dependency is introduced.

This implements one bounded portion of the proposed coordinator architecture. Full
ScanEngine, generic child routing/measurement assembly, descriptor recursion, C5
budgets and live home/cloud/UI acceptance remain pending. See the upstream draft
specs/traversal-coordinator.spec.yaml and local validation receipts.
