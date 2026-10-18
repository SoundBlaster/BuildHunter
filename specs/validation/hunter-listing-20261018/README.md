# Directory listing acquisition validation — 2026-10-18

Baseline: BuildHunter f3c66858360aa475690aae9d9445d21b4ca0b72f.
Runtime: c70724d469d823774450bc387ecb90be1c5e2905. Hunter: 29bc8b0f60d0d192fe8e2529216f50ce4c9854c6.

Local macOS: default 68 tests (64 library + 4 CLI), all features 72 (68 + 4) in
Debug/Release; vendored core 68 in both; plugin scanner 12 in both; CLI script 13;
vendor verifier 4; isolated fresh-cache offline CLI; clippy/fmt all pass. The new
application witness checks regular/symlink/directory/dangling marker types, error
and child preservation, and zero marker probes in disabled-policy mode. Seven new
upstream macOS tests cover generic listing/context/observer boundaries; Linux adds
raw names. Existing visitor, cloud, bulk, worker/result, profile/cancel/panic and
scope tests remain gates. The test-only post-list mutation uses a cfg(test) local
binding so production builds remain warning-free; no runtime stop/order change.

All eight paired Release budgets pass (seven samples, alternating order, warm
shared fixtures). many-roots median 73.045 -> 72.305 ms; source-tree
2.991 -> 2.944 ms. Samples/thresholds are in the report; no speedup or RAM claim.
The candidate commit is immutable; candidate_dirty=true reflects documentation.
Runtime binding/Cargo inputs were byte-checked against that commit.

HC current CLI/GUI and target public/store profiles pass. SpecPM has 0 errors and
26 unresolved external-capability warnings. Structural checks do not prove runtime
or owner acceptance; drafts remain draft. Current scan has 18 immutable bound paths;
classification has 10, with unchanged policy semantics and no new Swift UI acceptance.
All thirteen canonical snapshot files match pinned upstream Git objects/hashes.
Frozen maps/upstream draft, source/tool identities and complete commands are retained.
Actual root/plugin compiler targets and fixture locations are independently verified
on TB5; no unused cache inventory is included. Prior receipts remain historical.

Host owns guarded opening, marker I/O/context, post-list hooks, worker timing,
queue/receiver/join lifetime and status. Full startup, descriptor recursion, default
plugin migration and resource budgets remain pending. No local UI/home or live
cloud-provider acceptance is inferred.
