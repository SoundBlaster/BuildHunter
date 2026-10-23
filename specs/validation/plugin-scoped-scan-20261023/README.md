# R3 plugin descriptor selection — 2026-10-23

The independent plugin scanner selects the pinned Hunter ScopedFilesystem on
macOS/Linux. The default CLI/GUI policies/backend and numeric FFI stay unchanged.
Root observation, listings, worker sizing, coordinator fallback and markers share
one selected directory anchor. Outside/invalid paths and intermediate links are
refused; final links are observed as links. Prepared passes retain their directory.

Executed locally on arm64 macOS: 16 plugin tests in Debug, Release, and Release
with build-hunter/macos-bulk-metadata. Normal and feature-enabled clippy, rustfmt,
vendor identity, dependency graph, change-classifier/stack-gate tests, all current/
target/CI HC profiles and SpecPM passed. SpecPM: zero errors; 40 unresolved
capability warnings remain explicit. YAML duplicate keys and the new macOS-only
step condition were checked; actionlint was unavailable, so not executed.

Four new integration fixtures replace the root, intermediate parent and a listed
file from the actual discovery callback, and size a file with unreadable contents.
They assert original-tree bytes/no foreign target bytes and appropriate partial/
change status. The unreadable-file fixture includes a nested worker directory
and profiling: pathname bulk passes/fallbacks must remain zero even with that
feature enabled. Upstream tests separately prove parallel stream offsets and
retention across regular replacement within an opened measurement pass.

Bindings: PluginScanning and the significant external HunterScopedFilesystem
boundary; ScanScopedAccess/Measurement remain folded into buildhunter.current.scan.
The CI bulk-profile case is folded into existing MacOSChecks; scheduling, job names
and native ready-tip gates are unchanged. Draft contracts/target acceptance remain
separate from these execution receipts. Source/map/input SHA256 and full commands
are included; raw logs are on TB5 with recorded digests.

Limits: no cross-pass snapshot/inode ledger, Windows/other Unix acquisition, live
iCloud provider evidence, durable action identity, fresh whole-home CPU/RSS bound
or speedup claim. R4/C3/C4 decisions and R5 budgets remain separate. macOS app UI
acceptance is a separate CI/manual gate; local tests here exercise Rust integration.
