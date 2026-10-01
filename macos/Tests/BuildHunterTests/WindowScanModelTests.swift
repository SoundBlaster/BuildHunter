import Foundation
import Testing
@testable import BuildHunter

@Suite("Window scan lifecycle")
@MainActor
struct WindowScanModelTests {
    @Test("Demo source completes without manufacturing access warnings")
    func demoCompletesNormally() async {
        let model = WindowScanModel()
        model.acceptDemoTarget(named: "Demo")
        await model.waitForCurrentScan()
        #expect(model.rows.count == 3)
        #expect(model.phase == .completed)
        #expect(model.warnings.isEmpty)
    }

    @Test("Terminal event rejects later events from the same generation")
    func terminalEventRejectsLateUpdates() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let artifact = makeArtifact()

        model.apply(.discovered(generation: generation, artifact: artifact))
        model.apply(.completed(generation: generation, artifactID: artifact.id, simulatedBytes: 20))
        model.apply(.finished(generation: generation, result: .completed))
        model.apply(.warning(generation: generation, message: "late warning"))
        model.apply(.completed(generation: generation, artifactID: artifact.id, simulatedBytes: 99))

        #expect(model.rows[0].size == .simulated(20))
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
        source.yield(.completed(generation: generation, artifactID: artifact.id, simulatedBytes: 20))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)

        await model.waitForCurrentScan()

        #expect(model.rows == [ScanRow(id: artifact.id, relativePath: artifact.relativePath,
                                      language: artifact.language, kind: artifact.kind, size: .simulated(20))])
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

        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(model.warnings == ["fixture warning"])
    }

    @Test("Duplicate and out-of-order events do not duplicate rows or invent a size")
    func duplicateAndOutOfOrderEventsAreSafe() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let artifact = makeArtifact()

        model.apply(.completed(generation: generation, artifactID: artifact.id, simulatedBytes: 20))
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
        #expect(model.rows.allSatisfy { if case .simulated = $0.size { true } else { false } })

        model.showMockState(.stopped)
        #expect(model.phase == .stopped)
        #expect(model.rows.filter { $0.size == .partial(nil) }.count == 2)
        #expect(model.rows.filter { if case .simulated = $0.size { true } else { false } }.count == 1)

        model.showMockState(.incomplete)
        #expect(model.phase == .incomplete)
        #expect(model.warnings.count == 1)
        #expect(model.rows.filter { $0.size == .partial(nil) }.count == 1)
    }
#endif

    private func makeArtifact() -> DemoArtifact {
        DemoArtifact(id: UUID(uuidString: "A0000000-0000-4000-8000-000000000001")!,
                     relativePath: "DemoFixture/Alpha/.build", simulatedBytes: 20,
                     language: "Swift", kind: .buildOutput)
    }
}

@MainActor
private final class ControlledScanSource: ScanEventSource {
    private var continuations: [UInt64: AsyncStream<ScanEvent>.Continuation] = [:]
    private let terminationEvents: AsyncStream<UInt64>
    private let terminationContinuation: AsyncStream<UInt64>.Continuation

    init() {
        (terminationEvents, terminationContinuation) = AsyncStream.makeStream(of: UInt64.self)
    }

    func events(for generation: UInt64) -> AsyncStream<ScanEvent> {
        let (stream, continuation) = AsyncStream.makeStream(of: ScanEvent.self)
        continuations[generation] = continuation
        let terminationContinuation = self.terminationContinuation
        continuation.onTermination = { @Sendable _ in
            terminationContinuation.yield(generation)
        }
        return stream
    }

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
