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
    @ObservationIgnored private var lastRevision: UInt64?
    @ObservationIgnored private var sourceID: UUID?
    @ObservationIgnored private var reportID: UUID?

    var focus: ArtifactSunburstNode { snapshot.nodes[focusID] ?? snapshot.root }

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
        }
        sourceID = scan.id
        reportID = scan.reportID
        lastRevision = revision
        snapshot = prepared
        if prepared.nodes[focusID] == nil { focusID = "" }
        updateLayout()
    }

    func navigate(to path: String) {
        guard snapshot.nodes[path] != nil else { return }
        focusID = path
        updateLayout()
    }

    private func updateLayout() {
        let updated = ArtifactSunburstLayout(snapshot: snapshot, focusID: focusID)
        palette.include(updated)
        layout = updated
        children = focus.children.compactMap { snapshot.nodes[$0] }
    }
}
