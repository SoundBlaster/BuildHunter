# Repository instructions

Maintain architecture maps and SpecPM contracts with implementation changes.
Use installed hypercode-architecture-map, specpm-author-spec and specpm-review-spec
skills. Read specs/specification-progress.md and relevant contracts first.
Keep accepted product direction, proposed API, observed behavior and executed
validation distinct. Preserve historical evidence. Map bindings must refer to real
paths; structural validation alone does not prove runtime conformance.

Rifle is independent of Hunter and BuildHunter: no filesystem probes, artifact
vocabulary, product target resolution, plugin permissions or UI in the evaluator.
The host supplies facts. Missing inputs are errors; unavailable facts are unknown.

Use rtk-prefixed shell commands; GitHub Git remotes use SSH. Preserve unrelated
changes. Use no-wait for long builds/checks. Run fmt, tests and clippy for Rust
changes, and record relevant architecture/spec validation. Never claim unrun CI.
