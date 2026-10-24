# Bounded metadata batches — 2026-10-24

The optional macOS size pass now has count/estimated-weight retention limits,
checked before inserting each decoded observation. Compatibility reads remain
unbounded. Entries and per-entry errno observations share the count quota;
max_errors is unused. Duplicate basenames conservatively charge weight again.
The caller must treat the explicit limit as truncation. The fixed 256 KiB native
buffer, hash-table/allocator overhead and opaque errors are outside weight/RSS.

Debug/Release workspace checks passed: 150 core unit, 9 facade unit, 9 scoped
filesystem integration and 2 compile-fail documentation tests. Native tests verify
exact-cap exhaustion versus truncation; decoder tests reject the next entry/weight
without retaining it. Clippy all targets/features, HC parse/validate and SpecPM
passed (0 errors, 15 unresolved-capability warnings). Historical receipts are not
whole-process acceptance or a claim that the feature is enabled by default.
