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
    private(set) var filteredChildren: [ArtifactSunburstNode] = []
    var query = "" { didSet { updateChildren() } }
    @ObservationIgnored private var retainedOrder: [String: [String]] = [:]
    @ObservationIgnored private var palettes: [ArtifactSunburstPalette.Scope: ArtifactSunburstPalette] = [:]
    @ObservationIgnored private var lastScopes: [String: ArtifactSunburstPalette.Scope] = [:]
    @ObservationIgnored private var lastRevision: UInt64?
    @ObservationIgnored private var sourceID: UUID?
    @ObservationIgnored private var reportID: UUID?

    init(snapshot: ArtifactSunburstSnapshot = ArtifactSunburstSnapshot(rows: []),
         targetURL: URL? = nil, targetName: String? = nil) {
        self.snapshot = snapshot
        self.targetURL = targetURL
        self.targetName = targetName
        updateLayout()
    }

    var focus: ArtifactSunburstNode { snapshot.nodes[focusID] ?? snapshot.root }

    var displayedFolder: ArtifactSunburstNode { previewID.flatMap { snapshot.nodes[$0] } ?? focus }
    var canNavigateUp: Bool { !focusID.isEmpty }
    var displayedURL: URL? {
        targetURL.map { displayedFolder.id.isEmpty ? $0 : $0.appendingPathComponent(displayedFolder.id) }
    }
    var displayedPath: String {
        displayedURL?.path ?? (displayedFolder.id.isEmpty ? targetName ?? "All artifacts" : displayedFolder.id)
    }

    /// Coalesces streaming events while this window exists. Work occurs off the main actor
    /// and never runs from the table's per-event reducer.
    func follow(_ scan: WindowScanModel) async {
        while !Task.isCancelled {
            await refresh(from: scan)
            do { try await Task.sleep(for: .milliseconds(100)) } catch { return }
        }
    }

    func refresh(from scan: WindowScanModel) async {
        guard !Task.isCancelled else { return }
        let revision = scan.reportRevision
        guard sourceID != scan.id || lastRevision != revision else { return }
        let currentGeneration = scan.generation
        let rows = scan.rows
        let worker = Task.detached(priority: .userInitiated) {
            ArtifactSunburstSnapshot(rows: rows)
        }
        let prepared = await withTaskCancellationHandler {
            await worker.value
        } onCancel: {
            worker.cancel()
        }
        guard !Task.isCancelled, scan.generation == currentGeneration else { return }
        if sourceID != scan.id || reportID != scan.reportID {
            focusID = ""
            palette = ArtifactSunburstPalette()
            retainedOrder = [:]
            palettes = [:]
            lastScopes = [:]
            previewID = nil
            query = ""
        }
        targetURL = scan.targetURL
        targetName = scan.targetName
        sourceID = scan.id
        reportID = scan.reportID
        lastRevision = revision
        snapshot = prepared
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
        navigate(to: focus.parentID ?? "")
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

    private func prepareVisibleChildren() {
        guard !focusID.isEmpty, palette.inheritedBranchID == nil,
              let largest = largestChild(in: focus) else { return }
        // Reveal a late size leader on deliberate navigation, while keeping the
        // displayed membership fixed during subsequent scan updates.
        if let previous = retainedOrder[focusID], !previous.contains(largest.id) {
            retainedOrder[focusID] = [largest.id] + previous.prefix(5)
        }
    }

    private func updateLayout() {
        prepareVisibleChildren()
        let updated = ArtifactSunburstLayout(snapshot: snapshot, focusID: focusID, retainedOrder: retainedOrder)
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
        children = displayedFolder.children.compactMap { snapshot.nodes[$0] }
        filteredChildren = query.isEmpty || previewID != nil ? children
            : children.filter { $0.name.localizedStandardContains(query) }
    }
}
