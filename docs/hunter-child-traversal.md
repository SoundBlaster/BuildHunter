# Host-selected recursive child requests — 2026-10-15

Hunter's ChildTraversal now consumes the listed children in process_listing.
It asks the host for Visit, Skip or Stop before routing each original index.
Admitted observed directories become DirectoryRequest values with opaque listing
context and the host's measurement intent; all other types become indexed Entry
values. A directory symlink observed as a symlink remains an Entry. No new I/O
or refreshed file-type observation is performed.

BuildHunter's selection callback retains the same order: profile checkpoint every
32 original entries, stop check, then cloud-skip warning/selection. Skip never
compacts measurement indices. Stop is terminal and returns from process_listing;
previously queued jobs/results remain subject to existing scan shutdown.
An empty/exhausted listing does not call the selector.

For each directory request BuildHunter increments the same ancestor pending
counts, clones its product-owned chain into DirNode, assigns the next node ID,
and schedules request.into_task(id). Hunter does not inspect the u32 marker
context, infer ancestry, classify entries or select application status. Leaf
policy/discovery and per-slot worker measurements retain their original indices.
Selected-root metadata, parent marker fallback, speculative sizing, result
correlation and completion order are unchanged. CLI/GUI/public API/FFI are unchanged.

The helper consumes the existing Vec without another request vector. It clones
opaque context only for admitted directories; path ownership moves into requests.
BuildHunter retains one directory-path clone for its node arena as before and
explicitly drops the consumed plan before coordinator-side measurement. The
listing buffer and any unconsumed tail otherwise live until plan drop. Unbounded
work/result/arena/report storage remains; no whole-process RAM improvement follows.

Five new upstream tests cover skipped directory/context cloning, original ordering
and indices, terminal Stop, empty/all-skipped input, stale supplied observations
without I/O and Unix raw-path/symlink routing. An application regression mixes
nested files/directories with a host-pruned subtree and verifies exact apparent
sizes, inherited ancestor facts and one completion per finding. Existing cloud,
cancellation, lifecycle, worker/observer panic and bulk tests remain gates.
Fresh execution/source/map receipts: specs/validation/hunter-children-20261015.
Upstream draft snapshot/frozen maps: specs/evidence/hunter-children-20261015.

This is child request planning, not a full Hunter recursive scanner. Host still
owns the coordinator's node arena/result correlation, recursive accounting,
selection effects and final report/status. The next extraction boundary is
opaque task/result bookkeeping; product-specific ancestry stays in BuildHunter.
Merge the upstream Hunter PR first, preserving its source commit ancestry, then
this consumer. The seven-file std-only vendor snapshot keeps root CLI offline.
