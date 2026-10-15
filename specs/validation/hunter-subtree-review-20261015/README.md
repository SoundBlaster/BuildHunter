# PR #76 implementation binding correction

The scan implementation binding now resolves to immutable BuildHunter runtime
revision `5235f70656aca71b3e6ba7c4d75b69fd9e32836b`. All 14 owned/border files
exist at that revision and match current worktree bytes; `binding-paths.json`
records each Git object and SHA-256. The old `48badf9` source object lacks
`vendor/hunter-fs-core/src/subtree.rs`, reproducing the review finding.

Fresh SpecPM validation: zero errors, 22 existing unresolved external-capability
warnings. Both HC maps parse and all four profiles validate. Current/target map
bytes match the frozen `hunter-subtrees-20261015` maps; no roles, capabilities,
constraints, parent obligations or runtime source changed. The vendored Hunter
snapshot remains exactly pinned to d04c8fbbf4445e8bf3504f63f120d71fb4cc5a51.

`commands.json` retains actual command results (including the expected missing
old object), `source-identity.json` records current inputs and historical runtime
identity, and `tools.json` records validator executable identities. Historical
tests/performance/snapshots are untouched. No runtime tests/builds, UI, home scan
or performance measurements were rerun for this documentation-only correction.
Structural validation and resolvable bindings do not prove runtime conformance.
The unrelated local AGENTS.md edit is excluded from this change.
