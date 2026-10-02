# BuildHunter for macOS

This macOS 26+ arm64 SwiftUI app implements the scanner described by [the PRD](../docs/PRD.md). Each window owns an independent target and report. Choose a folder with `File → Open Folder…`, the clickable chooser, or folder drop. The table receives discovered artifact roots and measured sizes asynchronously from the in-process Rust scanner. Stop preserves partial results; replacing a target or rescanning starts a new generation and discards stale events.

The default source reads the selected folder through the Rust static library and C FFI. GUI classification uses SpecificationCore, and includes Python environments with their own `pyvenv.cfg`. Only outer artifact roots appear as rows. Sizes estimate allocated blocks, including directory metadata; they do not promise reclaimable space. Partial measurements and read errors keep the report incomplete. The app never deletes files.

Artifact classification policy is expressed with SpecificationCore using immutable facts. The source is pinned to upstream revision `483214469828c42f7b615654aa70d0acbecc4dbf` (SpecificationCore 2.1.0).

Debug builds include a **Mock State** menu in the window toolbar. It displays synthetic empty, scanning, completed, stopped, and incomplete reports, and clears any previous real target URL. Mock controls are compiled out of Release builds. `NestedA11yIDs` composes stable identifiers for test controls. UI tests select every state, check accessible status, open and dismiss the folder picker, and attach screenshots to `.xcresult`. CI exports and checks seven PNGs under `macos/.build/ci/ui-screenshots`.

Generate and open the Xcode project:

```sh
cd macos
xcodegen generate
xcodebuild -resolvePackageDependencies -project BuildHunter.xcodeproj -scheme BuildHunter
xcodebuild test -project BuildHunter.xcodeproj -scheme BuildHunter -destination 'platform=macOS,arch=arm64' -skipMacroValidation CODE_SIGNING_ALLOWED=NO
```

`-skipMacroValidation` is the user-authorized invocation-scoped bypass for the pinned macro dependency. It skips every macro's validation in that invocation and does not change global trust settings. [Verification instructions](../docs/verification.md) describe the Rust, CLI, Swift unit, UI, and Release checks.

Unsigned build/unit-test evidence does not establish signed sandbox behavior. Open/drop grants, window replacement/close, Finder integration, clipboard actions, and App Store readiness remain separate runtime/delivery gates. Finder and Copy Path actions have not been implemented yet.
