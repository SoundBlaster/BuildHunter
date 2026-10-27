# Template file exchange — 2026-10-26 (Moscow)

Owner clarification: the first free release requires a simple GUI editor and file
import/export; the Swift SDK remains separate. Runtime source is `0d095f3`, based
on PR #120's `89065ce`. Source/map/contract hashes are in source-identity.json.

- Import JSON connects the user-owned file in place.
- Export JSON validates and atomically writes a copy, preserving the open source,
  registration set and unsaved-draft state. A connected/current-source destination
  must use Save. Invalid drafts cannot overwrite an existing export.
- Save As switches this draft's registration to its chosen new path, preserving
  its identity/enabled state and old source file. It rejects another registration's
  destination before writing, and uses Save's external-edit check for the current
  source path. This fixes the previous duplicate active package after Save As.

## Verification

Debug build-for-testing and Release build passed on the verified TB5 output paths.
Three regression tests were added for export isolation/invalid overwrite, Save As
registration state/current-source conflict, and protection of another connected
file. They compile. The full local unit attempt timed out after 90 seconds before
executing test bodies; it is recorded as cancelled, not passed. No further local
retry or system permission change was made. Final CI must run these tests.

Native computer-use verification on the signed Debug app:

1. Opened the existing fixture (1.0.3), changed its draft to 1.0.4 and exported
   through the native Save panel. The UI retained Unsaved changes and one source.
2. File inspection confirmed original=1.0.3, exported=1.0.4 and exactly one
   registration. Export did not activate a duplicate or modify the source.
3. Save As wrote SwitchedTemplate.json. UI then showed one registered new path,
   no unsaved marker, and Check produced a valid 14-target catalog with digest
   prefix 9873d59efb06. Original source remained intact.

Exact file hashes/versions and selected registration metadata (no bookmark bytes)
are in native-file-check.json. Screenshots were emitted in the tool transcript;
no standalone screenshot file is claimed. No new scan performance, memory bound,
XCTest UI pass or Store approval is inferred from these checks.

HC parse and all current/target contexts passed. SpecPM reports 0 errors and 52
MVP required_capability_unresolved warnings. Template file exchange is folded into
existing TemplateManagement/ScopedTemplateFiles; target topology, snapshot/bridge
and C1–C5 contracts are unchanged. New selected-file export writes update C6 and
application effects_scope. Recovered specs remain draft. Previous source/CI/test
receipts remain historical and are not promoted to checks for this revision.
