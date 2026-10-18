# Guarded session integration validation

Baseline main: 404458276e3082fc37d302cdc0708a827e5ecc2e.
Candidate runtime/pin commit: 9a0e363ea7b36e57b93d7f5d5ecffa6e0319d6de.
Upstream core: 9f445cf8468e2f341d8e8e20782854fb391d5979 (19 exact files).
Source hashes, upstream byte comparison and effective TB5 paths are recorded here.

Default Debug: 72 tests. All-features Debug/Release: 77 each. Vendored core Release:
92 plus one compile-fail doctest. Independent plugin scanner: 12. Compiled Release
CLI: 13. Vendor integrity: 4. Formatting, all-target/all-feature clippy, HC parse and
seven profiles pass. SpecPM has zero errors and 32 unresolved external-capability
warnings. Initial standalone vendor --manifest-path --locked attempt lacked its own
lockfile; corrected to root cargo test -p hunter-fs-core --locked. The initial
failed invocation is retained separately; no vendor lockfile was added.

Paired Release harness: seven interleaved warm samples against the same fixture,
eight noise-adjusted regression budgets PASS. Source-tree median 18.808 -> 13.613 ms;
many-roots 162.159 -> 161.224 ms. First-artifact medians also vary and remain within
budget. These small warm fixtures establish no general speedup, CPU/RAM bound or
home-directory/cloud/UI acceptance. The benchmark dirty flag records docs/maps and
receipts only; performance-inputs.json proves all runtime/dependency/harness inputs
match the immutable candidate runtime commit. Old receipts remain historical.
