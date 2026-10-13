# macOS screenshot pilot — repeated startup diagnosis

Host: macOS 27.0 (26A428), Apple Silicon, Xcode 27.1.
The existing screenshot worktree and single BuildHunter DerivedData were reused.
Original app checkout and unrelated scheme changes were preserved.

## Unsigned rebuild

Run `20261013T085502074055Z`: build-for-testing passed in 166.143 seconds.
The test invocation hit the runner's 360-second timeout before any screenshot
scenario began. Its interrupted xcresult lacks `Info.plist` and is not readable;
it cannot establish a test pass. No PNG batch was produced.

The leftover runner (PID 12158, exact screenshot-product path) was sampled,
then terminated. Its stack remained at `_dyld_start`. Signature verification
reported `code has no resources but signature indicates they must be present`.
The runner had been built with `CODE_SIGNING_ALLOWED=NO`.

## Ad hoc signed rebuild

Run `20261013T112514589968Z`: the macOS tool now builds with
`CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- DEVELOPMENT_TEAM=`. No team
certificate is required. Build-for-testing passed in 10.610 seconds, and
`codesign --verify --deep --strict` confirmed a valid runner resource seal.

XCTest nevertheless failed after 344.725 seconds with **The test runner hung
before establishing connection**. The readable xcresult reports a runner
infrastructure failure, zero passed tests, and no executed screenshot scenario.
No PNG batch exists. Its sample also remained at `_dyld_start`.

This establishes a malformed-signature problem and its repair, but does not
establish that signing was the cause of the startup hang. Developer mode still
reported disabled during both attempts. The user authorized enabling it for a
separate diagnostic retry; administrator confirmation is required on the host.
Keep any subsequent run separate from these failed attempts.

## Developer mode enabled; UI automation authentication remains

At 16:38 UTC the user completed the administrator prompt.
`DevToolsSecurity -status` now confirms developer mode enabled. The same signed
binary was reused without rebuilding in runs `20261013T163846835411Z` and
`20261013T164059975410Z`.

The runner now executes and logs `Running tests...`, then fails initialization
after approximately 60 seconds with **Timed out while enabling automation mode**.
Neither screenshot test body executes and no PNG batch exists.
`automationmodetool` confirms Automation Mode disabled and user authentication
required. This is a later failure stage than the previous `_dyld_start` hang.
It does not establish successful test execution or capture.

No `enable-automationmode-without-authentication` setting was applied.
The next retry requires the user to complete XCTest's UI Automation prompt.
Developer mode and UI automation authentication are separate gates.

## Executed scenario and file transport repairs

After the user completed the UI Automation prompt, run
`20261013T164644047255Z` executed both test bodies: runtime opt-out passed;
the batch failed at sandboxed command/export file writes. The UI-test runner
now has screenshot-only write exceptions for the isolated app's checkpoint
cache and normalized output root. Paths start and end with `/`, including the
home-relative path. Its sandbox remains enabled; the target app sandbox is
unchanged. `ENABLE_APP_SANDBOX=NO` on the test target did not change the signed
runner and was removed rather than relied upon.

The failure hierarchy also showed some relaunches had no application windows.
The consumer now supplies a `prepare` callback that opens a main window through
the real File → New BuildHunter Window menu when needed. The transparent probe
has an explicit accessibility label. Earlier failed attempts remain in output.

The first complete geometry-valid batches had mixed light/dark appearance:
setting `NSWindow.appearance` and the launch defaults alone did not synchronize
SwiftUI. The capture theme is now applied at main/diagram root views through
`IsDarkScreenshotAppearance` (SpecificationCore) and `preferredColorScheme`.
Those intermediate batches are preserved but are not accepted light-theme evidence.

## Final verified light batch and repeat

| Run | Build | Result | Elapsed command time |
| --- | --- | --- | --- |
| `20261013T170307215242Z` | Incremental | 2 tests passed, 0 failed/skipped; 6 prepared PNGs | 44.849s |
| `20261013T170459869382Z` | Same binary, skipped | 2 tests passed, 0 failed/skipped; 6 prepared PNGs | 35.972s |

Checked matrix: macOS 27.0 / Apple Silicon, en-US, light, whole window
1280×800 points → 2560×1600 pixels at native 2x. Real controls select the completed
mock state and open the diagram: empty → results → diagram. Every state has an
AppKit Screener frame and a separate XCTest window reference. No folders were
scanned or changed. No ScreenCaptureKit session or screen-recording permission
was needed for this window/XCTest path.

Preparation preserved decoded RGB8 pixels and native geometry, removed only
opaque alpha, and verified raw hashes and closed consecutive trace timelines.
Selected full-size PNGs were inspected for the intended content and consistent
light appearance. Final and repeat manifests, metrics, summaries, preparation
logs and PNGs are versioned here; source hashes bind the final implementation.
The normal project was restored after capture.
The ordinary Debug app build also passed using the same DerivedData, without
`SCREENER_SCREENSHOTS`. Xcode reported a stale Rust static-library path outside
the allowed root paths from shared cache reuse; it did not fail the build.
Normal app runtime and Release configuration were not exercised.

The timings exclude the preceding integration, permission prompts, debugging,
failed attempts and manual review. They do not measure model-token savings or
a manual-versus-automation speedup. Debug mock controls are visible, so these
are pilot assets rather than a claim of final App Store marketing readiness.
Other locales, dark theme, other window/display sizes, GPU fidelity and Apple
acceptance remain unchecked.

## Artifacts

Raw logs, manifests and xcresults are retained under `screenshots/output/` with
the run IDs above. Versioned metrics and the signed test summary are beside this
file. Source hashes identify the changed tools; the later explanatory comment
does not alter the signing command that was run.

The tool update is mirrored in Screener's experimental macOS adapter. Python
syntax was checked and copied-tool provenance refreshed. Neither SDK changes,
screen-recording consent, actual screenshot fidelity, nor App Store acceptance
is established by these startup attempts.
