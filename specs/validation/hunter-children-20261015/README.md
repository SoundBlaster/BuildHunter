# Host-selected recursive child request adoption

Executed locally on macOS against source-identity.json: 60 default/64 all-feature
root tests; 24 canonical core tests in a temporary copy (Debug/Release); 12 plugin
integration tests (Debug/Release); CLI tests, clippy/fmt, four vendor inventory/
mutation tests, exact seven-file upstream comparison and fresh-cache offline CLI
check/install/launch. HC two maps/four profiles and SpecPM pass with zero errors
and 20 unresolved external-capability warnings. Raw commands and diagnostics are
in commands.json, tools in tools.json. No macOS GUI/Xcode/UI tests or real-home/
live-cloud scan ran locally for this Rust-only extraction.

Core development's missing-API red phase and fresh upstream validation are in
Hunter specs/validation/child-traversal-20261015 at the pinned revision. The new
BuildHunter mixed-tree regression validates original measurement alignment,
inherited ancestry, host pruning and exactly one completion per finding.
The directory arena, result correlation, ancestors/status and guards remain host-owned.
Contracts stay draft; this is partial child planning, not full target conformance.

Paired Release fixtures vs 4b40205 pass all eight noise-adjusted gates. Raw samples,
MAD, host/tool/harness identity and thresholds are in performance-report.json.
Dirty candidate_sha names only the checkout base; use source-identity.json for
actual inputs. No speedup or RAM improvement is claimed. The initial benchmark
before explicit early listing-buffer drop is retained as initial-performance-*.
Final source releases the exhausted plan before coordinator-side measurement.

Initial command-invocation failures are preserved: --locked cannot create the
standalone vendor crate lockfile, and the CLI test driver requires a binary path.
Final core tests use a temporary canonical copy with --offline; final CLI checks
build the binary and pass its path. These were harness invocation corrections,
not product test failures. Prior extraction/review receipts remain historical.
