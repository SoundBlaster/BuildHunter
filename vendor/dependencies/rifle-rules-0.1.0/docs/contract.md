# Implemented program contract

This is the first Rifle runtime slice of the agreed BuildHunter plugin v1 design.
The Rust API is new and reviewable; the wire version remains a draft. A successful
SpecPM/HC check establishes document structure, not runtime conformance.

`Plan::from_json(bytes)` validates only the abstract program. `Plan::facts()`
exposes ordered `Fact { id, kind }` declarations. `evaluate(values)` requires that
same order; facts are positional, so the host must not reorder equal-typed values.
It returns one `Outcome` per rule in declaration order. `Truth` is True, False or
Unknown. Unavailable facts carry an opaque host reason; diagnostics retain only
relevant unknowns, deduplicated and sorted by fact ID. No input references survive
an evaluate call; returned outcomes own their strings.

Rules do not win over each other. Every rule is evaluated; the host combines rule
IDs into targets, handles exclusions/conflicts and acquires facts. A loaded Plan
cannot be mutated through the public API. Values are validated before matching,
including inputs in branches that might short-circuit; this detects host errors.

## Validation

The program object has exactly `format`, `facts`, `rules`. Rule objects have
`id`, `when`; fact objects have `id`, `type`. Conditions are exactly one of
`all`, `any`, `not`, or a typed predicate with the fields in the schema. Strings
compare exactly without Unicode normalization or locale-dependent case folding.
String literal lengths count Unicode scalar values; surrogate escapes are rejected.
Duplicate keys are rejected during decoding, including nested objects.

| Limit | Value |
|---|---|
| Input bytes | 1,048,576 |
| Defensive JSON container nesting | 32 (valid depth-8 expressions fit below it) |
| Facts / rules | 128 / 256 |
| Condition depth / nodes per rule | 8 / 128; root depth is 1 |
| AND/OR children; `in` literals | 32 |
| ID length | 128 ASCII bytes |
| String literal length | 256 Unicode scalar values |

Rule and fact IDs are unique within their respective namespaces; every declared
fact must be referenced. `in` rejects duplicate literals. The format only admits
strings and booleans, with strict operand type equality. The per-rule/rule-count
limits imply at most 32768 expression nodes per program. Package/catalog aggregate
limits belong to the future host/catalog loader and are not enforced by this API.

Error codes: `invalid_shape`, `unsupported_format`, `duplicate_id`, `unknown_fact`,
`unused_fact`, `type_mismatch`, `limit_exceeded`, `missing_fact`.
Errors carry a JSON location (empty pointer for the document) and human message.
Decode errors have document location plus parser line/column in the message;
validation errors point at the enclosing invalid object or specific field.
Input errors use `/facts/<index>`. Codes are draft API, not a frozen ABI.

## Tests and remaining work

Contract tests cover all binary three-valued AND/OR combinations in both condition
orders, NOT outcomes, irrelevant/deduplicated diagnostics, strict parsing, known
fixtures, input mismatches, length/depth/node limits and concurrent plan reuse.
The tests do not prove every possible input safe; resource limits bound accepted
programs, not total process RSS or arbitrary host fact sizes.

No candidate name index is implemented: each bounded rule is evaluated directly.
Optimization must preserve the reference behavior, especially unknowns and
negative/marker-only rules. [Generic catalog and JCS identity](catalog-identity.md) are now available as a separate layer.
BuildHunter-specific package metadata/targets, marker I/O,
Swift authoring SDK and cross-language FFI remain unimplemented.
