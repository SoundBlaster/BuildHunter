# Architecture maps

- [current.hc](current.hc) / [current.hcs](current.hcs): observed composition at
  implementation baseline `6bab3c34684e81f14fbacd03445d297038edd9b3` plus
  independent PluginPolicy and opt-in PluginScanning packages; profiles `client=cli` and `client=gui`.
- [target.hc](target.hc) / [target.hcs](target.hcs): intended module composition,
  profiles `edition=public` and `edition=store`. Rifle implements pure programs,
  generic catalog validation and identity at revision `693a284`; its Swift SDK
  remains pending. Hunter extraction and scanner integration remain proposed.
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
