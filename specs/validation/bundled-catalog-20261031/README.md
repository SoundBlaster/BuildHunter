# Bundled edition preparation — 2026-10-31

Local signed Debug build-for-testing succeeded. The focused Swift Testing run
executed **14 tests in 2 suites, all passed**, including synthetic bundled/user
composition, stable digest/exclusions, invalid bundle and user-index diagnostics,
resource hashes, tampering, missing files, invalid locations and symlink rejection.
The existing ten user-template tests passed alongside four bundle tests.

Raw build/test logs and xcresult are retained in the checkout-specific TB5
`bundled-catalog-20261031` directory. These tests are app-hosted unit tests, not
a native save-panel/bookmark relaunch test or App Store purchase acceptance.
No private catalog bytes were used in public tests.

Affected boundary: current.templates/edition_resources (new draft obligation),
bounded_admission and scan_isolation (same budgets/ownership, additional source).
TemplateManagement folds the resource helper; its HCS property is refreshed.
No extra topology, ABI, bridge or snapshot contract change.
Source/map identities and structural command results are adjacent. Structural
validation does not establish runtime or Store conformance. Existing unresolved
SpecPM capability warnings retain their earlier meaning. Drafts remain draft.
