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

    init(snapshot: ArtifactSunburstSnapshot, focusID: String = "", retainedOrder: [String: [String]] = [:]) {
        let focus = snapshot.nodes[focusID] ?? snapshot.root
        var sectors: [Sector] = []
        func visit(_ parent: ArtifactSunburstNode, depth: Int, start: Double, end: Double) {
            guard depth < 3, parent.bytes > 0 else { return }
            let positive = parent.children.compactMap { snapshot.nodes[$0] }.filter { $0.bytes > 0 }
            // Keep existing slots in discovery order. A late, larger measurement must
            // not eject a visible folder or move it across its siblings.
            let previousIDs = retainedOrder[parent.id] ?? []
            let previousSet = Set(previousIDs)
            let existing = previousIDs.compactMap { snapshot.nodes[$0] }.filter { $0.bytes > 0 }
            let newNodes = positive.filter { !previousSet.contains($0.id) }
            let visible: [ArtifactSunburstNode]
            let remaining: [ArtifactSunburstNode]
            if positive.count > 7 {
                let ranked = newNodes.sorted { $0.bytes == $1.bytes ? $0.id < $1.id : $0.bytes > $1.bytes }
                let added = Array(ranked.prefix(max(0, 6 - existing.count))).sorted { $0.id < $1.id }
                visible = existing + added
                let visibleIDs = Set(visible.map(\.id))
                remaining = positive.filter { !visibleIDs.contains($0.id) }
            } else {
                visible = existing + newNodes
                remaining = []
            }
            let total = positive.reduce(0) { $0 + $1.bytes }
            guard total > 0 else { return }
            var cursor = start
            for (index, node) in visible.enumerated() {
                let upper = remaining.isEmpty && index == visible.count - 1
                    ? end : min(end, cursor + (end - start) * (node.bytes / total))
                if upper > cursor {
                    sectors.append(Sector(id: .node(node.id), parentID: parent.id, name: node.name,
                                          bytes: node.bytes, depth: depth, start: cursor, end: upper,
                                          isPartial: node.statistics.partialCount > 0))
                    visit(node, depth: depth + 1, start: cursor, end: upper)
                }
                cursor = upper
            }
            if !remaining.isEmpty, end > cursor {
                sectors.append(Sector(
                    id: .other(parent.id), parentID: parent.id, name: "Other (\(remaining.count))",
                    bytes: remaining.reduce(0) { $0 + $1.bytes }, depth: depth, start: cursor, end: end,
                    isPartial: remaining.contains { $0.statistics.partialCount > 0 }
                ))
            }
        }
        if focus.children.isEmpty, focus.bytes > 0 {
            sectors.append(Sector(id: .node(focus.id), parentID: focus.parentID ?? "", name: focus.name,
                                  bytes: focus.bytes, depth: 0, start: 0, end: 1,
                                  isPartial: focus.statistics.partialCount > 0))
        } else {
            visit(focus, depth: 0, start: 0, end: 1)
        }
        self.sectors = sectors
    }

    func sector(angle: Double, radius: Double) -> Sector? {
        sectors.first {
            angle >= $0.start && angle < $0.end && radius >= $0.innerRadius && radius <= $0.outerRadius
        }
    }
}

/// A palette belongs to one navigation scope. Its immediate branches have distinct
/// colors, shared by their descendants; the largest branch inherits the entry color.
struct ArtifactSunburstPalette: Equatable, Sendable {
    struct Swatch: Hashable, Sendable {
        let hue: Double
        var saturation: Double = 0.72
        var brightness: Double = 0.88

        static let other = Swatch(hue: 0, saturation: 0, brightness: 0.55)
    }

    struct Scope: Hashable, Sendable {
        var focusID = ""
        var inheritedColor: Swatch?
    }

    let scope: Scope
    private(set) var colors: [String: Swatch] = [:]
    private(set) var inheritedBranchID: String?
    private var sortedHues: [Double] = []

    init(scope: Scope = Scope()) {
        self.scope = scope
        // A folder reached through the sidebar may not have had a visible sector.
        // Give it a deterministic entry color, then apply the same inheritance rule.
        if let color = scope.inheritedColor ?? (scope.focusID.isEmpty ? nil : Swatch(hue: Self.preferredHue(for: scope.focusID))) {
            colors[scope.focusID] = color
            sortedHues = [color.hue]
        }
    }

    func color(for sector: ArtifactSunburstLayout.Sector) -> Swatch {
        sector.nodeID.flatMap { color(for: $0) } ?? .other
    }

    func color(for path: String) -> Swatch? {
        branchRoot(for: path).flatMap { colors[$0] }
    }

    mutating func include(_ layout: ArtifactSunburstLayout) {
        if inheritedBranchID == nil, let entryColor = colors[scope.focusID] {
            let children = layout.sectors.compactMap { sector -> (path: String, bytes: Double)? in
                guard sector.depth == 0, let path = sector.nodeID, path != scope.focusID else { return nil }
                return (path, sector.bytes)
            }
            if let largest = children.max(by: { $0.bytes == $1.bytes ? $0.path > $1.path : $0.bytes < $1.bytes }) {
                inheritedBranchID = largest.path
                colors[largest.path] = entryColor
            }
        }
        let paths = layout.sectors.compactMap(\.nodeID).sorted()
        for path in paths {
            guard let branch = branchRoot(for: path) else { continue }
            if colors[branch] == nil {
                let hue = contrastingHue(preferred: Self.preferredHue(for: branch), nearby: sortedHues)
                colors[branch] = Swatch(hue: hue)
                sortedHues.insert(hue, at: insertionIndex(for: hue))
            }
            // Cache visible paths as well, so identities remain directly inspectable.
            colors[path] = colors[branch]
        }
    }

    private func branchRoot(for path: String) -> String? {
        if path == scope.focusID { return path }
        let prefix = scope.focusID.isEmpty ? "" : scope.focusID + "/"
        guard path.hasPrefix(prefix) else { return nil }
        return path.dropFirst(prefix.count).split(separator: "/").first.map { prefix + $0 }
    }

    private func contrastingHue(preferred: Double, nearby: [Double]) -> Double {
        guard !sortedHues.isEmpty else { return preferred }
        let candidates = (0..<72).map { step in
            let hue = (preferred + Double(step) / 72).truncatingRemainder(dividingBy: 1)
            return (hue: hue, global: nearestDistance(to: hue),
                    local: nearby.map { Self.distance(hue, $0) }.min() ?? 1)
        }
        let greatestDistance = candidates.map { $0.global }.max()!
        if greatestDistance == 0 {
            // Even if all sampled hues were used, a gap between them remains.
            let gaps = sortedHues.indices.map { index in
                let end = index + 1 < sortedHues.count ? sortedHues[index + 1] : sortedHues[0] + 1
                return (start: sortedHues[index], width: end - sortedHues[index])
            }
            let widest = gaps.max { $0.width < $1.width }!
            return (widest.start + widest.width / 2).truncatingRemainder(dividingBy: 1)
        }
        // First exclude hues close to ANY previous folder, including other rings.
        // Among the well-separated candidates, prefer contrast with direct neighbors.
        return candidates.filter { $0.global >= greatestDistance * 0.8 }.max {
            abs($0.local - $1.local) < 0.000_001 ? $0.global < $1.global : $0.local < $1.local
        }!.hue
    }

    private func nearestDistance(to hue: Double) -> Double {
        let index = insertionIndex(for: hue)
        let before = sortedHues[(index + sortedHues.count - 1) % sortedHues.count]
        let after = sortedHues[index % sortedHues.count]
        return min(Self.distance(hue, before), Self.distance(hue, after))
    }

    private func insertionIndex(for hue: Double) -> Int {
        var lower = 0
        var upper = sortedHues.count
        while lower < upper {
            let middle = (lower + upper) / 2
            if sortedHues[middle] < hue { lower = middle + 1 } else { upper = middle }
        }
        return lower
    }

    private static func preferredHue(for path: String) -> Double {
        let hash = path.utf8.reduce(UInt64(14_695_981_039_346_656_037)) {
            ($0 ^ UInt64($1)) &* 1_099_511_628_211
        }
        return Double(hash >> 11) / 9_007_199_254_740_992
    }

    private static func distance(_ first: Double, _ second: Double) -> Double {
        let difference = abs(first - second)
        return min(difference, 1 - difference)
    }
}
