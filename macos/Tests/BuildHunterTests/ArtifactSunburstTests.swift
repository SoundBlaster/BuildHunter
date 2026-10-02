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
        var firstPalette = ArtifactSunburstPalette()
        var secondPalette = ArtifactSunburstPalette()
        firstPalette.include(first)
        secondPalette.include(second)
        #expect(firstPalette.colors == secondPalette.colors)
    }

    @Test("Folder colors survive drilling down, selecting an artifact and returning")
    func navigationColors() throws {
        let snapshot = ArtifactSunburstSnapshot(rows: [
            row("Apps/Alpha/.build", .measured(100)),
            row("Apps/Beta/target", .measured(200)),
            row("Tools/__pycache__", .measured(50))
        ])
        let overview = ArtifactSunburstLayout(snapshot: snapshot)
        var palette = ArtifactSunburstPalette()
        palette.include(overview)
        let originalColors = palette.colors
        for focus in ["Apps", "Apps/Alpha", "Apps/Alpha/.build", ""] {
            let focused = ArtifactSunburstLayout(snapshot: snapshot, focusID: focus)
            palette.include(focused)
            for sector in focused.sectors {
                let path = try #require(sector.nodeID)
                let original = try #require(originalColors[path])
                #expect(palette.color(for: sector) == original,
                        "The same folder must keep its color when focus changes to \(focus)")
            }
        }
    }

    @Test("Every descendant keeps the root branch color at every navigation depth")
    func branchColorInheritance() throws {
        let snapshot = ArtifactSunburstSnapshot(rows: [
            row("Apps/Alpha/.build", .measured(100)),
            row("Apps/Beta/target", .measured(200)),
            row("Tools/Linter/__pycache__", .measured(50))
        ])
        var palette = ArtifactSunburstPalette()
        for focus in ["", "Apps", "Apps/Alpha", "Apps/Alpha/.build", "Tools/Linter", ""] {
            let layout = ArtifactSunburstLayout(snapshot: snapshot, focusID: focus)
            palette.include(layout)
            for sector in layout.sectors {
                let path = try #require(sector.nodeID)
                let branch = String(path.split(separator: "/").first!)
                #expect(palette.color(for: sector) == palette.colors[branch],
                        "All rings in a branch retain the branch root's color after navigation")
            }
            #expect(palette.colors["Apps"] != palette.colors["Tools"])
        }
    }

    @Test("Seven sibling folders receive separated hues, not repeated palette slots")
    func contrastingSiblingColors() {
        let layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: (0..<7).map {
            row("Project\($0)/.build", .measured(100))
        }))
        var palette = ArtifactSunburstPalette()
        palette.include(layout)
        let inner = layout.sectors.filter { $0.depth == 0 }.map { palette.color(for: $0) }
        #expect(inner.count == 7)
        for (index, first) in inner.enumerated() {
            for second in inner.dropFirst(index + 1) {
                let difference = abs(first.hue - second.hue)
                #expect(min(difference, 1 - difference) > 0.055,
                        "Sibling hues should be separated by at least about 20 degrees")
            }
        }
        #expect(Set(palette.colors.values).count == 7,
                "Each top-level branch has one swatch shared by all of its children")
        #expect(palette.colors.values.allSatisfy {
            (0..<1).contains($0.hue) && $0.saturation > 0 && $0.brightness > 0
        })
    }

    @Test("Root branches contrast while every descendant shares its branch swatch")
    func distinctColorsAcrossBranches() {
        let paths = ["Apps/Alpha/.build", "Apps/Beta/target", "Apps/Gamma/.venv",
                     "Tools/Linter/__pycache__", "Server/target"]
        let layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: paths.map {
            row($0, .measured(100))
        }))
        var palette = ArtifactSunburstPalette()
        palette.include(layout)
        let swatches = ["Apps", "Tools", "Server"].compactMap { palette.colors[$0] }
        #expect(Set(palette.colors.values).count == swatches.count)
        for (index, first) in swatches.enumerated() {
            for second in swatches.dropFirst(index + 1) {
                let difference = abs(first.hue - second.hue)
                #expect(min(difference, 1 - difference) > 0.025,
                        "Different top-level branches must remain visually distinct")
            }
        }
    }

    @Test("Late discoveries and top-six membership changes retain assigned colors")
    func streamingColors() throws {
        var rows = (0..<9).map { row("Project\($0)/.build", .measured(Int64(100 + $0))) }
        var palette = ArtifactSunburstPalette()
        let first = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: rows))
        palette.include(first)
        let originalColors = palette.colors
        rows.append(row("A new first folder/.build", .measured(10_000)))
        rows[0] = row("Project0/.build", .partial(20_000))
        let updated = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: Array(rows.reversed())))
        palette.include(updated)
        for (path, color) in originalColors { #expect(palette.colors[path] == color) }
        #expect(palette.colors["Project0"] != nil)
        let other = try #require(updated.sectors.first { $0.id == .other("") })
        #expect(palette.color(for: other) == .other)
    }

    @Test("Other remains neutral at every focus and the selected target gets a real color")
    func neutralAggregation() throws {
        let snapshot = ArtifactSunburstSnapshot(rows: (0..<9).map {
            row("Apps/Project\($0)/.build", .measured(100))
        })
        var palette = ArtifactSunburstPalette()
        for focus in ["", "Apps"] {
            let layout = ArtifactSunburstLayout(snapshot: snapshot, focusID: focus)
            palette.include(layout)
            let other = try #require(layout.sectors.first { $0.id == .other("Apps") })
            #expect(palette.color(for: other) == .other)
        }
        let root = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [row(".", .measured(1))]))
        palette.include(root)
        let sector = try #require(root.sectors.first)
        #expect(palette.color(for: sector).saturation > 0)
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
        var palette = ArtifactSunburstPalette()
        palette.include(layout)
        #expect(palette.colors.count == layout.sectors.filter { $0.nodeID != nil }.count,
                "Color preparation must stay bounded by the visible sectors")
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

    @Test("Narrow sectors retain room for their fill after gaps and rounding", arguments: [160.0, 300.0, 500.0])
    func narrowSectorDecoration(plotRadius: Double) {
        let layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [
            row("Large/App/.build", .measured(199)), row("Small/App/.build", .measured(1))
        ]))
        let narrowSectors = layout.sectors.filter { $0.nodeID == "Small" || $0.nodeID?.hasPrefix("Small/") == true }
        #expect(narrowSectors.count == 3)
        for sector in narrowSectors {
            #expect(abs(sector.end - sector.start - 0.005) < 0.000_001)
            let innerEdgeWidth = 2 * plotRadius * sector.innerRadius * sin(.pi * (sector.end - sector.start))
            let decoration = sector.decoration(plotRadius: plotRadius)
            #expect(decoration.angularInset >= 0 && decoration.cornerRadius >= 0)
            #expect(2 * (decoration.angularInset + decoration.cornerRadius) < innerEdgeWidth)
        }
    }

    @Test("Wide sectors keep the requested gap and corner radius")
    func wideSectorDecoration() throws {
        let layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [
            row("A/.build", .measured(1)), row("B/.build", .measured(1))
        ]))
        let sector = try #require(layout.sectors.first)
        #expect(sector.decoration(plotRadius: 160).angularInset == 2)
        #expect(sector.decoration(plotRadius: 160).cornerRadius == 4)
    }

    @Test("Rendered radii preserve equal ring thickness, gaps and hit regions")
    func renderedRingBoundaries() {
        let layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [
            row("App/Core/.build", .measured(1))
        ]))
        var previousOuter: Double?
        for sector in layout.sectors {
            let renderedInner = sector.outerRadius * sector.innerRadiusRelativeToOuter
            #expect(abs(renderedInner - sector.innerRadius) < 0.000_001)
            #expect(abs(sector.outerRadius - renderedInner - (0.73 / 3)) < 0.000_001)
            if let previousOuter {
                #expect(abs(renderedInner - previousOuter - 0.025) < 0.000_001)
                #expect(layout.sector(angle: 0.5, radius: (renderedInner + previousOuter) / 2) == nil)
            }
            #expect(layout.sector(angle: 0.5, radius: (renderedInner + sector.outerRadius) / 2)?.id == sector.id)
            previousOuter = sector.outerRadius
        }
    }

    private func row(_ path: String, _ size: SizeState) -> ScanRow {
        ScanRow(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput, size: size)
    }
}
