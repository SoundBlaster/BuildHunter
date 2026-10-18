# Lifetime integration refresh — 2026-10-19

Rebased #84 onto main 1a09c34. Conflict resolution retained lifetime and active CI
records and advanced the package draft version. `source-comparison.json` proves
unchanged runtime/vendor/Cargo inputs; the current scan binding points at rebased
pin commit d54f573. All-feature root tests, exact vendored-core tests, clippy/fmt,
seven HC profiles and SpecPM validation are freshly recorded in commands.json.
The exact 18-file Hunter pin is compared with local immutable upstream Git objects.

Old Debug/Release, paired performance and UI/provider observations remain historical.
These fresh local checks do not establish fresh native UI/home acceptance or a new
performance/RAM budget. CI validates the final head separately. Hunter #15 remains
open and must merge before this consumer. Canary acceptance receipts are separately
recorded in stack-ci-accepted-20261019; CI roles/requirements are unchanged.
