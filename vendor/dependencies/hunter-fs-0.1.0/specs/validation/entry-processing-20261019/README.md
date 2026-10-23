# Entry processing validation

Baseline main 0682b3a80f2fcc77858bb1789c18c3a913227bd5; immutable core snapshot
2031df50f8324eaaf91856c5dc382d35694b2fd4. Local Debug/Release workspace: 115 unit
tests (106 core + 9 Unix), plus one compile-fail doctest. Seven new witnesses cover
routing slots/stop/prefix, empty routing, panic cleanup, root/worker/fallback and
cached-error/index consumption, unselected empty measurements and panic before I/O.
Fmt, all-target clippy, HC parse/validation and SpecPM pass. Source/tool hashes,
commands and effective TB5 output paths are retained.

Host policy, byte interpretation/aggregation, callback checkpoint cadence and
cancellation decisions remain outside this boundary. Generic callbacks may borrow
non-Send context. Source/fixture checks do not establish live home/cloud/UI, Windows/
Linux acceptance, general speedup, race-free traversal or whole-process memory bounds.
New recovered BoundarySpec stays draft; previous evidence remains historical.
