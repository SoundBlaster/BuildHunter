# Repository instructions

## Maintain architecture maps and specifications with code

Start from `specs/specification-progress.md`, `specs/hypercode/README.md`,
`specpm.yaml` and the relevant BoundarySpecs/claim matrix. Apply installed skills
`legacy-to-specs`, `hypercode-architecture-map`, `specpm-legacy-boundaries`,
`specpm-author-spec` and `specpm-review-spec` for specification work, plus the
relevant implementation skills.

For every feature, fix or refactor:

1. Inspect actual source state and identify affected roles, consumers, profiles,
   claim IDs and parent assembly obligations before changing code.
2. Maintain current maps and contracts alongside implementation. Recover a
   missing significant boundary or document why details are folded/excluded.
3. Refresh source/map identities, bindings and provenance. If a contract remains
   unchanged, record the affected IDs and reason in the progress/review record.
4. Keep observed behavior, desired requirements, owner acceptance and evidence
   distinct. New recovered BoundarySpecs stay draft until explicitly accepted;
   never weaken an accepted requirement to conceal an implementation conflict.
5. Preserve historical snapshots/results. Invalidate evidence affected by source
   changes and add new evidence; do not relabel old test results as current proof.
6. Run applicable HC parse/validation for affected profiles and SpecPM validation.
   Record actual tool identity, commands, diagnostics and limits. Structural
   success does not prove runtime conformance. Add/run application tests only
   within the authorized task scope.

Do not finish with affected current specifications knowingly stale. Maps describe
composition; they do not execute application policy or authorize effects.
Keep current source maps separate from the target Hunter/Rifle/BuildHunter/Arsenal
design; target module names do not imply existing crates/repositories.

## Workspace

- Preserve unrelated user changes, especially local Xcode scheme settings.
- Prefix shell commands with `rtk`; use `rtk proxy` for unsupported commands.
- Use `no-wait` for long operations and report meaningful state changes.
- GitHub Git remotes use SSH; retain existing HTTPS SwiftPM dependencies.
- Do not create private repositories, transfer code, publish Store builds or
  accept new recovered requirements merely because a design document lists them.

## Stacked PRs and macOS CI budget

This is the agreed workflow policy. Stack-aware macOS gating and its required
merge gate still need a separate implementation PR. Until they are deployed and
verified, keep every existing required check: this document does not authorize
skipping checks, treating pending jobs as passed, or weakening branch protection.

- Group consecutive, reviewable refactoring slices into bounded native stacks
  within each repository (typically 3–5 layers), rather than waiting for a full
  macOS run and merging after every small slice. Keep review diffs focused.
- Hunter and BuildHunter have separate stacks; cross-repository dependency links
  are not a native GitHub stack. Pin and validate the exact Hunter revision in
  BuildHunter, and merge the dependency before its consumer.
- Run inexpensive Linux checks on every layer: applicable Rust/CLI, schema,
  formatting/lint and vendor-integrity checks. Keep review coverage on every layer.
  Do not remove Hunter's inexpensive platform tests just to reduce BuildHunter's
  costly Xcode runs.
- Once stack-aware gating is implemented, run expensive macOS UI, Release and
  performance jobs on the ready top layer containing the complete batch. Keep the
  batch in draft while assembling it so each temporary top does not start a full
  run. Standalone ready PRs must still receive applicable macOS validation.
- Use verified native stack metadata (`stack.position == stack.size`) for top-layer
  selection. Missing metadata on an ordinary PR must not skip its checks; uncertain
  classification must request validation or block merge, never assume success.
- A required inexpensive merge gate must verify that the current stack tip has
  successful applicable macOS checks for the exact tested revision and base, and
  that every selected layer has its required checks/reviews. Invalidate evidence
  after relevant source, dependency, base or CI-configuration changes. Do not reuse
  an earlier tip's green result or equate an intentionally skipped job with a test
  pass. Avoid whole-workflow filters that leave required checks permanently pending.
- Merge the tested batch using stack-aware tooling only after final CI and GraphQL
  review-thread checks. A green tip validates the combined state, not every
  intermediate state: do not merge a lower prefix separately unless that prefix
  has its own applicable macOS validation. Do not bypass required approvals/checks.
- Cancel obsolete runs for the same validation target. Avoid duplicate expensive
  runs on every layer and every intermediate merge; define post-merge coverage
  explicitly in the CI implementation rather than silently disabling main checks.
- Documentation-only policy changes should use a separate docs-only PR when adding
  them to a runtime PR would retrigger its cumulative heavy-change classification.
  Never hide source changes from the diff classifier to save runner time.

References: [GitHub CI optimization for stacks](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/optimizing-ci-for-stacked-pull-requests),
[stack requirements](https://docs.github.com/en/pull-requests/reference/stacked-pull-requests).
