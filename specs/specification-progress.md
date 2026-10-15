# BuildHunter — architecture and specification progress

## Goal and current scope

Consumer goal: refine module boundaries for **Hunter / Rifle / BuildHunter /
Arsenal** using current implementation evidence before extraction. Completion
level: reviewable descriptive drafts, source bindings, target ownership and
parent obligations. The historical recovery pass authorized no runtime changes or application
acceptance. Later implementation passes are separately authorized and evidenced below.

Historical recovery baseline: `48badf9e097783505151dd47fbafe290f0af25e2`.
Current follow-up baseline: `6bab3c34684e81f14fbacd03445d297038edd9b3`, with
changed inputs pinned in `validation/plugin-scanner-20261014/source-identity.json`. User's dirty
Xcode scheme is excluded from claims and preserved; its digest is recorded.
Design documents are working-tree inputs pinned separately from source revision.
Current/target map identities are independent of source identity:
[identity record](evidence/modules-20261013/source-identity.json).

## Coverage

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
