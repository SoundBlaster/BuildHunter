# R5 engine and retained reports — 2026-10-24

The application pins Hunter `286cebffdac7f1396a8d3dffed8a7d54b2ea34eb` (#27/#28/#30).
Default limits and estimated-payload scope are documented in
`docs/scan-resource-budgets.md`. Raw filesystem/marker compatibility is unchanged.
Fresh source identities are separate from historical R4 evidence.

Debug/Release all-feature checks passed: 87 library tests and 4 CLI/profile unit
tests; all-target/all-feature Clippy passed. The offline snapshot verifier's 4
regressions passed. HC current/target/CI parse and applicable bare/CLI/GUI/public/Store
profiles passed. SpecPM structural validation passed; unresolved capabilities are
reported in the saved JSON, not runtime acceptance.

New regressions exercise finding/count payload rejection, oversized listing prefix,
scoped acquisition with the same limits, queue rejection with rendezvous responses,
busy producer cancellation/join, warning/diagnostic quotas, reserved limit summary,
partial completion and Finished ordering, and ordinary within-budget parity.
An initial test manifest incorrectly shared a rule between targets; separate rules
fixed the fixture. The initial Clippy large-error return was removed from the
private sender adapter. Current recorded runs use corrected sources.

Profile pending retention, GUI/export disclosure and fresh CPU/RSS/latency remain
next-layer obligations. This receipt does not establish an RSS ceiling, whole-home
performance or C5 owner acceptance. No heavy hosted jobs are requested while draft.
