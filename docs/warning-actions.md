# Warning row actions

The Scan Warnings table offers **Copy** and **Open in Finder** on both cell
contents and the row background. Copy uses complete warning messages in report
order. Right-clicking an unselected row targets that row; right-clicking a selected
row preserves the multi-row selection. Keyboard Copy and the toolbar remain
available. Open is enabled only for one warning with a known location.

The Rust callback already supplies a separate path. The Swift bridge copies that
path only when it is exact, non-empty UTF-8 and the selected root is exact. It
never reconstructs a location from the formatted message. The window model keeps
locations for the current report and clears them on rescan. A deduplicated message
with conflicting or missing origins loses its action for that report. Existing
lossy-path safeguards also apply; absolute paths, parent traversal and embedded
NUL are rejected. Mock, summary and general bridge warnings remain copyable
without a Finder action. The Rust event ABI and export schemas are unchanged.

Finder handling reuses `ArtifactFolderActions`: security-scoped access, a fallback
to the nearest existing folder within the selected root, and a notice when needed.
It requests Finder selection; it does not prove Finder completion. Menu callbacks
captured for an old report do nothing after report replacement.

## Local test runner signing

A local `build-for-testing` with `CODE_SIGNING_ALLOWED=NO` can compile successfully
while producing an application bundle with an incomplete linker signature. On
2026-10-25 macOS rejected the UI runner as damaged. Rebuilding the app and runner
with `CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- DEVELOPMENT_TEAM=` produced
bundles that passed `codesign --verify --deep --strict`. This is local ad-hoc
signing, not Developer ID signing or notarization. Gatekeeper remained enabled;
no quarantine attributes or system security settings were changed.

Passing that integrity check did not resolve a separate Debug launch stall: both
ad-hoc and Apple Development app builds remained in `dyld`'s `open` before entering
application code. A local single-binary Debug build also failed to reach UI test
actions. These attempts are recorded as failed/blocked validation, not menu passes;
the exact file responsible for the loader stall was not established. No signing
or Debug build overrides are committed to the Xcode project.

Current checks and their limits are recorded in
[the verification receipt](../specs/validation/warning-actions-20261025/README.md).
