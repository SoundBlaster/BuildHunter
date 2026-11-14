import Foundation
import Observation

@MainActor
@Observable
final class ArtifactDiagramModel {
    private(set) var snapshot = ArtifactSunburstSnapshot(rows: [])
    private(set) var layout = ArtifactSunburstLayout(snapshot: ArtifactSunburstSnapshot(rows: []))
    private(set) var palette = ArtifactSunburstPalette()
    private(set) var focusID = ""
    private(set) var children: [ArtifactSunburstNode] = []
    private(set) var previewID: String?
    private(set) var targetURL: URL?
    private(set) var targetName: String?
    private(set) var unsafePathPrefixes: Set<String> = []
    private(set) var filteredChildren: [ArtifactSunburstNode] = []
    var query = "" { didSet { updateChildren() } }
    @ObservationIgnored private var retainedOrder: [String: [String]] = [:]
    @ObservationIgnored private var palettes: [ArtifactSunburstPalette.Scope: ArtifactSunburstPalette] = [:]
    @ObservationIgnored private var lastScopes: [String: ArtifactSunburstPalette.Scope] = [:]
    @ObservationIgnored private var lastRevision: UInt64?
    @ObservationIgnored private var sourceID: UUID?
    @ObservationIgnored private var reportID: UUID?
    /// Sizes of the rows behind `snapshot`. A separate array, so the diagram never shares
    /// the scan's row buffer and the event reducer can keep mutating it in place.
    @ObservationIgnored private var snapshotSizes: [SizeState] = []
    @ObservationIgnored private var navigationTransitionActive = false
    private struct OtherGroup {
        let parentID: String
        let members: [String]
        let returnFocusID: String
        let returnPalette: ArtifactSunburstPalette
    }
    private var otherGroups: [OtherGroup] = []
    private var activeGroup: OtherGroup? { otherGroups.last.flatMap { $0.parentID == focusID ? $0 : nil } }
    var isShowingOther: Bool { activeGroup != nil && previewID == nil }
    var hasOtherNavigation: Bool { !otherGroups.isEmpty }

    init(snapshot: ArtifactSunburstSnapshot = ArtifactSunburstSnapshot(rows: []),
         targetURL: URL? = nil, targetName: String? = nil) {
        self.snapshot = snapshot
        self.targetURL = targetURL
        self.targetName = targetName
        updateLayout()
    }

    var focus: ArtifactSunburstNode {
        var node = snapshot.nodes[focusID] ?? snapshot.root
        guard let group = activeGroup else { return node }
        node.children = group.members.filter { snapshot.nodes[$0] != nil }
        node.total = ByteTotal()
        node.statistics = ArtifactSizeStatistics()
        node.artifact = nil
        for id in node.children {
            guard let child = snapshot.nodes[id] else { continue }
            node.total.add(child.total)
            node.statistics.artifactCount += child.statistics.artifactCount
            node.statistics.measuringCount += child.statistics.measuringCount
            node.statistics.partialCount += child.statistics.partialCount
            node.statistics.unavailableCount += child.statistics.unavailableCount
            node.statistics.zeroCount += child.statistics.zeroCount
        }
        return node
    }

    var displayedFolder: ArtifactSunburstNode { previewID.flatMap { snapshot.nodes[$0] } ?? focus }
    var canNavigateUp: Bool { !focusID.isEmpty || !otherGroups.isEmpty }
    var displayedURL: URL? {
        isShowingOther ? nil : actionURL(for: displayedFolder.id)
    }
    var displayedPath: String {
        if isShowingOther { return "Other · \(focus.children.count) folders" }
        return targetURL.map { displayedFolder.id.isEmpty ? $0.path : $0.appendingPathComponent(displayedFolder.id).path }
            ?? (displayedFolder.id.isEmpty ? targetName ?? "All artifacts" : displayedFolder.id)
    }

    func actionURL(for path: String) -> URL? {
        ArtifactFolderLocation.actionURL(root: targetURL, relativePath: path, unsafePrefixes: unsafePathPrefixes)
    }

    /// Coalesces streaming events while this window exists. Work occurs off the main actor
    /// and never runs from the table's per-event reducer.
    // Deliberately mirrors the table's polling loop; refresh policies stay independent.
    // jscpd:ignore-start
    func follow(_ scan: WindowScanModel) async {
        while !Task.isCancelled {
            await refresh(from: scan)
            do { try await Task.sleep(for: .milliseconds(100)) } catch { return }
        }
    }

    // jscpd:ignore-end
    func refresh(from scan: WindowScanModel) async {
        guard !Task.isCancelled, !navigationTransitionActive else { return }
        let revision = scan.reportRevision
        guard sourceID != scan.id || lastRevision != revision else { return }
        let currentGeneration = scan.generation
        let rows = scan.rows
        // A growing report extends the rows behind the current snapshot; update it in place.
        let extendsSnapshot = sourceID == scan.id && reportID == scan.reportID
            && snapshotSizes.count <= rows.count
        let previous = extendsSnapshot ? (snapshot, snapshotSizes) : nil
        let worker = Task.detached(priority: .userInitiated) {
            let snapshot = if let (snapshot, previousSizes) = previous {
                ArtifactSunburstSnapshot(rows: rows, updating: snapshot, previousSizes: previousSizes)
            } else {
                ArtifactSunburstSnapshot(rows: rows)
            }
            return (snapshot, rows.map(\.size))
        }
        let (prepared, preparedSizes) = await withTaskCancellationHandler {
            await worker.value
        } onCancel: {
            worker.cancel()
        }
        guard !Task.isCancelled, !navigationTransitionActive, scan.generation == currentGeneration else { return }
        if sourceID != scan.id || reportID != scan.reportID {
            focusID = ""
            palette = ArtifactSunburstPalette()
            retainedOrder = [:]
            palettes = [:]
            lastScopes = [:]
            previewID = nil
            query = ""
            otherGroups = []
        }
        targetURL = scan.targetURL
        targetName = scan.targetName
        unsafePathPrefixes = scan.unsafePathPrefixes
        sourceID = scan.id
        reportID = scan.reportID
        lastRevision = revision
        snapshot = prepared
        snapshotSizes = preparedSizes
        if prepared.nodes[focusID] == nil { focusID = "" }
        if let previewID, prepared.nodes[previewID] == nil { self.previewID = nil }
        updateLayout()
    }

    func navigate(to path: String) {
        guard let destination = snapshot.nodes[path] else { return }
        let returning = path.isEmpty || focusID.hasPrefix(path + "/")
        let scope: ArtifactSunburstPalette.Scope
        if path == focusID {
            scope = palette.scope
        } else if returning, let previous = lastScopes[path] {
            scope = previous
        } else {
            scope = .init(focusID: path, inheritedColor: palette.color(for: path) ?? ancestorColor(for: path))
        }
        var nextPalette = palettes[scope] ?? ArtifactSunburstPalette(scope: scope)
        // A new descent chooses today's largest child. Streaming and Up preserve
        // the existing viewport; they must not swap its colors as sizes arrive.
        if !returning, path != focusID, let inheritor = nextPalette.inheritedBranchID,
           inheritor != largestChild(in: destination)?.id {
            nextPalette = ArtifactSunburstPalette(scope: scope)
        }
        palette = nextPalette
        focusID = path
        previewID = nil
        query = ""
        updateLayout()
    }

    func navigateUp() {
        guard canNavigateUp else { return }
        if let group = activeGroup {
            otherGroups.removeLast()
            focusID = group.returnFocusID
            palette = group.returnPalette
            previewID = nil
            query = ""
            updateLayout()
            return
        }
        navigate(to: focus.parentID ?? "")
    }

    func showAll() {
        if let first = otherGroups.first, first.returnFocusID.isEmpty {
            palette = first.returnPalette
            palettes[first.returnPalette.scope] = first.returnPalette
            lastScopes[""] = first.returnPalette.scope
        }
        otherGroups = []
        navigate(to: "")
    }

    func openOther(_ sector: ArtifactSunburstLayout.Sector) {
        guard case .other(let parentID) = sector.id,
              let members = layout.otherMembers[sector.id], !members.isEmpty else { return }
        otherGroups.append(OtherGroup(parentID: parentID, members: members,
                                      returnFocusID: focusID, returnPalette: palette))
        focusID = parentID
        palette = ArtifactSunburstPalette(scope: .init(focusID: parentID))
        previewID = nil
        query = ""
        updateLayout()
    }

    /// The scanner continues; the next refresh coalesces all revisions received
    /// during the short navigation transition instead of moving its endpoints.
    func setNavigationTransitionActive(_ active: Bool) {
        navigationTransitionActive = active
    }

    func preview(_ path: String?) {
        let valid = path.flatMap { snapshot.nodes[$0] == nil ? nil : $0 }
        guard previewID != valid else { return }
        previewID = valid
        updateChildren()
    }

    private func ancestorColor(for path: String) -> ArtifactSunburstPalette.Swatch? {
        var ancestor = snapshot.nodes[path]?.parentID
        while let parent = ancestor {
            if let scope = lastScopes[parent], let color = palettes[scope]?.color(for: path) { return color }
            ancestor = snapshot.nodes[parent]?.parentID
        }
        return nil
    }

    private func largestChild(in folder: ArtifactSunburstNode) -> ArtifactSunburstNode? {
        folder.children.compactMap { snapshot.nodes[$0] }
            .filter { $0.bytes > 0 }
            .max { $0.bytes == $1.bytes ? $0.id > $1.id : $0.bytes < $1.bytes }
    }

    private func updateLayout() {
        let updated = ArtifactSunburstLayout(snapshot: snapshot, focusID: focusID, retainedOrder: retainedOrder,
                                             focusChildren: activeGroup?.members)
        for sector in updated.sectors {
            guard let path = sector.nodeID, path != focusID else { continue }
            if !(retainedOrder[sector.parentID] ?? []).contains(path) {
                retainedOrder[sector.parentID, default: []].append(path)
            }
        }
        palette.include(updated)
        palettes[palette.scope] = palette
        lastScopes[focusID] = palette.scope
        layout = updated
        updateChildren()
    }

    private func updateChildren() {
        children = displayedFolder.children.compactMap { snapshot.nodes[$0] }.sorted { left, right in
            if left.bytes != right.bytes { return left.bytes > right.bytes }
            let nameOrder = left.name.localizedStandardCompare(right.name)
            return nameOrder == .orderedSame ? left.id < right.id : nameOrder == .orderedAscending
        }
        filteredChildren = query.isEmpty || previewID != nil ? children
            : children.filter { $0.name.localizedStandardContains(query) }
    }
}

/// A separate chart identity prevents interpolation across incompatible annular marks.
/// Measurements within the same hierarchy still use Charts' normal animation.
struct ArtifactDiagramAnimationPolicy {
    struct Slot: Hashable {
        let id: ArtifactSunburstLayout.Sector.ID
        let parentID: String
        let depth: Int
    }

    static func topology(of layout: ArtifactSunburstLayout) -> [Slot] {
        layout.sectors.map { Slot(id: $0.id, parentID: $0.parentID, depth: $0.depth) }
    }
}
