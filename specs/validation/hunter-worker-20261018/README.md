# Directory worker protocol validation — 2026-10-18

Baseline: BuildHunter 39e8275f7b68dddce4e1be71e7c75d0eba1d9fa6.
Runtime commit: a2985eeb8e204ded3e9037b4e6d719d637c5d524.
Canonical Hunter: a1b30a7967ed40f146bf4540af7fcdda4e3809b4.

Local macOS tests: 67 default (63 library + 4 CLI), 71 all features (67 + 4)
in Debug/Release; vendored core 61 in both; plugin scanner 12 in both; CLI script
13; vendor verifier 4; isolated fresh-cache offline CLI; clippy/fmt all pass.
The new integration test verifies rejected listing delivery suppresses metadata
and bulk preparation, preserves the next queued task and restores the macOS I/O
policy. Existing listing-before-measurement, cancelled-prefix, worker/callback
panic, profile timing, bulk, marker, native root and scope/join tests remain gates.
Seven new upstream protocol tests include borrowed non-Send state and non-Copy IDs.

All eight paired Release fixture budgets pass (seven samples each, warm shared
fixtures, alternating order). many-roots median 75.287 -> 75.149 ms and source-tree
3.164 -> 3.212 ms are comparative witnesses, not a claimed speedup or RAM improvement.
Full command/sample outputs and budget definitions are preserved in the reports.
This is not a live home-tree, cloud-provider or local native UI run.

SpecPM: 0 errors, 25 external-capability resolution warnings; four HC profiles
parse/validate. These are structural checks, not runtime/owner acceptance. Maps and
new upstream draft are frozen separately. Binding paths exist at the immutable
runtime commit and match checked files. All twelve snapshot files match the
pinned upstream Git objects/hashes; independent scoped-metadata pin is unchanged.
Compiler output and temporary fixtures use separately verified TB5 targets for
root and plugin package. Only actual output/fixture locations are recorded.

Marker/prefetch interpretation, bulk/native effects, guards, setup, receiver/queue
lifetime, joins and final status remain host-owned. Full engine/startup extraction,
resource budgets and default plugin migration remain pending. Drafts stay draft;
prior receipts remain historical. No UI/home/cloud or memory acceptance is implied.

The performance harness ran at the immutable runtime revision with documentation
changes present (`candidate_dirty=true`). Runtime binding files and Cargo manifests
were byte-checked against that commit; documentation does not change measured code.
