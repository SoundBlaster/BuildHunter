# PR #68 diagnostic delivery review fix

Review: [discussion_r4268261516](https://github.com/SoundBlaster/BuildHunter/pull/68#discussion_r4268261516).
Ambiguous/Indeterminate diagnostics previously waited for a core event, commonly
Finished. The integration now invokes its single serial observer while processing
the candidate, with no retained pending-event queue and no state borrow across
the consumer callback. Report diagnostics remain retained; no RSS bound is claimed.

[TDD red witness](red-check.json): the new diagnostic-only cancellation regression
failed before the fix (Completed instead of Cancelled). Final test covers both
Ambiguous and Indeterminate and requires exactly one diagnostic followed by one
Cancelled terminal event, no artifacts/warnings, and no child classification.

[Final command receipts](commands.json): on macOS ARM64, all 5 scope + 12 scanning
tests pass in Debug/Release; formatting/clippy and both HC maps/four profiles pass.
[SpecPM](specpm.json): 0 errors and 15 unresolved-capability warnings.
[Fresh identity](source-identity.json) and [frozen maps](../../evidence/plugin-diagnostics-review-20261014/maps/)
replace current proof without rewriting historical records.
No default-client, live cloud, UI, home-scan or performance run is claimed.
Hosted CI on the new head remains separate/pending at publication.
