# Pure plugin adapter validation — 2026-10-14

Baseline: 998b1319fc2761a30f4f1db4df8d9bf35548f6fa.
Rifle dependency pinned to merged 693a284123faaa1ad3192cd88b259ba97a7b5910.
[commands.json](commands.json) records fresh local commands/tool identity/results;
[source-identity.json](source-identity.json) pins the implementation/doc/map inputs.

Default Rust suite and all-feature suite remain regression checks; the new pure
adapter has 15 tests covering closed validation, trust origin, references/types,
exclusions/probes, conflicts/unknowns, source ordering, symlinks and exact budgets.
Release adapter tests, default Release build, CLI integration and fmt/clippy are
recorded separately. Both HC maps and four profiles validate; SpecPM has zero
errors and 13 unresolved capability warnings (the prior 11 plus two required
external Rifle capabilities). Its MVP validator does not resolve these; obligations
are preserved and pinned integration tests are separate evidence.

Initial tests failed because the adapter did not exist, then compile checks exposed
Rifle's crate-private Error constructor and an existing test's ambiguous sum type.
BuildHunter now constructs its shared error data locally and spells sum::<u64>();
no library API or production scanner behavior was changed to mask the failures.

Limits: local macOS ARM64 Rust/CLI/policy evidence. No Swift/Xcode build, UI tests,
home scan, performance claim, filesystem-probe policy or Store release was run.
The current client scanner remains the legacy implementation even with the new
feature enabled. No complete basic-plugin parity is claimed. BuildHunterMAS name
and private visibility were confirmed; only its name/existence, not private code,
is reflected here. Original Xcode scheme changes stay in the primary checkout.
Full staged whitespace is checked before commit. Historical evidence stays intact.
