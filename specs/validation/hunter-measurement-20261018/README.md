# Typed measurement preparation validation — 2026-10-18

Baseline: BuildHunter dec3ad603bef36997498b9432ffae572ae019c77.
Runtime: c2cd2b857d73ff98b41f9653276918c23018cc5d; Hunter: 1eb300054c24bbf56a936287fbd586c4fb39079c.

Local macOS: default 69 tests (65 library + 4 CLI), all features 74 (70 + 4) in
Debug/Release; vendored core 73 in both; plugin scanner 12 in both; CLI script 13;
vendor verifier 4; isolated fresh-cache offline CLI; clippy/fmt all pass. Two new
host tests cover pre-cancelled directory error still preparing, batch entry
consumption, missing/Unsupported fallback counters, native error codes and injected
error precedence under the active thread guard. Five upstream tests cover typed
ordering/slots/ID/errors and prepared-state drop with borrowed non-Send resources,
pre-cancel/error/preparation, cancellation, empty slots and panic. Existing closure,
cloud, bulk, marker, profile/cancel/panic, worker/result and scope tests remain gates.

All eight paired Release fixture budgets pass (seven samples, alternating revisions,
warm shared fixtures). many-roots median 75.324 -> 75.027 ms; source-tree
3.261 -> 3.145 ms. Full samples/budgets are in the reports. No speedup or RAM claim.
The runtime commit is immutable; candidate_dirty=true reflects documentation, and
all runtime binding/Cargo inputs were byte-checked against the measured commit.

Four HC profiles parse/validate. SpecPM: 0 errors, 27 external-capability resolution
warnings. Structural checks and draft contracts do not prove owner acceptance.
All fourteen canonical snapshot files match the pinned Git objects/hashes; 20 scan
binding paths exist at the immutable runtime revision and match checked bytes.
Independent scoped-metadata pin is unchanged. Actual root/plugin compiler targets
and fixture locations are separately verified on TB5. Historical receipts remain
historical, and no unrelated cache inventory is included.

Native/bulk effects, fallbacks, error decoration and guards remain host-owned;
WorkerAdapter retains timing and Listed/Measured protocol. Marker/filter/classifier,
bridge/application/plugin contracts are unchanged, with no new Swift UI acceptance.
Full root/startup/guard assembly, descriptor recursion, default plugin migration
and whole-scan resource budgets remain pending. No local UI/home/live-provider
run or resource-budget acceptance is inferred.
