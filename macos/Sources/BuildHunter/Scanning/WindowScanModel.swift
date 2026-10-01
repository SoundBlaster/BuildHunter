import Foundation
import Observation

@MainActor
@Observable
final class WindowScanModel {
    private(set) var targetName: String?
    private(set) var targetURL: URL?
    private(set) var rows: [ScanRow] = []
    private(set) var warnings: [String] = []
    private(set) var generation: UInt64 = 0
    private(set) var phase: ScanPhase = .idle
    var isScanning: Bool { phase == .scanning }
    var isChoosingFolder = false

    private let source: any ScanEventSource
    private var scanTask: Task<Void, Never>?

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
        rows = []
        warnings = []
        beginScan()
    }

#if DEBUG
    func showMockState(_ mockState: MockScanState) {
        scanTask?.cancel()
        source.cancel(generation: generation)
        generation &+= 1
        targetName = mockState.targetName
        rows = []
        warnings = []
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
        rows = []
        warnings = []
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
            guard !rows.contains(where: { $0.id == artifact.id }) else { return }
            rows.append(ScanRow(id: artifact.id, relativePath: artifact.relativePath,
                                language: artifact.language, kind: artifact.kind, size: .measuring))
        case .completed(_, let artifactID, let bytes, let partial):
            guard let index = rows.firstIndex(where: { $0.id == artifactID }) else { return }
            guard rows[index].size == .measuring else { return }
            rows[index].size = partial ? .partial(bytes) : .measured(bytes)
        case .warning(_, let message):
            if !warnings.contains(message) { warnings.append(message) }
        case .finished(_, let result):
            switch result {
            case .completed:
                let hasUnmeasuredRows = markMeasuringRowsPartial()
                phase = warnings.isEmpty && !hasUnmeasuredRows ? .completed : .incomplete
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

    @discardableResult
    private func markMeasuringRowsPartial() -> Bool {
        var foundUnmeasuredRows = false
        rows = rows.map { row in
            var updated = row
            if case .measuring = row.size {
                updated.size = .partial(nil)
                foundUnmeasuredRows = true
            }
            return updated
        }
        return foundUnmeasuredRows
    }
}
