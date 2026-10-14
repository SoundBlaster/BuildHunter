# Hunter metadata integration — local verification, 2026-10-14

BuildHunter baseline 96780d7; Hunter pin 71f71fc (Hunter #1). Fresh execution
receipts and file/map identities are adjacent. Root default and all-feature tests
pass; plugin-scanner 12 tests pass in Debug/Release through the Git pin, plus
clippy/fmt. HC both maps/four profiles pass. SpecPM: 0 errors, 16 unresolved
capability warnings (15 previous, one new external hunter.fs.scoped_metadata).

Hunter independently passed 8 local macOS tests Debug/Release, including public
invalid-name regression; hosted Linux/macOS/Windows jobs on its exact pin are green.
Linux covers raw byte names; Windows compiles without a Unix API. Moved descriptor
race tests now belong to Hunter rather than the product adapter.

No GUI, live-provider, whole-tree performance, memory budget or default-client
migration claim. Full traversal/measurement and lifecycle extraction remain pending.
The previous runtime/retention receipts remain historical. Test/source/map evidence
is separate from owner acceptance of draft contracts.
