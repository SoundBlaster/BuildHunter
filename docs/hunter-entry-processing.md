# Child routing and selected metadata reconciliation

Hunter ChildTraversal::route consumes children and drives host select/directory/entry
callbacks, collecting only selected payloads. Original indices, type observations,
context cloning and stop behavior remain. Listing/suffix release before return and
selected measurement; empty input has no callbacks. Panic unwinds the selected prefix.
ArtifactChildren retains profile cadence, stop/cloud access, directory scheduling,
ancestry and file selection/classification. No extra child vector or stop check is added.

SelectedEntry stores original index/path and opaque host context; FileSlot uses an
optional product selection ID. PendingMeasure carries SelectedMeasurementPlan plus
product chain/parent length. The plan resolves selected directory first, using root
observation > worker result > one guarded backend fallback. Entry responses are taken
by original slot index once, including errors. Missing/out-of-range/consumed slots
fall back once on the unchanged path. Checkpoint runs at selected-entry ordinal before
lookup/I/O; no extra cancellation check is introduced. finish runs before selected/
cached buffers release. Panic skips finish and unwinds buffers.

ArtifactMeasurements retains byte mode, profile counter updates, affected ancestry
and finish_directory. Guard scope, metadata units, path handling, symlink-selection
policy, speculative result accounting, product events/status and FFI remain unchanged.
The generic helper does not assert selected entries are regular files, deduplicate
hard links, acquire a guard or guarantee a race-free tree snapshot/resource bound.

See upstream specs/entry-processing.spec.yaml (draft) and local validation receipts.
This is another bounded extraction, not the complete ScanEngine API. Engine facade,
branch-state composition, descriptor recursion and default plugin migration remain.
