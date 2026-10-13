# macOS screenshot pilot

The portable Screener App Store screenshot skill supplies the checkpoint protocol,
AppKit window adapter, XCTest helper, collector, lossless PNG preparation and gallery.
BuildHunter supplies its existing Debug mock-state controls and the real navigation
scenario: empty window → completed results → artifact diagram. The app-owned
`IsScreenshotReady` specification rejects in-flight scans/transitions and an
incomplete diagram snapshot. No folders are scanned or modified by this fixture.

Run from the repository root:

```sh
python3 screenshots/tools/run_macos.py --config screenshots/capture.json
python3 screenshots/tools/run_macos.py --config screenshots/capture.json --skip-build
python3 screenshots/tools/run_macos.py --prepare-output screenshots/output/RUN
```

The supplied config currently reuses the local BuildHunter Xcode DerivedData.
Adjust that field to your existing single cache before running elsewhere.
The pilot generates `macos/BuildHunter.xcodeproj` from `macos/project.screenshots.yml`;
regenerate from `macos/project.yml` before ordinary development/release builds.
The existing diagram-animation trace flag remains separate and is not enabled.

`SCREENER_SCREENSHOTS` is a dedicated Debug-derived configuration, and runtime
capture also requires the flag plus a valid UUID. Its bundle ID is
`dev.buildhunter.screenshots`, separate from the normal app. A runtime opt-out
test ensures the accessibility protocol does not activate without the flag.
The SDK pin remains `e447751afd8815de0ad1ec50ba2e4ca7c469d68e`.

The macOS runner uses local ad hoc signing. The screenshot UI-test configuration
also uses `ScreenshotRunner.entitlements` for write access to the isolated app's
checkpoint cache and this runner's output root; ordinary configurations do not
apply that entitlement file. Main-window opening remains real menu navigation.
Screenshot appearance is applied at both SwiftUI roots through
`IsDarkScreenshotAppearance`; late AppKit-only appearance changes were insufficient.

The target profile is a 1280×800 point whole window at native 2× (2560×1600),
English, light appearance. Both AppKit and XCTest window images are retained;
the latter are the compositor references to review for store use. Debug toolbar
controls remain visible, so these pilot images demonstrate the workflow and
are not asserted to be final marketing assets. Apple upload is a separate action.

Outputs contain the fresh xcresult, raw/opaque PNGs, closed `.vtrace`, manifest,
timings and `review.html`. See the validation record for actual execution results
and unchecked behavior; compilation alone does not establish UI automation.

The final 2026-10-13 light batch passed both tests and all three checkpoints in
44.849s including an incremental build; the same-binary buildless repeat passed
in 35.972s. Each produced six native PNGs and a review gallery. See
[the fresh report](validation/2026-10-13/README.md), including the earlier failed
attempts and permission/setup costs that these repeat timings exclude.
