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
