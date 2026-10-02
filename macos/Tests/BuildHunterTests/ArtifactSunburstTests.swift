import Foundation
import Testing
@testable import BuildHunter

@Suite("Artifact sunburst")
struct ArtifactSunburstTests {
    @Test("Folder totals conserve the sizes of the table rows")
    func totalsAndRanges() throws {
        let snapshot = ArtifactSunburstSnapshot(rows: [
            row("Apps/Alpha/.build", .measured(100)),
            row("Apps/Beta/target", .measured(300)),
            row("Tools/__pycache__", .measured(600))
        ])
        #expect(snapshot.root.bytes == 1_000)
        #expect(snapshot.nodes["Apps"]?.bytes == 400)
        #expect(snapshot.root.statistics.artifactCount == 3)
        let layout = ArtifactSunburstLayout(snapshot: snapshot)
        let apps = try #require(layout.sectors.first { $0.id == .node("Apps") })
        #expect(abs(apps.end - apps.start - 0.4) < 0.000_001)
        for sector in layout.sectors where sector.depth > 0 {
            let parent = try #require(layout.sectors.first { $0.id == .node(sector.parentID) })
            #expect(sector.start >= parent.start)
            #expect(sector.end <= parent.end)
        }
        #expect(layout.sectors.filter { $0.depth == 0 }.last?.end == 1)
    }

    @Test("Unknown and zero sizes never receive fabricated sector area")
    func incompleteMeasurements() {
        let snapshot = ArtifactSunburstSnapshot(rows: [
            row("Pending/.build", .measuring),
            row("Partial/target", .partial(50)),
            row("Unknown/__pycache__", .partial(nil)),
            row("Zero/.pytest_cache", .measured(0)),
            row("Invalid/.build", .measured(-1))
        ])
        #expect(snapshot.root.bytes == 50)
        #expect(snapshot.root.statistics.artifactCount == 5)
        #expect(snapshot.root.statistics.measuringCount == 1)
        #expect(snapshot.root.statistics.partialCount == 2)
        #expect(snapshot.root.statistics.unavailableCount == 2)
        #expect(snapshot.root.statistics.zeroCount == 1)
        let layout = ArtifactSunburstLayout(snapshot: snapshot)
        #expect(layout.sectors.allSatisfy { $0.bytes > 0 && $0.end > $0.start })
        #expect(layout.sectors.first?.id == .node("Partial"))
        #expect(layout.sectors.first?.isPartial == true)
        #expect(snapshot.nodes["Pending/.build"] != nil)
    }

    @Test("Path identities survive new scan UUIDs, event order and size changes")
    func stableIdentity() {
        let first = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [
            row("B/.build", .measured(100)), row("A/target", .measured(50))
        ]))
        let second = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [
            row("A/target", .measured(70)), row("B/.build", .measured(200))
        ]))
        #expect(first.sectors.map(\.id) == second.sectors.map(\.id))
        #expect(first.sectors.map(\.colorKey) == second.sectors.map(\.colorKey))
    }

    @Test("A selected artifact root is represented by a full sector")
    func selectedRoot() {
        let snapshot = ArtifactSunburstSnapshot(rows: [row(".", .measured(42))])
        let layout = ArtifactSunburstLayout(snapshot: snapshot)
        #expect(snapshot.root.bytes == 42)
        #expect(layout.sectors.count == 1)
        #expect(layout.sectors.first?.start == 0)
        #expect(layout.sectors.first?.end == 1)
    }

    @Test("Dense reports remain bounded without losing sizes or folder navigation")
    func denseReport() {
        let snapshot = ArtifactSunburstSnapshot(rows: (0..<10_000).map {
            row("Project\($0)/.build", .measured(10))
        })
        let layout = ArtifactSunburstLayout(snapshot: snapshot)
        let inner = layout.sectors.filter { $0.depth == 0 }
        #expect(inner.count == 7)
        #expect(inner.contains { $0.id == .other("") })
        #expect(inner.reduce(0) { $0 + $1.bytes } == 100_000)
        #expect(layout.sectors.count <= 399)
        #expect(snapshot.root.children.count == 10_000)
        let zoomed = ArtifactSunburstLayout(snapshot: snapshot, focusID: "Project9999")
        #expect(zoomed.sectors.first?.id == .node("Project9999/.build"))
        #expect(zoomed.sectors.first?.bytes == 10)
    }

    @Test("Large totals stay finite and empty reports have no sectors")
    func numericBoundaries() {
        let snapshot = ArtifactSunburstSnapshot(rows: [
            row("A/.build", .measured(.max)), row("B/.build", .measured(.max))
        ])
        #expect(snapshot.root.bytes > Double(Int64.max))
        #expect(ArtifactSunburstLayout(snapshot: snapshot).sectors.allSatisfy {
            $0.start.isFinite && $0.end.isFinite && $0.start >= 0 && $0.end <= 1
        })
        #expect(ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [])).sectors.isEmpty)
    }

    @Test("Hit testing distinguishes rings and ignores the centre and outside")
    func hitTesting() throws {
        let layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [
            row("App/.build", .measured(10))
        ]))
        let inner = try #require(layout.sectors.first { $0.depth == 0 })
        let outer = try #require(layout.sectors.first { $0.depth == 1 })
        #expect(layout.sector(angle: 0.5, radius: (inner.innerRadius + inner.outerRadius) / 2)?.id == inner.id)
        #expect(layout.sector(angle: 0.5, radius: (outer.innerRadius + outer.outerRadius) / 2)?.id == outer.id)
        #expect(layout.sector(angle: 0.5, radius: 0.1) == nil)
        #expect(layout.sector(angle: 0.5, radius: 1.1) == nil)
    }

    private func row(_ path: String, _ size: SizeState) -> ScanRow {
        ScanRow(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput, size: size)
    }
}
