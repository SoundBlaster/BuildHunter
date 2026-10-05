import Foundation
import Testing
@testable import BuildHunter

@Suite("Sunburst folder expansion")
struct ArtifactSunburstNavigationTests {
    private func snapshot() -> ArtifactSunburstSnapshot {
        ArtifactSunburstSnapshot(rows: [
            ("Apps/Alpha/.build", Int64(900)), ("Apps/Alpha/Inner/.build", 200), ("Apps/Beta/target", 100),
            ("Tools/__pycache__", 500)
        ].map { path, bytes in
            ScanRow(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput, size: .measured(bytes))
        })
    }

    @Test("Neighbors disappear before the selected branch expands")
    func fadeThenExpand() throws {
        let snapshot = snapshot()
        let source = ArtifactSunburstLayout(snapshot: snapshot)
        let destination = ArtifactSunburstLayout(snapshot: snapshot, focusID: "Apps")
        let plan = try #require(ArtifactSunburstNavigation(source: source, destination: destination, selectedID: "Apps"))
        let anchor = try #require(plan.frames(fade: 1, expansion: 0, reveal: 0).first { $0.id == .node("Apps") })
        let original = try #require(source.sectors.first { $0.nodeID == "Apps" })
        #expect(anchor.start == original.start && anchor.end == original.end)
        #expect(anchor.opacity == 1)
        #expect(plan.frames(fade: 1, expansion: 0, reveal: 0).filter { !$0.isSelectedBranch }.allSatisfy { $0.opacity == 0 })
        let expanded = try #require(plan.frames(fade: 1, expansion: 1, reveal: 0).first { $0.id == .node("Apps") })
        #expect(expanded.start == 0 && expanded.end == 1)
        #expect(expanded.innerRadius == original.innerRadius)
        #expect(expanded.outerRadius == original.outerRadius)
    }

    @Test("The final frame matches the destination, including newly exposed descendants")
    func destinationMatches() throws {
        let snapshot = snapshot()
        let source = ArtifactSunburstLayout(snapshot: snapshot)
        let destination = ArtifactSunburstLayout(snapshot: snapshot, focusID: "Apps")
        let plan = try #require(ArtifactSunburstNavigation(source: source, destination: destination, selectedID: "Apps"))
        let final = plan.frames(fade: 1, expansion: 1, reveal: 1).filter { $0.opacity > 0 }
        #expect(Set(final.map(\.id)) == Set(destination.sectors.map(\.id)))
        for sector in destination.sectors {
            let frame = try #require(final.first { $0.id == sector.id })
            #expect(abs(frame.start - sector.start) < 1e-12)
            #expect(abs(frame.end - sector.end) < 1e-12)
            #expect(abs(frame.innerRadius - sector.innerRadius) < 1e-12)
            #expect(abs(frame.outerRadius - sector.outerRadius) < 1e-12)
        }
    }

    @Test("Every animation intermediate has finite angles and bounded radii", arguments: ["Apps", "Apps/Alpha", "Apps/Alpha/.build"])
    func safeIntermediates(path: String) throws {
        let snapshot = snapshot()
        let source = ArtifactSunburstLayout(snapshot: snapshot)
        let destination = ArtifactSunburstLayout(snapshot: snapshot, focusID: path)
        let plan = try #require(ArtifactSunburstNavigation(source: source, destination: destination, selectedID: path))
        for step in 0...100 {
            let t = Double(step) / 100
            for (fade, expansion, reveal) in [(t, 0.0, 0.0), (1.0, t, 0.0), (1.0, 1.0, t)] {
                let frames = plan.frames(fade: fade, expansion: expansion, reveal: reveal)
                #expect(Set(frames.map(\.id)).count == frames.count)
                for frame in frames {
                    let finite = [frame.start, frame.end, frame.innerRadius, frame.outerRadius, frame.opacity].allSatisfy(\.isFinite)
                    #expect(finite)
                    #expect(frame.start >= 0 && frame.end <= 1 && frame.end > frame.start)
                    #expect(frame.innerRadius >= 0 && frame.outerRadius <= 1)
                    #expect(frame.outerRadius >= frame.innerRadius)
                    #expect(frame.opacity >= 0 && frame.opacity <= 1)
                }
            }
        }
    }

    @Test("Only a visible named sector can start a branch expansion")
    func missingSector() {
        let snapshot = snapshot()
        let source = ArtifactSunburstLayout(snapshot: snapshot)
        #expect(ArtifactSunburstNavigation(source: source, destination: source, selectedID: "missing") == nil)
        #expect(ArtifactSunburstNavigation(source: source, destination: source, selectedID: "") == nil)
    }
}

@Suite("Live diagram sessions")
@MainActor
struct ArtifactDiagramModelTests {
    @Test("Navigation freezes its endpoints while scanning continues, then catches up")
    func navigationCoalescesMeasurements() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        let first = artifact("Package/Alpha/.build")
        scan.apply(.discovered(generation: scan.generation, artifact: first))
        scan.apply(.completed(generation: scan.generation, artifactID: first.id, bytes: 80))
        await diagram.refresh(from: scan)
        diagram.navigate(to: "Package")
        let endpoint = diagram.layout
        diagram.setNavigationTransitionActive(true)
        let late = artifact("Package/Beta/target")
        scan.apply(.discovered(generation: scan.generation, artifact: late))
        scan.apply(.completed(generation: scan.generation, artifactID: late.id, bytes: 120))
        await diagram.refresh(from: scan)
        #expect(scan.rows.count == 2)
        #expect(diagram.layout == endpoint)
        diagram.setNavigationTransitionActive(false)
        await diagram.refresh(from: scan)
        #expect(diagram.focusID == "Package")
        #expect(diagram.focus.bytes == 200)
        #expect(diagram.layout.sectors.contains { $0.nodeID == "Package/Beta" })
    }

    @Test("Deep language badges follow discovery, preview and target replacement")
    func languageBadgesFollowLiveScan() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Mixed project")
        defer { scan.stop() }
        for (path, language) in [("Projects/Deep/Swift/.build", "Swift"),
                                 ("Projects/Deep/Rust/target", "Rust"),
                                 ("Projects/Python/__pycache__", "Python")] {
            let artifact = ScanArtifact(id: UUID(), relativePath: path, language: language, kind: .buildOutput)
            scan.apply(.discovered(generation: scan.generation, artifact: artifact))
        }
        await diagram.refresh(from: scan)
        #expect(diagram.children.first?.languages == [.python, .rust, .swift])
        diagram.preview("Projects")
        #expect(diagram.children.first { $0.id == "Projects/Deep" }?.languages == [.rust, .swift])
        diagram.navigate(to: "Projects/Deep")
        #expect(diagram.children.first { $0.name == "Rust" }?.languages == [.rust])
        scan.acceptDemoTarget(named: "Empty target")
        await diagram.refresh(from: scan)
        #expect(diagram.snapshot.root.languages.isEmpty)
        #expect(diagram.children.isEmpty)
    }

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

    @Test("Streaming refreshes update the snapshot to exactly what a full rebuild produces")
    func streamingRefreshIsIncremental() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        let artifacts = (0..<11_000).shuffled().map { index in
            artifact("Projects/group\(index % 97)/Project\(index)/Sources/.build")
        }
        for artifact in artifacts.prefix(10_000) {
            scan.apply(.discovered(generation: scan.generation, artifact: artifact))
        }
        await diagram.refresh(from: scan)

        for round in 0..<40 {
            for artifact in artifacts[(10_000 + round * 25)..<(10_000 + (round + 1) * 25)] {
                scan.apply(.discovered(generation: scan.generation, artifact: artifact))
            }
            for artifact in artifacts[(round * 25)..<((round + 1) * 25)] {
                scan.apply(.completed(generation: scan.generation, artifactID: artifact.id,
                                      bytes: Int64(round * 4_096), partial: round.isMultiple(of: 3)))
            }
            await diagram.refresh(from: scan)
        }

        #expect(diagram.snapshot == ArtifactSunburstSnapshot(rows: scan.rows))
    }

    @Test("A size change applied to a huge total matches a full rebuild exactly")
    func incrementalTotalsStayExact() {
        let unchanged = ScanRow(id: UUID(), relativePath: "A/.build", language: "Swift",
                                kind: .buildOutput, size: .measured(.max))
        let changing = ScanRow(id: UUID(), relativePath: "B/.build", language: "Swift",
                               kind: .buildOutput, size: .measuring)
        let trailing = ScanRow(id: UUID(), relativePath: "C/.build", language: "Swift",
                               kind: .buildOutput, size: .measured(2_048))
        let before = [unchanged, changing, trailing]
        var after = before
        after[1].size = .measured(1_024)

        let incremental = ArtifactSunburstSnapshot(
            rows: after, updating: ArtifactSunburstSnapshot(rows: before), previousSizes: before.map(\.size)
        )

        #expect(incremental == ArtifactSunburstSnapshot(rows: after))
    }

    @Test("Following a scan does not keep the scan's row storage alive")
    func refreshDoesNotRetainScanRows() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        let artifacts = ["A/.build", "B/.build", "C/.build"].map(artifact)
        for artifact in artifacts {
            scan.apply(.discovered(generation: scan.generation, artifact: artifact))
        }
        await diagram.refresh(from: scan)
        let storage = scan.rows.withUnsafeBufferPointer { $0.baseAddress }

        // A shared buffer would force the event reducer to copy every row.
        scan.apply(.completed(generation: scan.generation, artifactID: artifacts[0].id, bytes: 64))

        #expect(scan.rows.withUnsafeBufferPointer { $0.baseAddress } == storage)
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

    @Test("Navigation restores saved palettes; replacing the report discards them")
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

    @Test("Only the largest child inherits the entered folder color; other branches get distinct colors")
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
        let overview = diagram.palette
        let parentColor = try #require(overview.color(for: "Apps"))
        diagram.navigate(to: "Apps")
        #expect(diagram.palette.color(for: "Apps/Large") == parentColor)
        let smallColor = try #require(diagram.palette.color(for: "Apps/Small"))
        #expect(smallColor != parentColor)
        #expect(diagram.palette.color(for: "Apps/Small/.build") == smallColor)

        let late = artifact("Apps/Small/.venv")
        scan.apply(.discovered(generation: scan.generation, artifact: late))
        scan.apply(.completed(generation: scan.generation, artifactID: late.id, bytes: 1_000))
        await diagram.refresh(from: scan)
        #expect(diagram.palette.color(for: "Apps/Large") == parentColor)
        #expect(diagram.palette.color(for: "Apps/Small/.venv") == smallColor)
        diagram.navigate(to: "")
        for (path, color) in overview.colors { #expect(diagram.palette.color(for: path) == color) }
        diagram.navigate(to: "Apps")
        #expect(diagram.palette.color(for: "Apps/Small") == parentColor,
                "A deliberate new entry uses the largest child at that moment")
        #expect(diagram.palette.color(for: "Apps/Large") != parentColor)
    }

    @Test("Every entered level repeats the largest-child inheritance rule")
    func recursiveColorScopes() throws {
        let rows = [("Apps/Alpha/A/.build", 50), ("Apps/Alpha/B/target", 10),
                    ("Apps/Beta/.venv", 100), ("Apps/Gamma/.build", 20)].map { path, bytes in
            ScanRow(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput, size: .measured(Int64(bytes)))
        }
        let diagram = ArtifactDiagramModel(snapshot: ArtifactSunburstSnapshot(rows: rows))
        let overview = diagram.palette
        diagram.navigate(to: "Apps")
        let apps = diagram.palette
        let parent = try #require(overview.color(for: "Apps"))
        #expect(apps.color(for: "Apps/Beta") == parent)
        let alpha = try #require(apps.color(for: "Apps/Alpha"))
        let gamma = try #require(apps.color(for: "Apps/Gamma"))
        #expect(Set([alpha, gamma, parent]).count == 3)
        diagram.navigate(to: "Apps/Alpha")
        #expect(diagram.palette.color(for: "Apps/Alpha/A") == alpha)
        #expect(diagram.palette.color(for: "Apps/Alpha/A/.build") == alpha)
        #expect(diagram.palette.color(for: "Apps/Alpha/B") != alpha)
        #expect(diagram.palette.color(for: "Apps/Alpha/B/target") == diagram.palette.color(for: "Apps/Alpha/B"))
        diagram.navigateUp()
        #expect(diagram.palette == apps)
        diagram.navigateUp()
        #expect(diagram.palette == overview)
    }

    @Test("Entering the same folder from different levels preserves the color actually clicked")
    func skippedLevelEntryColors() throws {
        let rows = [("Apps/Alpha/A/.build", 50), ("Apps/Alpha/B/target", 10),
                    ("Apps/Beta/.venv", 100)].map { path, bytes in
            ScanRow(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput, size: .measured(Int64(bytes)))
        }
        let diagram = ArtifactDiagramModel(snapshot: ArtifactSunburstSnapshot(rows: rows))
        let original = try #require(diagram.palette.color(for: "Apps/Alpha"))
        diagram.navigate(to: "Apps/Alpha")
        #expect(diagram.palette.color(for: "Apps/Alpha/A") == original)
        let directEntry = diagram.palette
        diagram.navigateUp()
        let recolored = try #require(diagram.palette.color(for: "Apps/Alpha"))
        #expect(recolored != original)
        diagram.navigate(to: "Apps/Alpha")
        #expect(diagram.palette.color(for: "Apps/Alpha/A") == recolored)
        diagram.navigate(to: "")
        diagram.navigate(to: "Apps/Alpha")
        #expect(diagram.palette == directEntry)
    }

    @Test("Late discoveries cannot displace or reorder previously visible siblings")
    func stableStreamingOrder() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        for (index, path) in ["Zebra", "Middle", "Beta", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliet", "Kilo", "Lima"].enumerated() {
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
        #expect(diagram.layout.sectors.filter { $0.depth == 0 }.reduce(0) { $0 + $1.bytes } == 1_000_078)
        for (path, color) in colors { #expect(diagram.palette.colors[path] == color) }
        diagram.navigate(to: "Middle")
        diagram.navigate(to: "")
        #expect(diagram.layout.sectors.filter { $0.depth == 0 }.compactMap(\.nodeID) == original)
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

    @Test("Sidebar sorts numeric bytes descending during measurement, filtering and hover")
    func sidebarSizeSorting() async {
        let scan = WindowScanModel(source: DiagramIdleSource())
        let diagram = ArtifactDiagramModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        for (path, bytes) in [("Pack/Alpha/target", Int64(10)), ("Pack/Beta/.build", 200),
                              ("Pack/Gamma/.build", 200), ("Other/X/target", 1), ("Other/Y/target", 50)] {
            let entry = artifact(path)
            scan.apply(.discovered(generation: scan.generation, artifact: entry))
            scan.apply(.completed(generation: scan.generation, artifactID: entry.id, bytes: bytes))
        }
        scan.apply(.discovered(generation: scan.generation, artifact: artifact("Pack/Unknown/target")))
        await diagram.refresh(from: scan)
        #expect(diagram.children.map(\.name) == ["Pack", "Other"])
        diagram.navigate(to: "Pack")
        #expect(diagram.children.map(\.name) == ["Beta", "Gamma", "Alpha", "Unknown"])
        let sectorOrder = diagram.layout.sectors.filter { $0.depth == 0 }.map(\.id)
        diagram.query = "a"
        #expect(diagram.filteredChildren.map(\.name) == ["Beta", "Gamma", "Alpha"])
        diagram.preview("Other")
        #expect(diagram.filteredChildren.map(\.name) == ["Y", "X"])
        diagram.preview(nil)
        #expect(diagram.filteredChildren.map(\.name) == ["Beta", "Gamma", "Alpha"])
        let lateAlphaCache = artifact("Pack/Alpha/__pycache__")
        scan.apply(.discovered(generation: scan.generation, artifact: lateAlphaCache))
        scan.apply(.completed(generation: scan.generation, artifactID: lateAlphaCache.id, bytes: 500))
        await diagram.refresh(from: scan)
        #expect(diagram.children.map(\.name) == ["Alpha", "Beta", "Gamma", "Unknown"])
        #expect(diagram.layout.sectors.filter { $0.depth == 0 }.map(\.id) == sectorOrder,
                "Sorting the sidebar must not reorder diagram sectors")
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
