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
