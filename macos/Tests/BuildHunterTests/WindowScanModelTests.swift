import Foundation
import Testing
@testable import BuildHunter

@Suite("Window scan lifecycle")
@MainActor
struct WindowScanModelTests {
    @Test("Demo source completes without manufacturing access warnings")
    func demoCompletesNormally() async {
        let model = WindowScanModel(source: DemoScanSource())
        model.acceptDemoTarget(named: "Demo")
        await model.waitForCurrentScan()
        #expect(model.rows.count == 3)
        #expect(model.phase == .completed)
        #expect(model.warnings.isEmpty)
    }

    @Test("Rust FFI scans selected folders and applies SpecificationCore classification")
    func rustScannerStreamsMeasuredArtifacts() async throws {
        let root = FileManager.default.temporaryDirectory
            .appending(path: "BuildHunter-Rust-\(UUID().uuidString)", directoryHint: .isDirectory)
        let swiftBuild = root.appending(path: "Package/.build", directoryHint: .isDirectory)
        let nestedCache = swiftBuild.appending(path: "__pycache__", directoryHint: .isDirectory)
        let rustTarget = root.appending(path: "Service/target", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: nestedCache, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: rustTarget, withIntermediateDirectories: true)
        try Data("let fixture = true".utf8).write(to: swiftBuild.appending(path: "output.o"))
        try Data("[package]".utf8).write(to: root.appending(path: "Service/Cargo.toml"))
        defer { try? FileManager.default.removeItem(at: root) }

        let source = RustScanEventSource()
        let stream = source.events(for: 41, target: root)
        var artifacts: [ScanArtifact] = []
        var terminal: ScanTerminalResult?
        for await event in stream {
            switch event {
            case .discovered(_, let artifact): artifacts.append(artifact)
            case .finished(_, let result): terminal = result
            default: break
            }
        }

        #expect(terminal == .completed)
        #expect(artifacts.map(\.relativePath).sorted() == ["Package/.build", "Service/target"])
        #expect(artifacts.map(\.language).sorted() == ["Rust", "Swift"])
    }

    @Test("A selected artifact root is displayed as dot")
    func rustScannerDisplaysSelectedArtifactRoot() async throws {
        let parent = FileManager.default.temporaryDirectory
            .appending(path: "BuildHunter-SelectedRoot-\(UUID().uuidString)", directoryHint: .isDirectory)
        let selectedRoot = parent.appending(path: ".build", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: selectedRoot, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: parent) }

        let source = RustScanEventSource()
        let stream = source.events(for: 42, target: selectedRoot)
        var discoveredPath: String?
        var terminal: ScanTerminalResult?
        for await event in stream {
            switch event {
            case .discovered(_, let artifact): discoveredPath = artifact.relativePath
            case .finished(_, let result): terminal = result
            default: break
            }
        }

        #expect(discoveredPath == ".")
        #expect(terminal == .completed)
    }

    @Test("Overflow caused by the terminal event makes the scan incomplete")
    func terminalEnqueueOverflowIsReported() async {
        let generation: UInt64 = 1
        let (stream, continuation) = AsyncStream.makeStream(
            of: ScanEvent.self,
            bufferingPolicy: .bufferingNewest(2)
        )
        let bridge = RustScanBridgeContext(generation: generation, continuation: continuation,
                                           filters: SearchFilterSnapshot(excludedFilterIDs: [], catalog: []))
        bridge.yield(.discovered(generation: generation, artifact: makeArtifact()))
        bridge.yield(.discovered(
            generation: generation,
            artifact: ScanArtifact(id: UUID(), relativePath: "DemoFixture/Beta/.build",
                                   language: "Swift", kind: .buildOutput)
        ))
        bridge.finish(status: 0)

        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Overflow")
        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(model.warnings.contains { $0.contains("display buffer") })
    }

    @Test("Rust events are not dropped while the main actor is busy")
    func rustEventsSurviveABusyConsumer() async {
        let generation: UInt64 = 1
        let (stream, continuation) = RustScanEventSource.makeEventStream()
        let bridge = RustScanBridgeContext(generation: generation, continuation: continuation,
                                           filters: SearchFilterSnapshot(excludedFilterIDs: [], catalog: []))
        // The scanner can outrun the consumer by thousands of events before it reads any.
        let artifacts = (0..<5_000).map { index in
            ScanArtifact(id: UUID(), relativePath: "Project\(index)/.build", language: "Swift", kind: .buildOutput)
        }
        for artifact in artifacts {
            bridge.yield(.discovered(generation: generation, artifact: artifact))
            bridge.yield(.completed(generation: generation, artifactID: artifact.id, bytes: 4_096))
        }
        bridge.finish(status: 0)

        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Busy consumer")
        await model.waitForCurrentScan()

        #expect(model.rows.count == artifacts.count)
        #expect(model.rows.allSatisfy { $0.size == .measured(4_096) })
        #expect(model.warnings.isEmpty)
        #expect(model.phase == .completed)
    }

    @Test("Many distinct warnings are deduplicated in linear time")
    func manyWarningsApplyInLinearTime() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Unreadable home")
        let generation = model.generation
        // A home-folder scan can report thousands of unreadable paths.
        let messages = (0..<20_000).map { "Library/Private/\($0): Permission denied" }

        let start = ProcessInfo.processInfo.systemUptime
        for message in messages + messages {
            model.apply(.warning(generation: generation, message: message))
        }
        let elapsed = ProcessInfo.processInfo.systemUptime - start

        #expect(model.warnings == messages)
        // Quadratic membership checks take seconds here; a set takes milliseconds.
        #expect(elapsed < 1)
    }

    @Test("Terminal event rejects later events from the same generation")
    func terminalEventRejectsLateUpdates() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let artifact = makeArtifact()

        model.apply(.discovered(generation: generation, artifact: artifact))
        model.apply(.completed(generation: generation, artifactID: artifact.id, bytes: 20))
        model.apply(.finished(generation: generation, result: .completed))
        model.apply(.warning(generation: generation, message: "late warning"))
        model.apply(.completed(generation: generation, artifactID: artifact.id, bytes: 99))

        #expect(model.rows[0].size == .measured(20))
        #expect(model.warnings.isEmpty)
        #expect(model.phase == .completed)
    }

    @Test("An event stream without a terminal event becomes incomplete")
    func exhaustedStreamBecomesIncomplete() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        source.finish(generation: model.generation)

        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(model.warnings == ["Event stream ended before a terminal result."])
    }

    @Test("Stop remains distinct from completion and leaves unknown size unknown")
    func stopPreservesPartialUnknownSize() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        model.apply(.discovered(generation: generation, artifact: makeArtifact()))

        model.stop()
        await model.waitForCurrentScan()
        let terminatedGeneration = await source.nextTermination()

        #expect(model.phase == .stopped)
        #expect(model.rows.count == 1)
        #expect(model.rows[0].size == .partial(nil))
        #expect(terminatedGeneration == generation)
    }

    @Test("A completed stream applies streamed events and reaches the completed phase")
    func injectedSourceDeliversEvents() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let artifact = makeArtifact()
        source.yield(.discovered(generation: generation, artifact: artifact))
        source.yield(.completed(generation: generation, artifactID: artifact.id, bytes: 20))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)

        await model.waitForCurrentScan()

        #expect(model.rows == [ScanRow(id: artifact.id, relativePath: artifact.relativePath,
                                      language: artifact.language, kind: artifact.kind, size: .measured(20))])
        #expect(model.phase == .completed)
    }

    @Test("A warning makes the terminal report incomplete")
    func warningsMarkReportIncomplete() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        source.yield(.warning(generation: generation, message: "fixture warning"))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)

        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(model.warnings == ["fixture warning"])
    }

    @Test("A partial measurement keeps the report incomplete even without a warning")
    func partialMeasurementMarksReportIncomplete() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Partial")
        let generation = model.generation
        let artifact = makeArtifact()
        source.yield(.discovered(generation: generation, artifact: artifact))
        source.yield(.completed(generation: generation, artifactID: artifact.id, bytes: 20, partial: true))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)

        await model.waitForCurrentScan()

        #expect(model.rows.first?.size == .partial(20))
        #expect(model.phase == .incomplete)
    }

    @Test("Incomplete or unknown Rust terminal statuses cannot report success", arguments: [2, 99])
    func unsuccessfulRustStatusMarksReportIncomplete(status: UInt32) async {
        let (stream, continuation) = AsyncStream.makeStream(of: ScanEvent.self)
        let bridge = RustScanBridgeContext(generation: 1, continuation: continuation,
                                           filters: SearchFilterSnapshot(excludedFilterIDs: [], catalog: []))
        bridge.finish(status: status)

        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Incomplete")
        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(!model.warnings.isEmpty)
    }

    @Test("Duplicate and out-of-order events do not duplicate rows or invent a size")
    func duplicateAndOutOfOrderEventsAreSafe() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let artifact = makeArtifact()

        model.apply(.completed(generation: generation, artifactID: artifact.id, bytes: 20))
        model.apply(.discovered(generation: generation, artifact: artifact))
        model.apply(.discovered(generation: generation, artifact: artifact))

        #expect(model.rows.count == 1)
        #expect(model.rows[0].id == artifact.id)
        #expect(model.rows[0].size == .measuring)
    }

    @Test("Rescan clears the old report before accepting new events")
    func rescanClearsRowsAndWarnings() {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let firstGeneration = model.generation
        model.apply(.discovered(generation: firstGeneration, artifact: makeArtifact()))
        model.apply(.warning(generation: firstGeneration, message: "old warning"))

        model.rescan()
        model.apply(.warning(generation: firstGeneration, message: "late old warning"))

        #expect(model.rows.isEmpty)
        #expect(model.warnings.isEmpty)
        #expect(model.generation == firstGeneration + 1)
        #expect(model.targetName == "Alpha")
        #expect(model.phase == .scanning)
    }

    @Test("Artifact IDs can be reused after rescan or target replacement", arguments: [false, true])
    func reusedArtifactIDAfterRestart(replacingTarget: Bool) {
        let model = WindowScanModel(source: ControlledScanSource())
        let artifact = makeArtifact()
        model.acceptDemoTarget(named: "First")
        model.apply(.discovered(generation: model.generation, artifact: artifact))
        model.apply(.completed(generation: model.generation, artifactID: artifact.id, bytes: 20))

        if replacingTarget {
            model.acceptDemoTarget(named: "Second")
        } else {
            model.rescan()
        }
        model.apply(.discovered(generation: model.generation, artifact: artifact))
        model.apply(.completed(generation: model.generation, artifactID: artifact.id, bytes: 40))

        #expect(model.rows.count == 1)
        #expect(model.rows.first?.size == .measured(40))
        model.stop()
    }

    @Test("Replacing a target cancels its stream and isolates the new report")
    func replacementCancelsOldGeneration() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "First")
        let oldGeneration = model.generation
        model.apply(.discovered(generation: oldGeneration, artifact: makeArtifact()))
        model.apply(.warning(generation: oldGeneration, message: "old warning"))

        model.acceptDemoTarget(named: "Second")
        model.apply(.warning(generation: oldGeneration, message: "late warning"))
        let terminatedGeneration = await source.nextTermination()

        #expect(terminatedGeneration == oldGeneration)
        #expect(model.targetName == "Second")
        #expect(model.rows.isEmpty)
        #expect(model.warnings.isEmpty)
        #expect(model.phase == .scanning)
    }

    @Test("Window models keep target and result state independent")
    func windowsAreIsolated() {
        let first = WindowScanModel(source: ControlledScanSource())
        let second = WindowScanModel(source: ControlledScanSource())
        first.acceptDemoTarget(named: "Alpha")
        second.acceptDemoTarget(named: "Beta")
        first.apply(.discovered(generation: first.generation, artifact: makeArtifact()))

        #expect(first.rows.count == 1)
        #expect(second.rows.isEmpty)
        #expect(first.targetName == "Alpha")
        #expect(second.targetName == "Beta")
    }

#if DEBUG
    @Test("Mock scenarios discard the previous real target and never rescan that folder")
    func mockScenarioClearsRealTarget() {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.accept(target: URL(fileURLWithPath: "/real/project", isDirectory: true))

        model.showMockState(.results)
        #expect(model.targetURL == nil)

        model.rescan()
        #expect(source.lastTarget == nil)
        model.stop()
    }

    @Test("Debug mock scenarios render empty, streaming, completed, stopped, and incomplete states")
    func mockScenarios() {
        let model = WindowScanModel(source: ControlledScanSource())

        model.showMockState(.empty)
        #expect(model.targetName == nil)
        #expect(model.phase == .idle)
        #expect(model.rows.isEmpty)

        model.showMockState(.scanning)
        #expect(model.phase == .scanning)
        #expect(model.rows.count == 3)
        #expect(model.rows.allSatisfy { $0.size == .measuring })

        model.showMockState(.results)
        #expect(model.phase == .completed)
        #expect(model.rows.allSatisfy { if case .measured = $0.size { true } else { false } })

        model.showMockState(.stopped)
        #expect(model.phase == .stopped)
        #expect(model.rows.filter { $0.size == .partial(nil) }.count == 2)
        #expect(model.rows.filter { if case .measured = $0.size { true } else { false } }.count == 1)

        model.showMockState(.incomplete)
        #expect(model.phase == .incomplete)
        #expect(model.warnings.count == 1)
        #expect(model.rows.filter { $0.size == .partial(nil) }.count == 1)
    }
#endif

    private func makeArtifact() -> ScanArtifact {
        ScanArtifact(id: UUID(uuidString: "A0000000-0000-4000-8000-000000000001")!,
                     relativePath: "DemoFixture/Alpha/.build",
                     language: "Swift", kind: .buildOutput)
    }
}

@MainActor
private final class ControlledScanSource: ScanEventSource {
    private(set) var lastTarget: URL?
    private var continuations: [UInt64: AsyncStream<ScanEvent>.Continuation] = [:]
    private let terminationEvents: AsyncStream<UInt64>
    private let terminationContinuation: AsyncStream<UInt64>.Continuation

    init() {
        (terminationEvents, terminationContinuation) = AsyncStream.makeStream(of: UInt64.self)
    }

    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        lastTarget = target
        let (stream, continuation) = AsyncStream.makeStream(of: ScanEvent.self)
        continuations[generation] = continuation
        let terminationContinuation = self.terminationContinuation
        continuation.onTermination = { @Sendable _ in
            terminationContinuation.yield(generation)
        }
        return stream
    }

    func cancel(generation: UInt64) {}

    func yield(_ event: ScanEvent) {
        continuations[event.generation]?.yield(event)
    }

    func finish(generation: UInt64) {
        continuations[generation]?.finish()
    }

    func nextTermination() async -> UInt64? {
        var iterator = terminationEvents.makeAsyncIterator()
        return await iterator.next()
    }
}

@MainActor
private struct SingleStreamScanSource: ScanEventSource {
    let stream: AsyncStream<ScanEvent>

    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        stream
    }

    func cancel(generation: UInt64) {}
}

@Suite("Artifact table sorting")
@MainActor
struct ScanTableModelTests {
    @Test("Every header sorts both ways; sizes use bytes and keep unknown values last")
    func columnsAndUnknownSizes() {
        let rows = [
            ScanRow(id: UUID(), relativePath: "Z/target", language: "Rust", kind: .buildOutput, size: .measured(20)),
            ScanRow(id: UUID(), relativePath: "A/.build", language: "Swift", kind: .buildOutput, size: .partial(100)),
            ScanRow(id: UUID(), relativePath: "B/cache", language: "Python", kind: .cache, size: .measuring),
            ScanRow(id: UUID(), relativePath: "C/cache", language: "Python", kind: .cache, size: .partial(nil))
        ]
        for column in [ScanRowComparator.Column.path, .language, .kind] {
            let forward = ScanRowComparator(column: column)
            let reverse = ScanRowComparator(column: column, order: .reverse)
            for first in rows {
                for second in rows where forward.compare(first, second) != .orderedSame {
                    #expect(forward.compare(first, second) != reverse.compare(first, second))
                }
            }
        }
        #expect(ScanRowComparator.sorted(rows, by: [.init(column: .size)]).map(\.relativePath)
                == ["Z/target", "A/.build", "B/cache", "C/cache"])
        #expect(ScanRowComparator.sorted(rows, by: [.init(column: .size, order: .reverse)]).map(\.relativePath)
                == ["A/.build", "Z/target", "B/cache", "C/cache"])
        #expect(ScanRowComparator.sorted(Array(rows.reversed()), by: [.init(column: .kind)])
                == ScanRowComparator.sorted(rows, by: [.init(column: .kind)]))
    }

    @Test("Sorting during discovery does not break the scanner's row index or late measurements")
    func streamingSort() async {
        let scan = WindowScanModel(source: TableIdleSource())
        let table = ScanTableModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        let entries = ["Z/target", "A/.build"].map {
            ScanArtifact(id: UUID(), relativePath: $0, language: "Swift", kind: .buildOutput)
        }
        for entry in entries { scan.apply(.discovered(generation: scan.generation, artifact: entry)) }
        await table.refresh(from: scan)
        #expect(table.rows.map(\.relativePath) == ["A/.build", "Z/target"])
        table.sortOrder = [.init(column: .size, order: .reverse)]
        scan.apply(.completed(generation: scan.generation, artifactID: entries[0].id, bytes: 100))
        await table.refresh(from: scan)
        #expect(table.rows.first?.id == entries[0].id)
        scan.apply(.completed(generation: scan.generation, artifactID: entries[1].id, bytes: 200))
        await table.refresh(from: scan)
        #expect(table.rows.map(\.size) == [.measured(200), .measured(100)])
        #expect(scan.rows.map(\.size) == [.measured(100), .measured(200)])
        table.sortOrder = [.init(column: .path, order: .reverse)]
        await table.refresh(from: scan)
        #expect(table.rows.first?.id == entries[0].id)
        scan.rescan()
        await table.refresh(from: scan)
        #expect(table.rows.isEmpty)
    }
}

private struct TableIdleSource: ScanEventSource {
    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> { AsyncStream { _ in } }
    func cancel(generation: UInt64) {}
}
