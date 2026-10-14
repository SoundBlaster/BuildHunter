# Portable Hunter adoption evidence

Local macOS: root/default 56, all-feature 60, portable core 7 and opt-in scanner
12 tests pass (portable/integration also Release). Clippy/fmt, CLI, snapshot/mutation
checks and HC two maps/four profiles pass. Final SpecPM: 0 errors, 17 unresolved
capability warnings. Earlier YAML-note parse failure and repair are retained.

Empty-Cargo-home offline check/install/launch passed. Paired Release comparison
against 13aece6 passed all 8 comparisons over 4 scenarios; raw summaries/MAD/budget
are retained. Candidate was dirty: source hashes, not candidate_sha alone, identify
the runtime. Canonical Hunter and all four vendored files were compared byte-for-byte.
Hashes detect drift; they do not authenticate remote origin. Hunter #2 hosted
Linux/macOS/Windows checks are green on 8295d14. New consumer CI is separate evidence.

No real-home/UI/live-provider/memory gain or scan speedup claimed. Workers/recursion
and product classification remain in BuildHunter. Previous receipts remain historical.
