# BuildHunter macOS screenshot pilot — startup blocked

Target: Apple Silicon MacBook Pro, macOS 27.0 (26A428), Xcode 27.1 (27A9275).
Baseline `8c9f68d`; local pilot sources identified in `identity.json`.
The existing GUI checkout's rounding experiment and dirty project were preserved.

The generated opt-in configuration builds the app and both test bundles. The final
incremental build-for-testing took 5.508s. Initial dependency compilation and two
corrected adapter assumptions are preserved in earlier ignored run directories:
the pinned SDK does not accept recordFrame metadata, and macOS XCTest bundle paths
are relative to their test host. Neither requires an SDK API change.

The fresh UI invocation failed before any test case executed:
**The test runner hung before establishing connection.** XCTest spent 346.258s
before failing startup. See `startup-test-summary.json`, `startup-metrics.json` and
`screenshots/output/20261011T213713021587Z/en-US.xcresult` / `en-US.log`.
There are **zero captured PNGs**. No pixel fidelity, gallery, runtime opt-out or
repeat-run time is established by this attempt.

Diagnostics: the runner sample showed only `_dyld_start`; Developer mode reported
disabled. No automation permission dialog was visible in the inspected Xcode UI.
The disabled setting is a possible environment cause, not proof of the sole cause.
Changing it was requested separately and has not been performed.

The next bounded check is the same screenshot scenario with the built binary,
after resolving runner startup. Use `--skip-build` to keep that check separate
from compilation. Preserve failed runs; do not reclassify them after a later pass.
