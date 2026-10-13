# Structural validation — module recovery 2026-10-13

From BuildHunter repository root, implementation baseline
`48badf9e097783505151dd47fbafe290f0af25e2` plus separately hashed descriptive inputs.
[commands.json](commands.json) contains the actual argv, tool identities,
exit codes, stdout and stderr; [specpm.json](specpm.json) preserves full diagnostics.
The current/future maps and input identities are pinned in
[source-identity.json](../../evidence/modules-20261013/source-identity.json).

## Actual checks

- Hypercode parse of current and target maps: success.
- Hypercode validation/cascade/contracts for `client=cli`, `client=gui`,
  `edition=public`, `edition=store`: success, no reported issues.
- SpecPM 0.2.0 package validation: zero errors. Dependency warnings are retained
  in the JSON; exact count and per-boundary paths are authoritative there.
- `git diff --check`: success.
- Companion review: local Markdown links, source binding paths, claim IDs,
  constraint evidence references and baseline/map hashes checked separately.

`required_capability_unresolved` warnings are expected for declared dependencies,
including dependencies whose contracts are listed in this package: the current
MVP validator reports every requirement without solving it. Platform filesystem
metadata is a real external host capability. Do not remove required capabilities
to silence these warnings. GUI-only bridge and Store-only Arsenal dependencies
are optional at the generic package level, with profile obligations described in
the contracts/matrix; the validator does not enforce those conditions.

The installed SpecPM entrypoint imports the inspected local SpecPM source; its
executable/core/RFC hashes and source revision are recorded. Hypercode was built
from its clean local checkout in Release for these checks; the executable hash
is recorded independently of the source checkout. Untracked SpecPM browser
artifacts are listed in tool dirty state and were not used as schema authority.

Initial HC authoring used an unsupported range spelling and comma-separated
selectors. Those failed checks prompted narrow syntax corrections to documented
integer bounds and individual selectors; the saved successful runs validate the
final supported syntax. No checks were bypassed or represented as passing.

## Limits

These checks establish supported syntax, property contracts, package structure,
paths and declaration consistency. They do not establish dependency availability,
module import enforcement, ABI correctness, runtime ownership, memory budgets,
semantic CLI/GUI parity or Store approval. Current test source is a declaration
of intended checks, not a fresh pass result.

No BuildHunter application build/test, filesystem scan, plugin runtime execution,
private repository access or Store release was performed in this pass. Historical
CI and diagnostics stay historical. New module contracts remain drafts.
