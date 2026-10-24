# Task ledger extraction evidence

Seven new ledger tests cover non-reused IDs and out-of-order listing/measurement
correlation, message reservations/retirement, ignored measurements, payload release
on stop/drop, setup failure and invalid host operations. Tests initially failed
to compile before the API existed (tdd-missing-api.log); this was API development,
not a reproduced existing bug.

macOS workspace: 9 scoped + 31 core tests pass in Debug/Release. Clippy/fmt, HC
parse/validation and SpecPM pass with zero errors and three existing warnings
(two unresolved platform requirements, one duplicate metadata section ID). Raw
commands/tool/source identities are recorded. Contract remains draft; the ledger
does not validate wire order/identity or own workers/queues/recursive accounting.
No fixed memory budget, scan speedup or full lifecycle conformance is claimed.
