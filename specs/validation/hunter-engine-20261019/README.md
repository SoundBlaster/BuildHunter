# Composed ScanEngine integration validation

Baseline main 1f92df7fdad00787e02f2eb029fa09f6007c66ca; immutable candidate runtime
1c765b6e94ecd9b1ee9442ba351f3f00ab32847a. Exact upstream snapshot:
d728f1a9c13a578df930ff2cc4a42359398d1ee7. All 22 vendor files match byte-for-byte;
40 runtime/dependency/harness inputs match the immutable candidate commit.

Local default Debug: 72 tests. All-features Debug/Release: 77 each. Vendored Release
core: 114 plus two compile-fail doctests. Independent plugin scanner: 12. Compiled
Release CLI: 13. Vendor integrity: 4. Fmt, all-target/all-feature clippy and Hypercode
parse/seven profiles pass. SpecPM: zero errors, 35 unresolved-capability warnings.
Command outputs/tool versions, source/map hashes and effective TB5 paths are retained.

Eight new upstream integration witnesses plus existing product assertions cover
engine result/ledger/routing/measurement composition, root/worker/fallback and cached
errors, empty/pruned ignored work, abandon/stop cleanup, borrowed non-Send branch state,
control/disconnection semantics, panic cleanup and scoped-session join. Engine/session
compile-fail witnesses enforce consumption. Product receipt/profile cancellation and
real guard/callback/worker panic tests retain their assertions; the scripted driver now
uses ScanEngine. Helpers/receiver release and post-join Profile/Finished are unchanged.

Paired Release harness: seven interleaved warm fixture samples against main; all
eight noise-adjusted budgets PASS. Source-tree median 3.634 -> 3.618 ms; many-roots
78.646 -> 83.291 ms (about 6% higher in this run, within the configured budget).
This establishes no general speedup, CPU/RAM bound or fresh home/cloud/UI acceptance.
The dirty flag contains docs/maps/receipts only; performance-inputs.json proves exact
runtime/dependency/harness equality. Historical receipts remain historical.

Current serial engine composition is implemented; autonomous backend/worker factories,
descriptor recursion, default plugin migration and C5 budgets remain outstanding.
New recovered constraints remain draft; hosted CI is separate evidence.
