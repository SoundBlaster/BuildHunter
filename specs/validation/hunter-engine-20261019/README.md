# Composed ScanEngine integration validation

Baseline main 1f92df7fdad00787e02f2eb029fa09f6007c66ca; immutable candidate runtime
79b589313e95997758bd196dd60d0a8cd3b7eb03. Exact upstream snapshot:
920a51afb85efdb21951d61d1ef3894148017cbc. All 22 vendor files match byte-for-byte;
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
eight noise-adjusted budgets PASS. Source-tree median 3.270 -> 3.220 ms; many-roots
78.697 -> 73.770 ms. The first run had a roughly 6% increase for many-roots;
that historical result is retained in initial/, and the variation is not a speedup claim.
This establishes no general speedup, CPU/RAM bound or fresh home/cloud/UI acceptance.
The dirty flag contains docs/maps/receipts only; performance-inputs.json proves exact
runtime/dependency/harness equality. Historical receipts remain historical.

Current serial engine composition is implemented; autonomous backend/worker factories,
descriptor recursion, default plugin migration and C5 budgets remain outstanding.
New recovered constraints remain draft; hosted CI is separate evidence.

Initial local/runtime/performance receipts are retained in initial/. Hunter Ubuntu CI
flagged platform metadata layout in ListingAction and a test message envelope; the
final upstream pin adds two narrowly scoped large_enum_variant allowances with reasons,
keeping per-directory control inline instead of adding heap allocation. No runtime
sequence changes. Local suites, lint, source/pin equality and paired Release harness
were repeated for the new exact snapshot. Hosted final-head validation is separate.
