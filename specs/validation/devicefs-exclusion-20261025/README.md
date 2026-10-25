# CoreDevice/DeviceFS exclusion — 2026-10-25

Baseline: BuildHunter main `ed1df23b57d7be89ef8b0ac7d29d6f0aa6e8a23c`.
Runtime commit: `4a022516eb8e0e6ed11c75f4b10c8d61ed4a1d0d`.
The previous Release home run had 12 warnings under DeviceFS: six I/O errors and
six unavailable plugin classifications. `mount` on this Mac confirms a `devicefs`
filesystem at the standard CoreDevice path. Raw home/device paths stay local.

## Change and claims

Known per-user CoreDevice/DeviceFS paths now use the existing managed-location
admission before core root probes or recursive child routing/classification.
The lexical namespace policy applies even when unmounted and does not detect
arbitrary mounts or resolve links. An explicit warning keeps the result Incomplete;
affected artifact ancestors retain partial status. Errors elsewhere retain their
original meaning. Hunter, public/event ABI and default marker semantics are unchanged.

Current scan `managed_locations`, `root_admission` and `root_backend` claims and
StartupAdmission/Traversal map properties identify the change. Other current and
target contracts remain unchanged. Source/map hashes are separate from executable
identities; these remain descriptive drafts rather than owner acceptance.

## Verification

- Debug and Release all-feature Rust suites: 93 library + 5 CLI/profile tests each,
  zero failures. Five new regressions cover exact namespace boundaries, selected
  root/descendant zero core probes, recursive admission, both catalog client
  profiles without marker/diagnostic work, and a local lookalike build artifact.
- Release CLI build, all-target/all-feature Clippy with warnings denied, formatting.
- CLI integration: 14 tests; frozen migration parity: 3 tests, all passed.
- Focused Release CLI on the actual mounted DeviceFS and through its parent:
  one intentional skip warning, zero diagnostics/findings, Incomplete/exit 1.
  Selecting DeviceFS produces zero retained listing entries; its parent contains
  one listed child. Scan time is 1 ms per focused case on this warm machine;
  these are not whole-home speedup measurements.
- A temporary project lookalike still yields its one artifact with exit 0 and no
  warnings. Live aggregate results and binary hash are in `live-results.json`.
- Current/target Hypercode profiles and SpecPM structural checks are recorded in
  `structural-commands.json`; SpecPM reports 0 errors and 49 unresolved external
  capability warnings, retained in full in `specpm.json`.

The CLI canonicalizes/validates the selected directory before calling the core.
Zero core probes does not mean zero I/O by the client, folder picker or OS.
There is no new full home/GUI scan, RSS comparison or Store/provider acceptance.
The earlier home/Photos measurements are preserved under their original identity.
Native CI and review must validate the new PR revision before merge.
