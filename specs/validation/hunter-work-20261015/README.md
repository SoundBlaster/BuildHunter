# Scoped work adoption evidence — 2026-10-15

Local macOS: 58 default/62 all-feature root tests, 14 core and 12 integration
tests pass (core/integration also Release); CLI, clippy/fmt and snapshot/mutation
checks pass. HC both maps/four profiles pass; SpecPM errors/warnings in commands.json.
Empty-Cargo-home offline check/install/run passed. Previous receipts remain historical.

New root tests cover policy/observer unwind and worker probe panic, then successful
follow-up scans. Upstream panic witness requires all peers exit while coordinator
shutdown guard is still alive. Worker queue releases pending jobs outside its mutex;
closed enqueue is rejected and marked failed by host. In-flight blocking I/O remains
host-owned/uninterruptible; queue and whole-report memory are not bounded.

Paired Release benchmark against bb21773 passed all 8 gates over 4 fixtures.
Dirty candidate source hashes, not candidate_sha alone, identify compilation inputs;
raw samples/MAD/budget are retained. No real-home/UI/live-cloud acceptance, speedup
or memory-saving claim. Full filesystem recursion/task bodies remain host-owned.
