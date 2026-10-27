# Main template verification — 2026-10-27

Runtime baseline: `c9040bbd5f8fbfc6095f93a640e089c888ae9537` (merged PRs #120/#121).
The Debug and Release builds and all 158 unit tests passed. Computer Use drove the
fresh signed Release app on the Mac; no result below is inferred from a model or
from a build alone. Per-file hashes in source-identity.json distinguish the runtime
baseline from the changed UI-test source. Raw logs/result bundles stay in the
per-worktree TB5 directory `main-template-verification-20261027T0851`.

## Observed Release behavior

- Imported one ordinary JSON file from a temporary local folder and opened it in
  Settings → Templates. Check returned a valid 14-target catalog.
- Replacing the editor contents with `{` returned an EOF/invalid_shape diagnostic.
  The app remained usable; Reload asked before discarding the unsaved draft.
- Export JSON wrote a byte-identical copy without adding a second registration.
- A positive/negative Forge fixture returned only `positive/forge-out`, 8192 bytes.
  The negative sibling lacked the project marker and did not match.
- The Rule Profile window showed 28 phase rows. The custom resolved row recorded
  2 evaluations, 1 true and 1 false; the preliminary row recorded 2 unknowns.
- A home-folder scan was explicitly stopped. The main window showed stopped /
  partial results; the separate Rule Profile window showed Cancelled / partial
  metrics, with 455550 preliminary calls and 79698 resolved calls.
- GUI JSON exports retained catalog identity and all 28 rows. The completed
  fixture report uses report status `complete`; the stopped report uses `stopped`
  and its nested rule profile uses `cancelled`. Both nested profiles pass the
  checked-in JSON Schema. Raw home paths/reports are kept local; hashes are in
  private-artifacts.json and aggregate assertions in export-checks.json.
- Replacing the connected on-disk JSON with `{` blocked the next catalog and
  showed a warning with the file path and parse error. No silent built-in fallback
  or crash was observed. The fixture bytes were restored afterwards.
- The test-only registration was disconnected and rule profiling returned to its
  observed initial Off state. Source files remain on disk. No privacy permission,
  Gatekeeper, Automation or SIP setting was changed.

A first home scan finished before Stop could be clicked and was incomplete with
warnings. This pass is not a full-home correctness/performance claim, a memory
ceiling, or a Store validation. TestFlight's notification banner also appears in
an XCTest recording; no other app's UI/settings were operated.

## UI test failure and bounded repair

On the unchanged main baseline, the 3-test smoke passed the invalid-draft and
profile-panel tests. The real-folder test failed during synthesized path entry.
Its one isolated retry then failed to reach Scan complete. The latter recording
and XCTest interruption diagnostics show an empty Artifact Diagram window over
the chooser; the fixture had not been selected. Raw video/snapshots stay local.
These failures are not reclassified as successful runs.

The real-folder test now closes restored diagram windows before selecting its
owner, waits for the native PathTextField, targets that field explicitly and
asserts its exact value before confirming. Assertions about actual scanning,
Finder menus and the copied absolute path are preserved. The app itself and the
shared launch helper are unchanged.

The repaired test compiles. Two subsequent attempts stopped before any test body
with `Timed out while enabling automation mode`. Runner signature verification
passes and Developer Mode was already enabled. No new permission prompt or root
cause has been established. Owner input was requested; there are no further local
retries or security-setting changes in this receipt. The repaired UI test remains
unverified until XCTest can initialize and execute it. Native results and every
command retain their actual status in the adjacent JSON receipts.

## Architectural scope

Affected evidence concerns MacApplication presentation, TemplateManagement,
RuleProfileCollection/Presentation and current.application generation reduction;
current.templates draft_validation/file_exchange and rules.profile stopped_delivery
retain their contracts. Test-host/chooser setup stays folded into application
assembly. No runtime code, module, capability, map topology, API, acceptance state
or implementation binding changed. Existing maps/contracts are hashed without
relabelling older receipts. Current/target HC parse and four profile validations
pass; SpecPM reports no errors with its existing unresolved-capability warnings.
