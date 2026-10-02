import Foundation

struct ArtifactSizeStatistics: Equatable, Sendable {
    var artifactCount = 0
    var measuringCount = 0
    var partialCount = 0
    var unavailableCount = 0
    var zeroCount = 0

    mutating func include(_ size: SizeState) {
        artifactCount += 1
        switch size {
        case .measuring:
            measuringCount += 1
        case .measured(let bytes):
            if bytes < 0 { unavailableCount += 1 }
            if bytes == 0 { zeroCount += 1 }
        case .partial(let bytes):
            partialCount += 1
            if bytes.map({ $0 < 0 }) ?? true { unavailableCount += 1 }
            if bytes == 0 { zeroCount += 1 }
        }
    }
}

struct ArtifactSunburstNode: Identifiable, Equatable, Sendable {
    let id: String
    let name: String
    let parentID: String?
    var children: [String] = []
    var bytes: Double = 0
    var statistics = ArtifactSizeStatistics()
    var artifact: ScanRow?
}

/// A projection of report rows, not another filesystem traversal. Directory nodes aggregate
/// only the artifact roots in the table; no contents of an artifact root are invented.
struct ArtifactSunburstSnapshot: Equatable, Sendable {
    let nodes: [String: ArtifactSunburstNode]
    var root: ArtifactSunburstNode { nodes[""]! }

    init(rows: [ScanRow]) {
        var nodes = ["": ArtifactSunburstNode(id: "", name: "All artifacts", parentID: nil)]
        for row in rows {
            let components = row.relativePath == "." ? [] : row.relativePath.split(separator: "/").map(String.init)
            var ancestors = [""]
            var parent = ""
            for component in components {
                let path = parent.isEmpty ? component : parent + "/" + component
                if nodes[path] == nil {
                    nodes[path] = ArtifactSunburstNode(id: path, name: component, parentID: parent)
                    nodes[parent]!.children.append(path)
                }
                ancestors.append(path)
                parent = path
            }
            let bytes: Double
            switch row.size {
            case .measured(let value), .partial(.some(let value)):
                bytes = Double(max(0, value))
            case .measuring, .partial(nil):
                bytes = 0
            }
            for path in ancestors {
                nodes[path]!.bytes += bytes
                nodes[path]!.statistics.include(row.size)
            }
            nodes[parent]!.artifact = row
        }
        for key in Array(nodes.keys) {
            nodes[key]!.children.sort()
        }
        self.nodes = nodes
    }
}

struct ArtifactSunburstLayout: Equatable, Sendable {
    struct Sector: Identifiable, Equatable, Sendable {
        enum ID: Hashable, Sendable {
            case node(String)
            case other(String)
        }
        let id: ID
        let parentID: String
        let name: String
        let bytes: Double
        let depth: Int
        let start: Double
        let end: Double
        let colorKey: String
        let isPartial: Bool

        var innerRadius: Double { 0.22 + Double(depth) * (ringWidth + Self.ringGap) }
        var outerRadius: Double { innerRadius + ringWidth }
        // SectorMark resolves its inner ratio against the sector's outer radius.
        var innerRadiusRelativeToOuter: Double { innerRadius / outerRadius }
        private static let ringGap = 0.025
        private var ringWidth: Double { (1 - 0.22 - 2 * Self.ringGap) / 3 }
        var angularRange: Range<Double> { start..<end }
        var nodeID: String? { if case .node(let path) = id { path } else { nil } }

        struct Decoration: Equatable, Sendable {
            let angularInset: Double
            let cornerRadius: Double
        }

        func decoration(plotRadius: Double) -> Decoration {
            guard plotRadius.isFinite, plotRadius > 0 else {
                return Decoration(angularInset: 0, cornerRadius: 0)
            }
            // Bound decoration at the narrowest (inner) edge. Keeping each inset
            // and corner below one eighth of that width leaves room for the fill,
            // even when a folder occupies only a fraction of a percent.
            let halfAngle = .pi * min(0.5, max(0, end - start))
            let innerEdgeWidth = 2 * innerRadius * plotRadius * sin(halfAngle)
            let thickness = (outerRadius - innerRadius) * plotRadius
            let allowance = min(innerEdgeWidth / 8, thickness / 4)
            return Decoration(angularInset: min(2, allowance), cornerRadius: min(4, allowance))
        }
    }

    let sectors: [Sector]

    init(snapshot: ArtifactSunburstSnapshot, focusID: String = "") {
        let focus = snapshot.nodes[focusID] ?? snapshot.root
        var sectors: [Sector] = []
        func visit(_ parent: ArtifactSunburstNode, depth: Int, start: Double, end: Double, colorKey: String?) {
            guard depth < 3, parent.bytes > 0 else { return }
            let positive = parent.children.compactMap { snapshot.nodes[$0] }.filter { $0.bytes > 0 }
            // At most six named children plus Other per ring; the sidebar retains every child.
            let visible: [ArtifactSunburstNode]
            let remaining: [ArtifactSunburstNode]
            if positive.count > 7 {
                let ranked = positive.sorted { $0.bytes == $1.bytes ? $0.id < $1.id : $0.bytes > $1.bytes }
                visible = Array(ranked.prefix(6)).sorted { $0.id < $1.id }
                remaining = Array(ranked.dropFirst(6))
            } else {
                visible = positive
                remaining = []
            }
            let total = positive.reduce(0) { $0 + $1.bytes }
            guard total > 0 else { return }
            var cursor = start
            for (index, node) in visible.enumerated() {
                let upper = remaining.isEmpty && index == visible.count - 1
                    ? end : min(end, cursor + (end - start) * (node.bytes / total))
                let color = colorKey ?? node.id
                if upper > cursor {
                    sectors.append(Sector(id: .node(node.id), parentID: parent.id, name: node.name,
                                          bytes: node.bytes, depth: depth, start: cursor, end: upper,
                                          colorKey: color, isPartial: node.statistics.partialCount > 0))
                    visit(node, depth: depth + 1, start: cursor, end: upper, colorKey: color)
                }
                cursor = upper
            }
            if !remaining.isEmpty, end > cursor {
                sectors.append(Sector(
                    id: .other(parent.id), parentID: parent.id, name: "Other (\(remaining.count))",
                    bytes: remaining.reduce(0) { $0 + $1.bytes }, depth: depth, start: cursor, end: end,
                    colorKey: colorKey ?? "", isPartial: remaining.contains { $0.statistics.partialCount > 0 }
                ))
            }
        }
        if focus.children.isEmpty, focus.bytes > 0 {
            sectors.append(Sector(id: .node(focus.id), parentID: focus.parentID ?? "", name: focus.name,
                                  bytes: focus.bytes, depth: 0, start: 0, end: 1, colorKey: focus.id,
                                  isPartial: focus.statistics.partialCount > 0))
        } else {
            visit(focus, depth: 0, start: 0, end: 1, colorKey: nil)
        }
        self.sectors = sectors
    }

    func sector(angle: Double, radius: Double) -> Sector? {
        sectors.first {
            angle >= $0.start && angle < $0.end && radius >= $0.innerRadius && radius <= $0.outerRadius
        }
    }
}
