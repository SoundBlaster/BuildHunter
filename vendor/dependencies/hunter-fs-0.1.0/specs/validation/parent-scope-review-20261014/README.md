# Parent scope review fix

Regression first failed for root/.. + Parent: Known(true), metadata attempted.
Candidate components now validate before Parent selection. Both Own/Parent reject
root/.., child/.. and child/../child as OutOfScope without metadata attempts.
9 local macOS tests pass Debug/Release; Linux has one additional raw-byte test.
Clippy/fmt/HC pass; SpecPM 0 errors, 1 unresolved host capability warning.
Composition map roles are unchanged; scope constraint and source identity updated.
The foreignArtifacts review was a terminology false positive: it is the standard
SpecPM documentation field, not build-artifact classification vocabulary. AGENTS
clarifies this distinction without weakening the product independence rule.
