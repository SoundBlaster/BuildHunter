# R7 file-backed user templates — 2026-10-25 UTC

Owner-approved scope: [user templates](../../../docs/user-templates.md). Current
code starts at `0c863a6`, followed by accessible file actions (`e598a99`) and
reconnection of an open draft (`59508e3`). The latest source/map/contract inputs
are hashed in [source-identity.json](source-identity.json). Documentation bindings
point to the latest code commit; this receipt does not promote draft contracts.

## Executed evidence

- Rust `cargo test --locked --all-features`: 96 library + 5 CLI/profile tests
  passed. New tests cover external catalog extension, reserved/duplicate/invalid
  input, document ownership and copied FFI diagnostics. Clippy with warnings
  denied and formatting passed. No Hunter/Rifle vendor/dependency change.
- Initial Swift run: **149 tests in 15 suites passed**, including seven template
  tests (catalog/export identity, malformed packages, bounded regular files,
  observed external-edit conflict, registrations, retained invalid draft and a
  real custom scan with current/next-catalog isolation). This was an earlier
  working snapshot in this session, before the final rendering/accessibility and
  reconnect follow-ups. Do not present it as a final-commit rerun.
- Final Debug build-for-testing compiles the app, unit tests and the new Settings
  editor UI test. Final Release build compiles/signs the app. Exact commands and
  outcomes are in [commands.json](commands.json); build/test outputs stay on TB5.
- Two attempts to rerun final Swift units did **not** start test bodies. The first
  test-host sample shows `XCTestDriver._prepareTestConfigurationAndIDESession`
  waiting inside XCTest. It was stopped after 333 seconds; the bounded retry was
  stopped after 180 seconds. Both are recorded as cancelled, not passed and not
  assertion failures. Owned abandoned test hosts were stopped afterwards.
- The new XCTest UI scenario was compiled but not executed locally. The existing
  UI Automation authentication requirement still needs the owner, who was away.
  No system security setting, permission or quarantine state was changed.

## Native computer-use checks

Signed Debug app, isolated preferences suite
`BuildHunter.TemplateManual.20261025`, macOS 27.0 / Xcode 27.1 / Swift 6.4.
The following were observed through the actual native UI, not inferred from build
success or a proxy web view:

1. App menu Settings opens Search/Templates; New creates a complete editable JSON
   example. Check produces a valid 14-target catalog with a digest.
2. Replacing the draft with `{` reports a line/column JSON diagnostic and keeps
   the invalid draft in the editor. Restoring valid JSON remains usable.
3. Save As uses the native save panel, writes a selected JSON on TB5 and connects
   it. Its registration survives quitting/relaunching the process.
4. External disk version `1.0.3` versus loaded `1.0.0` and draft `1.0.2`: Save
   refuses the conflict; the draft remains. Reload asks before discard, then
   displays `1.0.3` from disk.
5. Final rendering exposes separate Open/Disconnect controls and an updating
   accessibility status value. Open reloads JSON with syntax colors; Check gives
   `Valid catalog · 14 targets · e5bbe720b115` for the saved fixture.
6. Disconnect removes the registration while leaving the JSON file available;
   Connect JSON through the native file panel registers that existing file again.

Screenshots and AX observations were emitted in the tool transcript. They were
not saved as standalone image files; no persistent screenshot artifact is claimed.
The final reconnect fix was also exercised natively: a dirty `1.0.9` draft
survived Disconnect → Connect and Check used its digest `8de64e4545b8`, rather
than the saved `1.0.3` digest `e5bbe720b115`. Reload/discard restored `1.0.3`.
The additional reconnect unit assertion is compiled but awaits execution.

Native checks do not establish an XCTest pass, Release runtime performance,
App Store acceptance, process-wide memory bounds or recovery after a crash.

## Contracts and architecture

The current map adds TemplateManagement (registry/draft/scoped files) and
UserTemplateAdmission under ClientCatalog. Detailed editor Views are folded into
one new `buildhunter.current.templates` boundary. Existing snapshot, bridge and
application drafts describe admission/diagnostics, immutable scan ownership and
explicit selected-file writes. Target module ownership remains unchanged.

HC parse/validation passes for current CLI/GUI and target public/Store contexts.
SpecPM 0.2.0 reports zero errors and 52 `required_capability_unresolved` warnings
from its MVP dependency resolver; these warnings do not prove missing runtime
modules or successful runtime assembly. See [structural commands](structural-commands.json)
and [SpecPM output](specpm.json). Tool identities are in [tools.json](tools.json).

The unrelated primary-checkout Xcode scheme remains unchanged (digest in the
identity record). No PR merge or Store release is part of this receipt. CLI file
import, Swift authoring SDK, commercial catalog/StoreKit and Store approval remain
separate work. Final XCTest/CI review remains necessary before merge.
