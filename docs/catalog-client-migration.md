# Catalog client migration (R4)

Baseline: BuildHunter main `ebcdcef981e27889a6e43c524c06cff63eceea14`.

## Layer 1: catalog preparation

All 13 public basic checks are declarative plugin documents in `plugins/basic`.
`ClientProfile::CliV1` and `GuiV1` preserve C2 classification differences. Their
full catalog digests differ; exclusions preserve each digest and retain unknown
future IDs. GUI environment precedence is represented in the rules, including
when the environment target is disabled. Host environment/nested admission is
separate from pure classification.

The name index uses only necessary positive conditions. `all` can use a necessary
child constraint; `any` requires constrained alternatives; negation stays in the
fallback. Unknown names evaluate every possible target. Selected target programs
are evaluated by Rifle, with candidate-specific marker requests. A 200-target
fixture and differential Rifle tests cover dispatch, not scan throughput.

`CatalogSnapshot` and catalog C ABI 1 provide an owned immutable handle and JSON
metadata with format/profile/digest/descriptors/exclusions. Input bytes are copied;
metadata is borrowed until destruction. The owner must retain the snapshot until
all scans/readers finish. Legacy scan entrypoints and defaults are unchanged in
this layer; the GUI has not switched to the new metadata or classification yet.

The host now resolves Rifle and the Hunter facade. Cargo source replacement ships
unmodified dependencies (with their license files) so a fresh Cargo home can
check/install/run the CLI offline. Hunter/Rifle remain on their reviewed revision
pins, with one verified `hunter-fs-core`; no library version is raised.
Xcode invokes Cargo on every build so Cargo can track nested Rust/plugin inputs.
Cargo still decides whether compilation is needed.

## Remaining layers

- Shared catalog-driven scanning, client admission and a versioned event ABI that
  carries target/catalog identity and typed diagnostics.
- CLI/GUI/Settings cutover, explicit C3 marker-link behavior, C4 invalid-byte path
  action handling, golden client parity and the 200-target end-to-end Release run.

These drafts do not silently accept changed marker/path semantics, claim Windows
scoped acquisition, close resource budgets (R5), or prove sandbox/Store acceptance.
