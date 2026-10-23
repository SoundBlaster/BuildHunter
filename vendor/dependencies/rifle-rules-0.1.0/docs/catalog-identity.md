# Program packages, catalog validation and content identity

This slice extends the pure Rifle evaluator. It does not add filesystem access,
BuildHunter taxonomy, marker bindings or target-conflict resolution.

## Public API and boundary

- `JsonIdentity::from_json(bytes)` admits bounded strict JSON, returns canonical
  bytes and their `Sha256Digest`. It does not validate a program or plugin schema.
- `ProgramPackage::from_json(bytes)` requires an object with `id`, `version` and
  `program`. It validates those fields and the complete pure Rifle program;
  additional fields remain opaque host data, included in identity.
- `Catalog::from_json_documents(&[documents])` validates every package, rejects
  duplicate package IDs and aggregate excess, then returns an owned immutable
  snapshot sorted by ASCII package ID. No partial catalog is returned on error.
- `packages()` exposes immutable packages and plans. Program rule/fact IDs are
  local to each package. Host-specific target IDs are not checked by this API.

This is an intentionally open generic envelope, NOT the closed BuildHunter
plugin schema. A host must validate its full format/bindings/targets, reserved IDs
and compatibility BEFORE accepting the snapshot as its effective plugin catalog.
An unknown host format or an executable-looking extra field is opaque to Rifle,
not permission to execute or a declaration that it is valid for BuildHunter.
The host must not discard disabled package validation ambiguities silently.

Package IDs use the existing ASCII ID grammar. Version is an exact three-part
numeric string with no leading zeroes and at most 32 characters. Two versions of
the same package ID cannot coexist in a catalog. Version-range solving, replacement
policy and updates are host duties; package version is not the wire format version.
Empty catalogs are valid; a host requiring a bundled catalog must enforce that.

## Canonicalization and hashing

Canonicalization follows [RFC 8785 / JCS](https://www.rfc-editor.org/rfc/rfc8785.html)
using pinned `serde_json_canonicalizer` 0.4.1 (MIT), with `serde_json`'s
`float_roundtrip` decoding. Dependencies are fixed by Cargo.lock. The implementation
is described in its [upstream repository](https://github.com/evik42/serde-json-canonicalizer).

JCS serializes strings without Unicode normalization, orders object keys by UTF-16
code units and preserves array order. Tests pin the RFC serialization example,
Unicode key ordering and number-format boundaries. Identical semantic object field
order/whitespace yields the same bytes; changing arrays or strings can change identity.

The Rifle admission profile rejects duplicate keys before parsing loses them,
invalid Unicode, non-finite numbers and excess bytes/depth. Plain integer tokens
outside ±(2^53−1) are rejected as `unsupported_number`, even when a particular value
might be representable. Decimal/exponent tokens use binary64 interpretation and
rounding according to JCS. Exact large IDs/counters should be strings; this is a
restricted admission profile, not a promise to preserve arbitrary-precision JSON.

Package digest is SHA-256 over JCS of the ENTIRE package document, including
opaque descriptions, bindings or host metadata. It is not a hash of the program
alone. Input formatting is not retained; fixture files outside the document are
not covered. `Sha256Digest` is a 32-byte value; Display uses 64 lowercase hex digits.
A digest establishes content identity, not provenance, permission or signature.

Catalog digest is SHA-256 over JCS of an array of `{id, version, digest}` records,
sorted by ASCII package ID. The algorithm matches the agreed BuildHunter catalog
identity definition, assuming the host supplied the complete validated documents.
The empty catalog uses JCS `[]`. No settings are inserted into this digest:
client profile and exclusions belong in the host scan/report identity separately.

## Budgets and lifecycle

| Resource | Enforced bound |
|---|---|
| JSON per document / JSON container nesting | 1 MiB / 32 |
| Packages / total raw JSON | 64 / 16 MiB |
| Total program rules / expression nodes | 4096 / 32768 |
| Facts/rules/expressions per program | Existing evaluator limits |

Byte/package limits are checked before loading. Rule/node totals are checked
while validating, with all earlier partial allocations dropped on error. Canonical
bytes for each package are released after hashing; snapshots retain plans, headers
and digests rather than the full metadata JSON. This is not a whole-process RSS
limit or latency guarantee. Target/marker budgets are host-specific, not Rifle duties.

Package errors gain `/program` or `/packages/<input-index>` JSON-location prefixes.
Aggregate budget failures identify `/packages`; duplicate IDs identify their input
entry. Presentation/order of the first error can depend on the input order;
identity and the sorted successful snapshot cannot. Updating source JSON does not
mutate an already loaded snapshot; starting another scan requires a new catalog.

## Verification and remaining work

The existing evaluator tests remain regression coverage after moving strict
bounded parsing into a shared decoder. New tests cover JCS/known SHA vectors,
Unicode/negative-zero handling, reordering, full-content identity, invalid admission,
headers, atomic invalid-program rejection, duplicate IDs, exact aggregate budgets
and owned snapshots. Cross-platform outcomes are checked by the existing CI.

This does not implement BuildHunter plugin validation or a conflict-free effective
catalog, SDK authoring, filesystem probes, package authenticity or target matching.
Those remain separate layers. The Rust API is reviewable, not a frozen ABI.
