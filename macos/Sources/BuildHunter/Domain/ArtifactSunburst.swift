import Foundation

struct ArtifactSizeStatistics: Equatable, Sendable {
    var artifactCount = 0
    var measuringCount = 0
    var partialCount = 0
    var unavailableCount = 0
    var zeroCount = 0

    mutating func include(_ size: SizeState) {
        add(size, count: 1)
    }

    /// Reverses `include`, so a size change can be applied without recounting a folder.
    mutating func exclude(_ size: SizeState) {
        add(size, count: -1)
    }

    private mutating func add(_ size: SizeState, count: Int) {
        artifactCount += count
        switch size {
        case .measuring:
            measuringCount += count
        case .measured(let bytes):
            if bytes < 0 { unavailableCount += count }
            if bytes == 0 { zeroCount += count }
        case .partial(let bytes):
            partialCount += count
            if bytes.map({ $0 < 0 }) ?? true { unavailableCount += count }
            if bytes == 0 { zeroCount += count }
        }
    }
}

/// An exact sum of non-negative `Int64` sizes. 128 bits cannot overflow for any report, so
/// adding and removing sizes in any order gives the same total as summing them once.
struct ByteTotal: Equatable, Sendable {
    private var high: UInt64 = 0
    private var low: UInt64 = 0

    mutating func add(_ bytes: UInt64) {
        let (sum, carry) = low.addingReportingOverflow(bytes)
        low = sum
        if carry { high &+= 1 }
    }

    mutating func subtract(_ bytes: UInt64) {
        let (difference, borrow) = low.subtractingReportingOverflow(bytes)
        low = difference
        if borrow { high &-= 1 }
    }

    var value: Double { Double(high) * 0x1p64 + Double(low) }
}

enum ArtifactLanguage: String, CaseIterable, Hashable, Sendable {
    case python = "Python"
    case rust = "Rust"
    case swift = "Swift"

    var assetName: String { "Language" + rawValue }
}

struct ArtifactSunburstNode: Identifiable, Equatable, Sendable {
    let id: String
    let name: String
    let parentID: String?
    var children: [String] = []
    var total = ByteTotal()
    var statistics = ArtifactSizeStatistics()
    var artifact: ScanRow?
    var languages: Set<ArtifactLanguage> = []

    var bytes: Double { total.value }
}

/// A projection of report rows, not another filesystem traversal. Directory nodes aggregate
/// only the artifact roots in the table; no contents of an artifact root are invented.
struct ArtifactSunburstSnapshot: Equatable, Sendable {
    let nodes: [String: ArtifactSunburstNode]
    var root: ArtifactSunburstNode { nodes[""]! }

    init(rows: [ScanRow]) {
        var nodes = ["": ArtifactSunburstNode(id: "", name: "All artifacts", parentID: nil)]
        for row in rows {
            Self.insert(row, into: &nodes, keepingChildrenSorted: false)
        }
        for key in Array(nodes.keys) {
            nodes[key]!.children.sort()
        }
        self.nodes = nodes
    }

    /// Equivalent to `init(rows:)` when `rows` extends the rows behind `previous`, which is how
    /// a report grows: rows are appended and only their sizes change. `previousSizes` holds
    /// those rows' sizes in order; only changed rows walk their ancestors.
    init(rows: [ScanRow], updating previous: ArtifactSunburstSnapshot, previousSizes: [SizeState]) {
        var nodes = previous.nodes
        for (old, row) in zip(previousSizes, rows) where old != row.size {
            var path: String? = Self.leafPath(row.relativePath)
            while let current = path {
                nodes[current]!.total.subtract(Self.bytes(old))
                nodes[current]!.total.add(Self.bytes(row.size))
                nodes[current]!.statistics.exclude(old)
                nodes[current]!.statistics.include(row.size)
                path = nodes[current]!.parentID
            }
            nodes[Self.leafPath(row.relativePath)]!.artifact = row
        }
        for row in rows.dropFirst(previousSizes.count) {
            Self.insert(row, into: &nodes, keepingChildrenSorted: true)
        }
        self.nodes = nodes
    }

    private static func leafPath(_ relativePath: String) -> String {
        relativePath == "." ? "" : relativePath.split(separator: "/").joined(separator: "/")
    }

    private static func bytes(_ size: SizeState) -> UInt64 {
        switch size {
        case .measured(let value), .partial(.some(let value)):
            UInt64(max(0, value))
        case .measuring, .partial(nil):
            0
        }
    }

    private static func insert(_ row: ScanRow, into nodes: inout [String: ArtifactSunburstNode],
                               keepingChildrenSorted: Bool) {
        let components = row.relativePath == "." ? [] : row.relativePath.split(separator: "/").map(String.init)
        var ancestors = [""]
        var parent = ""
        for component in components {
            let path = parent.isEmpty ? component : parent + "/" + component
            if nodes[path] == nil {
                nodes[path] = ArtifactSunburstNode(id: path, name: component, parentID: parent)
                if keepingChildrenSorted {
                    let siblings = nodes[parent]!.children
                    var low = siblings.startIndex
                    var high = siblings.endIndex
                    while low < high {
                        let middle = (low + high) / 2
                        if siblings[middle] < path { low = middle + 1 } else { high = middle }
                    }
                    nodes[parent]!.children.insert(path, at: low)
                } else {
                    nodes[parent]!.children.append(path)
                }
            }
            ancestors.append(path)
            parent = path
        }
        let bytes = bytes(row.size)
        for path in ancestors {
            nodes[path]!.total.add(bytes)
            nodes[path]!.statistics.include(row.size)
            if let language = ArtifactLanguage(rawValue: row.language) {
                nodes[path]!.languages.insert(language)
            }
        }
        nodes[parent]!.artifact = row
    }
}

struct ArtifactSunburstLayout: Equatable, Sendable {
    /// One sixtieth of a complete turn (six degrees).
    static let minimumSectorAngle = 1.0 / 60.0
    static let maximumVisibleChildren = 12

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
            let selectedVisible: [ArtifactSunburstNode]
            let selectedRemaining: [ArtifactSunburstNode]
            if positive.count > Self.maximumVisibleChildren {
                let ranked = newNodes.sorted { $0.bytes == $1.bytes ? $0.id < $1.id : $0.bytes > $1.bytes }
                let added = Array(ranked.prefix(max(0, Self.maximumVisibleChildren - existing.count))).sorted { $0.id < $1.id }
                selectedVisible = existing + added
                let visibleIDs = Set(selectedVisible.map(\.id))
                selectedRemaining = positive.filter { !visibleIDs.contains($0.id) }
            } else {
                selectedVisible = existing + newNodes
                selectedRemaining = []
            }

            // A parent can show only as many readable sectors as fit at the floor.
            // Keep already visible slots first, then group every displaced child
            // with any existing overflow in Other.
            let span = end - start
            let capacity = max(1, Int(floor(span / Self.minimumSectorAngle + 1e-12)))
            var visible = selectedVisible
            var remaining = selectedRemaining
            let candidateCount = visible.count + (remaining.isEmpty ? 0 : 1)
            if candidateCount > capacity {
                let retainedCount = max(0, capacity - 1)
                remaining.append(contentsOf: visible.dropFirst(retainedCount))
                visible = Array(visible.prefix(retainedCount))
            }

            let otherBytes = remaining.reduce(0) { $0 + $1.bytes }
            let weights = visible.map(\.bytes) + (remaining.isEmpty ? [] : [otherBytes])
            guard !weights.isEmpty else { return }
            let widths = Self.minimumBoundedWidths(weights: weights, span: span)
            var cursor = start
            for (index, node) in visible.enumerated() {
                let upper = index == widths.count - 1 && remaining.isEmpty
                    ? end : min(end, cursor + widths[index])
                if upper > cursor {
                    sectors.append(Sector(id: .node(node.id), parentID: parent.id, name: node.name,
                                          bytes: node.bytes, depth: depth, start: cursor, end: upper,
                                          isPartial: node.statistics.partialCount > 0))
                    visit(node, depth: depth + 1, start: cursor, end: upper)
                }
                cursor = upper
            }
            if !remaining.isEmpty, end > cursor {
                let upper = end
                sectors.append(Sector(
                    id: .other(parent.id), parentID: parent.id, name: "Other (\(remaining.count))",
                    bytes: otherBytes, depth: depth, start: cursor, end: upper,
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

    /// Applies a minimum width to small shares, then redistributes the remaining
    /// angle among larger shares in proportion to their byte totals.
    private static func minimumBoundedWidths(weights: [Double], span: Double) -> [Double] {
        guard weights.count > 1 else { return [span] }
        var widths = Array(repeating: 0.0, count: weights.count)
        var active = Set(weights.indices)
        var remainingAngle = span
        var remainingWeight = weights.reduce(0, +)

        while !active.isEmpty {
            let undersized = active.filter {
                remainingAngle * (weights[$0] / remainingWeight) < minimumSectorAngle
            }.sorted()
            guard !undersized.isEmpty else {
                for index in active {
                    widths[index] = remainingAngle * (weights[index] / remainingWeight)
                }
                break
            }
            for index in undersized {
                widths[index] = minimumSectorAngle
                remainingAngle -= minimumSectorAngle
                remainingWeight -= weights[index]
                active.remove(index)
            }
        }
        return widths
    }

    func sector(angle: Double, radius: Double) -> Sector? {
        sectors.first {
            angle >= $0.start && angle < $0.end && radius >= $0.innerRadius && radius <= $0.outerRadius
        }
    }
}

/// A frozen navigation plan. Scan updates cannot change its geometry midway through.
/// Entering a folder works like a zoom: neighbors fade, then in one motion the selected
/// sector opens to a full turn while sinking into the center disc, its descendants move
/// straight to their new rings, and newly exposed levels slide in from the outer edge.
/// Returning plays the same path backwards.
struct ArtifactSunburstNavigation: Sendable {
    enum Direction: Sendable { case descend, ascend }
    let direction: Direction

    /// The disc behind the center button, where an entered folder ends up.
    static let centerRadius = 0.21

    struct Frame: Identifiable, Sendable {
        let id: ArtifactSunburstLayout.Sector.ID
        let start: Double
        let end: Double
        let innerRadius: Double
        let outerRadius: Double
        let opacity: Double
        let depth: Double
        let isSelectedBranch: Bool
    }

    private struct Entry: Sendable {
        let id: ArtifactSunburstLayout.Sector.ID
        let source: ArtifactSunburstLayout.Sector?
        let destination: ArtifactSunburstLayout.Sector?
        let isSelectedBranch: Bool
    }

    /// Angles and radii at one end of the motion.
    private struct Geometry {
        let start: Double
        let end: Double
        let inner: Double
        let outer: Double
        let depth: Double
    }

    private let anchor: ArtifactSunburstLayout.Sector
    private let entries: [Entry]
    /// Rings the entered branch moves inward: the anchor's own ring plus the center.
    private let shift: Int

    init?(source: ArtifactSunburstLayout, destination: ArtifactSunburstLayout, selectedID: String,
          direction: Direction = .descend) {
        let parent = direction == .descend ? source : destination
        let child = direction == .descend ? destination : source
        // All artifacts can skip levels. Contract into the closest visible ancestor;
        // a folder grouped out of its parent contracts into that parent's Other.
        let ancestor = direction == .ascend ? parent.sectors.filter {
            $0.nodeID.map { selectedID.hasPrefix($0 + "/") } ?? false
        }.max { ($0.nodeID?.count ?? 0) < ($1.nodeID?.count ?? 0) } : nil
        let other = direction == .ascend ? parent.sectors.filter {
            if case .other = $0.id { return $0.parentID.isEmpty || selectedID.hasPrefix($0.parentID + "/") }
            return false
        }.max { $0.parentID.count < $1.parentID.count } : nil
        guard let anchor = parent.sectors.first(where: { $0.nodeID == selectedID }) ?? ancestor ?? other,
              anchor.start.isFinite, anchor.end.isFinite, anchor.end > anchor.start else { return nil }
        self.direction = direction
        self.anchor = anchor
        shift = anchor.depth + 1
        let branch = anchor.nodeID ?? selectedID
        let destinations = Dictionary(uniqueKeysWithValues: child.sectors.map { ($0.id, $0) })
        var entries = parent.sectors.map { sector in
            let path = sector.nodeID ?? sector.parentID
            return Entry(id: sector.id, source: sector, destination: destinations[sector.id],
                         isSelectedBranch: sector.id == anchor.id || path == branch || path.hasPrefix(branch + "/"))
        }
        let sourceIDs = Set(parent.sectors.map(\.id))
        entries.append(contentsOf: child.sectors.filter { !sourceIDs.contains($0.id) }.map {
            Entry(id: $0.id, source: nil, destination: $0, isSelectedBranch: true)
        })
        self.entries = entries
    }

    static let fadeDuration = 0.14
    static let zoomDuration = 0.55
    static let duration = fadeDuration + zoomDuration

    /// Seconds until the second phase starts: the zoom when entering, the fade when returning.
    var firstPhaseDuration: Double { direction == .descend ? Self.fadeDuration : Self.zoomDuration }

    /// Eased `fade` and `zoom` progress `elapsed` seconds into the transition. The view
    /// evaluates this every display frame instead of letting SwiftUI interpolate between
    /// endpoint states, so every frame lies on the planned trajectory.
    func progress(at elapsed: Double) -> (fade: Double, zoom: Double) {
        let time = elapsed.isFinite ? max(0, elapsed) : 0
        let first = min(1, time / firstPhaseDuration)
        let second = min(1, max(0, time - firstPhaseDuration) / (Self.duration - firstPhaseDuration))
        return direction == .descend
            ? (Self.easeOut(first), Self.smooth(second))
            : (Self.easeOut(second), Self.smooth(first))
    }

    /// Cubic ease-out, matching the neighbors' quick fade.
    static func easeOut(_ x: Double) -> Double { 1 - pow(1 - min(1, max(0, x)), 3) }

    /// A critically damped spring like SwiftUI's `.smooth`, scaled to end exactly at 1.
    static func smooth(_ x: Double) -> Double {
        let x = min(1, max(0, x))
        func spring(_ x: Double) -> Double { 1 - (1 + 2 * .pi * x) * exp(-2 * .pi * x) }
        return spring(x) / spring(1)
    }

    /// `fade` and `zoom` each run from 0 to 1 in time order. Entering fades the neighbors,
    /// then zooms; returning zooms back out, then fades the neighbors in again.
    func frames(fade: Double, zoom: Double) -> [Frame] {
        // Both directions share one path, expressed as entering: 0 is the parent view.
        let hidden = direction == .descend ? unit(fade) : 1 - unit(fade)
        let progress = direction == .descend ? unit(zoom) : 1 - unit(zoom)
        return entries.map { entry in
            let from = startGeometry(entry)
            let to = endGeometry(entry, from: from)
            let opacity: Double = if !entry.isSelectedBranch {
                1 - hidden
            } else if entry.source == nil {
                progress
            } else if entry.destination == nil {
                1 - progress
            } else {
                1
            }
            return Frame(id: entry.id,
                         start: mix(from.start, to.start, progress),
                         end: mix(from.end, to.end, progress),
                         innerRadius: mix(from.inner, to.inner, progress),
                         outerRadius: mix(from.outer, to.outer, progress),
                         opacity: opacity,
                         depth: mix(from.depth, to.depth, progress),
                         isSelectedBranch: entry.isSelectedBranch)
        }
    }

    /// The parent view. A newly exposed descendant waits beyond the outer edge, inside the
    /// angle of the selected sector, so it slides in as the branch moves inward.
    private func startGeometry(_ entry: Entry) -> Geometry {
        if let source = entry.source { return geometry(source) }
        let destination = entry.destination!
        return ring(Double(destination.depth + shift),
                    start: mix(anchor.start, anchor.end, destination.start),
                    end: mix(anchor.start, anchor.end, destination.end))
    }

    /// The child view. The selected sector without a slot of its own becomes the center
    /// disc; other branch sectors without a slot keep moving with their ring and fade.
    private func endGeometry(_ entry: Entry, from: Geometry) -> Geometry {
        if let destination = entry.destination { return geometry(destination) }
        guard entry.isSelectedBranch, let source = entry.source else { return from }
        if entry.id == anchor.id {
            return Geometry(start: 0, end: 1, inner: 0, outer: Self.centerRadius, depth: 0)
        }
        let span = anchor.end - anchor.start
        return ring(Double(max(0, source.depth - shift)),
                    start: unit((source.start - anchor.start) / span),
                    end: unit((source.end - anchor.start) / span))
    }

    private func geometry(_ sector: ArtifactSunburstLayout.Sector) -> Geometry {
        Geometry(start: sector.start, end: sector.end, inner: sector.innerRadius,
                 outer: sector.outerRadius, depth: Double(sector.depth))
    }

    /// A ring by depth, as in the layout; rings past the plot collapse onto its edge.
    private func ring(_ depth: Double, start: Double, end: Double) -> Geometry {
        let template = ArtifactSunburstLayout.Sector(id: .other(""), parentID: "", name: "", bytes: 0,
                                                     depth: 0, start: 0, end: 1, isPartial: false)
        let step = ArtifactSunburstLayout.Sector(id: .other(""), parentID: "", name: "", bytes: 0,
                                                 depth: 1, start: 0, end: 1, isPartial: false).innerRadius
            - template.innerRadius
        let inner = min(1, template.innerRadius + depth * step)
        let outer = min(1, inner + template.outerRadius - template.innerRadius)
        return Geometry(start: start, end: end, inner: inner, outer: outer, depth: depth)
    }

    private func unit(_ value: Double) -> Double { value.isFinite ? min(1, max(0, value)) : 0 }
    private func mix(_ start: Double, _ end: Double, _ progress: Double) -> Double {
        start * (1 - progress) + end * progress
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

        /// Blends toward `target` along the shorter way around the hue circle.
        func blended(with target: Swatch, by progress: Double) -> Swatch {
            let t = progress.isFinite ? min(1, max(0, progress)) : 0
            var delta = (target.hue - hue).truncatingRemainder(dividingBy: 1)
            if delta > 0.5 { delta -= 1 } else if delta < -0.5 { delta += 1 }
            let blendedHue = (hue + delta * t).truncatingRemainder(dividingBy: 1)
            return Swatch(hue: blendedHue < 0 ? blendedHue + 1 : blendedHue,
                          saturation: saturation + (target.saturation - saturation) * t,
                          brightness: brightness + (target.brightness - brightness) * t)
        }
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
