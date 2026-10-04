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

    @Test("A continuously occupied event queue releases delivered payloads", arguments: [1, 2, 8])
    func occupiedQueueReleasesDeliveredEvents(capacity: Int) async throws {
        let channel = ScanEventChannel(capacity: capacity)
        for index in 0..<capacity {
            #expect(channel.send(.warning(generation: 1, message: "\(index)")))
        }
        for index in 0..<10_000 {
            let event = try #require(await channel.next())
            guard case .warning(_, let message) = event else {
                Issue.record("Expected a warning event")
                return
            }
            #expect(message == "\(index)", "FIFO must survive repeated wraparound")
            if index == 9_999 {
                #expect(channel.retainedEventCount == capacity - 1,
                        "Delivered payloads must be released even while the queue stays occupied")
            }
            #expect(channel.send(.warning(generation: 1, message: "\(index + capacity)")))
        }
        #expect(channel.bufferedCount == capacity)
        #expect(channel.retainedEventCount == capacity)
        channel.finish()
        for index in 10_000..<(10_000 + capacity) {
            let event = try #require(await channel.next())
            guard case .warning(_, let message) = event else {
                Issue.record("Expected a buffered warning event")
                return
            }
            #expect(message == "\(index)")
        }
        #expect(await channel.next() == nil)
        #expect(channel.retainedEventCount == 0)
    }

    @Test("Closing a partly consumed queue releases its remaining payloads")
    func closingQueueReleasesPayloads() async throws {
        let channel = ScanEventChannel(capacity: 2)
        channel.send(.warning(generation: 1, message: "first"))
        channel.send(.warning(generation: 1, message: "second"))
        _ = try #require(await channel.next())
        channel.close()
        #expect(channel.bufferedCount == 0)
        #expect(channel.retainedEventCount == 0)
        #expect(!channel.send(.warning(generation: 1, message: "closed")))
        #expect(await channel.next() == nil)
    }

    @Test("Rust events wait for buffer space instead of being dropped")
    func rustEventsApplyBackpressure() async throws {
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let artifacts = (0..<1_000).map { index in
            ScanArtifact(id: UUID(), relativePath: "Project\(index)/.build", language: "Swift", kind: .buildOutput)
        }
        let producer = ProducerProbe()
        // The Rust scanner calls back on its own thread and can outrun the consumer.
        Thread {
            for artifact in artifacts {
                channel.send(.discovered(generation: 1, artifact: artifact))
                channel.send(.completed(generation: 1, artifactID: artifact.id, bytes: 4_096))
            }
            channel.send(.finished(generation: 1, result: .completed))
            channel.finish()
            producer.markFinished()
        }.start()
        try await Task.sleep(for: .milliseconds(50))
        #expect(channel.bufferedCount == 8, "the producer waits once the buffer is full")
        #expect(!producer.isFinished)

        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Busy consumer")
        await model.waitForCurrentScan()

        #expect(model.rows.count == artifacts.count)
        #expect(model.rows.allSatisfy { $0.size == .measured(4_096) })
        #expect(model.warnings.isEmpty)
        #expect(model.phase == .completed)
    }

    @Test("Abandoning the event stream wakes a producer blocked on a full buffer")
    func abandonedStreamReleasesTheProducer() async throws {
        let channel = ScanEventChannel(capacity: 2)
        let producer = ProducerProbe()
        var stream: AsyncStream<ScanEvent>? = channel.makeStream(onClose: { producer.markClosed() })
        Thread {
            var accepted = 0
            for index in 0..<10 where channel.send(.warning(generation: 1, message: "\(index)")) {
                accepted += 1
            }
            producer.markFinished(accepted: accepted)
        }.start()
        try await Task.sleep(for: .milliseconds(50))
        #expect(!producer.isFinished)
        #expect(stream != nil)

        stream = nil
        for _ in 0..<200 where !producer.isFinished {
            try await Task.sleep(for: .milliseconds(10))
        }

        #expect(producer.isFinished)
        #expect(producer.isClosed)
        #expect(producer.accepted == 2)
    }

    @Test("Repeated warnings keep their first-seen order without duplicates")
    func repeatedWarningsAreDeduplicatedInOrder() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Unreadable home")
        let generation = model.generation
        let messages = (0..<1_000).map { "Library/Private/\($0): Permission denied" }

        for message in messages + messages.reversed() {
            model.apply(.warning(generation: generation, message: message))
        }

        #expect(model.warnings == messages)
        model.rescan()
        model.apply(.warning(generation: model.generation, message: messages[0]))
        #expect(model.warnings == [messages[0]], "a new report forgets earlier warnings")
    }

    @Test("Rust artifact IDs map to the same stable UUIDs, built from bytes")
    func stableIDsAreBuiltFromBytes() {
        #expect(stableID(0).uuidString == "00000000-0000-4000-8000-000000000000")
        #expect(stableID(0xABCDEF).uuidString == "00000000-0000-4000-8000-000000ABCDEF")
        #expect(stableID(0xFFFF_FFFF_FFFF).uuidString == "00000000-0000-4000-8000-FFFFFFFFFFFF")
        #expect(stableID(0x1_0000_0000_0001) == stableID(1), "only the low 48 bits are kept")
        #expect(Set((0..<UInt64(10_000)).map(stableID)).count == 10_000)
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
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let bridge = RustScanBridgeContext(generation: 1, channel: channel,
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

private final class ProducerProbe: @unchecked Sendable {
    private let lock = NSLock()
    private var finished = false
    private var closed = false
    private var acceptedCount = 0

    var isFinished: Bool { lock.withLock { finished } }
    var isClosed: Bool { lock.withLock { closed } }
    var accepted: Int { lock.withLock { acceptedCount } }

    func markFinished(accepted: Int = 0) {
        lock.withLock {
            finished = true
            acceptedCount = accepted
        }
    }

    func markClosed() {
        lock.withLock { closed = true }
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

    @Test("Streaming refreshes merge new rows into the same order as a full sort")
    func streamingRefreshIsIncremental() async {
        let scan = WindowScanModel(source: TableIdleSource())
        let table = ScanTableModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        let artifacts = (0..<11_000).shuffled().map { index in
            ScanArtifact(id: UUID(), relativePath: "Projects/group\(index % 97)/Project\(index)/.build",
                         language: index.isMultiple(of: 3) ? "Rust" : "Swift", kind: .buildOutput)
        }
        for artifact in artifacts.prefix(10_000) {
            scan.apply(.discovered(generation: scan.generation, artifact: artifact))
        }
        await table.refresh(from: scan)

        for round in 0..<40 {
            for artifact in artifacts[(10_000 + round * 25)..<(10_000 + (round + 1) * 25)] {
                scan.apply(.discovered(generation: scan.generation, artifact: artifact))
            }
            for artifact in artifacts[(round * 25)..<((round + 1) * 25)] {
                scan.apply(.completed(generation: scan.generation, artifactID: artifact.id, bytes: Int64(round)))
            }
            await table.refresh(from: scan)
        }

        #expect(table.rows == ScanRowComparator.sorted(scan.rows, by: table.sortOrder))
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
