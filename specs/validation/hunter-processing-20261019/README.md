# Entry processing integration validation

Baseline main defea70cf2a6daee4b380ba8e3862be4b39ad86d; immutable candidate runtime
979b6b89c9772005bf764db72ffdc4cb046247f1. Exact upstream snapshot:
2031df50f8324eaaf91856c5dc382d35694b2fd4. All 21 vendor files compare byte-for-byte;
39 Rust/Cargo/vendor/harness inputs match the immutable candidate commit.

Local default Debug: 72 tests. All-features Debug/Release: 77 each. Vendored Release
core: 106 plus one compile-fail doctest. Independent plugin scanner: 12. Compiled
Release CLI: 13. Vendor integrity: 4. Formatting, all-target/all-feature clippy,
Hypercode parse/seven profiles pass. SpecPM: zero errors, 34 unresolved-capability
warnings. Initial lint run requested is_multiple_of at two moved checkpoint sites;
the runtime checkpoint applies that equivalent predicate. Initial and final command
outputs, source hashes/tool versions and effective TB5 paths are retained.

Seven new upstream witnesses cover original child slots, stop/prefix cleanup,
empty/panic routing, metadata precedence/cached errors/slot consumption/fallbacks,
empty resolution and panic cleanup. Existing product tests retain their assertions.
Product classification, access/cancel policy, ancestry, byte mode/profile updates,
completion status and event assembly remain in host adapters. No extra threads,
stop probes, metadata retry or directory enumeration are introduced. Directory
inclusion and selected-entry types remain host obligations; callbacks run before
owned selected/cached buffers release. The caller retains guard responsibility.

Paired Release harness: seven interleaved warm fixture samples against main; all
eight noise-adjusted budgets PASS. Source-tree median 3.435 -> 3.267 ms; many-roots
80.340 -> 79.544 ms. This establishes no general speedup, CPU/RAM bound or fresh
home/cloud/UI acceptance. The candidate dirty flag contains docs/maps/receipts only;
performance-inputs.json proves runtime/dependency/harness equality to the immutable
commit. Historical evidence stays historical. Engine facade, opaque branch-state
composition, descriptor recursion, plugin migration and resource budgets remain
pending. New recovered constraints remain draft; hosted CI is separate evidence.
