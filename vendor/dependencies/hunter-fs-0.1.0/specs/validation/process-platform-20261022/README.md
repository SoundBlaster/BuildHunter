# Process platform extraction validation

Baseline Hunter main 6f41d5265763aa2c67f28bd28a2f98d8a49fbc26;
MIT source origin BuildHunter a9d3dec61c716a63009b3108e5fdacf06440c824,
`src/thread_qos.rs` and `src/process_resources.rs`. Public visibility and docs
are added; OS acquisition/layouts, requested-QoS semantics and caller-owned
capture/apply boundaries are unchanged. Runtime/map inputs are identified by
per-file SHA256 in source-identity.json, separately from the baseline commit.

Local macOS all-feature Debug and Release each pass 144 unit tests (9 facade,
135 core) plus two compile-fail doctests. The three existing native QoS tests
moved with the code, as did the self CPU/RSS smoke test. New tests reject invalid
or overflowing CPU timeval conversions and retain absent counters as None.
Non-macOS no-op/unsupported branches require corresponding hosted execution;
Linux/Windows results must be checked separately, not inferred from this run.
Fmt and all-target/all-feature clippy pass. HC parse/validation pass. SpecPM
0.2.0 reports zero errors and 14 pre-existing unresolved-capability/duplicate-ID
warnings; no new requirement is accepted by structural success.

Raw commands and actual TB5 routing/tool identities are retained. Initial local
clippy found test-module ordering and SpecPM found scalar effect entries; both
were corrected and the complete final checks passed. No fresh ~/ scan, UI test,
scheduler guarantee, RSS bound or performance improvement is claimed.
