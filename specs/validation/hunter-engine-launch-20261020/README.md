# Engine launch integration validation

Baseline main 65c5b2ffd38d419aaf3541bfd594882e13a0832d; immutable candidate runtime
e8e670452374c1cf5599bce10431c71516a1f707. Exact upstream core snapshot:
40c0f5384c70879f529c6069d835dafff316567a. All 23 vendor files match byte-for-byte;
41 runtime/dependency/harness inputs match the immutable candidate commit.

Local default Debug: 72 tests. All-features Debug/Release: 77 each. Vendored Release
core: 118 plus two compile-fail doctests. Independent plugin scanner: 12. Compiled
Release CLI: 13. Vendor integrity: 4. Fmt, all-target/all-feature clippy and Hypercode
parse/seven profiles pass. SpecPM: zero errors, 35 unresolved-capability warnings.
Raw commands/tool versions, source/map hashes and effective TB5 paths are retained.

Four upstream launch witnesses cover original channel ownership, receiver-before-
collection, host-only state release before helper stop/queue close/join, non-Send
host/result, factory panic, collection panic with outer backend/guard cleanup after
join and declined root. Existing host profile/guard/cancel/callback/worker tests keep
their assertions. Product collect extracts the same report tuple before join; final
Profile/Finished and status precedence stay after join. Root/child/measurement
algorithms, backend selection, classification and public C ABI are unchanged.

Paired Release harness: seven interleaved warm samples against main; all eight
noise-adjusted budgets PASS. Source-tree median 5.956 -> 5.789 ms; many-roots
118.458 -> 102.987 ms. Results establish the configured fixture budget only, not a
general speedup, CPU/RAM bound or fresh home/cloud/UI acceptance. The candidate dirty
flag contains docs/maps/receipts only; performance-inputs.json proves exact runtime/
dependency/harness equality. Prior receipts remain historical. Common engine startup
composition is implemented; backend choice/factories, descriptor recursion, default
plugin migration and C5 budgets remain separate. New contracts stay draft; hosted
final-head checks are separate evidence.
