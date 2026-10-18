# Stack CI activation — 2026-10-18

Applied the additive main ruleset first (required `Stack CI gate`, GitHub Actions
app 15368, strict up-to-date checks), then enabled `STACK_MACOS_CI=true` and
dispatched the publisher from main. Existing branch protections and bypass actors
were preserved. Bootstrap publisher succeeded. The old #84 head was left pending
for missing/ambiguous plan receipts; previous green CI is not reused as new proof.

Native canary validation is pending. No measured savings or merge acceptance is
claimed here. These are administrative observations, not runtime scanner evidence.
CI maps and the draft delivery contract retain their current roles/requirements.

## Provenance and limits

`acquisition.json` records replay commands, acquisition times and tool identities.
`variable-details.json` and `ruleset-current.json` retain server timestamps: the
ruleset update precedes creation of the opt-in variable. `mutation-order.json`
compares these timestamps; this establishes this activation's observed ordering,
not a claim about every possible concurrent administrator. `publisher-output.txt`
contains timestamped bootstrap step diagnostics, including #84's pending reason;
`status-84.json` is a separately timed re-read, not relabelled as the initial result.
The original before/proposal/bootstrap snapshots remain historical.

`canary-first-head.json` records successful CI for the first tip 877d2c7. That
head's full preflight was rejected for unresolved review threads; it is not final
acceptance of the revised tip. Draft/ready plans, blocked merge state, prefix and
pending-macOS rejection probes are preserved as scoped first-head observations.
Current CI map properties now describe repository activation; runtime maps and
scan/plugin contracts remain unchanged. Final hosted acceptance and measured
savings are still pending. No credential values are recorded.

The Hypercode executable does not support `--version`: that identity probe's exit 1
is retained in `review-validation.json`. `hypercode-identity.json` records its binary
SHA-256 instead. All map/profile, SpecPM and offline-test validation commands pass;
the unsupported version flag is not reported as a failed conformance check.
