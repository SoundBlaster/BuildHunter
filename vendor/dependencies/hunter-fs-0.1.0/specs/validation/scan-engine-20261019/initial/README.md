# Composed scan engine validation

Baseline main 5cf35e9d8bb05557a828ea85b6085038b5735ac4; immutable core snapshot
d728f1a9c13a578df930ff2cc4a42359398d1ee7. Debug/Release workspace tests pass:
123 unit tests (114 portable core + 9 Unix scoped metadata), plus two compile-fail
doctests for consuming session/engine. Eight new engine witnesses cover deferred root/
pruned child and cached errors, immediate original-path fallback/completion lifetime,
empty ignored measurements, abandon/stop cleanup, preflight/checkpoint receipt ordering,
control/disconnection outcome, panic cleanup, and real scoped-session join composition.
Borrowed non-Send branch context remains accepted. A control failure at zero outstanding
work deliberately returns Drained; it is not a product success status.

Fmt, all-target clippy, Hypercode parse/validation and SpecPM pass (zero errors;
13 existing unresolved-capability/duplicate-boundary-ID warnings). Tool/source hashes,
raw command outputs and effective TB5 paths are retained. Host still owns protocol,
errors/policy/aggregation/status, guards and receiver-before-join/final publication.
This does not establish live home/cloud/UI acceptance, autonomous backend/worker
assembly, resource bounds or a general speedup. New recovered contracts stay draft;
previous receipts are historical. Hosted CI is independent of these local results.
