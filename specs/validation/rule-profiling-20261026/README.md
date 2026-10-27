# Rule profiling and template documentation — 2026-10-26

Runtime source: `6da2ce553f598be4d74494953cf8dcf8bf907c61` (profiling implementation
`c76d344cad18fe04a85ece1c95c51a49e553bee1`, followed by package metadata).
Base editor PR head: `d64568c37772c290b4c39893096761c717a6bc7b`.
Rifle dependency: `af35dedf46d8c9978af1e30985f9c157a3f266c2`, Rifle PR #3.
Current documentation/maps/bindings are identified by source-identity.json; docs
receipts do not rewrite earlier validation or imply owner acceptance of all drafts.

## Executed checks

- Final default scanner: 97 library + 5 CLI/profile tests pass.
- Policy adapter: 20 tests pass. Scoped scanner integration: 18 tests pass.
- CLI integration on Release: 15 tests pass; profile opt-in, outcome parity,
  package versions, stable identity and existing-output protection are exercised.
- Plugin/profile wire schemas: 13 tests pass. Final native and CLI profiles both
  validate against the shared profile schema. Never-evaluated means encode null.
- Rust fmt and clippy (all-targets/all-features, warnings denied) pass. Pin/active
  dependency verification passes. A fresh Cargo home can check/install/launch
  the CLI offline with the vendored Rifle revision; no downloaded dependency needed.
- Debug build-for-testing and Release build pass. Both app signatures verify
  with codesign --verify --deep --strict. Exact commands/limits: commands.json
  and signing.json. Outputs/caches were on verified TB5, UUID
  04B9F65E-894C-4A7C-B517-FC542E8D8E36, separate worktree path.
- Current/target HC parse and profile validation, plus SpecPM validation, have
  actual results in structural-commands.json. Required-capability resolver
  warnings are retained and are not execution failures or acceptance.

## Native verification

Using cua_repl with the Debug app and an isolated preferences suite: enable the
Search setting; scan 50 Forge projects and one missing-marker negative fixture;
observe 50 findings, zero warnings and 28 phase rows; open the independent
resizable table with a Phase column and explicit Completed status; export JSON.
The custom rule has 51 resolved evaluations: 50 true and one false. Preliminary
unknowns include intentionally unread markers. Versioned catalog has 4 packages.
See native-check.json and native-profile.json. Screenshots were inspected inline;
there is no claimed saved screenshot image. No standard user preferences changed.

The shared-schema check exposed omitted null averages in an intermediate Swift
export; explicit encoding fixed it and the final native JSON passes. The failed
intermediate report remains only in local TB5 working artifacts, not final proof.

## Release comparison

Eight alternating scans of 200 disposable Cargo fixtures retain identical target
counts, bytes and digest with profiling off/on. Raw observations are in
release-comparison.json; cli-profile.json is one final aggregate plus its catalog.
The scans are only milliseconds long and do not establish an overhead percentage,
a speed improvement, whole-home performance, CPU attribution or a memory ceiling.
No global regression threshold is inferred from scheduler/cache noise.

## Limits and remaining verification

A local XCTest unit attempt was cancelled after 90s without confirmed test bodies.
The log stops after test-session setup/result-bundle preparation. This is neither
a pass nor an assertion failure; exact cause/permission state is not confirmed.
Do not equate successful build-for-testing or native checks with that suite pass.
No UI automation permission/security settings were altered or bypassed. Fresh
GitHub macOS unit/UI results remain necessary for the PR; no Store acceptance is
claimed. New profiling/delivery contracts remain reviewable drafts.
