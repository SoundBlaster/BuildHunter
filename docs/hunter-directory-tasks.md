# Directory task/result planning — 2026-10-15

Hunter #4 at e9b77e44f50ac72e543b727a11afa2c28666226e owns DirectoryTask,
DirectoryListing, DirectoryResult, DirectoryMeasurement and the positional
DirectoryMeasurementPlan. IDs and task/listing contexts remain opaque. BuildHunter
uses usize IDs and u32 contexts for its own parent/listing marker bits; Hunter never
interprets them. Measurement intent plus a host prefetch boolean selects the plan.
Construction clones only selected child paths, as before, and borrows the task path.
No new metadata reads are introduced by planning.

BuildHunter still chooses prefetch and recursive children, supplies guarded directory
metadata and optional Darwin bulk preparation/file lookup, records timings, publishes
result messages and applies ancestor/nesting/error/status accounting. Root private
worker messages wrap the Hunter result envelope; setup failure remains host-specific.
Rust public scanner/CLI/GUI/numeric FFI are unchanged. The entire recursive filesystem
coordinator is not yet a generic Hunter entrypoint.

Slots retain original child indices with None for initially observed directories and
symlinks; other entry types retain legacy eligibility. Preparation follows directory
metadata even after directory failure or pre-cancellation. The per-slot stop check
returns an aligned prefix. The host must publish Listed before invoking measure and
send Measured even for a cancelled partial pass. The enum cannot enforce sequencing;
a root integration test observes Listed inside the metadata hook, cancels, then
requires a same-ID Measured response with no file measurements. Existing cancellation,
worker/observer panic, nested accounting and bulk tests remain acceptance gates.

The std-only canonical snapshot now contains six pinned files including src/task.rs.
Verifier inventory and hashes match that exact upstream source. Offline fresh-cache
build/install/run remains required. New source/map/test/performance receipts are under
specs/validation/hunter-tasks-20261015; prior receipts remain historical. No live-home,
UI/cloud, whole-memory or speedup claim follows from this extraction.

Merge Hunter #4 first, then this consumer. Next boundary: define generic recursive
child-selection/aggregation requests so the remaining coordinator can delegate tree
planning without importing artifact ancestry or marker policy into Hunter.
