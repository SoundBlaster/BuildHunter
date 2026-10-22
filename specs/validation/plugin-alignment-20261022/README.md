# Plugin dependency alignment validation

Baseline runtime main 6dcf9ba91e67c7d5f3270369d05e03d01da1db5e;
lower documentation layer 5abb07df5b250f394dfd9988113325794ba06f6b.
Facade and exact 27-file core snapshot now share upstream revision
c5354e973b5d1d90c438c94b78c06898ffe62500. A scanner-workspace Git-source
patch resolves transitive core to the existing verified vendored path. Rifle
remains at 693a284123faaa1ad3192cd88b259ba97a7b5910; no version bumps.

Policy: 15 tests in Debug and Release. Scanner: 12 integration tests in Debug
and Release, including scoped markers, links/replacement, cancellation and
arbitrary target routing. Both fmt and all-target clippy pass. Eight graph
regressions reject duplicates/stale pins/wrong sources and exercise Unix and
Windows graph shapes. Actual locked/offline host metadata confirms one path
core, separate policy-only dependencies and no Rifle in default CLI. Filtered
Linux/Windows graphs were also checked, without claiming target execution.

The default CLI check/install/help regression passes with a fresh Cargo home
and no network, with temporary outputs on TB5. Four vendor tests/integrity pass.
Seven HC parse/profile checks and final target-profile checks pass. SpecPM has
zero errors and 39 existing unresolved-capability warnings. These are structural
checks, not requirement acceptance. Host runtime/default and policy manifests/
lockfiles and scanner implementation sources are unchanged. ScopedMetadata
bytes match the former facade pin. New graph assembly is scoped to repository
workspace roots; external consumers must own their Cargo patches.

Raw commands, selected graph identities, per-file input hashes and actual TB5
routes are retained. Fresh-cache fixture scratch products are removed by the
existing test harness. CI now runs graph checks on all three Rust platforms;
actual Linux/Windows build and macOS UI/Release CI are separate pending evidence.
No live home/cloud scan or fresh performance/RSS/latency claim. Historical audit
and receipts remain historical; C2/C3/C4/default client migration stay open.
