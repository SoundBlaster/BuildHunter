# Portable Hunter extraction — 2026-10-14

Hunter PR #2 introduces std-only `hunter-fs-core` at
`8295d1418c03d1295cea86f4781352469fa27c78`. BuildHunter now uses it for the
single-directory entry loop and apparent/allocated metadata-byte conversion.
This applies to legacy CLI/GUI and opt-in scanning through the shared scanner.

Hunter owns iterator error/type acquisition, cancellation-before-type-lookup and
owned path/error observations. BuildHunter opens the iterator through its probes
under the existing guard, resolves legacy symlink markers in the same callback
position, and owns recursion, worker prefetch, timing, diagnostics, sizes aggregation,
status/nesting, security-scoped access and terminal lifetime. macOS bulk acquisition
remains local but produces the shared MeasuredSize. The Rust public scanner and
numeric FFI are unchanged; no Swift source or default classification change.

This is partial extraction, not an independent recursive scanner. Portable listing
still uses pathname DirEntry identity and is not descriptor-bound against selected
root replacement. Entry file_type does not follow link targets; whether to traverse
or classify a link remains host policy. Empty listings finish without invoking the
stop predicate; blocking filesystem calls cannot be interrupted by this callback.
Hard links are still counted per path; byte conversion is not reclaimable space.

## Offline distribution

Root Cargo now has one path dependency, with no registry/Git dependencies in its
CLI graph. `vendor/hunter-fs-core` is an exact upstream distribution snapshot, not
an independently edited fork. UPSTREAM.json pins repository, commit, source directory
and four SHA-256 file identities. `.gitattributes` prevents checkout newline conversion.
Local source equality was checked against Hunter before publishing the consumer.
CI verifies hashes, includes a source-mutation regression and executes core tests.
These hashes detect unintended drift; they are not a remote-origin authentication
proof or signature. Changes must originate in Hunter then update snapshot/pin/hashes.
The old empty-Cargo-home offline build/install/launch test remains the acceptance gate.

The Unix scoped marker API continues to use the previous direct Git pin in the
independent plugin scanner package. It uses libc and is intentionally outside the
root offline CLI graph. This separation is packaging, not a different rules engine.

## Validation scope

Root/default/all-feature, standalone portable crate, plugin integration, CLI,
offline and existing paired Release comparison checks provide the evidence.
Fresh receipts/source/map identities are under specs/validation/hunter-portable-20261014.
Performance comparisons use the same host/fixture and noise-adjusted budget; no
whole-tree speedup or memory saving is inferred from the code move. UI/Release
hosted CI and live cloud permission prompts remain separate acceptance evidence.

Merge order: Hunter #2 before this consumer. The snapshot records an immutable
upstream source commit; preserve/reconcile it when the upstream PR merges.
Next boundary: separate recursive work scheduling/measurement from product marker
prefetch and scanner status policy before moving the worker orchestration.
