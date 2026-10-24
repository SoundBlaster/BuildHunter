# Initial Rifle validator/evaluator validation — 2026-10-14

[commands.json](commands.json) preserves exact executed commands, outputs and tool
identity. [source-identity.json](source-identity.json) pins the working-tree source,
tests, schema, maps and specifications relative to bootstrap commit 4f5798f.

- 15 Rust semantic integration tests passed in Debug and Release (all targets).
- fmt and clippy (`-D warnings`) passed.
- The runnable example returned Unknown with the inaccessible `project` fact.
- Five CI classifier tests passed; both public fixtures pass the copied schema.
- Hypercode parse/validation passed. SpecPM: zero errors and zero warnings.
- Local YAML parsing/wiring, document links and source bindings were checked.
  GitHub CI has not yet supplied Linux/macOS/Windows evidence for this PR.
- Full staged change whitespace is checked before commit.

The first test run preceded the implementation and failed due to missing public
API imports; final commands here record the completed source. A later test helper
was corrected to accept the empty document JSON pointer. Neither initial failure
is represented as a successful run.

Limits: local macOS ARM64 evidence, not cross-platform runtime certification.
No filesystem integration, benchmarks, Swift SDK, JCS or catalog behavior was
executed. The stored HC map is composition, not a call graph. The public program
schema is a byte-identical copy from BuildHunter 79fa493; this tests the pure
program slice only. Source and tests are new Rifle code, not claims that Hunter
or the BuildHunter adapter have been extracted.
