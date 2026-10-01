# BuildHunter macOS skeleton

This macOS 26+ arm64 SwiftUI app is the first GUI skeleton described by `../docs/PRD.md`. It has one independent model per window, a `File → Open Folder…` command, a clickable chooser, folder drop handling, a streaming-shaped table, Stop/Rescan generation handling, and read-only sandbox entitlements.

**The scanner is deliberately simulated.** Choosing or dropping a folder only supplies its display name. The demo source does not inspect that folder or access its contents. All fixture paths and approximate sizes are synthetic; Finder and Copy Path actions are therefore not offered.

Artifact classification policy is expressed with SpecificationCore using immutable facts. The source is pinned to upstream revision `483214469828c42f7b615654aa70d0acbecc4dbf` (SpecificationCore 2.1.0).

Debug builds include a **Mock State** menu in the window toolbar. It can display an empty window, an active scan, completed sizes, a stopped scan with partial sizes, or an incomplete report with a warning. These fixtures are synthetic and are compiled out of Release builds.

Generate and open the Xcode project:

```sh
cd macos
xcodegen generate
xcodebuild -resolvePackageDependencies -project BuildHunter.xcodeproj -scheme BuildHunter
xcodebuild test -project BuildHunter.xcodeproj -scheme BuildHunter -destination 'platform=macOS,arch=arm64'
```

This skeleton does not claim real file access, sandbox runtime validation, Rust FFI, measurement accuracy, Finder integration, or App Store readiness.
