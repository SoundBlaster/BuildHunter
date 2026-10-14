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
