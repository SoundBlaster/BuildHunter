# Architecture maps

- [current.hc](current.hc) / [current.hcs](current.hcs): observed composition at
  implementation baseline `96780d7f9a9a9e5a6a19a53e2eea2f4f4ff1a2aa` plus
  independent PluginPolicy, opt-in PluginScanning and pinned Hunter metadata; profiles `client=cli` and `client=gui`.
- [target.hc](target.hc) / [target.hcs](target.hcs): intended module composition,
  profiles `edition=public` and `edition=store`. Rifle implements pure programs,
  generic catalog validation and identity at revision `693a284`; its Swift SDK
  remains pending. Hunter extraction is partial (scoped metadata); full traversal/sizing remains proposed.
- [Module design](../../docs/modular-architecture.md), [progress](../specification-progress.md),
  [claims and assembly review](../claim-matrix.md), [package](../../specpm.yaml).

One root per map. The current map describes shared runtime once and keeps actual
independent CLI/macOS clients distinct. `ClientPolicy` describes profile differences,
not two falsely identical classifiers. The target map describes four approved
module names; StoreDistribution is the composition role owned by the approved private
`BuildHunterMAS` repository. The repository exists; its build/release pipeline
is pending. Contexts describe selected profiles; the application
does not read these files or infer settings from them.

Composition is not execution order, dataflow, imports or a call graph. Dependencies
and parent lifecycle obligations are explicit in the Markdown/BoundarySpecs.
Current and target trees are deliberately separate: a structural diff between
them would not prove that implementation was refactored. The pure policy and scanner integration are separate opt-in Rust packages;
CLI/GUI classification and FFI are not connected to them. Unix marker access is
descriptor-bound; traversal/measurement still use the existing pathname backend.

Source bindings use real repository paths and symbols, not executable imports.
File existence and HC property validation do not prove semantic correspondence.
Exact source/map/tool identities and frozen map copies are in
`../validation/plugin-diagnostics-review-20261014/source-identity.json` and frozen maps in
`../evidence/plugin-diagnostics-review-20261014/maps/`. Review validation commands/results
are in `../validation/plugin-diagnostics-review-20261014/`. Scanner/CI
fixture receipts remain historical; the diagnostic delivery fix has fresh
source and command identities. The initial scanner
execution receipt under `plugin-scanner-20261014` remains historical; its Linux
fixture was corrected after the first hosted run. Initial recovery records under
`modules-20261013` and adapter/review records under `plugin-adapter-20261014` / `plugin-review-20261014`
remain historical, not current proof. Pure policy and CLI tests do not prove
filesystem integration, GUI behavior, ABI compatibility or performance.

With Hypercode CLI in PATH, from the repository root:

```sh
hypercode parse specs/hypercode/current.hc
hypercode parse specs/hypercode/target.hc
hypercode validate specs/hypercode/current.hc --hcs specs/hypercode/current.hcs --ctx client=cli
hypercode validate specs/hypercode/current.hc --hcs specs/hypercode/current.hcs --ctx client=gui
hypercode validate specs/hypercode/target.hc --hcs specs/hypercode/target.hcs --ctx edition=public
hypercode validate specs/hypercode/target.hc --hcs specs/hypercode/target.hcs --ctx edition=store
specpm validate . --json
```

The `.hcs` contracts validate supported property types/presence and ring capacity
range only. They do not enforce imports, absence of filesystem I/O, policy parity,
runtime ownership or the whole-process memory bound. Those are review/test duties.

Latest partial extraction: current/target maps now bind Hunter metadata at
71f71fc; fresh identities and map copies are under hunter-metadata-20261014.
Prior records above remain historical. Full Hunter scanner extraction is pending.

Parent scope review updates the current Hunter pin to 5c8f3c9. Fresh map/source
identities are in hunter-parent-review-20261014; initial extraction records remain historical.

Portable extraction adds HunterPortable roles at 8295d14 via exact vendored
source. Fresh identities and map copies are under hunter-portable-20261014.
Full recursive scanner extraction remains pending.

Generic scoped work extraction at Hunter 836d764 adds HunterWork roles. Fresh
identities/maps are under hunter-work-20261015. Host filesystem recursion/task bodies
remain local; worker launch return is not the join boundary. Prior records are historical.

Directory task/result planning at Hunter e9b77e4 adds HunterDirectoryTasks roles.
Latest source/map identities: hunter-tasks-20261015. Context interpretation,
guarded backend callbacks, recursive selection and aggregation remain host-owned.

Documentation-only upstream review advances the core snapshot to 612b861.
Latest identities/maps: hunter-task-review-20261015; runtime source and roles
are unchanged. The previous task extraction receipts remain historical.

Host-selected recursive child requests at Hunter 491399c849f9d95beae564caab32dd668be75a2c add
HunterChildTraversal/ChildAdmission/RecursiveRequests roles; current/target
map identities are under hunter-children-20261015. Product selection/aggregation
and full recursive coordinator remain host-owned. Previous receipts are historical.

Serial task/result bookkeeping at Hunter a24a94526f14819d50c928a666e847ca22eb1ae1 adds HunterTaskLedger
with TaskIdentity/ResponseAccounting/DeferredState. Current map/source identities:
hunter-ledger-20261015; host protocol, actual worker lifetime and subtree accounting
remain separate. Prior records are historical.

Subtree totals/readiness at Hunter d04c8fbbf4445e8bf3504f63f120d71fb4cc5a51 add HunterSubtreeProgress
with ByteAggregation/CompletionReadiness/PartialCompletion. Current source/map
identities: hunter-subtrees-20261015. Membership, dependent completion ordering and
actual scan lifetime remain host duties; prior receipts are historical.

Serial result execution at Hunter 3af2a4dd549f473e79364b45f248197ee4ec57ee adds HunterScanExecution
with ResultPump/IdleCheckpoint/StopBoundary. Latest map/source identities and frozen
maps are under hunter-execution-20261015. Root admission, backend effects, product
finalization/status and actual scan lifetime remain host assembly responsibilities.

Native backend/root observation at Hunter 2d94443ae3c0614ff35d4afd98e3f9d0d01e02db adds
HunterFilesystemBackend with NativePathAccess/RootInitialization. Current identities
and frozen maps are under hunter-backend-20261015. Guard/admission/marker/bulk effects
and full engine lifecycle remain host responsibilities.

Directory worker protocol at Hunter a1b30a7967ed40f146bf4540af7fcdda4e3809b4 adds
HunterDirectoryWorker with TaskExecution/ListingFirst/PublicationBoundary. Current
identities and frozen maps are under hunter-worker-20261018. Host retains marker/
prefetch and guarded metadata/bulk/timing policy; full startup remains pending.

Owned listing acquisition at Hunter 29bc8b0f60d0d192fe8e2529216f50ce4c9854c6 adds HunterDirectoryListing with
OrderedCollection/ContextObservation/AcquisitionOutcome. Current source/map
identities and frozen maps are under hunter-listing-20261018. The local marker
observer keeps product semantics and I/O; full engine startup remains pending.

Typed preparation at Hunter 1eb300054c24bbf56a936287fbd586c4fb39079c adds HunterMeasurementBackend with
DirectoryPreparation/PreparedFileEffects/SharedSlotExecution. Host Measurement now
shows NativeMeasurementBackend/BatchPreparation/MetadataFallback separately. Fresh
identities/frozen maps are under hunter-measurement-20261018; full startup is pending.

Scoped startup/join at Hunter 31fd0bad749cdac63bc42f36ffe72bcd81b0c1ad adds HunterScopedSession with
HelperStartup/QueueShutdown/JoinBoundary and product ScanSession admission/launch/
terminal roles. Fresh identities/maps are under hunter-session-20261018; root
admission/guard and full generic engine assembly remain pending.

Root admission now delegates sequencing to Hunter `cc1d8054ea0882e8f954e8bb6b0e6de03cff1495`; the host still owns
cloud policy/guards, scheduling/status and the skip-versus-error control flow.
See [root admission](../../docs/hunter-root-admission.md) and the new
`hunter-admission-20261018` evidence; full engine assembly remains pending.

Guard/backend lifetime and native Darwin policy RAII now live in Hunter `3a3db1e270ff54ba757027df3a55b9d378ee0525`.
Host scope placement, cloud paths/error decoration and product status stay local;
see [guarded backend lifetime](../../docs/hunter-backend-lifetime.md) and `hunter-lifetime-20261018` evidence.
This supersedes earlier pending guard-lifetime ownership notes for the integrated
Rust coordinator/workers; full engine/backend algorithm assembly remains pending.

Guarded queue/single-launch assembly now binds Hunter `9f445cf8468e2f341d8e8e20782854fb391d5979`.
HunterScanAssembly owns queue/count and the consumed launch token; root admission
remains inside the product coordinator. Current identities and structural/runtime
receipts are under `hunter-assembly-20261019`; full recursive engine is pending.

Owned root/task-response coordinator now binds Hunter 10268f69c41890bb23ab4cdc33d25f3aac8e46d1.
HunterTraversalCoordinator owns admission/pump/accounting; ArtifactHandler remains
product interpretation without queue/ledger fields. Fresh identities and seven-profile
validation are under hunter-coordinator-20261019; complete engine remains pending.

Child routing/selected metadata resolution now bind Hunter 2031df50f8324eaaf91856c5dc382d35694b2fd4.
HunterEntryProcessing owns mechanics; ArtifactChildren/ArtifactMeasurements retain
policy and aggregation. Latest source/command identities: hunter-processing-20261019.

Current serial composition now uses Hunter ScanEngine with one ArtifactHandler/ScanHandler.
See hunter-engine-20261019 receipts; prior stage descriptions remain historical.
