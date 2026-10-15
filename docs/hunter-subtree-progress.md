# Subtree size/completion progress — 2026-10-15

Hunter SubtreeProgress now replaces the private ArtifactProgress fields/math.
Each selected result still has one state in BuildHunter's progress vector, with
the same public ID mapping. The state accumulates u64 bytes with saturation,
supports replacing a known leaf total, retains sticky partial information and
tracks host-announced descendant directories/root completion. It owns no IDs,
paths, hierarchy or heap buffers and performs no metadata acquisition.

BuildHunter still routes each measured file/directory contribution to the same
selected ancestor chain, chooses allocated/apparent units and counts outer roots
once in ScanReport.total_bytes. Error/churn/cloud handling marks the same affected
states partial, while final scan status remains product-owned. No file-type,
symlink, filter/classification or default CLI/GUI/FFI behavior changes.

Before scheduling a child directory, the host adds one pending directory to every
selected ancestor state. finish_directory retires that contribution, marks a new
selected root finished when appropriate and checks is_ready in reverse chain order.
Readiness means root processed + zero announced directories + not completed; it
is not proof of worker join or filesystem consistency. Empty/pruned directories
and workers finishing before/after root measurement retain prior behavior.

The host finalizes known leaves directly, normal directories after readiness,
and stopped/unresolved results as partial in reverse discovery order. complete
returns an owned bytes/partial value once, then None; BuildHunter treats duplicate
completion as an internal invariant violation. The partial result combines stored
partial OR host stop state. Early completion does not clear pending counters,
cancel/join work or freeze future host updates. Queue/result/access/sampler lifetime
and inner-before-outer event ordering remain host assembly obligations.

Eight upstream tests cover empty/early/late readiness, separate nested states,
saturation, leaf replacement, partial flags, forced prefixes, one-shot completion
and invalid counters. The new application regression injects a nested file metadata
error and checks exact known bytes, partial propagation to all affected ancestors,
unchanged Incomplete status and exactly one completion from inner to outer results.
Existing pruning, cancellation, worker/observer panic and profiling tests remain gates.

Root CLI uses a nine-file exact std-only vendor snapshot. Storage strategy and
retained report/arena capacities are unchanged; no RAM reduction or fixed budget
is claimed. Fresh source/test/performance/map identities are in
specs/validation/hunter-subtrees-20261015, with upstream draft and frozen maps in
specs/evidence/hunter-subtrees-20261015. Contracts remain draft; math tests do not
establish full generic scan lifecycle or live-cloud/GUI acceptance.

Merge the upstream Hunter PR first with its pinned commit retained in main ancestry,
then the consumer PR. The remaining generic extraction concerns full scan execution
and host-supplied lifecycle/backend effects; product membership and final status
remain separate responsibilities.
