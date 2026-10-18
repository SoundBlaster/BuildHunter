# Stack CI activation — 2026-10-18

Applied the additive main ruleset first (required `Stack CI gate`, GitHub Actions
app 15368, strict up-to-date checks), then enabled `STACK_MACOS_CI=true` and
dispatched the publisher from main. Existing branch protections and bypass actors
were preserved. Bootstrap publisher succeeded. The old #84 head was left pending
for missing/ambiguous plan receipts; previous green CI is not reused as new proof.

Native canary validation is pending. No measured savings or merge acceptance is
claimed here. These are administrative observations, not runtime scanner evidence.
CI maps and the draft delivery contract retain their current roles/requirements.
