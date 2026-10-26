# BuildHunter — architecture and specification progress

Latest feature slice: [R7 file-backed user templates](../docs/user-templates.md),
2026-10-25 feature branch. Its contracts/evidence are recorded below and in
`validation/user-templates-20261025`; it does not close CLI/SDK/Store delivery.

Latest state: [merged-main audit, 2026-10-22](../docs/modular-refactor-audit-20261022.md).
Public baselines: BuildHunter `6dcf9ba91e67c7d5f3270369d05e03d01da1db5e`,
Hunter `f1c9f921fed2ea04aa8e67b84075d9fd3a504e2d`, Rifle
`693a284123faaa1ad3192cd88b259ba97a7b5910`. The planned common-code extraction
is implemented; R1–R8 distinguish dependency alignment, default plugin migration,
new safety/resource work and delivery. Draft acceptance is not promoted.
The current audit supersedes historical next-step descriptions below; dated
implementation receipts retain their original scope.

## Goal and current scope

Consumer goal: refine module boundaries for **Hunter / Rifle / BuildHunter /
Arsenal** using current implementation evidence before extraction. Completion
level: reviewable descriptive drafts, source bindings, target ownership and
parent obligations. The historical recovery pass authorized no runtime changes or application
acceptance. Later implementation passes are separately authorized and evidenced below.

Historical recovery baseline: `48badf9e097783505151dd47fbafe290f0af25e2`.
Historical pilot follow-up baseline: `6bab3c34684e81f14fbacd03445d297038edd9b3`, with
changed inputs pinned in `validation/plugin-scanner-20261014/source-identity.json`. User's dirty
Xcode scheme is excluded from claims and preserved; its digest is recorded.
Design documents are working-tree inputs pinned separately from source revision.
Current/target map identities are independent of source identity:
[identity record](evidence/modules-20261013/source-identity.json).

## Coverage

Initial recovery coverage below is retained for provenance. Current implementation
ownership, remaining requirements and evidence limits are in the linked audit.

| HC role/path | Consumers / profiles | Contract or folding/exclusion | Source/map baseline | Evidence / review | Unknown / next action |
|---|---|---|---|---|---|
| Current SharedRuntime.Scanner | CLI, GUI; portable / optional macOS bulk metadata | `buildhunter.current.scan`; traversal/measurement/cloud/QoS/profile folded | pinned source + current map | source review; draft | C3/C5; deeper backend/resource math not recovered separately |
| Current SharedRuntime.ArtifactClassification | CLI policy; GUI SpecificationCore | `buildhunter.current.classification`; selection/prefetch/markers/filter catalog folded | same | source + test source; draft | C1/C2; catalog not plugin runtime |
| Current SharedRuntime.RustSwiftBridge | GUI finite-table / callback fallback | `buildhunter.current.bridge`; queue/access/table folded | same | source + test source; draft | C4; new ABI not frozen |
| Current Clients.CommandLine / MacApplication | CLI; macOS window, diagram, warnings, Settings, export | `buildhunter.current.application`; client effects and projections folded | same | source review; draft | C6; full Charts/layout/OS actions runtime outside recovery |
| Target Hunter and its parts | Any future in-process client | `hunter.design.scan`; internal roles folded | target map + design hashes | desired/inferred obligations; draft | Generic fact acquisition and cancellation/memory contract |
| Target Rifle and its parts | Public/private catalog authors; BuildHunter adapter | `rifle.design.rules`; schema/validation/plan/evaluation/SDK folded | pinned Rifle 693a284 | pure programs/catalog/identity implemented; architecture spec draft | Swift SDK and product integration |
| Target BuildHunter and its parts | Public CLI/macOS product, paid composition | `buildhunter.design.product`; basic rules/policy/assembly/UI folded | same | design + migration review; draft | C1–C6 decisions and parity acceptance |
| Target Arsenal and its parts | Private Store composition | `arsenal.design.catalog`; packages/fixtures folded | same | design only; draft | Private license, catalog content and integration CI |
| Target StoreDistribution | Store publisher | Folded into `buildhunter.design.product/edition,store_delivery`; not a separate spec/repo promise | current target map | BuildHunterMAS repository created; design draft | Release pipeline pending |
| Future universal search / index / LLM | Future separately named product | Deliberately excluded, not an unexplored BuildHunter requirement | ADR/module design | owner explicitly deferred | Separate product planning later |
| Detailed chart geometry, SwiftUI layout, full native accessibility, profiler numerical conformance, Screener fidelity | Current product users/developers | Outside this extraction-contract scope; coarse presentation/profile roles only | current source identity | not fully recovered/verified | Separate deeper recovery if those boundaries change |

## Passes completed

1. Oriented source entrypoints (`main`, App scenes, scan/FFI, generation reducer),
   previous ADRs and commercial plan; no existing BuildHunter maps/spec package.
2. Pilot: current scan + bridge lifecycle, a representative capability spanning
   Rust effects and Swift consumer ownership; surfaced marker and queue limits.
3. Expanded to artifact classification/client assembly; described significant
   boundaries without creating a package per private type or view.
4. Reconciled four target modules and Store composition; created claim/assembly
   matrix with C1–C6 instead of rewriting legacy behavior to match desired design.
5. Simplified maps: shared runtime once, independent clients retained, profile
   differences in HCS; current and intended architecture kept separate.
6. Authored eight draft BoundarySpecs in one descriptive SpecPackage. This package
   is a planning/review artifact, not four released library SpecPackages. Future
   repository release boundaries and versions will be established on extraction.
7. Reviewed bindings/capability declarations and validated structures; actual
   commands, tool/schema identities, warnings and limits are in the
   [validation record](validation/modules-20261013/README.md).

## Review correction — 2026-10-13, PR #63

- C6 / `buildhunter.current.application/constraints.effects_scope`: narrowed the
  observed read-only guarantee to scanning/Finder/copy. Explicit report/profile
  outputs may be inside the target; preferences/clipboard have separate effects.
  This corrects a draft observation, not owner acceptance or a change to PRD BH012.
  The broad PRD target-mutation wording and its explicit export exception still
  require owner interpretation before promotion to an accepted precise contract.
- Corrected the historical whitespace-check coverage claim and added an explicit
  baseline-to-result check. Original command outputs remain untouched.
- No source or map changes: all current/target roles, profiles and other claim IDs
  retain their prior scope. New contract/document digests and structural checks:
  [review validation](validation/review-fixes-20261013/README.md).
- Detailed contracts remain drafts pending the owner's architecture review.

## Reader review — 2026-10-13

Reworked `docs/modular-architecture.md` after owner feedback that unexplained
terms and abrupt jumps made the architecture hard to review. The document now
introduces public/self-build and paid Mac App Store delivery first, defines
plugin/target/fact/marker/IR/plan/FFI, walks one finding through the proposed
adapter, and separates repository ownership from runtime modules and delivery.
The private release repository remains a proposal with an unapproved name.

Affected target claims: `hunter.design.scan`, `rifle.design.rules`,
`buildhunter.design.product` and `arsenal.design.catalog`; their ownership and
interfaces are unchanged. C1–C6 and current contracts retain their scope.
No new API/schema, code extraction or owner acceptance is implied. Maps and
BoundarySpecs remain unchanged because the revision explains the same boundaries.
Historical hashes refer to earlier prose; current document identities and fresh
structural validation are in the [reader-review record](validation/reader-review-20261013/README.md).

## Acceptance and verification

- Accepted by owner: names, product direction, public/private split, future
  universal app separate and deferred.
- Not accepted as detailed requirements: newly recovered normative constraints,
  exact future APIs/schema and behavior changes needed for C1–C6.
- Source-reviewed: the entrypoints and contracts above with pinned file digests.
- Structurally checked: HC parse/cascade/contracts for four profiles and SpecPM
  package validation, with dependency warnings retained and explained.
- Not run here: app tests/builds, live home scans, Store archive/review or plugin
  benchmarks. Historical CI/runtime reports do not validate new target modules.

Draft coverage is complete for this **module-refinement scope**, not the whole
application behavior. Next useful boundary: `Rifle.RuleModel + Evaluation` and
`BuildHunter.ScanAssembly` facts/selection seam after C2/C3/C4 owner decisions.

## Maintenance and continuation

Follow root AGENTS.md for every code change. Re-read actual source, identify
affected claims and refresh bindings/evidence. Record unchanged-contract reasons
and preserve historical map/source/validation snapshots. If changing a folded
boundary needs more consumer detail, recover it then; an old coarse map does not
prove unchanged semantics. Add a dated pass/evidence directory rather than
overwriting the baseline above. Do not mark a target contract implemented merely
because a new repository exists or its structural validator succeeds.

## Plugin contract pass — 2026-10-13

Scope: concrete reviewable v1 package/program contract, typed facts/predicates,
unknown/conflict semantics, limits, compatibility and examples. See
[contract](../docs/plugin-contract-v1.md) and
[validation](validation/plugin-contract-20261013/README.md).

Updated target claims: Rifle `predicate_semantics,wire_validation`; BuildHunter
`plugin_adapter,match_conflicts`; Hunter `plugin_fact_acquisition`; Arsenal
`shared_plugin_contract`. Existing current contracts are unchanged. The map's
RuleModel/CatalogValidation/Evaluation, ScanAssembly, MetadataMeasurement and
PluginFixtures roles already cover these responsibilities; map composition is
unchanged. No new boundary or runtime module is introduced by the schema tests.

Acceptance remains draft. Executed evidence covers JSON wire shape and document
structure; semantic evaluation, SDK export, JCS and filesystem integration remain
unimplemented. Next implementation boundary is the pure Rifle evaluator after
review of the proposed unknown/conflict, probe scope and limit decisions.

## Plugin vocabulary review correction — 2026-10-13

PR #65 review found that the architecture and ADR advertised artifact-ancestor
facts beyond the concrete v1 binding schema. Reconciled the overview, ADR and
contract: v1 exposes name/kind/own-parent markers only; ancestor classification,
nested presentation and totals belong exclusively to BuildHunter host policy.
Updated `buildhunter.design.product/plugin_adapter`; current implementation
contracts and map composition are unchanged. Added a schema rejection regression
for ancestor bindings. No new binding, runtime behavior or owner acceptance.
Fresh evidence: [vocabulary review](validation/plugin-vocabulary-review-20261013/README.md).

## CI scheduling — 2026-10-14

Added conservative job-level change classification and tests. Documentation/spec
changes retain lightweight schema validation while heavy Rust/macOS/performance
jobs depend on the complete event diff. Manual/unknown comparisons run full CI.
See [policy](../docs/ci-change-policy.md). All current/target BoundarySpec claims
and HC maps remain unchanged: scheduling affects validation execution, not product
behavior, module composition or accepted contracts. Historical evidence is retained;
fresh checks are in [CI validation](validation/ci-gating-20261014/README.md).

## Pure plugin adapter and BuildHunterMAS — 2026-10-14

Baseline implementation: 998b1319fc2761a30f4f1db4df8d9bf35548f6fa.
Added observed opt-in PluginPolicy/CatalogValidation/EffectivePlan/CandidateClassification
roles and `buildhunter.plugin.policy` contract. Current maps now bind this pilot
separately from legacy ArtifactClassification. Rifle is pinned to merged 693a284;
public Cargo dependency reads use HTTPS for credential-free hosted CI, Git remotes SSH.

Existing current scan/classification/bridge/application claims remain unchanged:
no scanner/FFI/client wiring is performed. src/lib.rs adds only a feature-gated
module and an explicit u64 type in an existing test (serde_json adds another
PartialEq impl). Original evidence stays historical; full Rust regression checks
plus fresh map/spec checks are recorded in
[adapter validation](validation/plugin-adapter-20261014/README.md).

Host-origin validation, refs/types, reserved IDs, exclusions/probe elimination,
three-valued target aggregation and sorted diagnostics are implemented in a pure
API. Filesystem acquisition, current default parity C2–C4, UI notices and performance
acceptance remain pending. No new recovered requirement is accepted by test success.

Owner supplied BuildHunterMAS as the approved private release repository. Access
and privacy confirmed; local SSH clone contains a README, not a working release
pipeline. Target StoreDistribution map retains its role but now names BuildHunterMAS.
Arsenal content and Store signing/publishing remain separate work.


## PR #67 review corrections — 2026-10-14

Three findings addressed: refresh the map entrypoint, rebind both map baselines
and isolate optional plugin dependencies from default CLI Cargo resolution.
PluginPolicy now resides in independent `crates/buildhunter-plugin-policy`, with
its own lockfile, explicit CI checks and no root workspace dependency. Root
Cargo.toml/Cargo.lock/src/lib.rs are restored to the pre-pilot source state.
`scripts/test_offline_cli.py` checks and installs the CLI using an empty Cargo
home with `--locked --offline`, then runs the installed executable.
Affected claims: `buildhunter.plugin.policy/opt_in` and its source/test bindings;
current PluginPolicy composition; target Rifle and StoreDistribution identity.
Classification semantics and accepted plugin requirements are unchanged.
Earlier snapshots and command receipts stay historical. Fresh review evidence:
[validation/plugin-review-20261014](validation/plugin-review-20261014/README.md).
No scanner integration, Swift UI or performance result is implied.


## Opt-in scanner integration — 2026-10-14

Added PluginScanning/CandidateObservation/MarkerAcquisition/TargetRouting roles
and `buildhunter.plugin.scanner` draft boundary. Observer bypasses legacy name and
marker gates; independent integration derives scoped facts from active catalog
bindings and emits real target metadata. Immutable catalog, nested totals,
cancellation and existing worker/event lifetime are preserved. Root CLI remains
dependency-free; default client/FFI policy is unchanged.

Changed contracts: current.scan inbound observer and source evidence;
plugin.policy exclusions clarify its separate I/O consumer; new plugin.scanner
owns Unix marker effects/routing. Detailed requirements remain drafts. C1 is
implemented for the independent API only; C2–C4 default migration, Windows marker
backend, descriptor-bound traversal and private composition remain outstanding.
Validation/identities: [plugin-scanner-20261014](validation/plugin-scanner-20261014/README.md).


## PR #68 Linux fixture correction — 2026-10-14

Only Ubuntu Rust CI failed: invalid_utf8_name_is_unknown_with_lossless_path
expected Indeterminate while the required forge.project marker was absent.
The implementation correctly evaluated Unknown AND false as false/NoMatch.
Added the positive marker to isolate invalid-name handling and extended the
platform-independent policy test to cover invalid name + false marker => NoMatch
with no irrelevant unavailable diagnostics. Production code and contracts are
unchanged. Affected claims: plugin.policy/classification,fact_boundary and
plugin.scanner/candidate_gate,routing. Fresh source/map identity and receipts:
[plugin-scanner-ci-20261014](validation/plugin-scanner-ci-20261014/README.md).
Initial local/macOS results do not prove Linux fixture execution; hosted CI is
separate evidence. All earlier records remain unchanged.


## PR #68 live diagnostics review — 2026-10-14

Resolved delayed diagnostic delivery: Ambiguous/Indeterminate events formerly
queued until another core event, often terminal Finished. Now the single serial
observer receives each diagnostic during candidate processing, after the internal
state borrow is released. Removed the redundant event queue; reports still retain
diagnostics and no constant whole-scan memory bound is claimed. Cancellation is
observed before descendants, with exactly one terminal event. The new regression
failed before the fix (Completed instead of Cancelled), then passed for both
Ambiguous and Indeterminate diagnostic-only scans.
Affected claims: plugin.scanner/routing,lifecycle; public API/ownership unchanged.
Fresh receipts and identities: [plugin-diagnostics-review-20261014](validation/plugin-diagnostics-review-20261014/README.md).
Historical source/test records remain unchanged; default clients and pure matcher
semantics are unaffected. Hosted CI is separate evidence.

## Partial Hunter extraction — 2026-10-14

Consumer goal: remove product/plugin vocabulary from reusable scoped filesystem
acquisition. Hunter PR #1 owns `hunter.fs.scoped_metadata` at
`71f71fc9166da3ce948fce372101c5a46dfd0f36`; BuildHunter pins it in the independent
plugin-scanner package. Local `scope.rs` only owns product mapping and optional
acquisition. `.git` pruning, catalog rules, materialization-policy lifetime and
legacy traversal remain host-owned. No default CLI/GUI/FFI changes.

Affected current roles: SharedRuntime.HunterMetadata and PluginScanning.MarkerAcquisition.
Affected contracts: plugin scanner marker_scope/effective_acquisition/lifecycle/compatibility;
Hunter design generic_facts/no_product_dependency/io_contract/plugin_fact_acquisition.
Runtime requirements are preserved; Hunter design remains partial/draft. C2/C3/C4
default migration is not accepted by this extraction. Previous whole-scan/memory
evidence is historical and supplies no speedup or memory-budget proof for this pin.

Upstream source/contract/map snapshots preserve original paths/revision under
specs/evidence/hunter-metadata-20261014/upstream. Fresh source/map identities and
local command receipts live under specs/validation/hunter-metadata-20261014.
Full descriptor-bound traversal/measurement is the next boundary; Windows and
live dataless-provider behavior remain unverified.

## Hunter Parent scope review — 2026-10-14

Pin updated to 5c8f3c9a1a018039b5aea9b731f2fb744f625b5a. Upstream rejects parent
components before selecting Parent; root/.. cannot become an unrelated root lookup.
Affected marker_scope/io_contract bindings refreshed; map roles are unchanged.
Prior snapshots/receipts remain historical. Fresh upstream snapshots and local
integration/source/map receipts live under hunter-parent-review-20261014.
Default clients remain unchanged; no new runtime performance/cloud claim.

## Portable Hunter adoption — 2026-10-14

Hunter #2 at 8295d1418c03d1295cea86f4781352469fa27c78 now owns std-only directory
entry iteration/errors/type lookup/cancellation prefix and metadata byte conversion.
Root BuildHunter adopts the exact vendored source, preserving fresh-cache offline
build/install and default client/FFI behavior. Current Scanner.Traversal/Measurement
and new SharedRuntime.HunterPortable are affected; marker vocabulary, recursion,
workers, profiles/status/nesting and cloud guards remain host-owned.

Affected constraints: current scan sizes/symlink_scope/terminal/scan_lifetime and
Hunter design generic_facts/no_product_dependency/io_contract/lifecycle. Scope is
partial observed extraction with draft upstream contract, not complete acceptance
of hunter.design.scan. C2/C3/C4 default policy/path differences are unchanged; C5
whole-process memory remains unbounded/unmeasured by this move.

UPSTREAM.json and fresh receipts identify canonical source and runtime vendored
copy. Previous source/map/test receipts remain historical. Map/spec validation
is separate from regression tests and performance comparison; no speedup inferred.
Next: disentangle recursive scheduling/measurement from product prefetch/status
before extracting worker orchestration.

## Vendored input inventory review — 2026-10-14

BuildHunter #71 verifier now checks the full recursive inventory, allowing only
the pinned four files plus UPSTREAM.json and their src directory. Extra build.rs,
nested Rust/example files, symlinks and non-regular entries are rejected before
source hash checks. New tests first reproduced acceptance of the extra inputs.
No runtime scanner or upstream Hunter code changes; current scan sizes/lifetime
contracts and map roles remain unchanged. Fresh verifier/documentation identities
and Python/structural receipts: specs/validation/hunter-vendor-review-20261014.

## Scoped work coordination — 2026-10-15

Hunter #3 now owns WorkQueue<T>, exit/panic shutdown and scoped worker launch
at 836d764328212fcccb944e0f0a69ba79421b3dcd. Current HunterWork/TaskScheduling/
WorkerLifecycle and Scanner.Traversal change ownership bindings. Root retains
ListTask/product recursion, worker task bodies, cloud/QoS, results/profiles/terminal.

Affected current scan scan_lifetime/memory_scope and Hunter design lifecycle/
memory_budget/no_product_dependency/io_contract retain scope: outer thread Scope
returns only after joins, generic queue is unbounded, callbacks/metadata setup remain
host-owned. close now rejects future jobs and releases pending payloads; in-flight
work still needs host cancellation. Root treats rejected enqueue as failure.
No C2/C3/C4 default-policy/path migration or C5 whole-memory acceptance.

New tests exercise worker panic before coordinator cleanup, callback unwind and
follow-up scans. Exact canonical vendoring/offline and paired Release checks are
recorded under specs/validation/hunter-work-20261015. Previous records remain
historical; no full generic filesystem recursion or speed/memory gain inferred.
Next boundary: generic directory-task/result planning detached from ancestor
artifact accounting and prefetch. Draft upstream work contract remains reviewable.

## Directory task/result planning — 2026-10-15

Hunter #4 at e9b77e44f50ac72e543b727a11afa2c28666226e owns opaque task/listing/
result types and positional measurement plans. SharedRuntime.HunterDirectoryTasks
and Scanner task/result bindings change. Local u32 context interpretation is
unchanged parent/own marker policy; all classifier/ancestor accounting stays local.

Affected current scan sizes/scan_lifetime/terminal, classification marker vocabulary
and Hunter design generic_facts/no_product_dependency/lifecycle/io_contract retain
behavioral scope. Host publishes Listed first, keeps platform guard through plan
callbacks, and sends Measured for a partial/cancelled pass. Enum does not enforce
protocol. Backend preparation still follows directory metadata even if cancelled
or directory failed. Slots retain original child positions and a truncated prefix.

New core tests and worker integration witness cover these obligations. Six-file
exact vendoring, offline and Release comparison remain validated independently.
C2/C3/C4 product migration and C5 whole-memory budget are unchanged; no new scan
speedup/full-recursion/UI/live-cloud acceptance. Fresh source/maps/receipts are
in specs/validation/hunter-tasks-20261015; prior results remain historical.
Next boundary: generic recursive selection/aggregation requests detached from
artifact ancestry and speculative prefetch. Contracts remain descriptive drafts.

## Directory task contract review — 2026-10-15

Hunter #4 removes consumer classification terms from its public scope/README.
Core snapshot now pins 612b861; all Rust sources are byte-identical to e9b77e4.
Affected roles HunterPortable, HunterWork and HunterDirectoryTasks retain their
contracts/assembly duties; no capability or runtime behavior changed. Source/map
identities, updated upstream draft and fresh structural/vendor validation live in
validation/hunter-task-review-20261015 and evidence/hunter-task-review-20261015.
Prior runtime/performance receipts remain historical; no acceptance is inferred
from a wording correction.

## Recursive child planning — 2026-10-15

ChildTraversal/DirectoryRequest now route host-admitted children through the
pinned std-only Hunter core. Product marker/classification/ancestry, cloud/stop
and profiling effects, coordinator state and terminal status remain local.
The opt-in plugin scanner and default CLI/GUI share this mechanical traversal
without switching classification policies or ABI. The remaining generic
coordinator/result bookkeeping is the next boundary; whole-scan memory budgets
remain open. See docs/hunter-child-traversal.md and the fresh source/runtime/map
receipts in specs/validation/hunter-children-20261015. Historical receipts are
retained and do not validate this revision; contracts remain draft.

## Task/result ledger — 2026-10-15

HunterTaskLedger now owns non-reused IDs, opaque listing/deferred state and
expected-response counting. BuildHunter supplies DirNode/PendingMeasure, retires
each received message before stop/dispatch, preserves ignored speculative results
and mirrors counts into existing profiling. Queue rejection/setup failure, guard
lifetime, scope join, subtree completion, C1–C4 product decisions and terminal
status remain host-owned. C5 is open: arena/map capacity and unbounded channels/
reports remain. New draft capability and current scan bindings are covered by
specs/validation/hunter-ledger-20261015 and specs/evidence/hunter-ledger-20261015;
prior receipts remain historical. See docs/hunter-task-ledger.md for protocol and
ownership duties; no full target lifecycle/resource acceptance is claimed.

## Subtree numeric progress — 2026-10-15

HunterSubtreeProgress now owns saturating byte totals, explicit leaf totals,
sticky partial flags, pending-directory/root readiness and one-shot completions.
BuildHunter retains the progress vector/ID mapping, C1–C4 membership/policy/units,
error/status classification and dependent event order. C5 remains open: no new
heap buffers are introduced by the primitive, but retained reports/arena/channel
budgets are unchanged. Draft current scan/target claims and maps are refreshed;
source/runtime/performance receipts are in specs/validation/hunter-subtrees-20261015,
upstream draft/frozen maps in specs/evidence/hunter-subtrees-20261015. Previous
receipts remain historical. See docs/hunter-subtree-progress.md for assembly duties.

## PR #76 implementation binding review — 2026-10-15

Corrected current scan implementation binding to immutable BuildHunter revision
5235f70656aca71b3e6ba7c4d75b69fd9e32836b, which contains all declared owned/border
files, including the new vendored subtree primitive. Every bound file is checked
against its Git object and the current bytes; the original recovery baseline
48badf9 remains historical. No runtime code, contract requirement, capability,
map role/profile or parent assembly obligation changes. Current/target map bytes
remain identical to the frozen hunter-subtrees-20261015 snapshots, so composition
and their runtime identity remain unchanged. Fresh binding/source/map identities
and structural receipts: validation/hunter-subtree-review-20261015. Earlier tests
and performance receipts retain their original source scope; they were not rerun
for this documentation correction. Structural success is not runtime acceptance.

## Serial scan result execution — 2026-10-15

HunterScanExecution at 3af2a4dd549f473e79364b45f248197ee4ec57ee now owns result waiting, optional
idle timeouts, checkpoints and receipt-before-stop dispatch ordering. BuildHunter
supplies ScanCoordinator for policy-specific messages and retains root admission,
guarded filesystem backends, partial result finalization, status precedence and
terminal publication after actual worker/sampler scope join. The new deterministic
profile-cancellation witness covers queued and idle cases; existing cancellation,
observer/worker panic, nested partial sizes and plugin integration remain gates.

Affected current scan result_execution/scan_lifetime/memory_scope and target Hunter
generic_facts/lifecycle/memory_budget are source-bound; constraints remain drafts.
C1–C4 product migration and C5 storage/channel/report budgets remain outstanding.
No new owned buffers are introduced by the pump, but no process RAM improvement
is inferred. Fresh identities/runtime/structural receipts are under
specs/validation/hunter-execution-20261015 and frozen maps/upstream contract under
specs/evidence/hunter-execution-20261015. Earlier snapshots/results remain historical.
Full root/backend execution assembly is still a separate extraction boundary.

## Native backend/root observation — 2026-10-15

HunterFilesystemBackend at 2d94443ae3c0614ff35d4afd98e3f9d0d01e02db now owns the native metadata/
listing seam and one-call selected-root Directory/Entry observation. BuildHunter
Probes implements the interface while retaining hooks, error decoration, counters,
cloud/access admission and platform guards. Marker-following and bulk acquisition
remain local. Root metadata reuse, missing-root Incomplete, symlink skipping and
pre-cancelled startup semantics are covered by new application witnesses. Existing
worker/callback/terminal/profile and plugin integration remain regression gates.

Affected current scan root_backend/symlink_scope/cloud_guard and target Hunter
generic_facts/io_contract/lifecycle retain draft status. Maps/bindings now identify
the seam and native race limits; full root/backend execution assembly, descriptor-
bound traversal, C1–C4 migration and C5 budgets are still pending. No speedup, RAM
improvement or live-cloud acceptance is inferred. Fresh identities/runtime/structural
receipts are in specs/validation/hunter-backend-20261015, frozen maps/upstream
contract in specs/evidence/hunter-backend-20261015. Historical receipts are retained.

## Directory worker protocol — 2026-10-18

HunterDirectoryWorker at a1b30a7967ed40f146bf4540af7fcdda4e3809b4 owns queue-pop, host listing/prefetch callbacks,
plan construction, Listed-before-measurement publication and Measured delivery.
A rejected result stops this worker before its next effect/task; it does not close
the shared queue. BuildHunter WorkerAdapter retains marker/prefetch interpretation,
portable/bulk acquisition, fallback/error decoration and timers. Its materialization
guard lives outside the complete loop; SetupFailed, scope shutdown/join, receiver
drop and Finished boundaries retain the previous assembly. Seven upstream tests
cover the protocol, including borrowed non-Send state. The new application test
checks receiver loss, no metadata/bulk work, preserved next task and restored I/O
policy; existing publication/cancellation, worker panic, bulk and profile tests remain gates.

Affected current scan directory_worker/scan_lifetime/cloud_guard and target Hunter
generic_facts/io_contract/lifecycle remain draft. Current Scanner/Traversal and
HunterDirectoryWorker roles are updated for CLI and GUI; target map remains a
partial extraction. C1–C4 default plugin migration, C5 budgets, root/backend setup
and descriptor-bound recursion remain pending. Source/maps/runtime/structural
receipts: specs/validation/hunter-worker-20261018; frozen maps/upstream draft:
specs/evidence/hunter-worker-20261018. Earlier receipts remain historical. No
new speedup, RAM budget or native UI/home/live-provider acceptance is inferred.

Next boundary: separate policy-free directory acquisition from listing marker
collection, then assess startup/backend assembly using the same guarded adapter.

## Directory acquisition and marker observer — 2026-10-18

HunterDirectoryListing at 29bc8b0f60d0d192fe8e2529216f50ce4c9854c6 now owns collection of host-opened entries/errors
and the visit outcome. Its synchronous entry observer receives opaque mutable
context before each child is retained. BuildHunter observe_legacy_marker keeps
regular-marker bits and intentional symlink-marker I/O/errors local and in the
same encounter order; it performs no marker probes when legacy policy is disabled.
Do not move these callbacks to a second pass: cancellation and live-tree races
would change. Post-list hooks, guarded opening, worker timing and status stay local.

Seven upstream macOS tests cover order/context/errors, cancellation, failed/empty
input, symlink observations, callback panic and supplied paths; Linux adds raw names.
The new application witness covers regular/symlink/directory/dangling marker types,
retained children/errors and disabled-policy probe suppression. Existing marker,
bulk, worker/result, profile/cancel/panic, scope and plugin tests remain gates.

Affected current scan directory_listing/symlink_scope/cloud_guard and classification
selection/catalog bindings are refreshed. Classification decisions/filters and
C1–C4 remain unchanged; marker effects stay in scan, not pure decision policy.
Current CLI/GUI maps add HunterDirectoryListing; target generic_facts/io_contract/
lifecycle are still partial and draft. Full startup, descriptor recursion, default
plugin migration and C5 budgets remain pending. No new Swift UI/home/live-provider
acceptance or speedup/RAM improvement is inferred. Fresh source/maps/commands:
specs/validation/hunter-listing-20261018; frozen maps/upstream draft:
specs/evidence/hunter-listing-20261018. Historical receipts remain historical.

Next boundary: separate guarded metadata measurement/backend preparation from the
product worker adapter before assembling a full reusable scan startup/lifecycle.

## Typed measurement backend — 2026-10-18

HunterMeasurementBackend at 1eb300054c24bbf56a936287fbd586c4fb39079c provides typed directory observation/preparation
and owned prepared file state. The existing closure and new typed plan APIs share
one original-position slot loop. BuildHunter src/measurement.rs owns MetadataBackend
and PreparedFiles: native/bulk acquisition, injected probe errors, entry consumption,
Unsupported/missing fallback, counters and error decoration. WorkerAdapter now only
times/delegates measurement. The thread materialization guard remains outside the
complete worker loop and prepared state drops before result publication or on unwind.

Five upstream tests cover ordering/slots/ID/errors, borrowed non-Send state,
pre-cancelled directory error/preparation, cancellation, empty/omitted slots and
prepared-state drop including panic. Two host witnesses cover directory error plus
pre-cancellation and synthetic batch consumption/fallback/error provenance under
the active guard. Existing cloud, bulk, marker, profile/cancel/panic, scope and plugin
tests remain gates. No new local UI/home/live-provider acceptance is claimed.

Current scan measurement_backend/sizes/cloud_guard/lifetime and target Hunter
io_contract/lifecycle claims remain draft. Classification, bridge, application and
plugin contracts are unchanged: no marker, filter, FFI or client semantics moved.
Their prior implementation-specific evidence remains scoped to unchanged claims;
current scan binding adds both measurement modules at the immutable runtime commit.
CLI/GUI maps show host native effects and generic Hunter mechanics separately; target
ownership is still partial. Full startup/guard assembly, descriptor recursion,
default plugin migration and C5 budgets remain pending. Fresh sources/maps/commands:
specs/validation/hunter-measurement-20261018; frozen maps/upstream draft:
specs/evidence/hunter-measurement-20261018. Prior receipts stay historical; no speedup
or memory-budget improvement is inferred.

Next boundary: consolidate root admission and scoped worker startup behind an
explicit host-effects interface, preserving SetupFailed/Finished and join ordering.

## Scoped startup and terminal boundary — 2026-10-18

HunterScopedSession at 31fd0bad749cdac63bc42f36ffe72bcd81b0c1ad assembles helper startup, worker launch, synchronous
caller-thread coordination, helper-stop guard drop, queue closure and actual scope
join. BuildHunter src/scan_session.rs now owns admission/profile/QoS/channel setup,
coordinator construction, status precedence and event/report publication. The root
materialization guard remains active through join/final publication; worker setup
failure stays an opaque SetupFailed response interpreted by the existing pump.
Receiver is moved into the coordinator and unread replies release before join,
including unwind; exact relative capture-destructor ordering is not promised.

Five upstream witnesses cover helper stop before queue close, worker/helper join,
caller-thread non-Send results, opaque setup failure, receiver release on coordinator/
startup unwind and worker-panic peer closure. Two product tests cover the failed-
admission handler (Warning then Finished(Failed), no work) and final profile drain
before one caller-thread Finished. These do not inject an actual OS guard failure.
Existing callback/worker panic, SetupFailed routing, profile/cancel, native/bulk,
marker and plugin tests remain gates. Panic unwinds rather than emitting product
Finished; arbitrary user callbacks must still terminate for scope join to finish.

Affected current scan scoped_session/scan_lifetime/result_execution/terminal/cloud_guard
and target lifecycle claims remain draft. Current CLI/GUI maps distinguish product
ScanSession from generic HunterScopedSession. Classification, filters, bridge ABI,
application and plugin contracts stay unchanged; root source entrypoint rename is
private. Full generic root admission/guard assembly, descriptor recursion, default
plugin migration and C5 budgets are still pending. Fresh source/map/command receipts:
specs/validation/hunter-session-20261018; frozen upstream/maps:
specs/evidence/hunter-session-20261018. Historical receipts remain historical;
no speedup, RAM bound or native UI/home/live-provider acceptance is inferred.

Next boundary: introduce a generic root-admission outcome so callers can decide
whether to schedule a directory, measure an entry or return an owned setup error
without encoding BuildHunter status vocabulary into Hunter.

## Root admission outcome — 2026-10-18

HunterRootAdmission at cc1d8054ea0882e8f954e8bb6b0e6de03cff1495 adds `admit_root` and `RootAdmission`:
opaque host preflight once on the original path, then one backend observation only
if allowed. `Skipped(reason)` has no backend observation; `Observed` owns original
Directory/Entry metadata; `Unavailable` owns the unchanged error. Callback effects
remain independent. No new stop check, path normalization, enumeration or retry.
BuildHunter still owns cloud policy/guards, reasons/warnings, classification,
scheduling and terminal status. Skip warns and returns before the pump; metadata
error warns and follows the existing pump/finalization path. Startup guard failure
continues to use the separate product session Warning/Finished(Failed) path.

Four upstream seam tests cover borrowed non-Send reasons/backend, ordering and
single metadata use, original error identity and callback panic with no backend I/O.
Existing host root tests cover pre-cancelled files, missing roots, final symlinks
and synthetic macOS cloud paths. They are not live-provider validation. Current
root_admission/root_backend/cloud_guard and target lifecycle remain draft; marker,
filter, bridge, GUI and plugin contracts are unchanged. Frozen maps/upstream draft:
specs/evidence/hunter-admission-20261018; fresh commands/source identities:
specs/validation/hunter-admission-20261018. Historical evidence stays historical.

Full guard/backend assembly, descriptor recursion, default plugin migration and C5
budgets remain pending. This slice makes no speedup, RAM-bound or UI/home acceptance
claim. Next boundary: define generic backend/guard setup lifetime without moving
product cloud policy or status vocabulary into Hunter.

## Stack-aware CI budget — 2026-10-18

Separate BuildValidation/CI map and draft buildhunter.delivery.ci recover planning,
per-layer cheap checks, ready-tip macOS checks, plan receipts, trusted publication,
review-aware preflight and additive ruleset proposal. Application current/target
runtime maps and scan/classification/bridge/plugin contracts are unchanged: CI is
an out-of-runtime delivery boundary, not an additional scanner component.

The source default remains opt-in. Repository activation was applied on 2026-10-18:
main requires the Actions-produced Stack CI gate with strict up-to-date checks,
and STACK_MACOS_CI=true. Bootstrap publisher 39162179998 ran from main and left the
old #84 evidence pending. Current activation/provenance is recorded separately in
specs/validation/stack-ci-rollout-20261018. Native canary #90 was subsequently verified and merged at 1a09c34; its timed
receipts are in specs/validation/stack-ci-accepted-20261019. Measured runner-minute
savings remain unclaimed; local fixtures are not hosted acceptance.
Status checks cannot prohibit intentional prefix selection; use fresh tip-only
preflight and native stack merge. Hard server-side merge-target restriction needs
separate bot authorization. Runtime source evidence remains historical/current for
its unchanged claim scope; new CI identities/commands are recorded separately in
specs/validation/stack-ci-20261018. No user Xcode scheme changes are included.

### Stack CI review fixes — 2026-10-18

The delivery evidence verifier accepts both exact and ref-qualified workflow paths
while still requiring the exact workflow filename, event, head and linked PR.
Regression fixtures cover branch refs, pull-request refs and a wrong ref-qualified
workflow. AGENTS now explicitly requires every layer to be ready, including on
synchronize events. Draft buildhunter.delivery.ci scheduling/evidence requirements
are unchanged; source binding is refreshed. CI maps and all runtime contracts/maps
are unchanged because no roles, edges or runtime behavior changed. Earlier receipts
remain historical; fresh review validation is in stack-ci-review-20261018.

### Draft readiness canary — 2026-10-18

Add an offline three-layer regression: a draft at any depth holds macOS work for
the entire batch while cheap checks remain applicable; clearing that draft enables
only the final tip. The draft buildhunter.delivery.ci scheduling requirement is
unchanged, and its source binding is refreshed. CI/current/target runtime maps and
all scanner/plugin/bridge contracts remain unchanged: no component or role changed.
Fresh local validation is in specs/validation/stack-ci-canary-20261018; hosted rollout
observations are recorded separately rather than inferred from these fixtures.

### Hosted activation and review provenance — 2026-10-18

Current CI map properties and the claim matrix now describe the active required
Actions-bound gate, strict base checks and true repository variable. Runtime maps
and scanner/plugin/bridge contracts remain unchanged. The draft delivery contract
retains its requirements; provenance distinguishes deployed settings, the first
canary head's green CI, review-blocked preflight and pending revised-tip acceptance.
Frozen active maps, timestamped acquisitions, tool identities, server mutation order
and bootstrap pending diagnostics are in stack-ci-rollout-20261018. Historical
pre-activation evidence is retained separately, not promoted to current proof.

## Guarded backend lifetime — 2026-10-18

HunterBackendLifetime at 3a3db1e270ff54ba757027df3a55b9d378ee0525 now owns with_guarded_backend and native
NonMaterializingIoGuard. Enter guard, prepare backend, run synchronous body, drop
owned backend, then restore/drop guard on the same thread. Original Guard/Backend
setup errors remain distinct. Guard restoration is best effort; abort/double panic
and unused callback-capture destructor ordering are outside the cleanup contract.
Borrowed external backend state remains externally owned. Host must not extract
guard-dependent resources and must finish scoped work inside the body.

BuildHunter coordinator includes worker/sampler join and final Profile/Finished in
the lifetime body. Workers prepare an owned WorkerAdapter and keep prepared metadata
state use/drop inside it. Public NonMaterializingIoGuard remains a compatibility
alias; cloud path policy, dataless error decoration, QoS/status/terminal choices stay
local. Actual backend factories are infallible; generic backend failure is tested
upstream, not asserted as an existing product failure path. Independent plugin
scanner gets the same alias; its separately pinned descriptor helper is unchanged.

Five generic witnesses cover ordering, setup failures, same-thread non-Send use,
factory/body/destructor panic and join-before-release. One native Darwin witness
covers nested policy restoration/unwind. Two host witnesses cover injected worker
guard failure -> SetupFailed without I/O and backend panic -> same-thread guard
release. Existing actual-macOS probe/cancel/callback-panic tests remain gates.
These are not live provider or UI/home acceptance, speedup or RAM-bound evidence.
Current backend_lifetime/cloud_guard/scoped_session/scan_lifetime and target lifecycle
claims remain draft; classification, filter, bridge ABI, GUI and plugin rules stay
unchanged. New current map also repairs the stale HunterChildTraversal selector
(previously split into an unmatched HunterChildScanSession and duplicate Traversal).
This source-binding correction does not change runtime behavior or historical maps.

Fresh source/maps/commands: specs/validation/hunter-lifetime-20261018; frozen upstream
and maps: specs/evidence/hunter-lifetime-20261018. Prior evidence remains historical.
Full backend algorithm assembly, descriptor recursion, default plugin migration and
C5 resource budgets remain pending. Next boundary: define reusable scan assembly
that composes root admission, guarded backend scope and scoped workers without
moving product classification/status or expanding the existing no-follow scope.

### Lifetime PR rebase onto activated CI — 2026-10-19

Rebased #84 onto main 1a09c34071c2a9e0a6dc5615bc99ee2d664f2531 after native stack
#90 merged. Combined the lifetime and CI progress/claim records, and advanced the
package draft version without dropping either contract. Runtime, Cargo and vendor
inputs are byte-identical to the pre-rebase #84 head 45b6f8b; Hunter remains pinned
to 3a3db1e270ff54ba757027df3a55b9d378ee0525 (upstream #15 is still open). The scan
implementation binding identifies the rebased immutable runtime/pin commit.

Current/target runtime maps and delivery CI maps retain their roles and properties;
no runtime behavior or accepted requirements changed. Historical lifetime benchmark
receipts remain historical. Fresh rebase input comparisons, structural/vendor checks
and local tests are in specs/validation/hunter-lifetime-rebase-20261019; fresh hosted
validation must bind the new head/base/budget mode. No fresh local UI/home or provider
acceptance and no new speedup, RAM-bound or billed-cost claim follows from the rebase.

### Guarded scan assembly — 2026-10-19

Hunter `9f445cf8468e2f341d8e8e20782854fb391d5979` adds the draft
hunter.fs.scan_assembly boundary. `with_scan_session` composes guarded backend
lifetime with the empty owned queue and capped parallelism. Its non-cloneable
ScanSession token is consumed once for helper/worker startup, caller coordinator,
stop/close and join. Product finalization remains inside the outer guarded body.
Root `admit_root` remains inside Scanner::run rather than being moved before worker
startup: this preserves existing event/cancellation and SetupFailed timing.

Affected claims: current scan session_assembly/scoped_session/scan_lifetime/terminal,
target Hunter ScanLifecycle and BuildHunter ScanAssembly. Classification, filter,
bridge ABI, GUI behavior, root no-follow scope and admission policy are unchanged;
their prior receipts remain historical, not fresh end-to-end acceptance. Four
upstream witnesses plus a compile-fail single-use check cover integrated ordering,
setup rejection, unwind/channel release and unused token/invalid cap cleanup.
Fresh identities/commands are in specs/validation/hunter-assembly-20261019.

This remains bounded session assembly. Backend algorithms, descriptor recursion,
default plugin migration, complete generic scanner, C5 resource budgets and live
home/cloud/UI acceptance remain outstanding. Next boundary: define the generic
coordinator adapter for root scheduling and task/result accounting, keeping product
classification and status/event translation in BuildHunter.

### Owned traversal coordinator — 2026-10-19

Hunter 10268f69c41890bb23ab4cdc33d25f3aac8e46d1 now owns TraversalCoordinator and
TraversalTasks: original root admission, private ledger, reserve/telemetry/publish,
receipt accounting and serial result pump. A consuming run/drive returns loop outcome
and unanswered count, separately from worker join or product status. initialize=false
skips pump/finish; ordinary pump return calls product finish before releasing ledger.
The ledger now releases at coordinator return, before the caller drops unread receiver;
receiver still drops before scoped worker join. Queue lifetime remains ScanSession-owned.

The former private Scanner is ArtifactHandler implementing TraversalHandler. It no
longer owns queue/ledger. Marker vocabulary, classification, ancestry, child admission,
selected-subtree membership, artifact progress/report and error/status translation
remain local. This is an observed intermediate boundary, not the complete proposed
ScanEngine/measurement-tracker API. Current traversal_coordinator/result_execution/
root_admission/scoped_session/scan_lifetime claims and target lifecycle binding change;
classification/filter, bytes/no-follow policy, bridge ABI, GUI and plugin rules do not.

Seven generic witnesses cover dynamic scheduling/measurement retirement, borrowed
skip reason without observation, original root error, receipt/timeout cancellation,
disconnection counts, rejected publication and panic payload release. Existing host
scripted ignored-measurement/profile-cancel and real guard/callback/worker tests stay
regression gates. Fresh validation: specs/validation/hunter-coordinator-20261019.
Historical receipts remain historical. C5 budgets, descriptor recursion, default
plugin migration, live home/cloud/UI acceptance and complete generic engine remain
outstanding. Next extraction: generic child-routing/measurement mechanics under the
same host selection contract; keep domain interpretation and accumulated result local.

### Child routing and selected metadata reconciliation — 2026-10-19

Hunter 2031df50f8324eaaf91856c5dc382d35694b2fd4 now owns ChildTraversal::route and
SelectedMeasurementPlan::resolve. ArtifactChildren retains access/checkpoint/stop,
classification and directory scheduling; ArtifactMeasurements retains byte aggregation,
affected ancestry, profile cadence and finish_directory. FileSlot is an alias of
SelectedEntry<Option<u64>>; the opaque context is the product selection ID. Pending
state carries the common selected plan and product chain/parent length separately.

The consuming child pass keeps original indices/type observations/context clones,
collects only selected payloads and releases listing/suffix before measurement.
Selected resolution preserves root > worker > fallback precedence, original cache
slot consumption and cached errors without retry. Missing/out-of-range/already-taken
slots fall back once, with selected-entry ordinal checkpoints before lookup/I/O.
No extra cancellation check is added. Host finish precedes owned selected/cached
buffer release; panic skips finish and unwinds. Metadata paths/guard scope and byte
units/deduplication remain unchanged.

Affected current entry_processing/sizes/scan_lifetime and target traversal/measurement
bindings remain draft. Marker/filter/catalog, no-follow scope, result/FFI/GUI/status
contracts are unchanged; historical receipts are not relabeled. Seven new upstream
witnesses cover routing stop/empty/panic, metadata precedence/errors/fallback, empty
selection and measurement panic/payload release. Fresh source/maps/command identities:
specs/validation/hunter-processing-20261019. Full engine facade/host-opaque branch
state composition, descriptor recursion, default plugin migration, C5 budgets and
live home/cloud/UI acceptance remain pending. Next boundary: compose the extracted
coordinator, routing and measurement contracts behind a reusable scan entrypoint.

### Composed serial ScanEngine — 2026-10-19

Hunter 920a51afb85efdb21951d61d1ef3894148017cbc composes the coordinator, child routing
and selected metadata resolver behind consuming ScanEngine run/drive. ScanHandler
is the single product host contract. Hunter owns typed result dispatch, listing-state
acquisition/speculative reservation, child callbacks and empty/immediate/deferred/
ignored measurement branches. PendingSelection retains the selected plan and opaque
ArtifactBranch (ancestry/parent length); product adapters and PendingMeasure are removed.

ArtifactHandler retains cloud/access/cancel decisions, marker/classification policy,
directory scheduling meaning, byte units/aggregation/profiles, warning/status mapping
and partial completion. prepare_directory returns Abandon, Complete or Traverse.
Response receipt/stop ordering and listing telemetry are preserved. Empty/pruned work
still consumes ignored speculative responses; cached errors are never retried. No extra
stop check, enumeration, metadata I/O or thread is added. Finish callbacks precede
buffer/branch release; partial finish precedes ledger release. Drained is a loop result,
not product success: zero outstanding work can win after a host control failure.

Affected current scan_engine/entry_processing/traversal_coordinator/result_execution/
root_admission/scan_lifetime and target composition claims remain draft. Runtime
witnesses include eight engine integration cases, borrowed non-Send branch state,
consuming facade compile-fail, existing host guard/profile/cancel/panic cases and paired
Release budgets. Fresh receipts: specs/validation/hunter-engine-20261019; upstream
specs/scan-engine.spec.yaml. Historical extraction records and receipts stay historical.
Default classification/filter/no-follow, C ABI/FFI, GUI/status and plugin semantics are
unchanged. This completes the serial composition facade, not descriptor recursion,
backend algorithms/factories, default plugin migration, C5 budgets or live home/cloud/UI
acceptance. Next boundary: reduce product startup wiring using the existing guarded
session and engine without moving product report/status interpretation upstream.

### Engine launch and pre-join collection — 2026-10-20

Local mains were fast-forwarded to Hunter 4e018c1ad0490342b7e69579916c093a101c50c3
and BuildHunter 65c5b2ffd38d419aaf3541bfd594882e13a0832d before this slice. Exact
upstream core snapshot 40c0f5384c70879f529c6069d835dafff316567a extends the existing
hunter.fs.scan_assembly boundary with ScanChannel, EngineLaunch and consuming
ScanSession::run_engine; no new package/capability is introduced for this composition.

The host allocates the same unbounded channel before QoS/profile setup and clones the
worker producer. The launcher starts existing helpers/workers, releases the original
producer before host construction/profile start, calls ScanEngine, releases unread
receiver and invokes collect before helper stop/queue closure/join. The product collect
returns the original report inputs and drops host-only progress before join rather
than retaining a whole ArtifactHandler across it. Factory/engine panic skips collect;
collection panic still joins before backend/guard drop. A worker panic may surface at
join after collection, so collection is not success and cannot publish terminal events.

BuildHunter retains guard placement, backend/worker/helper effects, cloud preflight,
QoS/profile capture, classification/aggregation and post-join profile/status/report/events.
The former ArtifactHandler engine-run wrapper is test-only for scripted protocol tests;
production uses the common launch path. Current session_assembly/scoped_session/scan_lifetime
and target startup bindings are refreshed. Root/child/measurement/selection policies,
C ABI/FFI, GUI, status precedence and plugin contracts are unchanged.

Four new launch witnesses cover exact ownership/order, non-Send host/result, factory/
collection panic and declined admission. Existing helper/worker/engine/profile/guard
regressions remain. Fresh source/command/performance/map receipts are in
specs/validation/hunter-engine-launch-20261020; old receipts stay historical. Contracts
remain draft. Common engine startup composition is implemented; autonomous native/batch
backend choice, descriptor recursion, default plugin migration, C5 budgets and fresh
home/cloud/UI acceptance remain outside this slice. Before another extraction, audit
remaining target obligations to distinguish reusable engine work from product policy.

### Target reconciliation and extraction closure — 2026-10-20

Compared current public main sources in BuildHunter/Hunter/Rifle with target HC roles,
the design BoundarySpecs and plugin delivery plan. The source-grounded matrix and
acceptance backlog are in docs/modular-refactor-audit-20261020.md. Common traversal,
result coordination and guarded engine launch are integrated. Concrete reusable code
still hosted in BuildHunter is Darwin batch metadata acquisition/preparation, requested
QoS and process resource snapshots. Product classification/selection semantics,
aggregation, warnings/status, profile ABI and GUI intentionally remain local.

Next proposed code boundary is the actual batch metadata backend, followed by OS
primitives. An autonomous factory is not a separate completion goal. Full descriptor
recursion, C5 budgets, default plugin migration (C1/C2/C3/C4), Swift SDK, user import
and commercial delivery are separate unfinished capabilities, not further relocation
of the serial loop. Extraction closure criteria distinguish these tracks explicitly.

Current composition map and implementation bindings are unchanged: runtime inputs
remain identical to main 7ab00f16ae437fe88427c2acbe1b651b30b044f1. Target HCS now
identifies partial backend/telemetry and missing authoring/default-plugin roles, without
changing the role tree. Hunter/Rifle design provenance is reconciled without changing
constraints or accepting draft requirements. Overview/pilot summaries are corrected;
historical records and receipts stay intact. Source identities and fresh structural
checks are in specs/validation/modular-audit-20261020: HC parse/7 profiles pass, SpecPM
has zero errors and 35 unresolved-capability warnings, vendor integrity passes.
No application builds/tests, new performance claim or home/cloud/UI acceptance.

### Darwin batch metadata and prepared size lookup — 2026-10-21

Candidate integration runtime 713196e92d02f8e0d44b7a46ce08285e62f2c618 pins Hunter
7e8243c738613f985e1568278f1a69df6c5036b7 (#22 → #23). Hunter owns actual opt-in
getattrlistbulk acquisition/checked decoder and pure PreparedDirectorySizes lookup.
The 25-file vendored snapshot is exact. BuildHunter removes its local decoder and
keeps file-slot eligibility, test hook precedence, counters, portable fallback effects
and dataless error decoration. Directory preparation/stop/guard and join/status
boundaries, default feature state, classification, C ABI and plugin semantics remain.

Current scan measurement_backend/directory_worker/root_backend bindings and
current/target metadata ownership are updated. Target constraints remain unchanged
and draft; historical audit/launch/extraction receipts keep their original scope.
Fresh command/source/map identities and paired portable/bulk Release budgets are in
specs/validation/hunter-batch-20261021. Checks do not establish a general speedup,
whole-scan RAM bound, live home/cloud/UI or descriptor-bound tree safety.
Next concrete extraction: requested QoS and OS process CPU/RSS primitives. C1–C5
default-client migration/security/resource work and SDK/commercial delivery remain
separate capabilities; the serial engine is not reopened for another wrapper.

### Process platform extraction — 2026-10-22

Requested QoS and current-process CPU/RSS acquisition now live in Hunter's core
(`thread_qos` and `process_resources`). BuildHunter re-exports these modules
privately to keep existing session/profile call sites and their ordering intact.
The local implementations are removed, not wrapped in a second OS layer.

The host still captures QoS during coordinator setup, applies it on each worker
and optional sampler, establishes the CPU baseline and owns sampling cadence,
resource deltas, whole-process attribution, summaries and profile/FFI/UI semantics.
Requested QoS is not effective priority; apply is best-effort and has no restore
behavior. CPU/RSS availability remains independent; no new budget or performance
claim follows. Historical extraction evidence and outstanding C1–C5 requirements
are unchanged. Fresh source hashes and checks: `specs/validation/hunter-process-20261022`.

### Process-platform Xcode build-input review — 2026-10-22

PR #99 review found a stale `src/process_resources.rs` build-phase input.
Updated both checked-in project and XcodeGen configuration to the vendored path,
and declared vendored `thread_qos.rs` alongside it. All eight input files exist
and generator/project declarations match; plist syntax passes. Runtime/map/spec
contracts (`buildhunter.current.scan/process_platform_assembly`) are unchanged;
existing Rust execution receipts remain scoped to unchanged runtime sources.
New input identities/checks: `validation/hunter-process-20261022/review-build-inputs`.
Hosted macOS checks must rerun on the fix; earlier green CI is not its proof.

### Extraction closure audit — 2026-10-22

Rechecked source owners and exact public main/pin identities after #99. The
current HC trees, source bindings and `hunter.design.scan` / `buildhunter.current.scan`
constraints already reflect batch/prepared/process primitives; no requirement,
acceptance state or runtime behavior changes. Remaining tasks are R1–R8 in
`docs/modular-refactor-audit-20261022.md`; legacy dated notes are historical.
Fresh structural/source evidence: `validation/modular-audit-20261022`.

### Pilot dependency alignment (R1) — 2026-10-22

Align the independent scanner facade pin with the application's exact Hunter
snapshot. Workspace-root patch shares its transitive core with the vendored
application package; policy/default workspaces retain their separate dependency
sets and Rifle pin. CI verifies locked active host graphs and rejects duplicate
or unexpected core sources. Scoped marker implementation bytes are unchanged.
`buildhunter.plugin.scanner/dependency_identity` now describes this repository's
assembly boundary, including external-root patch limitations. Other C1–C6 and
design claims are unchanged; requirements stay draft. Current map pin/property
and fresh source/test/graph evidence: `validation/plugin-alignment-20261022`.

### Migration baseline fixtures (R2) — 2026-10-22

Freeze current profiles in `tests/fixtures/migration-parity-v1.json`, consumed by
real CLI and GUI bridge tests; deterministic lifecycle TSV covers core fault
status/partial/Finished sequencing. `docs/migration-parity-baseline.md` records
C2 differences and C3/C4 migration gates without accepting a behavior change.
Affected claims: current classification selection/client_profiles/taxonomy,
current scan sizes/terminal/lifetime and bridge invalid_utf8/cancellation.
Runtime policy/ABI and requirement acceptance stay unchanged; the new current-map
property binds test evidence only. Fresh source/run identities are recorded under
`validation/migration-parity-20261022`. Baseline coverage does not close R3/R4 or
owner decisions on marker links and lossless action paths.

### R3 scan-scoped acquisition seam — 2026-10-23

Hunter facade/core are aligned to `bfcd75f8996b5a958c81238bb6d44d9fa1f88047`.
`scan_with_candidate_access` routes admission, listings, worker preparation and
coordinator fallback through one borrowed backend until join. Failed acquisition
is authoritative; prepared handles release before terminal publication. Ordinary
CLI/GUI acquisition/selection and the numeric FFI remain unchanged. EntryType
replaces opaque native file types in generic routing, preserving its four decisions.

Affected roles: Scanner/Measurement and HunterFilesystemBackend/ScanScopedAccess,
HunterDirectoryListing, HunterEntryProcessing and existing route/task envelopes.
The optional interface is folded into `buildhunter.current.scan`; the concrete
macOS/Linux ownership boundary belongs to Hunter's new scoped-filesystem spec.
The plugin package pin is aligned but does not select that backend in this layer.
Current maps/bindings are refreshed; target requirements stay draft and unchanged.
Receipts: `validation/hunter-scoped-access-20261023`. Std-backed spy tests prove
routing/lifetime and fail-closed fallback, not descriptor safety. R3 integration,
Windows support, C3/C4 owner decisions, R4 and R5 remain separate.

### R3 scoped diagnostic boundary review — 2026-10-23

Review of #106 found that legacy error decoration performed a new pathname stat
after a scoped failure. Probes now passes an explicit ErrorScope through every
root/listing/prepared/fallback error decoration path. Scoped errors preserve their
original payload without metadata reads; only standard scans retain native
dataless decoration. The regression simulates unrelated dataless metadata and
checks no lookup and unchanged OS errors. Source/contract/map/run identities:
`validation/scoped-error-review-20261023`. CloudGuard/Measurement are existing
roles; no new ownership node or accepted requirement is introduced.

### R3 descriptor plugin integration — 2026-10-23

The plugin pilot now selects Hunter ScopedFilesystem for listings and every
measurement path; markers share its root handle. Supported platforms are macOS/
Linux; other platforms fail closed before events. Root replacement retains the
original directory, intermediate links are refused, and changed leaf links are
not counted. Prepared-pass and independent-pass semantics stay distinct: later
regular entries can change, so no filesystem snapshot or inode ledger is claimed.

HunterScopedFilesystem is a significant external boundary, now represented with
three acquisition roles and pinned upstream excerpts/full-file identities. The
consumer contract adds traversal_scope; code and tests are in the independent
plugin package. No default CLI/GUI/catalog/FFI behavior or target requirements
changed. Existing R2 profile differences and C3/C4 owner gates remain open.

Fresh local results/identities: `validation/plugin-scoped-scan-20261023`.
Four integration regressions cover root/intermediate/leaf replacement and a
file with unreadable contents. These supplement upstream and routing tests; they
do not establish live cloud behavior, C5 budgets, GUI acceptance or speedup.
R3 macOS/Linux integration is implemented; unsupported platform acquisition remains
explicitly pending. R4 default-client cutover and R5 budgets are separate tracks.

The existing macOS Rust CI job also runs the plugin Release suite with the bulk
feature enabled. Its nested worker fixture asserts zero pathname bulk passes and
fallbacks. This is folded into MacOSChecks; job names, topology and ready-tip
scheduling/merge gates are unchanged. CI maps/contracts are refreshed accordingly.

The plugin layer was rebased onto the scoped-error review correction. Runtime
selection/fixtures are unchanged, but error-path evidence must use the corrected
engine. Current top-level hashes and repeated plugin Debug/Release/bulk checks:
`validation/plugin-scoped-review-20261023`. Prior pre-review receipts remain
historical and do not prove the fixed diagnostic boundary.

## 2026-10-23 — R4 layer 1: client catalog preparation

Baseline main ebcdcef. ClientCatalog is a new significant boundary with a draft
`buildhunter.catalog.snapshot`; BasicProfiles/Identity/Handle are folded into it.
PluginPolicy gains conservative CandidateDispatch, folded into the policy spec.
Default CLI/GUI scan bindings and C2/C3/C4 behavior are still the R2 baseline; this
layer adds metadata/ownership APIs and public basic documents. The former opt-in
pure policy is now a host dependency, with exact Cargo source replacement retaining
fresh-cache offline delivery. Previous receipts are historical; current source and
validation identities: `validation/r4-basic-catalog-20261023`. No new acceptance,
resource bound or end-to-end speedup is inferred from pure dispatch tests.

## 2026-10-23 — R4 layer 2: catalog client scan and ABI

ClientCatalogScan is a significant boundary (`buildhunter.catalog.scan`); client
admission, dynamic marker acquisition and event transport are folded into it.
C1 is exercised through every-node catalog decisions and per-candidate requests;
C2 remains the two R2 profiles. C3 uses the explicit legacy-file-markers/1 migration
profile, preserving existing marker links/selected-root context without claiming
R3 descriptor safety. C4 transports raw paths; final GUI action safety belongs to
the next layer. Source/run identities: `validation/r4-catalog-bridge-20261023`.
This preparation does not accept new requirements or switch default clients.

## 2026-10-24 — R4 layer 3: default client catalog cutover

CLI and GUI now select ClientCatalogScan; Settings enumerate ClientCatalog targets.
CandidateSelection/MarkerVocabulary bindings describe the dynamic path and
LegacyPolicyCompatibility explicitly folds the retained numeric/SpecificationCore
APIs. Catalog decoding, typed diagnostics, target IDs, frozen schema-2 export and
lossy action guards belong to existing bridge/application boundaries. No new
accepted requirements, Hunter/Rifle dependency bump or user plugin import is implied.

C1 has end-to-end dynamic-name/probe witnesses; C2 keeps the R2 client profiles.
C3 remains explicit legacy-file-markers/1 compatibility, not R3 confinement. C4
rejects lossy path actions while retaining safe ancestors. C5 budgets remain R5.
The 200-target Release benchmark is part of existing Rust/CLI jobs; ready-tip
macOS scheduling and merge policy stay unchanged. Fresh source/tool/command hashes
and raw benchmark rounds: `validation/r4-client-cutover-20261023`. Historical
preparation receipts retain their earlier scope. Local Xcode compilation is not
GUI execution: the local test host stalled in dyld/open and was terminated after
five minutes; CI execution is still required.

### R5 bounded engine/report retention — 2026-10-24

Hunter #27/#28/#30 provide optional nonblocking count/weight admission, bounded
responses, sparse completed-slot release and bounded listing/native batch prefixes.
BuildHunter pins exact `286cebffdac7f1396a8d3dffed8a7d54b2ea34eb` and gates its
findings/progress, warnings, client target map and diagnostics. The new draft
`buildhunter.scan.resources` owns host policy; `current.scan/memory_scope` and
`catalog.scan/diagnostics` reflect explicit truncation. No classification or
no-follow policy change is intended. Fresh source/tests are recorded under
`validation/r5-engine-budgets-20261024`; R4 receipts remain historical.

Profile backlog, GUI/export disclosure and end-to-end CPU/RSS/latency measurements
are the next layer. Per-scan estimated payloads do not establish a whole-process
RSS ceiling or owner acceptance of C5.

### R5 pending profiles and consumer disclosure — 2026-10-24

Pending profile samples now retain a bounded suffix with exact omission accounting.
The 88-byte numeric ABI is unchanged; optional catalog-profile metadata carries
limits, high water and omission count. CLI JSON/profile and GUI JSON export retain
that metadata. GUI CPU/RAM series break at disclosed gaps; missing RAM intervals
are not integrated. Static Swift profile smoke and Debug/Release Xcode test builds
are distinct from full native XCTest execution.

Fresh CLI CPU/RSS/latency receipts (three runs per input and variant) are in
`validation/r5-profile-budgets-20261024` and
`docs/r5-resource-measurements-20261024.md`. Consumer/report/ring/projection and
multiple-window accounting scope is documented; no absolute RSS ceiling follows.
A GUI scan revealed popover-associated font-cache retention and two blocked native
opens. The next stack layer investigates numeric transitions; no completed home
GUI scan or whole-app C5 acceptance is claimed by these measurements.

### R5 live metric rendering workaround — 2026-10-24

A source-grounded font-cache hypothesis was tested by removing only per-tick
`numericText` from live metric/timer labels. Line/sunburst motion stays independent.
The new `current.application/live_metric_rendering` claim and HC
`ReportPresentation/LiveMetricRendering` role capture this consumer obligation.
Four-minute sampled RSS stayed below 291 MiB with decreasing per-minute maxima,
versus 650 MiB in the earlier shorter animated-label run. Heap raw retention and
local reference trees support the workaround; xctrace attach failed and is not
claimed as evidence. Fresh receipt: `validation/r5-profile-rendering-20261024`.

The bounded queue/listing/report/profile implementation and controlled stop/join
regressions are separate from this UI fix. Full home GUI completion remains
unverified because native opens did not return; do not claim whole-system
interruption, a universal RSS ceiling or owner acceptance from these measurements.
Final ready-tip CI must run the complete native test suite.

## 2026-10-24 — R5 layer 4: scoped adapter parity

The final admission audit found the independent scoped plugin adapter retained
typed diagnostics without applying the new host quota and cloned full target
metadata for each finding. Both adapters now call one `DiagnosticRetention` gate
before retained clone/publication. The experimental pilot exposes Arc target
ownership, preserving descriptor lifetime after input catalog release. These roles
are folded into ScanResources and PluginScanning; their draft contracts and source
bindings are refreshed. Default-client diagnostics/status/ABI and no-follow scope
are unchanged. Within-quota pilot diagnostic-only status remains Completed.

Count/weight-stop and shared-ownership regressions supplement existing platform
acquisition tests. The earlier three-run CLI and controlled GUI measurements remain
versioned source snapshots; a fresh final-layer CLI home receipt is separate.
Catalog/temporary allocations, global RSS and uninterruptible syscalls remain
outside this acceptance claim. Current source/structural/execution receipts:
`validation/r5-plugin-budgets-20261024`. Full hosted native XCTest remains a tip-CI
gate rather than a claim inferred from local build-for-testing. C5 owner acceptance
and the two stalled GUI native opens are not resolved by this adapter correction.

## CoreDevice/DeviceFS admission — 2026-10-25

Scope: fix entry into the standard connected-device namespace while scanning
local artifacts. The previous Release home report completed traversal but retained
12 DeviceFS I/O/indeterminate-classification warnings; the mount was independently
observed as `devicefs`. These errors are not relabelled as iCloud or disappearance.

BuildHunter's existing cloud preflight is generalized to managed-location admission,
adding only the per-user Library/Developer/CoreDevice/DeviceFS namespace and its
Data-volume spelling. Exact components and lexical dot/parent normalization avoid
skipping project lookalikes. The namespace is excluded even if unmounted. Root core
admission and recursive child admission share one policy; warning/Incomplete and
ancestor-partial semantics remain explicit. Hunter and the strict plugin pilot's
separate descriptor setup/marker contract are unchanged. CLI canonicalization and
GUI folder-picker effects remain outside the core's no-probe preflight guarantee.

Affected draft claims: current.scan managed_locations/root_admission/root_backend;
StartupAdmission and Traversal fold the policy without a new component or package.
Catalog CLI/GUI inherit it through current.scan. Existing classification decisions,
bridge/event ABI, cloud thread guards, non-macOS runtime and target architecture
retain their contracts. No draft requirement is promoted to owner acceptance.

Fresh source/map hashes, structural diagnostics and actual checks are recorded in
[devicefs-exclusion-20261025](validation/devicefs-exclusion-20261025/README.md).
Historical home/RSS/Photos receipts identify the previous binary; this change has
focused native Release CLI evidence, not a fresh home/GUI/RSS or Store acceptance.

## R7 minimal file-backed templates — 2026-10-25

Owner scope is fixed in `docs/user-templates.md`: free authoring/use in both
editions, ordinary JSON source files suitable for the user's own Git repository,
minimal native editor/validation, explicit save/reload and no watcher/IDE/Git client.
`buildhunter.current.templates` is a new significant boundary for registry, draft
and file effects. UI details are folded into it; no separate spec per View.
Current CatalogSnapshot/bridge/application drafts now describe external-origin
admission, package versions, no partial fallback and selected template writes.
Existing C1–C5 scan semantics, Hunter/Rifle modules and queue/target scope lifetimes
are unchanged. C6 explicitly includes template Save/Save As as user-requested
writes, distinct from read-only traversal.

Requirements acceptance does not promote recovered BoundarySpecs or Store approval.
Current source/map identities, native checks, test outcomes and XCTest limits:
`validation/user-templates-20261025`. CLI file import, SDK and commercial delivery
remain separate work. Historical receipts above retain their original scope.

## First-release file exchange clarification — 2026-10-26 (Moscow)

The owner explicitly requires the simple GUI editor plus file import/export for
the first free release. Import retains external source references; Export JSON
writes a validated copy without activating it or clearing the dirty draft. Save As
now switches one registration (preserving enabled state and old source data),
rather than enabling a duplicate package. Same-source Save As keeps the external
edit check; another registered destination is rejected before writing.

The new behavior is folded into existing TemplateManagement/ScopedTemplateFiles;
no new significant boundary or target module is introduced. Affected contracts:
`buildhunter.current.templates/file_exchange,scoped_writes` and application
`effects_scope`. Snapshot, bridge, classification and C1–C5 contracts are unchanged.
Fresh source/map identities and checks: `validation/template-export-20261026`.
Previous PR #120 checks passed for 89065ce; that CI result is historical for this
follow-up. Swift SDK remains separate and does not gate the first editor release.

## Rule documentation and profiling — 2026-10-26

Owner asked for author documentation and per-rule metrics. New draft boundary
`buildhunter.rules.profile` covers PluginPolicy.RuleProfiling, client
RuleProfileCollection and RuleProfilePresentation. It composes pinned Rifle
`af35dedf46d8c9978af1e30985f9c157a3f266c2` timed outcomes with host phases/identity,
bounded counters, optional event 8 and JSON export. Ordinary classification and
filesystem/marker policy requirements are unchanged; equivalence tests exercise
outcomes/bytes/status/probes. Hunter and target topology have no change.
Current source/map identities and actual checks: validation/rule-profiling-20261026.
Prior receipts remain historical. New recovered specs stay draft; source, native
UI, local test-host limitations and GitHub CI are reported separately. Timing
scope excludes host I/O and is not a speedup or CPU/RAM attribution claim.

## Local XCTest identity and smoke verification — 2026-10-26

The local retry exposed a test-input error: Select All followed by typing into
the text-view element had inserted `{` into valid JSON rather than replacing it.
The test now invokes native Select All, types into the current responder and
asserts the replacement before checking validation. A separate real-folder test
scopes its Open Folder query to one empty window to avoid ambiguous multiwindow
matches. No application validation, window lifecycle or scan policy is changed.

macOS TCC logs identify repeated removable-volume prompts for the generated
ad hoc UI runner after a code-hash requirement mismatch. The UI test target now
uses the app's existing development team/signing style; CI's explicit unsigned
override remains supported. Local test execution uses the normal runtime TMPDIR,
while build outputs, caches and result bundles remain on verified TB5. Permission
approvals, TCC resets and Full Disk Access are not automated.

Affected roles/claims: MacApplication presentation, TemplateManagement and
`buildhunter.current.templates/draft_validation`; test-host setup is folded into
application assembly, not a new runtime component or capability. Current/target
HC topology, template contracts and runtime bindings remain unchanged. Fresh
source/map identities, signatures and actual checks are recorded in
[test-approval-20261026](validation/test-approval-20261026/README.md). Historical
timeouts retain their original status; these results do not prove all UI flows
or permission persistence for every future signing identity/macOS version.
