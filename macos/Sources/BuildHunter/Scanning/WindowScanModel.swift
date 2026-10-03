import Foundation
import Observation

@MainActor
@Observable
final class WindowScanModel {
    let id = UUID()
    private(set) var targetName: String?
    private(set) var targetURL: URL?
    private(set) var rows: [ScanRow] = []
    private(set) var warnings: [String] = []
    private(set) var generation: UInt64 = 0
    private(set) var phase: ScanPhase = .idle
    var isScanning: Bool { phase == .scanning }
    var isChoosingFolder = false
    @ObservationIgnored private(set) var reportRevision: UInt64 = 0
    @ObservationIgnored private(set) var reportID = UUID()

    private let source: any ScanEventSource
    private var scanTask: Task<Void, Never>?
    @ObservationIgnored private var rowIndices: [UUID: Int] = [:]

    init(source: any ScanEventSource = RustScanEventSource()) {
        self.source = source
    }

    func acceptDemoTarget(named name: String) {
        replaceTarget(name: name, url: nil)
    }

    func accept(target url: URL) {
        replaceTarget(name: url.lastPathComponent, url: url)
    }

    private func replaceTarget(name: String, url: URL?) {
        source.cancel(generation: generation)
        scanTask?.cancel()
        generation &+= 1
        targetName = name
        targetURL = url
        clearReport()
        beginScan()
    }

#if DEBUG
    func showMockState(_ mockState: MockScanState) {
        scanTask?.cancel()
        source.cancel(generation: generation)
        generation &+= 1
        targetName = mockState.targetName
        targetURL = nil
        clearReport()
        phase = mockState.targetName == nil ? .idle : .scanning

        guard targetName != nil else { return }
        let activeGeneration = generation
        for artifact in mockState.artifacts {
            apply(.discovered(generation: activeGeneration, artifact: artifact))
        }
        for (artifact, bytes) in mockState.completedArtifacts {
            apply(.completed(generation: activeGeneration, artifactID: artifact.id, bytes: bytes))
        }
        for warning in mockState.warnings {
            apply(.warning(generation: activeGeneration, message: warning))
        }
        if let terminalResult = mockState.terminalResult {
            apply(.finished(generation: activeGeneration, result: terminalResult))
        }
    }
#endif

    func rescan() {
        guard targetName != nil else { return }
        source.cancel(generation: generation)
        scanTask?.cancel()
        generation &+= 1
        clearReport()
        beginScan()
    }

    func stop() {
        guard phase == .scanning else { return }
        let cancelledGeneration = generation
        generation &+= 1
        phase = .stopped
        markMeasuringRowsPartial()
        source.cancel(generation: cancelledGeneration)
        scanTask?.cancel()
    }

    func apply(_ event: ScanEvent) {
        guard event.generation == generation, phase == .scanning else { return }
        switch event {
        case .discovered(_, let artifact):
            guard rowIndices[artifact.id] == nil else { return }
            rowIndices[artifact.id] = rows.count
            rows.append(ScanRow(id: artifact.id, relativePath: artifact.relativePath,
                                language: artifact.language, kind: artifact.kind, size: .measuring))
            reportRevision &+= 1
        case .completed(_, let artifactID, let bytes, let partial):
            guard let index = rowIndices[artifactID] else { return }
            guard rows[index].size == .measuring else { return }
            rows[index].size = partial ? .partial(bytes) : .measured(bytes)
            reportRevision &+= 1
        case .warning(_, let message):
            if !warnings.contains(message) { warnings.append(message) }
        case .finished(_, let result):
            switch result {
            case .completed:
                markMeasuringRowsPartial()
                let hasPartialRows = rows.contains { if case .partial = $0.size { true } else { false } }
                phase = warnings.isEmpty && !hasPartialRows ? .completed : .incomplete
            case .stopped:
                phase = .stopped
                markMeasuringRowsPartial()
            case .failed(let message):
                if !warnings.contains(message) { warnings.append(message) }
                phase = .incomplete
                markMeasuringRowsPartial()
            }
        }
    }

    func waitForCurrentScan() async {
        await scanTask?.value
    }

    private func clearReport() {
        reportID = UUID()
        rows = []
        rowIndices = [:]
        warnings = []
        reportRevision &+= 1
    }

    private func beginScan() {
        phase = .scanning
        let activeGeneration = generation
        let stream = source.events(for: activeGeneration, target: targetURL)
        scanTask = Task { [weak self] in
            var terminalResult: ScanTerminalResult?
            var receivedTerminalResult = false
            for await event in stream {
                guard !Task.isCancelled, let self else { return }
                guard event.generation == activeGeneration else { continue }
                if case .finished(_, let result) = event {
                    terminalResult = result
                    receivedTerminalResult = true
                    continue
                }
                if receivedTerminalResult, case .warning = event {
                    self.apply(event)
                    continue
                }
                guard !receivedTerminalResult else { continue }
                self.apply(event)
                guard self.phase == .scanning else { return }
            }
            guard !Task.isCancelled, let self,
                  self.generation == activeGeneration, self.phase == .scanning else { return }
            if let terminalResult {
                self.apply(.finished(generation: activeGeneration, result: terminalResult))
                return
            }
            self.warnings.append("Event stream ended before a terminal result.")
            self.phase = .incomplete
            self.markMeasuringRowsPartial()
        }
    }

    private func markMeasuringRowsPartial() {
        rows = rows.map { row in
            var updated = row
            if case .measuring = row.size {
                updated.size = .partial(nil)
            }
            return updated
        }
        reportRevision &+= 1
    }
}

/// Coalesces sorting outside the event reducer and outside SwiftUI body evaluation.
@MainActor
@Observable
final class ScanTableModel {
    private(set) var rows: [ScanRow] = []
    var sortOrder: [ScanRowComparator] = [.init(column: .path)]
    @ObservationIgnored private var lastRevision: UInt64?
    @ObservationIgnored private var sourceID: UUID?
    @ObservationIgnored private var appliedOrder: [ScanRowComparator] = []

    func follow(_ scan: WindowScanModel) async {
        while !Task.isCancelled {
            await refresh(from: scan)
            do { try await Task.sleep(for: .milliseconds(100)) } catch { return }
        }
    }

    func refresh(from scan: WindowScanModel) async {
        guard !Task.isCancelled else { return }
        let revision = scan.reportRevision
        let order = sortOrder
        guard sourceID != scan.id || lastRevision != revision || appliedOrder != order else { return }
        let generation = scan.generation
        let input = scan.rows
        let worker = Task.detached(priority: .userInitiated) { ScanRowComparator.sorted(input, by: order) }
        let sorted = await withTaskCancellationHandler { await worker.value } onCancel: { worker.cancel() }
        guard !Task.isCancelled, generation == scan.generation, order == sortOrder else { return }
        rows = sorted
        lastRevision = revision
        sourceID = scan.id
        appliedOrder = order
    }
}
