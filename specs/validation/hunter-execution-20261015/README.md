# Serial scan execution adoption evidence

Local macOS: 63 default and 67 all-feature root tests pass; all-feature Release
also passes. The exact vendored core passes 49 tests in Debug/Release, independent
plugin scanner 12 in Debug/Release, CLI integration 13, and vendor mutation suite 4.
Clippy, root/vendor fmt and fresh-Cargo-home offline check/install/launch pass.
Separate root/plugin target paths were preflighted on the verified TB5 SSD and
confirmed with Cargo metadata; offline fixtures/builds use TB5 temporary paths.

The new application witness cancels from a profile checkpoint with/without a
queued setup error: receipt accounting is preserved, dispatch suppressed, and
pending-task telemetry matches the ledger. Existing ignored measurement, nested
partial totals, terminal status and callback/worker panic gates remain green.
The receiver is captured by the scope closure and released before scope join,
including unwind; moving it out of Scanner does not extend queued payload lifetime.
Ten upstream execution tests and absent-API red phase are identified in Hunter's
scan-execution-20261015 record at the pinned upstream revision.

Paired Release fixtures against main 10e8aeed pass all eight noise-adjusted budgets.
Builds, fixture and temporary baseline worktree were routed to TB5; report samples
are paired on that same volume/host. No speedup or RAM improvement is inferred.
performance-report.json retains raw samples, MAD, harness/compiler/host identity.
Its candidate SHA identifies the immutable runtime commit, not the later docs commit.

SpecPM: zero errors, 23 unresolved external-capability warnings (one new Hunter
execution dependency). Both maps parse and all four CLI/GUI/public/store profiles
validate. Ten canonical files match the exact pinned upstream Git objects; all
15 current-scan binding paths exist and match the immutable runtime revision.
Source/map/tool identities, frozen map/upstream snapshots and actual command
receipts are included; historical extraction/test records remain untouched.

Root/backend admission, actual worker/helper scope lifetime, product policy,
partial finalization, status and terminal events remain host-owned. Contracts
stay draft. No local Xcode/UI, real-home or live-cloud run was performed, and no
whole-engine, fixed memory-budget or default plugin migration acceptance follows.
