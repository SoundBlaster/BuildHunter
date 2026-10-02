import Foundation
import Testing
@testable import BuildHunter

@Suite("Live diagram sessions")
@MainActor
struct ArtifactDiagramModelTests {
    @Test("Discovery, measurement and stop update the same report projection")
    func streamedUpdates() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        let firstArtifact = artifact("Package/.build")
        scan.apply(.discovered(generation: scan.generation, artifact: firstArtifact))
        await diagram.refresh(from: scan)
        #expect(diagram.snapshot.root.statistics.measuringCount == 1)
        #expect(diagram.layout.sectors.isEmpty)

        scan.apply(.completed(generation: scan.generation, artifactID: firstArtifact.id, bytes: 80))
        await diagram.refresh(from: scan)
        #expect(diagram.snapshot.root.bytes == 80)
        #expect(diagram.layout.sectors.count == 2)
        diagram.navigate(to: "Package")
        #expect(diagram.children.map(\.id) == ["Package/.build"])

        let unfinished = artifact("Other/target")
        scan.apply(.discovered(generation: scan.generation, artifact: unfinished))
        scan.stop()
        await diagram.refresh(from: scan)
        #expect(diagram.snapshot.root.bytes == 80)
        #expect(diagram.snapshot.root.statistics.measuringCount == 0)
        #expect(diagram.snapshot.root.statistics.partialCount == 1)
        #expect(diagram.focusID == "Package", "Stopping a scan must preserve diagram navigation")
    }

    @Test("Replacing the target clears the diagram and ignores stale scan events")
    func replacement() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Old")
        let oldGeneration = scan.generation
        let old = artifact("Old/.build")
        scan.apply(.discovered(generation: oldGeneration, artifact: old))
        scan.apply(.completed(generation: oldGeneration, artifactID: old.id, bytes: 100))
        await diagram.refresh(from: scan)
        diagram.navigate(to: "Old")
        scan.acceptDemoTarget(named: "New")
        scan.apply(.completed(generation: oldGeneration, artifactID: old.id, bytes: 999))
        await diagram.refresh(from: scan)
        #expect(diagram.snapshot.root.statistics.artifactCount == 0)
        #expect(diagram.snapshot.root.bytes == 0)
        #expect(diagram.focusID == "")
        #expect(diagram.layout.sectors.isEmpty)
        scan.stop()
    }

    @Test("Multiple windows keep separate models and repeated diagram opens reuse a report")
    func windowIdentity() {
        let store = ScanWindowStore()
        let first = WindowScanModel(source: DiagramIdleSource())
        let second = WindowScanModel(source: DiagramIdleSource())
        store.register(first)
        store.register(second)
        store.prepareDiagram(for: first)
        store.prepareDiagram(for: first)
        #expect(first.id != second.id)
        #expect(store.model(for: first.id) === first)
        #expect(store.model(for: second.id) === second)
        first.acceptDemoTarget(named: "First")
        second.acceptDemoTarget(named: "Second")
        #expect(store.model(for: first.id)?.targetName == "First")
        #expect(store.model(for: second.id)?.targetName == "Second")
        store.scanWindowClosed(first.id)
        #expect(first.phase == .stopped)
        #expect(second.phase == .scanning)
        #expect(store.model(for: first.id) === first)
        store.diagramWindowClosed(first.id)
        #expect(store.model(for: first.id) == nil)
        store.scanWindowClosed(second.id)
        #expect(store.model(for: second.id) == nil)
    }

    @Test("The live update task follows measurements and terminates when cancelled")
    func followingAndCancellation() async throws {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        let first = artifact("App/.build")
        scan.apply(.discovered(generation: scan.generation, artifact: first))
        let task = Task { await diagram.follow(scan) }
        defer {
            task.cancel()
            scan.stop()
        }
        try await waitUntil { diagram.snapshot.root.statistics.measuringCount == 1 }
        scan.apply(.completed(generation: scan.generation, artifactID: first.id, bytes: 64))
        try await waitUntil { diagram.snapshot.root.bytes == 64 }
        task.cancel()
        await task.value

        let later = artifact("Other/target")
        scan.apply(.discovered(generation: scan.generation, artifact: later))
        scan.apply(.completed(generation: scan.generation, artifactID: later.id, bytes: 128))
        try await Task.sleep(for: .milliseconds(150))
        #expect(diagram.snapshot.root.bytes == 64, "A closed window must stop publishing updates")
    }

    @Test("A replacement report resets focus even if its relative paths match")
    func samePathInNewReport() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        for target in ["First", "Second"] {
            scan.acceptDemoTarget(named: target)
            let entry = artifact("App/.build")
            scan.apply(.discovered(generation: scan.generation, artifact: entry))
            scan.apply(.completed(generation: scan.generation, artifactID: entry.id, bytes: 64))
            await diagram.refresh(from: scan)
            #expect(diagram.focusID == "")
            diagram.navigate(to: "App")
        }
        scan.stop()
    }

    @Test("Navigation and streaming share one palette; replacing the report discards it")
    func paletteLifecycle() async throws {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "First")
        defer { scan.stop() }
        let first = artifact("Apps/Alpha/.build")
        scan.apply(.discovered(generation: scan.generation, artifact: first))
        scan.apply(.completed(generation: scan.generation, artifactID: first.id, bytes: 100))
        await diagram.refresh(from: scan)
        #expect(diagram.palette.colors.count == 3)
        diagram.navigate(to: "Apps/Alpha")
        let selected = try #require(diagram.layout.sectors.first)
        #expect(diagram.palette.color(for: selected) == diagram.palette.colors["Apps/Alpha"])
        let original = diagram.palette.colors

        let later = artifact("Apps/Aardvark/target")
        scan.apply(.discovered(generation: scan.generation, artifact: later))
        scan.apply(.completed(generation: scan.generation, artifactID: later.id, bytes: 10_000))
        await diagram.refresh(from: scan)
        diagram.navigate(to: "")
        for (path, color) in original { #expect(diagram.palette.colors[path] == color) }
        #expect(diagram.palette.colors["Apps/Aardvark"] != nil)
        scan.stop()
        await diagram.refresh(from: scan)
        for (path, color) in original { #expect(diagram.palette.colors[path] == color) }

        scan.acceptDemoTarget(named: "Second")
        await diagram.refresh(from: scan)
        #expect(diagram.palette.colors.isEmpty)
        #expect(diagram.focusID.isEmpty)
    }

    @Test("Closing a diagram keeps its owner's scan running")
    func diagramDoesNotCancelScan() {
        let store = ScanWindowStore()
        let scan = WindowScanModel(source: DiagramIdleSource())
        store.register(scan)
        scan.acceptDemoTarget(named: "Fixture")
        store.prepareDiagram(for: scan)
        store.diagramWindowClosed(scan.id)
        #expect(scan.phase == .scanning)
        #expect(store.model(for: scan.id) === scan)
        store.scanWindowClosed(scan.id)
        #expect(store.model(for: scan.id) == nil)
    }

    @Test("Drilling in transfers the folder color to its largest child only once")
    func largestChildInheritsColor() async throws {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        for (path, bytes) in [("Apps/Small/.build", Int64(10)), ("Apps/Large/target", 100)] {
            let entry = artifact(path)
            scan.apply(.discovered(generation: scan.generation, artifact: entry))
            scan.apply(.completed(generation: scan.generation, artifactID: entry.id, bytes: bytes))
        }
        await diagram.refresh(from: scan)
        let parentColor = try #require(diagram.palette.colors["Apps"])
        let smallColor = diagram.palette.colors["Apps/Small"]
        diagram.navigate(to: "Apps")
        #expect(diagram.palette.colors["Apps/Large"] == parentColor)
        #expect(diagram.palette.colors["Apps/Small"] == smallColor)

        let late = artifact("Apps/Small/.venv")
        scan.apply(.discovered(generation: scan.generation, artifact: late))
        scan.apply(.completed(generation: scan.generation, artifactID: late.id, bytes: 1_000))
        await diagram.refresh(from: scan)
        diagram.navigate(to: "")
        diagram.navigate(to: "Apps")
        #expect(diagram.palette.colors["Apps/Large"] == parentColor)
        #expect(diagram.palette.colors["Apps/Small"] == smallColor,
                "Streaming must not transfer the inherited color to a new size leader")
    }

    @Test("Late discoveries cannot displace or reorder previously visible siblings")
    func stableStreamingOrder() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        for (index, path) in ["Zebra", "Middle", "Beta", "Delta", "Echo", "Foxtrot", "Golf"].enumerated() {
            let entry = artifact("\(path)/.build")
            scan.apply(.discovered(generation: scan.generation, artifact: entry))
            scan.apply(.completed(generation: scan.generation, artifactID: entry.id, bytes: Int64(index + 1)))
            await diagram.refresh(from: scan)
            let names = diagram.layout.sectors.filter { $0.depth == 0 }.compactMap(\.nodeID)
            #expect(names.first == "Zebra", "New sectors append after existing sectors")
        }
        let original = diagram.layout.sectors.filter { $0.depth == 0 }.compactMap(\.nodeID)
        let colors = diagram.palette.colors
        let late = artifact("Aardvark/.build")
        scan.apply(.discovered(generation: scan.generation, artifact: late))
        scan.apply(.completed(generation: scan.generation, artifactID: late.id, bytes: 1_000_000))
        await diagram.refresh(from: scan)
        #expect(diagram.layout.sectors.filter { $0.depth == 0 }.compactMap(\.nodeID) == original)
        #expect(diagram.layout.sectors.filter { $0.depth == 0 }.reduce(0) { $0 + $1.bytes } == 1_000_028)
        for (path, color) in colors { #expect(diagram.palette.colors[path] == color) }
        diagram.navigate(to: "Middle")
        diagram.navigate(to: "")
        #expect(diagram.layout.sectors.filter { $0.depth == 0 }.compactMap(\.nodeID) == original)
    }

    @Test("Color inheritance follows a previously explored child when an ancestor is selected later")
    func inheritanceAcrossSkippedLevels() {
        let rows = [
            ScanRow(id: UUID(), relativePath: "Apps/Large/.build", language: "Swift", kind: .buildOutput, size: .measured(100)),
            ScanRow(id: UUID(), relativePath: "Apps/Small/target", language: "Rust", kind: .buildOutput, size: .measured(10))
        ]
        let diagram = ArtifactDiagramModel(snapshot: ArtifactSunburstSnapshot(rows: rows))
        diagram.navigate(to: "Apps/Large")
        diagram.navigate(to: "Apps")
        #expect(diagram.palette.colors["Apps/Large"] == diagram.palette.colors["Apps"])
        #expect(diagram.palette.colors["Apps/Large/.build"] == diagram.palette.colors["Apps"])
        #expect(diagram.palette.colors["Apps/Small"] != diagram.palette.colors["Apps"])
    }

    @Test("Hover previews full paths and children without changing navigation, color or the saved filter")
    func hoverPreviewAndUp() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.accept(target: URL(fileURLWithPath: "/Users/test/Projects", isDirectory: true))
        defer { scan.stop() }
        for path in ["Apps/Alpha/.build", "Apps/Beta/target", "Tools/__pycache__"] {
            let entry = artifact(path)
            scan.apply(.discovered(generation: scan.generation, artifact: entry))
            scan.apply(.completed(generation: scan.generation, artifactID: entry.id, bytes: 100))
        }
        await diagram.refresh(from: scan)
        diagram.navigate(to: "Apps")
        diagram.query = "Alpha"
        let colors = diagram.palette.colors
        let layout = diagram.layout
        diagram.preview("Tools")
        #expect(diagram.focusID == "Apps")
        #expect(diagram.displayedPath == "/Users/test/Projects/Tools")
        #expect(diagram.displayedURL?.path == diagram.displayedPath)
        #expect(diagram.filteredChildren.map(\.id) == ["Tools/__pycache__"])
        #expect(diagram.palette.colors == colors)
        #expect(diagram.layout == layout)
        diagram.preview(nil)
        #expect(diagram.filteredChildren.map(\.id) == ["Apps/Alpha"])
        #expect(diagram.displayedPath == "/Users/test/Projects/Apps")
        diagram.preview("Apps/Alpha")
        diagram.navigateUp()
        #expect(diagram.focusID.isEmpty)
        #expect(diagram.previewID == nil)
        #expect(diagram.displayedPath == "/Users/test/Projects")
        #expect(!diagram.canNavigateUp)
        diagram.navigateUp()
        #expect(diagram.focusID.isEmpty)
        diagram.preview("missing")
        #expect(diagram.previewID == nil)
        scan.accept(target: URL(fileURLWithPath: "/tmp/New target", isDirectory: true))
        await diagram.refresh(from: scan)
        #expect(diagram.previewID == nil)
        #expect(diagram.displayedPath == "/tmp/New target")
        #expect(diagram.filteredChildren.isEmpty)
    }

    private func artifact(_ path: String) -> ScanArtifact {
        ScanArtifact(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput)
    }

    private func waitUntil(_ predicate: () -> Bool) async throws {
        let clock = ContinuousClock()
        let deadline = clock.now.advanced(by: .seconds(5))
        while !predicate(), clock.now < deadline {
            try await Task.sleep(for: .milliseconds(10))
        }
        try #require(predicate(), "The live diagram must publish the scan update")
    }
}

private struct DiagramIdleSource: ScanEventSource {
    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        AsyncStream { _ in }
    }
    func cancel(generation: UInt64) {}
}
