# Rifle specification progress

Goal: implement the pure program validator/evaluator from the agreed v1 contract.
Repository bootstrap baseline: 4f5798f. New source is pinned by file hashes in
[initial validation](validation/initial-20261014/README.md); no extraction from
Hunter/BuildHunter runtime occurs. The schema originates from BuildHunter
79fa49303e73a595330fcb7441d5f8c2f430098d, copied byte-for-byte.

| Role | Contract / evidence | State |
|---|---|---|
| ProgramLoading.StrictJson | rifle.program/validation; src/json.rs; parser tests | Implemented, local tests; CI pending |
| ProgramLoading.SemanticValidation | rifle.program/validation,limits; src/validate.rs | Implemented, local tests; CI pending |
| ValidatedPlan | rifle.program/input_contract; private indexed expressions | Immutable public API; host order obligation explicit |
| Evaluation | rifle.program/pure,three_valued; src/lib.rs; truth tables | Implemented, local tests; no benchmark claim |
| ContractTests | tests/contract.rs | Significant semantic/negative/concurrency scenarios; not exhaustive input proof |
| Host adapters, target resolution, SDK/FFI | Excluded from this initial slice | Separate next work, not implied implemented |

One implementation boundary is sufficient: parsing/plan/evaluation share a pure
consumer contract. HC nesting is composition only. Root AGENTS.md requires maps
and contracts to follow future source changes. Preserve prior validation records.

Accepted: owner-approved plugin contract direction. New Rust API and BoundarySpec
remain reviewable drafts. Fresh validation distinguishes local debug/Release,
structural checks and actual GitHub CI. Next boundary: catalog/identity validation
and conformance before host integration; performance optimization needs baseline
measurement, not only a successful build.

## Generic catalog / identity pass — 2026-10-14

Baseline: fb319c7ccaa005f6f63ffbeaa034dd3b4ddb7c73 (merged Rifle #1).
Added `rifle.catalog`: ContentIdentity/CanonicalJson/Sha256Digest and
ProgramCatalog/PackageValidation/AggregateValidation map roles. Consumer
contract: full-document identity, generic id/version/program envelopes, atomic
budgets and load-order-independent catalog identity. Target IDs, bindings,
reserved namespaces and semantic target conflicts remain host-owned.

Existing `rifle.program` claim IDs are unchanged. Its strict decoder moved into
shared `src/json.rs`; evaluation semantics and program limits remain unchanged.
The existing semantic tests are regression evidence, not merely an unchanged map.
Update map binding for StrictJson; preserve earlier source/map snapshots.

Owner-approved identity direction is implemented in this slice; new Rust API and
BoundarySpec remain reviewable. Safe integer-token admission is a documented
restricted JCS profile; decimal/exponent values use binary64 interpretation.
No arbitrary-precision preservation, cryptographic authenticity or full host-schema
validation claimed. Fresh evidence: [catalog validation](validation/catalog-20261014/README.md).

Next boundary: BuildHunter plugin adapter validation (facts/bindings/target links,
reserved IDs/exclusions) using these immutable programs and full-package identities;
Swift SDK and Hunter extraction remain subsequent stages.
