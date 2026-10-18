# Claim origin, consumer sufficiency and assembly review

Historical recovery baseline: `48badf9e097783505151dd47fbafe290f0af25e2`.
Current follow-up baseline: `6bab3c34684e81f14fbacd03445d297038edd9b3`;
changed inputs and validation are pinned in `validation/plugin-scanner-20261014/`.
All BoundarySpecs, including the new adapter boundary, remain `draft`. The owner accepted module names/product
direction; detailed recovered/target constraints have not been separately accepted.
Source reading, existing test source, structural validation and historical runtime
reports are different evidence levels. Historical recovery ran structural checks only; later execution receipts are
scoped to the explicitly named adapter/integration passes below.

## Recovered current boundaries

Claim references below are `BoundarySpec metadata.id / constraint id`.
Exact source file digests are in `evidence/modules-20261013/source-identity.json`.
Bindings inside BoundarySpecs declare source files, while symbols below narrow
the actual reasoning scope; a shared physical file can serve several logical roles.

| Consumer question | Draft claim(s) | Pinned source / symbols | Evidence and limits | Review item |
|---|---|---|---|---|
| When may host release access/callbacks? | `buildhunter.current.scan/scan_lifetime`; `buildhunter.current.bridge/payload_ownership`, `access_lifetime` | `src/lib.rs: scan_with_probes, run_ffi_scan, send_ffi_event`; `RustScanEventSource.swift: events, RustScanJob.finish` | Source: scoped workers, copied events and post-return teardown. Not a runtime race proof. | Parent owns control/context/scope until worker return, not just cancel request. |
| What do bytes mean? | `buildhunter.current.scan/sizes` | `src/lib.rs: metadata_bytes, measure_file, ScanReport.total_bytes` | Source: Unix allocated/apparent distinction, per-path hardlinks, nested totals. | Future Hunter must preserve units/measurement scope; no reclaimable-space promise. |
| What does scan completion mean? | `buildhunter.current.scan/terminal`; `buildhunter.current.application/generation` | `src/lib.rs: scan_with_probes, io_error`; `WindowScanModel.apply, beginScan` | Source: failure/cancel/errors/churn precedence; GUI changed results can remain completed with notices. | Do not flatten changed/partial/incomplete into one status during migration. |
| Can plugins introduce arbitrary names today? | `buildhunter.current.classification/selection`, `catalog_lifetime` | `src/lib.rs: is_policy_candidate, may_classify_directory, marker_flag, SEARCH_FILTER_CATALOG` | Source: hard-coded names/5 marker bits gate policy calls. Existing filter JSON is not plugin loading. | **C1**: replace selection and prefetch together, with catalog-derived facts. |
| Are CLI and GUI equivalent? | `buildhunter.current.classification/client_profiles`, `taxonomy`; `buildhunter.current.application/cli_profile` | `rust_cli_policy`, `src/main.rs: main`; `ClassifyArtifactRoot`, `IsOuterArtifactRoot`, `RustScanBridgeContext` | Source: named/any-name env differences, default env eligibility, nested reporting, tox/nox kinds, lossy CLI JSON. | **C2**: preserve explicit profiles first; decide future harmonization separately. |
| Are symlinks followed? | `buildhunter.current.scan/symlink_scope` | `src/lib.rs: list_directory, Probes.marker_is_file, visit_file`; Rust test `symlinked_markers_count_when_they_resolve_to_files` | Source/test expectation: traversal excludes links, marker existence resolves recognized links. Test was not run here. | **C3**: distinguish traversal guarantee from marker metadata probes; stricter behavior requires decision. |
| Is displayed path a safe exact handle? | `buildhunter.current.bridge/invalid_utf8` | `path_bytes`, `RustScanBridgeContext.handle`, `decode`, `ArtifactFolderActions`, `artifactURL` | Source: Rust raw bytes → lossy GUI String + warning → actions construct URL. Exact identity for invalid UTF-8 unproven. | **C4**: choose lossless path/handle contract or disable actions before new ABI. |
| Is memory bounded? | `buildhunter.current.scan/memory_scope`; `buildhunter.current.bridge/backpressure`; `buildhunter.current.application/view_projection` | Rust `WorkQueue`, mpsc channel, `Scanner.tasks` / `DirectoryTaskLedger`; Swift `ScanEventChannel.next/close`, `ArtifactNativeTable.Coordinator.update` | Source: ring slots bounded/released; worker/result/arena storage is separate. #61 report is historical evidence for table retention only. | **C5**: separate transport, pending work, retained reports and RSS budgets. |
| Does cancellation wait for GUI consumption? | `buildhunter.current.bridge/cancellation`, `backpressure` | `RustScanJob.cancel`, `ScanEventChannel.close`, `ScanEventStreamLifetime` | Source: close discards queued events and wakes producer; host reads can still stall return. | Do not promise an absolute cancellation latency without scenario measurement. |
| Are settings stable during scan? | `buildhunter.current.classification/filter_snapshot`; `buildhunter.current.bridge/policy_table` | `SearchFilterSettings.snapshot`, `RustScanEventSource.events`, `RustCompiledPolicyTable.compile` | Source: immutable snapshot; representability fallback. Unknown exclusions retained. | New catalog/settings must freeze together with compatible schema identity. |
| Do companion windows run new scans? | `buildhunter.current.application/shared_report` | `ScanWindowStore`, `BuildHunterApp`, `ArtifactDiagramModel` | Source: same model per scan ID; close main stops scan while retained companion report can remain. | Diagram/warnings ownership belongs to product, not Hunter/Rifle. |
| Does read-only forbid all writes? | `buildhunter.current.application/effects_scope`, `finder_handoff` | `BuildHunterWindow.exportResults`, `ScanResultsExportPanel`, `ArtifactFolderActions`; CLI `--profile` path | Source: scan/Finder/copy do not mutate the filesystem tree; explicit report/profile outputs may be inside the target, and clipboard/preferences have effects. `.opened` is API handoff, not OS completion. | **C6**: document scoped effects and completion evidence honestly. |

## Target boundaries and parent obligations

These rows are desired requirements/inferred integration duties supported by
the modular design/ADR, not claims that the new repositories or APIs exist.

| Consumer / target claims | Child contribution | Parent assembly duty | Origin / evidence / acceptance |
|---|---|---|---|
| Future clients → `hunter.design.scan/generic_facts`, `no_product_dependency` | Hunter reads supplied scope and facts, measures selected nodes, emits lifecycle events | BuildHunter supplies bounded marker/selection vocabulary and generic decisions; no Hunter→Rifle import | Owner's module split + inferred dependency isolation; design evidence; detailed contract draft |
| Catalog authors → `rifle.design.rules/pure_evaluation`, `swift_authoring`, `bounded_plan` | Rifle validates portable rules and evaluates supplied facts; SDK exports data | Product acquires facts, interprets matches and handles absence/errors; authoring never grants I/O | Owner's abstract plugin engine + ADR; IR/API still open; no runtime evidence |
| Reproducible scan → `rifle.design.rules/reproducibility`; `buildhunter.design.product/assembly_owner` | Immutable versioned plans/matches and exact scan identity | Product checks matching vocabulary/schema/catalog/filter snapshot, correlates node IDs and owns typed routing | Inferred integration duty; acceptance/tests pending |
| Swift app → `buildhunter.design.product/policy_owner`, `migration_parity` | Hunter effects, Rifle matching, SpecificationCore product policy | Product compiles appropriate policy plan; preserves bridge lifetime, profile differences, sizing/partial semantics | ADR requirement + source-derived migration duties; no new ABI implementation |
| Public builder → `buildhunter.design.product/edition`; `arsenal.design.catalog/base_and_license` | Public basic plugins and SDK work without private content | Public CI/build cannot require Arsenal credentials or silently change baseline defaults | Agreed commercial direction; exact release composition draft |
| Paid buyer → `arsenal.design.catalog/data_only`, `fixtures`, `delivered_visibility`; `buildhunter.design.product/store_delivery` | Private catalog data/tests, same public evaluator | Private composition pins compatible releases/digest and fails on missing required catalog; ships in app bundle | Agreed model + packaging duty; price, private license and Store acceptance unresolved |
| Future universal app → `buildhunter.design.product/product_scope`; `rifle.design.rules/domain_ownership` | Hunter/Rifle reusable without artifact vocabulary | Separate product supplies its taxonomy/UI and any future index/LLM; no implementation in this pass | Explicit owner scope; product name and architecture deferred |

## Scenario walk-through and unresolved decisions

- **Normal / empty:** current scan returns a report even with no findings; target
  adapter must preserve empty matches without treating them as loader failure.
- **Invalid root / inaccessible / dataless:** current guard/root/read errors emit
  status/notices; a future plugin cannot expand access or hide missing facts.
- **Excluded parent / nested:** GUI exclusion traverses eligible descendants,
  CLI may list nested findings, totals count outer roots once. Outer-root policy
  belongs to BuildHunter; Hunter retains generic measurement semantics.
- **Stop / replacement / closed window:** generation rejection, queue wake-up,
  worker completion and scope release form one parent lifecycle obligation.
- **Profile change:** CLI/GUI and compiled/fallback paths are identified separately;
  new desired rules do not retroactively establish parity of old implementations.
- **Rule/catalog/source change:** running plans remain frozen. Source changes
  invalidate affected current claims; changed map hashes invalidate structural
  evidence. An old test run or unchanged map is not fresh code conformance.

Owners for C1–C6: BuildHunter/Hunter/Rifle maintainer (product owner for behavior
decisions). Next acceptance task: choose C2/C3/C4 semantics, then schema/facts
contract and minimal Rifle pilot. C1/C5/C6 are migration obligations, not authority
to refactor or fix application code during this descriptive pass.

## Review result

Current recovery covers the four significant contract groups needed for module
extraction. Target names and dependency direction are explicit. All detailed
BoundarySpecs remain draft; Store/private implementation is absent. Deeper Charts,
layout, accessibility, telemetry sampling math and native host behavior are folded
or explicitly outside this pass, not silently considered verified. The package
is reviewable for architecture planning, not a certification of the application.

## Plugin contract proposal — 2026-10-13

[Plugin v1](../docs/plugin-contract-v1.md) concretizes the target boundary without
claiming implementation. Rifle `predicate_semantics,wire_validation` own pure
three-valued evaluation and validation. BuildHunter `plugin_adapter,match_conflicts`
own bindings, exclusions and ambiguity; Hunter `plugin_fact_acquisition` supplies
authorized facts; Arsenal `shared_plugin_contract` shares the same format.

No-follow markers, out-of-scope unknowns, lossless name matching and conflict
handling are proposed decisions, pending owner acceptance and C2–C4 migration
checks. Wire-shape tests validate schemas/examples only. The semantic scenario
table, JCS identity and performance budgets await the Rust pilot; no runtime
conformance is inferred from SpecPM or JSON Schema validation.

## Opt-in adapter evidence — 2026-10-14

`buildhunter.plugin.policy` implements the pure host-policy seam for the draft
v1 contract: closed bindings/targets, host-supplied origin, frozen exclusions,
three-valued target outcomes and budgets. In-memory fixtures validate this seam,
not C1 scanner candidate/prefetch integration or C2–C4 filesystem/client parity.
The current scanner remains unchanged in default and all-feature CLI builds; the adapter is now built independently;
private Store composition is now named BuildHunterMAS, with pipeline still pending.


## Opt-in scanner integration — 2026-10-14

`buildhunter.plugin.scanner` addresses C1 for an independent caller: its all-node
observer and active marker acquisition bypass legacy name/bit gates together.
Default CLI/GUI still use the legacy C1 selection. Full target metadata is routed
without changing the numeric ABI. Unix marker descriptors establish no-follow
lookups even when an intermediate component is replaced; this does not establish
race-free shared traversal/measurement. Windows backend is explicitly unsupported.
C2/C3 baseline-client migration and C4 GUI action/ABI identity remain outstanding.
Ambiguous/unknown policy results remain typed diagnostics; Completed still names
traversal completion. C5 report/diagnostic retention has no constant-memory claim.

### Hunter metadata extraction (2026-10-14)

C1 opt-in marker acquisition now delegates to pinned Hunter 71f71fc. C3 safety
for opt-in markers is unchanged; default legacy marker behavior and traversal
replacement races remain separate open boundaries. C5 report/whole-process
memory budgets are unchanged and not established by this extraction.
Parent obligations: BuildHunter installs materialization policy before opening
Hunter scope, holds access through worker completion and maps technical outcomes
to plugin facts. Upstream tests/source snapshots and fresh command identities
are under specs/evidence/hunter-metadata-20261014 and specs/validation/hunter-metadata-20261014.

### Portable Hunter adoption — 2026-10-14

C1/C2/C3/C4 classification/marker behavior remains host-owned. C5 memory budgets
are not established by moving entry iteration and byte conversion. Parent assembly
now supplies guarded iterators and metadata to hunter.fs.portable and consumes
owned path/error/type observations in the existing marker callback order.
Root Cargo is offline-buildable with an exact vendored std-only source snapshot;
no Rifle or libc dependency is introduced into that graph. Fresh execution/source
identities: specs/validation/hunter-portable-20261014.

### Generic scoped work adoption — 2026-10-15

C5 stays open: the task queue is unbounded; close releases pending tasks but no
whole-scan memory budget follows. Product C1–C4 planning remains host-owned.
Assembly must retain coordinator shutdown guard inside std scope, release access
after scope join, propagate worker panic and close result/sampler lifetimes.
Current API and worker/callback unwind regressions bind these obligations to
Hunter 836d764; fresh receipts: specs/validation/hunter-work-20261015.

### Directory task/result extraction — 2026-10-15

Opaque contexts do not transfer C1/C2/C3/C4 marker/classification semantics into
Hunter. C5 budgets remain open. Parent assembly publishes Listed before callbacks,
correlates IDs, retains platform/access guards and maps aligned/prefix results into
ancestor/status accounting. Source/runtime witness: specs/validation/hunter-tasks-20261015.

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
