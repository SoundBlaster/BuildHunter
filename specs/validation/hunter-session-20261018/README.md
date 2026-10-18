# Scoped worker session validation — 2026-10-18

Baseline: BuildHunter bf3b1bd236b7dbf510eb71b83c9cbd3df95e835a.
Runtime: b4647e5de8821db51a65b300922a1cb39c925e91.
Hunter: 31fd0bad749cdac63bc42f36ffe72bcd81b0c1ad.

Local macOS: default 71 tests (67 library + 4 CLI), all features 76 (72 + 4) in
Debug/Release; vendored core 78 in both; plugin scanner 12 in both; CLI script 13;
vendor verifier 4; isolated offline CLI; clippy/fmt all pass. Five upstream witnesses
cover helper stop before queue close, actual all-thread join, caller-thread non-Send
results, opaque setup failure, receiver release on coordinator/startup unwind and
worker-panic peer closure. Two product tests exercise failed-admission handler event
order/no work and final profile drain before a single caller-thread Finished. These
do not inject a real OS guard failure. Existing SetupFailed routing, worker/callback
panic, marker/cloud, bulk, profile/cancel and plugin tests remain gates.

All eight paired Release budgets pass (seven samples, alternating revisions, warm
shared fixtures). many-roots median 85.789 -> 84.507 ms, source-tree 3.450 -> 3.557 ms.
Samples/thresholds are in reports; no speedup or memory-budget gain is inferred.
The candidate commit is immutable; dirty=true reflects documentation, while all
runtime/Cargo inputs were byte-checked against the measured commit.

HC current CLI/GUI and target public/store profiles pass. SpecPM: 0 errors and
28 external-capability-resolution warnings. Structural validation and drafts do
not imply owner acceptance. Twenty-two runtime binding paths match the immutable
commit; fifteen canonical snapshot files match pinned Git objects/hashes. Independent
scoped-metadata pin is unchanged. Actual root/plugin compiler outputs and fixtures
use separately verified TB5 paths; no unused cache inventory. Historical receipts
remain historical.

Host keeps admission/guards/QoS, sampler/channel bodies, setup/status/terminal and
FFI behavior. Hunter owns scoped launch/helper stop/queue closure/join. Unread
receivers release before join, including unwind; exact relative capture-destructor
ordering is not promised. Arbitrary callbacks/destructors must terminate for join;
no watchdog or active syscall interruption. Full generic root/guard assembly,
default plugin migration, descriptor recursion and C5 budgets remain pending. No
local native UI/home/live-provider run or resource-budget acceptance is claimed.
