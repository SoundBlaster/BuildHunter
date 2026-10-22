# Hunter process platform integration validation

Baseline BuildHunter main a9d3dec61c716a63009b3108e5fdacf06440c824;
immutable integration runtime `61e091dadba0a58f011017d84cfd21f18fdae3aa`. Exact Hunter pin
c5354e973b5d1d90c438c94b78c06898ffe62500 (Hunter #25), 27 files
byte-equal to upstream. Source/map identity hashes are separate from this commit.
The primary checkout and its local Xcode scheme remain untouched.

Default local Debug passes 68 unit tests. All-feature Debug and Release each pass
206 unit tests (67 scanner, 4 CLI, 135 upstream) plus two compile-fail doctests.
The native QoS/self-accounting tests moved upstream; invalid/overflow conversion
and absent-data tests were added there. Existing host resource math, profile,
slow-consumer, cancellation and terminal ordering coverage remains in place.
Fmt, all-target/all-feature clippy and four offline vendor tests pass. Seven current/target HC parse and validation/profile checks pass. SpecPM has zero errors
and 39 unresolved-capability warnings, including the two new external platform
capabilities. Structural validation does not resolve those dependencies or prove
runtime conformance; requirements stay draft.

OS algorithms/layouts and calls are unchanged, with public visibility/docs and
additional upstream tests. BuildHunter imports existing private names and deletes
its local copies. Host `scan_session.rs`, `profile.rs` and `resources.rs` remain
byte-identical to baseline: capture/apply placement, baseline, sampler timing,
percentages, means/peaks, ABI and final publication are not relocated. No Cargo
feature default or dependency version changes. No fresh signed GUI/home scan,
Linux/Windows runtime acceptance, CPU/RSS budget or performance improvement is
claimed; hosted cross-platform/macOS jobs are separate evidence.

Actual commands, tool identities and verified TB5 output paths are retained;
historical receipts are not relabeled as current proof.
