# Architecture maps

- [current.hc](current.hc) / [current.hcs](current.hcs): observed composition at
  implementation baseline `998b1319fc2761a30f4f1db4df8d9bf35548f6fa` plus
  the independent pure PluginPolicy adapter; profiles `client=cli` and `client=gui`.
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
them would not prove that implementation was refactored. The adapter is a separate opt-in Rust package; CLI/GUI classification and FFI
are not connected to it.

Source bindings use real repository paths and symbols, not executable imports.
File existence and HC property validation do not prove semantic correspondence.
Exact source/map/tool identities and frozen map copies are in
`../validation/plugin-review-20261014/source-identity.json` and frozen maps in
`../evidence/plugin-review-20261014/maps/`. Review validation commands/results
are in `../validation/plugin-review-20261014/`. Initial recovery records under
`modules-20261013` and the initial adapter records under `plugin-adapter-20261014`
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
