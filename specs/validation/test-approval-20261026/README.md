# Local XCTest identity and smoke verification — 2026-10-26

Implementation: `c80aeee967ccb295c4e3bacb55050d1b8d9f9710` (runtime behavior remains at the previous
profiling revision; per-file hashes identify this build/test follow-up).

This follow-up changes test input/query selection and UI-runner signing, not
application/runtime semantics. Current inputs are pinned by source-identity.json.
Full logs and `.xcresult` bundles remain in the per-worktree TB5 build directory
listed in commands.json; compact results and hashes are preserved here.

## Observed failure and correction

The editor test originally inserted `{` into a valid JSON title instead of
replacing the document. Native Edit → Select All plus typing into the current
responder now asserts exactly `{` before checking rejection and draft retention.
The real-folder test previously matched Open Folder in two windows; it now scopes
the action to one empty window. A later retry showed an empty restored companion
window during an unhandled interruption. A temporary launch-helper probe was not
retained: TCC logs subsequently identified the pending identity-transition
approval, so that retry cannot establish a window-lifecycle defect. These test
failures do not establish a validation failure or denied scan-folder access.

TCC logs show SystemPolicyRemovableVolumes prompts for
`dev.buildhunter.uitests.xctrunner` after an existing cdhash requirement failed to
match the rebuilt ad hoc runner (`tcc-before.json`). The app already used Apple
Development signing. Local command overrides first verified that signing the
runner with the same development identity works. The UI-test target now uses the
same identity/team/style in XcodeGen source and the regenerated project. CI's
unsigned command override is unchanged.

Local test execution unsets the compiler TMPDIR override; the real-folder fixture
was observed under the runner's internal `Library/Containers/.../Data/tmp`.
Build outputs, dependency caches and result bundles remain on verified TB5
(UUID 04B9F65E-894C-4A7C-B517-FC542E8D8E36). No TCC reset, Full Disk Access grant,
Gatekeeper bypass or automatic permission approval was performed.

## Executed checks

The initial signed run passed all 3 selected UI tests and all 153 unit tests.
A generated-project rebuild retained development signing without command-line
identity overrides. Its UI retry was interrupted after the restored diagram
obscured the Open action; it is recorded as interrupted, not passed. TCC
subsequently confirmed that the transition from the old ad hoc code requirement
to development signing still needed one user approval. A later retry waited
during app launch and was interrupted rather than repeatedly generating runs.
The final current-source build-for-testing passed without identity overrides.
The subsequent unit retry was interrupted after result-bundle preparation, before
confirmed test bodies; its cause is not established by this observation. The
earlier 153-test unit pass is retained with its exact command and signing mode.
The follow-up results are in commands.json; structural-commands.json records
current/target HC checks and SpecPM validation. Earlier interrupted attempts are
not relabelled as passes.

## Contract and evidence scope

Affected draft: `buildhunter.current.templates/draft_validation`; the focused
native test demonstrates Check and invalid-draft retention, not every contract
clause, file exchange, all UI flows or Store approval. MacApplication assembly
folds test-host signing; no new runtime role/capability is introduced. Current
and target HC topology and all runtime contracts/bindings remain unchanged;
source-identity.json pins both maps separately from changed test/build inputs.

The signed runner retains an identical designated requirement across two
configured rebuilds (signing.json). TCC still reports the old ad hoc requirement
and one new prompt for the development identity (tcc-after.json). Until the user
approves that request and a subsequent run completes, prompt persistence across
launch/rebuild is not empirically established. Stable signing avoids changing
identity with each rebuild. This is not a promise
that a clean account, a different identity/certificate, every macOS version or
future protected-resource access needs no initial user approval. The two local
changes were applied together; this is not an isolated TMPDIR-versus-signing
causality benchmark. Read-only TCC logs and actual signatures identify the
observed repeated-prompt mechanism.
