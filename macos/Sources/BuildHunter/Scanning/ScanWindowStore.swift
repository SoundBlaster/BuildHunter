import Foundation
import Observation

/// Owns only live windows. A diagram shares its scan's model and never starts a second scan.
@MainActor
@Observable
final class ScanWindowStore {
    private struct Entry {
        let model: WindowScanModel
        var hasScanWindow: Bool
        var hasDiagramWindow: Bool
    }
    private var entries: [UUID: Entry] = [:]

    func register(_ model: WindowScanModel) {
        if entries[model.id] == nil {
            entries[model.id] = Entry(model: model, hasScanWindow: true, hasDiagramWindow: false)
        } else {
            entries[model.id]?.hasScanWindow = true
        }
    }

    func prepareDiagram(for model: WindowScanModel) {
        register(model)
        entries[model.id]?.hasDiagramWindow = true
    }

    func model(for id: UUID) -> WindowScanModel? { entries[id]?.model }

    func scanWindowClosed(_ id: UUID) {
        entries[id]?.model.stop()
        entries[id]?.hasScanWindow = false
        removeIfUnused(id)
    }

    func diagramWindowClosed(_ id: UUID) {
        entries[id]?.hasDiagramWindow = false
        removeIfUnused(id)
    }

    private func removeIfUnused(_ id: UUID) {
        guard let entry = entries[id], !entry.hasScanWindow, !entry.hasDiagramWindow else { return }
        entries.removeValue(forKey: id)
    }
}
