# BuildHunter for macOS

This macOS 26+ arm64 SwiftUI app implements the scanner described by [the PRD](../docs/PRD.md). Each window owns an independent target and report. Choose a folder with `File → Open Folder…`, the clickable chooser, or folder drop. The table receives discovered artifact roots and measured sizes asynchronously from the in-process Rust scanner. Stop preserves partial results; replacing a target or rescanning starts a new generation and discards stale events.

The default source reads the selected folder through the Rust static library and C FFI. GUI classification uses SpecificationCore, and includes Python environments with their own `pyvenv.cfg`. Only outer artifact roots appear as rows. Sizes estimate allocated blocks, including directory metadata; they do not promise reclaimable space. Partial measurements and read errors keep the report incomplete. The app never deletes files.

Artifact classification policy is expressed with SpecificationCore using immutable facts. The source is pinned to merged upstream revision `3a672ea9a081853bb0500c96a85243f01e39bcdd` pending release alignment. Its default `AggressiveInlining` trait favors synchronous evaluation speed; Tracing disables forced inlining. See the upstream [performance configuration guide](https://github.com/SoundBlaster/SpecificationCore/blob/3a672ea9a081853bb0500c96a85243f01e39bcdd/Sources/SpecificationCore/Documentation.docc/EvaluationPerformance.md) for consumer opt-out and measured tradeoffs. Library microbenchmark gains do not establish a whole-scan speedup.

Open **BuildHunter → Settings…** (⌘,) to select the artifact types searched in new
scans and Rescan. All current types are enabled initially. Preferences persist
between launches, but each scan captures an immutable selection; changing settings
does not rewrite an existing report. The grouped controls are generated from the
Rust filter catalog, including future languages/framework types. See
[Search settings](../docs/search-settings.md) for CLI equivalents and matching semantics.

Open **Diagram** (⇧⌘D) to see the same report in a separate animated sunburst window. Folder navigation, live measurement updates, partial-size indicators, and Reduce Motion support are described in [Live artifact diagram](../docs/artifact-diagram.md).

Debug builds include a **Mock State** menu in the window toolbar. It displays synthetic empty, scanning, completed, stopped, and incomplete reports, and clears any previous real target URL. Mock controls are compiled out of Release builds. `NestedA11yIDs` composes stable identifiers for test controls. UI tests select every state, check accessible status, open and dismiss the folder picker, verify the companion diagram, and attach screenshots to `.xcresult`. CI exports and checks ten PNGs under `macos/.build/ci/ui-screenshots`.

Generate and open the Xcode project:

Install Rust with rustup first. The Rust pre-build script uses Cargo from `PATH`,
then checks `${CARGO_HOME:-$HOME/.cargo}/bin/cargo`. This also supports Xcode
launched from Finder, which does not inherit your shell's Cargo path.

```sh
cd macos
xcodegen generate
xcodebuild -resolvePackageDependencies -project BuildHunter.xcodeproj -scheme BuildHunter
xcodebuild test -project BuildHunter.xcodeproj -scheme BuildHunter -destination 'platform=macOS,arch=arm64' -skipMacroValidation CODE_SIGNING_ALLOWED=NO
```

`-skipMacroValidation` is the user-authorized invocation-scoped bypass for the pinned macro dependency. It skips every macro's validation in that invocation and does not change global trust settings. [Verification instructions](../docs/verification.md) describe the Rust, CLI, Swift unit, UI, and Release checks.

Unsigned build/unit-test evidence does not establish signed sandbox behavior. Open/drop grants, window replacement/close, Finder integration, clipboard actions, and App Store readiness remain separate runtime/delivery gates. Finder and Copy Path actions have not been implemented yet.

### Scan profile panel

Expand **Scan profile** in the main report window to inspect live throughput.
Choose entries/s or measured bytes/s; current, average, peak and elapsed time
remain available after completion. Byte throughput describes metadata sizing,
not disk reads. History is bounded to 600 intervals; lifetime statistics remain
intact. Mock scanning/completed/stopped states include deterministic profile
samples, and UI tests capture the expanded completed panel. The panel, incoming
chart samples, metric selection and numeric readouts animate smoothly; Reduce
Motion disables these animations. Rolling points retain absolute timestamp
identities, and a fresh scan resets the chart without morphing the old timeline.
