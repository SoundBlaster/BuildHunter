# Owned traversal coordinator validation

Baseline main aee3c7691045322b81474cda4df950972131deb7; candidate runtime/pin commit
a8c9c82f39c6591095fbf6f5d06ccf3f2029b6b5. Exact upstream core snapshot:
10268f69c41890bb23ab4cdc33d25f3aac8e46d1, all 20 files compared byte-for-byte.

Local default Debug: 72 tests. All-features Debug/Release: 77 each. Vendored Release
core: 99 plus one compile-fail doctest. Independent plugin scanner: 12. Compiled
Release CLI: 13. Vendor integrity: 4. Formatting, all-target/all-feature clippy, HC
parse/seven profiles pass. SpecPM: zero errors, 33 unresolved-capability warnings.
Source hashes, command outputs/tool versions and effective TB5 paths are retained.

Seven upstream witnesses and existing product guards, setup-failure, root/symlink,
callback/worker panic, speculative measurement and profile/cancel gates cover this
boundary. Scripted host tests now read the owned coordinator outcome instead of
accessing the former private Scanner ledger; their assertions remain unchanged.
Ledger release is now after handler finish/before coordinator return, followed by
unread receiver release before enclosing worker join. Product status, classification,
child routing, selected-size membership and final event/report assembly stay local.

Paired Release harness: seven interleaved warm fixture samples against main; eight
noise-adjusted budgets PASS. Source-tree median 3.188 -> 3.159 ms; many-roots
74.006 -> 73.906 ms. This establishes no general speedup, CPU/RAM bound or fresh
home/cloud/UI acceptance. The candidate dirty flag contains docs/maps/receipts only;
performance-inputs.json proves all Rust/Cargo/vendor/harness inputs match the pinned
runtime commit. Historical receipts remain historical. Full engine assembly and
resource budgets remain pending; new recovered constraints remain draft.
