# Architecture maps

- [current.hc](current.hc) / [current.hcs](current.hcs): observed composition at
  `48badf9e097783505151dd47fbafe290f0af25e2`, profiles `client=cli` and `client=gui`.
- [target.hc](target.hc) / [target.hcs](target.hcs): intended module composition,
  profiles `edition=public` and `edition=store`; no extracted module is asserted.
- [Module design](../../docs/modular-architecture.md), [progress](../specification-progress.md),
  [claims and assembly review](../claim-matrix.md), [package](../../specpm.yaml).

One root per map. The current map describes shared runtime once and keeps actual
independent CLI/macOS clients distinct. `ClientPolicy` describes profile differences,
not two falsely identical classifiers. The target map describes four approved
module names; StoreDistribution is a proposed composition role, not a fifth
approved repository name. Contexts describe selected profiles; the application
does not read these files or infer settings from them.

Composition is not execution order, dataflow, imports or a call graph. Dependencies
and parent lifecycle obligations are explicit in the Markdown/BoundarySpecs.
Current and target trees are deliberately separate: a structural diff between
them would not prove that implementation was refactored. No base/head code
change comparison is claimed in this recovery-only pass.

Source bindings use real repository paths and symbols, not executable imports.
File existence and HC property validation do not prove semantic correspondence.
Exact source/map/tool identities and frozen map copies are in
`../evidence/modules-20261013/`; validation commands/results are in
`../validation/modules-20261013/`. No application test was run by mapping.

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
