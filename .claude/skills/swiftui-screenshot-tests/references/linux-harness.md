# Linux Swift harness for animation geometry

BuildHunter keeps its layout and animation math in plain Swift under
`macos/Sources/BuildHunter/Domain/`. That code has no SwiftUI or AppKit
imports, so it compiles on Linux. A throwaway SwiftPM package around it lets
you:

- run the Swift Testing suites for the geometry (e.g. the sunburst navigation
  suite);
- dump animation frames as JSON and render them with
  `scripts/render_sunburst_frames.py`.

Build it in the scratchpad, not in the repo.

## 1. Toolchain

Match CI's Swift version, or newer. Check `xcodebuild -version` in a CI log, or
the Xcode selected in `.github/workflows/macos.yml`.

```bash
v=6.4.0   # the release you want
cd "$SCRATCH" && mkdir -p swift && cd swift
curl -fsSLO "https://download.swift.org/swift-$v-release/ubuntu2404/swift-$v-RELEASE/swift-$v-RELEASE-ubuntu24.04.tar.gz"
tar xzf "swift-$v-RELEASE-ubuntu24.04.tar.gz"
export PATH="$PWD/swift-$v-RELEASE-ubuntu24.04/usr/bin:$PATH"
swift --version
```

Tarballs are named after the exact release (`6.4.0`, `6.1.2`). If a URL 404s,
list `https://download.swift.org/` for the right folder. The toolchain is about
1 GB unpacked, so delete it when you finish if disk is tight.

## 2. Package layout

```
harness/
  Package.swift
  Sources/BuildHunter/   <- symlinks to the pure Domain files the code under test needs
  Sources/Dump/main.swift
  Tests/NavTests/Nav.swift   <- the suite copied out of macos/Tests/BuildHunterTests
```

```swift
// swift-tools-version: 6.4
import PackageDescription
let package = Package(
    name: "Harness",
    targets: [
        .target(name: "BuildHunter"),   // same module name, so `@testable import BuildHunter` works unchanged
        .executableTarget(name: "Dump", dependencies: ["BuildHunter"]),
        .testTarget(name: "NavTests", dependencies: ["BuildHunter"]),
    ]
)
```

Symlink, don't copy. Edits in the repo are then picked up on the next build:

```bash
R=/home/user/BuildHunter/macos/Sources/BuildHunter
ln -s $R/Domain/ArtifactSunburst.swift $R/Domain/ArtifactPolicyContext.swift $R/Scanning/ScanEvent.swift harness/Sources/BuildHunter/
```

Start with the file under test, run `swift build`, and add a symlink for each
"cannot find type" error until it builds. Stop if the chain pulls in SwiftUI,
AppKit or Charts. The fix there is to move the pure part into `Domain/`, which
is a good change for the app too.

To run one test suite, copy it out of the big test file, from its `@Suite`
line to the next suite. Prepend `import Foundation`, `import Testing` and
`@testable import BuildHunter`. Then run `swift test --package-path harness`.

## 3. Dump frames

`Sources/Dump/main.swift` builds a small realistic tree, constructs the plan,
and samples it across the animation's parameters. Then it prints JSON in the
shape `render_sunburst_frames.py` expects:

```swift
import Foundation
@testable import BuildHunter

let rows: [(String, Int64)] = [("Projects/App/.build", 900), ("Projects/App/Packages/Core/.build", 400),
                               ("Projects/Lib/target", 500), ("Downloads/old/target", 350)]
let snapshot = ArtifactSunburstSnapshot(rows: rows.map { path, bytes in
    ScanRow(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput, size: .measured(bytes))
})
let parent = ArtifactSunburstLayout(snapshot: snapshot)
let child = ArtifactSunburstLayout(snapshot: snapshot, focusID: "Projects")
let plan = ArtifactSunburstNavigation(source: parent, destination: child, selectedID: "Projects")!

var out: [[String: Any]] = []
for step in 0...10 {
    let zoom = Double(step) / 10
    let frames = plan.frames(fade: 1, zoom: zoom).map { f -> [String: Any] in
        let id: String = switch f.id { case .node(let p): p; case .other(let p): "other:" + p }
        return ["id": id, "start": f.start, "end": f.end, "inner": f.innerRadius,
                "outer": f.outerRadius, "opacity": f.opacity]
    }
    out.append(["label": "zoom \(zoom)", "frames": frames])
}
FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: out))
```

The API names above match `ArtifactSunburst.swift` when this was written. If
they have changed, read the current file and adapt; the point is the pattern.

## 4. Render and read

```bash
swift run --package-path harness Dump > descend.json
python3 -I $SK/render_sunburst_frames.py descend.json --out descend.png --center-radius 0.21 --cols 6
```

- start/end are turn fractions clockwise from 12 o'clock;
- inner/outer are radius fractions;
- colors hash by the first `--branch-depth` path components, so a branch
  keeps its hue through the animation;
- `--center-radius` draws the center button on top, as the app does.

Read the sheet frame by frame:
- does the selected sector grow to 360° while it moves inward?
- do neighbors fade out without jumping?
- does any frame have a sector with inner > outer, a radius above 1, or a
  missing sector?

Assert anything you find in the copied test suite, and port the assertion back
to `macos/Tests/BuildHunterTests/`. The rendering is for humans; the tests keep
it fixed.
