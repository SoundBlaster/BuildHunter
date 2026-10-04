# ADR-0002: Compile the built-in Swift scan policy over a finite domain

Status: accepted for the issue 25 feature branch; integration validation pending.

## Contract

H13 removes per-candidate Swift policy callbacks only for the current pure,
finite built-in policy. Swift still owns classification and filtering through
SpecificationCore. Rust owns a versioned input projection, not a second copy of
those decisions. Legacy `bh_scan` remains available for arbitrary policies.

Vocabulary version 1 has 18 valid name classes, in this order:

0. Other valid UTF-8 (`other-node` as representative).
1. `.git`
2. `.build`
3. `target`
4. `__pycache__`
5. `.pytest_cache`
6. `.mypy_cache`
7. `.ruff_cache`
8. `.pytype`
9. `.tox`
10. `.nox`
11. `build`
12. `dist`
13. `.venv`
14. `venv`
15. `.egg-info` suffix (`sample.egg-info`).
16. `.pyc` suffix (`sample.pyc`).
17. `.pyo` suffix (`sample.pyo`).

Exact names are checked before suffixes. For each valid class, the index is
`class * 256 + flags`. Flag bits 0/1/2 are directory/symlink/artifact ancestor;
bit 3 is own `pyvenv.cfg`; bits 4–7 are the four parent marker bits for Cargo,
pyproject, setup.py and setup.cfg. Unknown bits and unused own/parent markers
do not affect this projection. Retain the four parent bits individually rather
than assuming a future rule treats Python project markers identically.

Cell 4608 represents invalid UTF-8 and always traverses, matching the existing
callback's decode-before-policy behavior. There are exactly 4609 cells and
55,308 bytes of C decisions (three UInt32 values per entry). Table construction
is bounded; it is not a classifier memoization cache.

Expose additive C ABI:

```c
enum { BH_POLICY_TABLE_VERSION = 1, BH_POLICY_TABLE_CELL_COUNT = 4609 };
uint32_t bh_policy_table_version(void);
size_t bh_policy_table_cell_count(void);
int32_t bh_policy_table_cell_facts(size_t index, BHCandidateFacts *facts);
int32_t bh_scan_with_policy_table(void *control, const uint8_t *root,
    size_t root_len, uint8_t apparent_size, uint32_t table_version,
    const BHCandidateDecision *decisions, size_t decision_count,
    BHEventCallback event_callback, void *context);
```

Cell facts borrow process-lifetime representative bytes; indices outside the
domain or null output pointers return 0. The new scan function validates version,
count and actions before traversal, copies the entries once into immutable Rust
storage, and retains existing event/cancellation/partial-size/error semantics.
Invalid table arguments return status 3 without invoking callbacks or scanning.
Caller pointers must be readable for the call; no C API can validate pointer
ownership. Existing classification code values remain unchanged.

Swift uses a single shared decision helper for both the legacy callback and
table compilation: candidate guard, outer-root guard, ordered classification,
then the captured immutable filter snapshot. Invalid UTF-8 produces traverse
before constructing a Swift context. Table entries never duplicate those rules.

Compile only when version/count match and every descriptor's relevant matching
operation is expressible in the vocabulary: exact names from classes 1–14,
suffixes from classes 15–17, or `requiresVenvMarker` (which ignores its name and
suffix lists in the existing policy). Preserve descriptor order, IDs, groups and
excluded IDs. Unsupported custom descriptors use legacy callbacks. Future
languages/frameworks can add a vocabulary version or remain on that fallback.

## Acceptance

- Exhaustive table/direct Swift policy parity over all cells, plus input names
  distinct from representatives, suffix overlaps, marker priority, unknown bits,
  nested roots and invalid UTF-8.
- Settings changes do not alter a running table; unknown/custom exact names,
  suffixes and Unicode matching retain the legacy path.
- Rust table projection tests, ABI validation, owned-table lifetime and scan
  report/event parity with callback scanning, including cancellation/failure.
- A scan through the table invokes zero Swift policy callbacks; event callbacks
  continue normally. Legacy callback scans remain supported.
- Release measurements include both lookup and table-construction overhead,
  parity outside timing, raw repeated samples and coarse noise-aware gates.
- CLI defaults and GUI kind mapping remain distinct as before. No release or
  dependency version bump occurs before integration verification.
