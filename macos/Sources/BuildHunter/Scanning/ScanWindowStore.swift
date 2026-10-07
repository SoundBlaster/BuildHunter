import Foundation
import Observation

/// Owns live windows. Companion windows share their scan's model and never start another scan.
@MainActor
@Observable
final class ScanWindowStore {
    private struct Entry {
        let model: WindowScanModel
        var hasScanWindow: Bool
        var hasDiagramWindow: Bool
        var hasWarningsWindow: Bool
    }
    private var entries: [UUID: Entry] = [:]

    func register(_ model: WindowScanModel) {
        if entries[model.id] == nil {
            entries[model.id] = Entry(model: model, hasScanWindow: true, hasDiagramWindow: false,
                                      hasWarningsWindow: false)
        } else {
            entries[model.id]?.hasScanWindow = true
        }
    }

    func prepareDiagram(for model: WindowScanModel) {
        register(model)
        entries[model.id]?.hasDiagramWindow = true
    }

    func model(for id: UUID) -> WindowScanModel? { entries[id]?.model }

    func prepareWarnings(for model: WindowScanModel) {
        register(model)
        entries[model.id]?.hasWarningsWindow = true
    }

    func warningsWindowClosed(_ id: UUID) {
        entries[id]?.hasWarningsWindow = false
        removeIfUnused(id)
    }

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
        guard let entry = entries[id], !entry.hasScanWindow, !entry.hasDiagramWindow,
              !entry.hasWarningsWindow else { return }
        entries.removeValue(forKey: id)
    }
}
