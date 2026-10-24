# Bounded listing retention — 2026-10-24

Optional collectors retain ordered prefixes within entry/error counts and estimated
payload weight. A typed limit is separate from the original DirectoryVisit result;
hosts must handle it even when an uncooperative visitor returns Completed.
Retained raw paths, error codes and admitted observer/context changes are preserved.
The legacy collectors remain unbounded.

Local Debug and Release: 148 core unit, 9 facade unit, 9 filesystem integration and
2 compile-fail doc tests passed. Clippy all targets/features passed. Tests cover
entry/weight/error rejection, an uncooperative 10,000-event source, observer-error
retention, exact-cap successful exhaustion and open-failure precedence. HC parse
and bare validation passed; SpecPM 0 errors / 15 unresolved-capability warnings.

The current event, iterator internals, opaque context/error allocations and RSS are
outside the estimate. A visitor must poll the supplied stop predicate to stop I/O.
No timing, whole-process memory or BuildHunter runtime acceptance is claimed here.
