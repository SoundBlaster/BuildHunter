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

    @Test("A tiny positive sibling receives the six degree floor and larger shares redistribute")
    func minimumAngleForExtremeRatio() throws {
        let layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [
            row("Large/.build", .measured(9_999)), row("Small/.build", .measured(1))
        ]))
        let large = try #require(layout.sectors.first { $0.id == .node("Large") })
        let small = try #require(layout.sectors.first { $0.id == .node("Small") })
        let minimum = ArtifactSunburstLayout.minimumSectorAngle
        #expect(abs((small.end - small.start) - minimum) < 1e-12)
        #expect(abs((large.end - large.start) - (1 - minimum)) < 1e-12)
        #expect(large.bytes == 9_999 && small.bytes == 1,
                "The angle floor must not change measured byte totals")
        #expect(large.start == 0)
        #expect(large.end == small.start)
        #expect(small.end == 1)
    }

    @Test("A narrow parent groups overflow into Other while retaining a prior child slot")
    func minimumAngleCapacityAndRetainedOrder() throws {
        let rows = [
            row("Large/.build", .measured(87)),
            row("Tiny/A/.build", .measured(1)),
            row("Tiny/B/.build", .measured(1)),
            row("Tiny/C/.build", .measured(1))
        ]
        let snapshot = ArtifactSunburstSnapshot(rows: rows)
        let layout = ArtifactSunburstLayout(snapshot: snapshot, retainedOrder: ["Tiny": ["Tiny/B"]])
        let tiny = try #require(layout.sectors.first { $0.id == .node("Tiny") })
        let children = layout.sectors.filter { $0.parentID == "Tiny" }.sorted { $0.start < $1.start }
        let minimum = ArtifactSunburstLayout.minimumSectorAngle

        #expect(children.map(\.id) == [.node("Tiny/B"), .other("Tiny")])
        #expect(children[1].bytes == 2)
        #expect(children.allSatisfy { $0.end - $0.start >= minimum - 1e-12 })
        #expect(children.first?.start == tiny.start)
        #expect(children.last?.end == tiny.end)
        #expect(abs(children[0].end - children[1].start) < 1e-12,
                "Sibling sectors must meet without overlap or gaps")

        let refreshed = ArtifactSunburstLayout(
            snapshot: ArtifactSunburstSnapshot(rows: Array(rows.reversed())),
            retainedOrder: ["Tiny": ["Tiny/B"]]
        )
        #expect(refreshed.sectors.filter { $0.parentID == "Tiny" }.map(\.id) == children.map(\.id))
        #expect(snapshot.nodes["Tiny"]?.bytes == 3)
    }

    @Test("A one-sector parent aggregates every child into Other")
    func minimumAngleSingleSlotAggregation() throws {
        let layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: [
            row("Massive/.build", .measured(9_999)),
            row("Tiny/A/.build", .measured(1)),
            row("Tiny/B/.build", .measured(1))
        ]))
        let tiny = try #require(layout.sectors.first { $0.id == .node("Tiny") })
        let children = layout.sectors.filter { $0.parentID == "Tiny" }
        let other = try #require(children.first { $0.id == .other("Tiny") })
        #expect(children.count == 1)
        #expect(other.start == tiny.start && other.end == tiny.end)
        #expect(other.end - other.start >= ArtifactSunburstLayout.minimumSectorAngle - 1e-12)
        #expect(other.bytes == 2)
    }

    @Test("Every displayed sibling group conserves its parent interval without overlap")
    func siblingIntervalConservation() throws {
        let snapshot = ArtifactSunburstSnapshot(rows: [
            row("Large/.build", .measured(9_999)),
            row("Small/A/.build", .measured(1)),
            row("Small/B/.build", .measured(1)),
            row("Dense/0/.build", .measured(1)),
            row("Dense/1/.build", .measured(1)),
            row("Dense/2/.build", .measured(1)),
            row("Dense/3/.build", .measured(1))
        ])
        let layout = ArtifactSunburstLayout(snapshot: snapshot)
        let minimum = ArtifactSunburstLayout.minimumSectorAngle

        for (parentID, siblings) in Dictionary(grouping: layout.sectors, by: \.parentID) {
            let ordered = siblings.sorted { $0.start < $1.start }
            guard let first = ordered.first, let last = ordered.last else { continue }
            if first.depth == 0 {
                #expect(first.start == 0 && last.end == 1)
            } else {
                guard let parent = layout.sectors.first(where: { $0.id == .node(parentID) }) else {
                    Issue.record("Missing parent sector for \(parentID)")
                    continue
                }
                #expect(first.start == parent.start)
                #expect(last.end == parent.end)
            }
            for sector in ordered {
                #expect(sector.start.isFinite && sector.end.isFinite && sector.end > sector.start)
                #expect(sector.end - sector.start >= minimum - 1e-12)
                if sector.id == .other(parentID) {
                    #expect(sector.bytes > 0)
                }
            }
            for pair in zip(ordered, ordered.dropFirst()) {
                #expect(abs(pair.0.end - pair.1.start) < 1e-12,
                        "Siblings under \(parentID) must be contiguous and nonoverlapping")
            }
        }
        #expect(layout.sectors.filter { $0.parentID == "Small" }.contains { $0.id == .other("Small") })
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

    @Test("An entered artifact leaf preserves its incoming color")
    func leafEntryColor() throws {
        let snapshot = ArtifactSunburstSnapshot(rows: [row("Apps/.build", .measured(100))])
        let incoming = ArtifactSunburstPalette.Swatch(hue: 0.64)
        var palette = ArtifactSunburstPalette(scope: .init(focusID: "Apps/.build", inheritedColor: incoming))
        let layout = ArtifactSunburstLayout(snapshot: snapshot, focusID: "Apps/.build")
        palette.include(layout)
        let sector = try #require(layout.sectors.first)
        #expect(palette.color(for: sector) == incoming)
        #expect(palette.inheritedBranchID == nil)
    }

    @Test("An unmeasured folder assigns its entry color only after a positive size arrives")
    func delayedInheritance() {
        let incoming = ArtifactSunburstPalette.Swatch(hue: 0.64)
        var palette = ArtifactSunburstPalette(scope: .init(focusID: "Apps", inheritedColor: incoming))
        let pending = ArtifactSunburstSnapshot(rows: [
            row("Apps/Zero/.build", .measured(0)), row("Apps/Unknown/target", .measuring)
        ])
        palette.include(ArtifactSunburstLayout(snapshot: pending, focusID: "Apps"))
        #expect(palette.inheritedBranchID == nil)
        let measured = ArtifactSunburstSnapshot(rows: [
            row("Apps/Zero/.build", .measured(0)), row("Apps/Unknown/target", .partial(10))
        ])
        palette.include(ArtifactSunburstLayout(snapshot: measured, focusID: "Apps"))
        #expect(palette.color(for: "Apps/Unknown") == incoming)
        #expect(palette.color(for: "Apps/Unknown/target") == incoming)
        #expect(palette.color(for: "Apps/Zero") == nil)
    }

    @Test("Equal largest children use the same path tie-break regardless of discovery order", arguments: [false, true])
    func inheritedColorTie(reverse: Bool) {
        let rows = [row("Apps/Alpha/.build", .measured(100)), row("Apps/Beta/target", .measured(100))]
        let snapshot = ArtifactSunburstSnapshot(rows: reverse ? Array(rows.reversed()) : rows)
        let incoming = ArtifactSunburstPalette.Swatch(hue: 0.64)
        var palette = ArtifactSunburstPalette(scope: .init(focusID: "Apps", inheritedColor: incoming))
        palette.include(ArtifactSunburstLayout(snapshot: snapshot, focusID: "Apps"))
        #expect(palette.color(for: "Apps/Alpha") == incoming)
        #expect(palette.color(for: "Apps/Beta") != incoming)
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

    @Test("Late discoveries and visible membership changes retain assigned colors")
    func streamingColors() throws {
        var rows = (0..<14).map { row("Project\($0)/.build", .measured(Int64(100 + $0))) }
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
        let snapshot = ArtifactSunburstSnapshot(rows: (0..<14).map {
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

    @Test("Twelve siblings remain individually visible; the thirteenth is grouped without losing bytes")
    func expandedSectorLimit() {
        let rows = (0..<13).map { row("Project\($0)/.build", .measured(100)) }
        let twelve = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: Array(rows.prefix(12))))
        let twelveRoots = twelve.sectors.filter { $0.depth == 0 }
        #expect(twelveRoots.count == 12)
        #expect(twelveRoots.allSatisfy { $0.nodeID != nil })
        let thirteen = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: rows))
        let roots = thirteen.sectors.filter { $0.depth == 0 }
        #expect(roots.compactMap(\.nodeID).count == 12)
        #expect(roots.first { $0.id == .other("") }?.bytes == 100)
        #expect(roots.reduce(0) { $0 + $1.bytes } == 1_300)
        #expect(roots.allSatisfy { $0.end - $0.start >= ArtifactSunburstLayout.minimumSectorAngle - 1e-12 })
    }

    @Test("Dense reports remain bounded without losing sizes or folder navigation")
    func denseReport() {
        let snapshot = ArtifactSunburstSnapshot(rows: (0..<10_000).map {
            row("Project\($0)/.build", .measured(10))
        })
        let layout = ArtifactSunburstLayout(snapshot: snapshot)
        let inner = layout.sectors.filter { $0.depth == 0 }
        #expect(inner.count == 13)
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
            #expect(abs(sector.end - sector.start - ArtifactSunburstLayout.minimumSectorAngle) < 0.000_001,
                    "The minimum angle is active for this tiny folder and its single-child descendants")
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
