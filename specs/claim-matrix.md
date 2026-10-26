# Claim origin, consumer sufficiency and assembly review

Latest implementation on PR branch: [Hunter batch integration](../docs/hunter-batch-metadata.md),
2026-10-21. This extends the earlier main audit; it is not yet a merged-main claim.


Latest reconciliation: [2026-10-20 modular audit](../docs/modular-refactor-audit-20261020.md),
BuildHunter main 7ab00f16ae437fe88427c2acbe1b651b30b044f1, Hunter main
860d995374d677d68b39093101da25c3cacfff9b, Rifle main
693a284123faaa1ad3192cd88b259ba97a7b5910. Earlier recovery rows and dated extraction
records below are historical snapshots. Their old next steps and source symbols
must be read with the later ownership updates, not as a current implementation inventory.

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
| Can catalogs introduce arbitrary names today? | `buildhunter.current.classification/selection`, `catalog_lifetime` | `scan_client`, `CatalogSnapshot`, `PluginCatalog::candidate_marker_requests` | R4 default clients bypass the legacy name/marker/prefetch gate; 200 synthetic target names execute end-to-end. User import is still R7. | **C1**: dynamic selection/facts integrated; fresh execution receipt under r4-client-cutover-20261023. |
| Are CLI and GUI equivalent? | `buildhunter.current.classification/client_profiles`, `taxonomy` | `plugins/basic/{cli-v1,gui-v1}`, `scan_client`, `presentation_kind` | R4 preserves the frozen R2 differences rather than harmonizing environment/nested/kind behavior. | **C2**: preserve explicit versioned profiles; future harmonization separate. |
| Are symlinks followed? | `buildhunter.catalog.scan/compatible_profiles` | `scan_client` legacy-file-markers/1; opt-in `PluginScanning` strict scoped access | Default traversal excludes link entries; compatibility marker metadata resolves links and selected-root parent markers as R2 did. Strict scoped pilot remains separate. | **C3**: preservation is explicit; future stricter default needs a deliberate profile/golden-data change. |
| Is displayed path a safe exact handle? | `buildhunter.current.bridge/invalid_utf8` | raw root bytes, `firstUnsafePathPrefix`, `ArtifactFolderLocation.actionURL`, table/diagram consumers | R4 disables Finder/Copy for lossy components and descendants; safe ancestors remain usable. Strings do not become inode/snapshot capabilities. | **C4**: disable lossy actions; source/synthetic tests are distinct from runtime acceptance. |
| Is memory bounded? | `buildhunter.current.scan/memory_scope`; `buildhunter.current.bridge/backpressure`; `buildhunter.current.application/view_projection` | Rust `WorkQueue`, mpsc channel, `Scanner.tasks` / `DirectoryTaskLedger`; Swift `ScanEventChannel.next/close`, `ArtifactNativeTable.Coordinator.update` | Source: ring slots bounded/released; worker/result/arena storage is separate. #61 report is historical evidence for table retention only. | **C5**: separate transport, pending work, retained reports and RSS budgets. |
| Does cancellation wait for GUI consumption? | `buildhunter.current.bridge/cancellation`, `backpressure` | `RustScanJob.cancel`, `ScanEventChannel.close`, `ScanEventStreamLifetime` | Source: close discards queued events and wakes producer; host reads can still stall return. | Do not promise an absolute cancellation latency without scenario measurement. |
| Are settings stable during scan? | `buildhunter.current.classification/filter_snapshot`; `buildhunter.current.bridge/policy_table` | `SearchFilterSettings.snapshot`, `CatalogSnapshot`, scan-owned catalog handle | Frozen exclusions, metadata and target IDs; unknown valid preference IDs retained. Settings enumerate plugin targets. | New report/export captures immutable catalog identity with its rows; no default Swift callback/table. |
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

## R7 user templates — 2026-10-25

| Consumer obligation | Current boundary / source | Evidence and limits |
|---|---|---|
| Source stays user-owned, free, versionable outside the app | `buildhunter.current.templates/free_files`; TemplateLibrary | References/bookmarks/enabled flags only; disconnect never deletes JSON. No Git integration or paid gate. |
| Bad input cannot silently change the effective rules | `bounded_admission`; TemplateFiles + CatalogSnapshot::with_templates | Bounded regular-file reads, External origin, full-catalog validation; enabled invalid/missing files fail preparation. |
| Draft and active scan are separate | `draft_validation`, `scan_isolation`; TemplateWorkspace / RustScanEventSource | Check does not save; a running scan retains an admitted immutable catalog. Tests and source identities in validation/user-templates-20261025. |
| Explicit write exceptions remain honest (C6) | `scoped_writes`; application/effects_scope | Coordinated comparison and atomic selected-file save; no CAS guarantee for racing uncoordinated writers. |

The current map adds TemplateManagement with registry/draft/scoped-file roles and
UserTemplateAdmission under ClientCatalog. Target module ownership is unchanged;
user-file management belongs to BuildHunter, not generic Hunter or Rifle. These
drafts do not establish App Store approval, release readiness or a crash-recovery
autosave store. R7 CLI/SDK and commercial content/purchase work remain open.

## First-release template import/export — 2026-10-26 (Moscow)

`buildhunter.current.templates/file_exchange` adds an owner-required explicit
Export JSON action. It preserves registrations, source identity and unsaved state.
`scoped_writes` now makes Save As switch only the selected registration and refuse
another registration's destination. Both validate before writing; Save As to the
current source uses Save's observed-edit protection. Application `effects_scope`
includes selected template exports. Detailed controls stay folded into the existing
TemplateManagement boundary; module ownership and scan/catalog contracts are unchanged.
Current source/test/build/native evidence: validation/template-export-20261026.
The earlier 149-test and CI receipts are not relabeled as results for new source.

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
