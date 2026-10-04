import Foundation
import XCTest
@testable import BuildHunter

/// Run separately in Release: fixture construction and validation are outside measurements.
@MainActor
final class ScanPerformanceTests: XCTestCase {
    func testApply1000ArtifactRows() async {
        measureRows(count: 1_000)
    }

    func testApply10000ArtifactRows() async {
        measureRows(count: 10_000)
    }

    func testPrepare10000ArtifactDiagramRows() async {
        let rows = (0..<10_000).map { index in
            ScanRow(id: UUID(), relativePath: "Project\(index)/.build", language: "Swift",
                    kind: .buildOutput, size: .measured(4_096))
        }
        measure(metrics: [XCTClockMetric(), XCTMemoryMetric()], options: measurementOptions()) {
            startMeasuring()
            let snapshot = ArtifactSunburstSnapshot(rows: rows)
            let layout = ArtifactSunburstLayout(snapshot: snapshot)
            var palette = ArtifactSunburstPalette()
            palette.include(layout)
            let sorted = ScanRowComparator.sorted(rows, by: [.init(column: .path)])
            stopMeasuring()
            XCTAssertEqual(sorted.count, rows.count)
            XCTAssertEqual(sorted.first?.relativePath, "Project0/.build")
            XCTAssertEqual(sorted.last?.relativePath, "Project9999/.build")
            XCTAssertEqual(snapshot.root.statistics.artifactCount, rows.count)
            XCTAssertEqual(snapshot.root.bytes, Double(rows.count * 4_096))
            XCTAssertLessThanOrEqual(layout.sectors.count, 399)
            XCTAssertLessThanOrEqual(palette.colors.count, layout.sectors.count)
        }
    }

    func testClassify10000Candidates() async {
        let policy = ClassifyArtifactRoot()
        // Directories and standalone bytecode files; file checks must stay string work.
        let candidates = (0..<10_000).map { index in
            switch index % 3 {
            case 0:
                ArtifactPolicyContext(nodeName: ".build", isDirectory: true, isSymbolicLink: false,
                                      ownMarkerFiles: [], parentMarkerFiles: [])
            case 1:
                ArtifactPolicyContext(nodeName: "target", isDirectory: true, isSymbolicLink: false,
                                      ownMarkerFiles: [], parentMarkerFiles: ["Cargo.toml"])
            default:
                ArtifactPolicyContext(nodeName: "module\(index).cpython-312.pyc", isDirectory: false,
                                      isSymbolicLink: false, ownMarkerFiles: [], parentMarkerFiles: [])
            }
        }
        let options = measurementOptions()
        measure(metrics: [XCTClockMetric(), XCTMemoryMetric()], options: options) {
            var classified = 0
            startMeasuring()
            for candidate in candidates {
                if policy.decide(candidate) != nil { classified += 1 }
            }
            stopMeasuring()
            XCTAssertEqual(classified, candidates.count)
        }
    }

    /// Compares the old String/Set bridge projection with ABI-native marker facts. XCTest's
    /// memory metric records the allocation-sensitive work; there is deliberately no exact
    /// nanosecond or byte threshold because host noise varies substantially.
    func testLegacySetMarkerBridgeProjectionAllocationProfile() {
        let candidates = markerBridgeCandidates(count: 10_000)
        let policy = ClassifyArtifactRoot()
        measure(metrics: [XCTClockMetric(), XCTMemoryMetric()], options: measurementOptions()) {
            startMeasuring()
            var classified = 0
            for candidate in candidates {
                let context = ArtifactPolicyContext(
                    nodeName: candidate.name, isDirectory: true, isSymbolicLink: false,
                    ownMarkerFiles: legacyMarkerNames(candidate.ownMask),
                    parentMarkerFiles: legacyMarkerNames(candidate.parentMask)
                )
                if policy.decide(context) != nil { classified += 1 }
            }
            stopMeasuring()
            XCTAssertGreaterThan(classified, 0)
        }
    }

    func testDirectMaskMarkerBridgeFactsAllocationProfile() {
        let candidates = markerBridgeCandidates(count: 10_000)
        let policy = ClassifyArtifactRoot()
        measure(metrics: [XCTClockMetric(), XCTMemoryMetric()], options: measurementOptions()) {
            startMeasuring()
            var classified = 0
            for candidate in candidates {
                let context = ArtifactPolicyContext(
                    nodeName: candidate.name, isDirectory: true, isSymbolicLink: false,
                    ownMarkerFacts: ArtifactMarkerFacts(rawValue: candidate.ownMask),
                    parentMarkerFacts: ArtifactMarkerFacts(rawValue: candidate.parentMask)
                )
                if policy.decide(context) != nil { classified += 1 }
            }
            stopMeasuring()
            XCTAssertGreaterThan(classified, 0)
        }
    }

    /// Both bridge representations are checked at 10x input sizes, with a broad allowance
    /// that catches nonlinear work without turning runner timing into a speed claim.
    func testMarkerBridgeRepresentationsScaleWithCandidateCount() {
        let small = markerBridgeCandidates(count: 1_000)
        let large = markerBridgeCandidates(count: 10_000)
        for usesMasks in [false, true] {
            let smallElapsed = markerBridgeElapsed(small, usesMasks: usesMasks)
            let largeElapsed = markerBridgeElapsed(large, usesMasks: usesMasks)
            XCTAssertLessThanOrEqual(
                largeElapsed, smallElapsed * 20 + 0.025,
                "10,000 marker bridge candidates must scale within 20x the 1,000-candidate median plus 25 ms."
            )
        }
    }

    /// A streaming table refresh merges 25 new rows into 10,000 sorted ones. It must stay well
    /// below a full localized re-sort; both are measured in the same process, interleaved.
    func testIncrementalTableRefreshBeatsFullSort() async throws {
        let order = [ScanRowComparator(column: .path)]
        let rows = (0..<10_025).shuffled().map { index in
            ScanRow(id: UUID(), relativePath: "Projects/group\(index % 97)/Project\(index)/.build",
                    language: "Swift", kind: .buildOutput, size: .measured(4_096))
        }
        let existing = Array(rows.prefix(10_000))
        let previous = ScanTableProjection(existing, by: order, extending: nil).sortedIndices
        let expected = ScanRowComparator.sorted(rows, by: order)
        try assertFaster(name: "table-refresh.json", factor: 5) { incremental in
            let start = ProcessInfo.processInfo.systemUptime
            let projection = ScanTableProjection(rows, by: order, extending: incremental ? previous : nil)
            let elapsed = ProcessInfo.processInfo.systemUptime - start
            XCTAssertEqual(projection.rows, expected)
            return elapsed
        }
    }

    /// A streaming diagram refresh with 25 new rows and 25 measured rows on top of 10,000
    /// must stay well below rebuilding every folder node from every row.
    func testIncrementalDiagramSnapshotBeatsFullRebuild() async throws {
        let rows = (0..<10_025).map { index in
            ScanRow(id: UUID(), relativePath: "Projects/group\(index % 97)/Project\(index)/Sources/.build",
                    language: "Swift", kind: .buildOutput, size: .measuring)
        }
        let existing = Array(rows.prefix(10_000))
        let previous = ArtifactSunburstSnapshot(rows: existing)
        var updated = rows
        for index in 0..<25 { updated[index].size = .measured(4_096) }
        let expected = ArtifactSunburstSnapshot(rows: updated)
        try assertFaster(name: "diagram-refresh.json", factor: 3) { incremental in
            let start = ProcessInfo.processInfo.systemUptime
            let snapshot = incremental
                ? ArtifactSunburstSnapshot(rows: updated, updating: previous, previousSizes: existing.map(\.size))
                : ArtifactSunburstSnapshot(rows: updated)
            let elapsed = ProcessInfo.processInfo.systemUptime - start
            XCTAssertEqual(snapshot, expected)
            return elapsed
        }
    }

    /// Interleaved medians of five samples after a warm-up: the candidate must take at most
    /// `1 / factor` of the baseline. A ratio in one process tolerates slow runners.
    private func assertFaster(name: String, factor: Double, sample: (_ candidate: Bool) -> Double) throws {
        _ = sample(false)
        _ = sample(true)
        var baseline: [Double] = []
        var candidate: [Double] = []
        for iteration in 0..<5 {
            if iteration.isMultiple(of: 2) {
                baseline.append(sample(false))
                candidate.append(sample(true))
            } else {
                candidate.append(sample(true))
                baseline.append(sample(false))
            }
        }
        let baselineMedian = baseline.sorted()[2]
        let candidateMedian = candidate.sorted()[2]
        let report: [String: Any] = [
            "baseline_seconds": baseline, "candidate_seconds": candidate,
            "baseline_median_seconds": baselineMedian, "candidate_median_seconds": candidateMedian,
            "minimum_speedup": factor
        ]
        let data = try JSONSerialization.data(withJSONObject: report, options: [.sortedKeys, .prettyPrinted])
        let attachment = XCTAttachment(data: data, uniformTypeIdentifier: "public.json")
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
        XCTAssertLessThanOrEqual(candidateMedian * factor, baselineMedian,
                                 "The candidate must be at least \(factor)x faster than the baseline.")
    }

    /// A tenfold input increase must not approach quadratic growth. The fixed noise allowance
    /// protects very short runs; this is a coarse complexity guard, not a frame-time promise.
    func testEventProcessingScalesWithRowCount() async throws {
        let smallEvents = events(count: 1_000)
        let largeEvents = events(count: 10_000)
        _ = elapsedApplying(smallEvents)
        _ = elapsedApplying(largeEvents)
        var small: [Double] = []
        var large: [Double] = []
        for iteration in 0..<5 {
            if iteration.isMultiple(of: 2) {
                small.append(elapsedApplying(smallEvents))
                large.append(elapsedApplying(largeEvents))
            } else {
                large.append(elapsedApplying(largeEvents))
                small.append(elapsedApplying(smallEvents))
            }
        }
        let smallMedian = small.sorted()[small.count / 2]
        let largeMedian = large.sorted()[large.count / 2]
        let report: [String: Any] = [
            "small_rows": 1_000, "large_rows": 10_000,
            "small_seconds": small, "large_seconds": large,
            "small_median_seconds": smallMedian, "large_median_seconds": largeMedian,
            "maximum_growth": 20, "noise_allowance_seconds": 0.025
        ]
        let data = try JSONSerialization.data(withJSONObject: report, options: [.sortedKeys, .prettyPrinted])
        let attachment = XCTAttachment(data: data, uniformTypeIdentifier: "public.json")
        attachment.name = "event-scaling.json"
        attachment.lifetime = .keepAlways
        add(attachment)
        XCTAssertLessThanOrEqual(
            largeMedian, smallMedian * 20 + 0.025,
            "10,000 rows must scale within 20x the median time for 1,000 rows plus 25 ms of noise."
        )
    }

    /// Deduplicating 10,000 distinct warnings must stay near-linear: a membership scan of the
    /// warning list grows a hundredfold for a tenfold input.
    func testWarningProcessingScalesWithCount() async throws {
        let small = (0..<1_000).map { ScanEvent.warning(generation: 1, message: "Private/\($0): Permission denied") }
        let large = (0..<10_000).map { ScanEvent.warning(generation: 1, message: "Private/\($0): Permission denied") }
        try assertScalesNearLinearly(name: "warning-scaling.json", smallCount: 1_000, largeCount: 10_000) { isLarge in
            let events = isLarge ? large : small
            let model = makeModel()
            let start = ProcessInfo.processInfo.systemUptime
            for event in events { model.apply(event) }
            let elapsed = ProcessInfo.processInfo.systemUptime - start
            XCTAssertEqual(model.warnings.count, events.count)
            return elapsed
        }
    }

    /// Five interleaved samples per size after a warm-up; the large median may grow at most
    /// 20x the small one plus 25 ms of noise, as in the event-scaling gate.
    private func assertScalesNearLinearly(
        name: String, smallCount: Int, largeCount: Int, sample: (_ isLarge: Bool) -> Double
    ) throws {
        _ = sample(false)
        _ = sample(true)
        var small: [Double] = []
        var large: [Double] = []
        for iteration in 0..<5 {
            if iteration.isMultiple(of: 2) {
                small.append(sample(false))
                large.append(sample(true))
            } else {
                large.append(sample(true))
                small.append(sample(false))
            }
        }
        let smallMedian = small.sorted()[small.count / 2]
        let largeMedian = large.sorted()[large.count / 2]
        let report: [String: Any] = [
            "small_count": smallCount, "large_count": largeCount,
            "small_seconds": small, "large_seconds": large,
            "small_median_seconds": smallMedian, "large_median_seconds": largeMedian,
            "maximum_growth": 20, "noise_allowance_seconds": 0.025
        ]
        let data = try JSONSerialization.data(withJSONObject: report, options: [.sortedKeys, .prettyPrinted])
        let attachment = XCTAttachment(data: data, uniformTypeIdentifier: "public.json")
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
        XCTAssertLessThanOrEqual(
            largeMedian, smallMedian * 20 + 0.025,
            "\(largeCount) items must scale within 20x the median time for \(smallCount) plus 25 ms of noise."
        )
    }

    private func measureRows(count: Int) {
        let batch = events(count: count)
        measure(metrics: [XCTClockMetric(), XCTMemoryMetric()], options: measurementOptions()) {
            let model = makeModel()
            startMeasuring()
            for event in batch { model.apply(event) }
            stopMeasuring()
            validate(model, count: count)
        }
    }

    private func markerBridgeCandidates(count: Int) -> [(name: String, ownMask: UInt32, parentMask: UInt32)] {
        let names = ["venv", "target", "build", ".tox", ".build", "worker.pyc"]
        return (0..<count).map { index in
            return (names[index % names.count], UInt32(index & 31), UInt32((index * 7) & 31))
        }
    }

    private func markerBridgeElapsed(
        _ candidates: [(name: String, ownMask: UInt32, parentMask: UInt32)], usesMasks: Bool
    ) -> Double {
        let policy = ClassifyArtifactRoot()
        var classified = 0
        let start = ProcessInfo.processInfo.systemUptime
        for candidate in candidates {
            let context: ArtifactPolicyContext
            if usesMasks {
                context = ArtifactPolicyContext(
                    nodeName: candidate.name, isDirectory: true, isSymbolicLink: false,
                    ownMarkerFacts: ArtifactMarkerFacts(rawValue: candidate.ownMask),
                    parentMarkerFacts: ArtifactMarkerFacts(rawValue: candidate.parentMask)
                )
            } else {
                context = ArtifactPolicyContext(
                    nodeName: candidate.name, isDirectory: true, isSymbolicLink: false,
                    ownMarkerFiles: legacyMarkerNames(candidate.ownMask),
                    parentMarkerFiles: legacyMarkerNames(candidate.parentMask)
                )
            }
            if policy.decide(context) != nil { classified += 1 }
        }
        let elapsed = ProcessInfo.processInfo.systemUptime - start
        XCTAssertGreaterThan(classified, 0)
        return elapsed
    }

    private func legacyMarkerNames(_ flags: UInt32) -> Set<String> {
        var names = Set<String>()
        if flags & 1 != 0 { names.insert("Cargo.toml") }
        if flags & 2 != 0 { names.insert("pyproject.toml") }
        if flags & 4 != 0 { names.insert("setup.py") }
        if flags & 8 != 0 { names.insert("setup.cfg") }
        if flags & 16 != 0 { names.insert("pyvenv.cfg") }
        return names
    }

    private func elapsedApplying(_ batch: [ScanEvent]) -> Double {
        let model = makeModel()
        let start = ProcessInfo.processInfo.systemUptime
        for event in batch { model.apply(event) }
        let elapsed = ProcessInfo.processInfo.systemUptime - start
        validate(model, count: (batch.count - 1) / 2)
        return elapsed
    }

    private func validate(_ model: WindowScanModel, count: Int) {
        XCTAssertEqual(model.rows.count, count)
        XCTAssertEqual(model.phase, .completed)
        XCTAssertTrue(model.warnings.isEmpty)
        XCTAssertTrue(model.rows.allSatisfy { $0.size == .measured(4_096) })
    }

    private func makeModel() -> WindowScanModel {
        let model = WindowScanModel(source: PerformanceIdleSource())
        model.acceptDemoTarget(named: "Performance fixture")
        return model
    }

    private func events(count: Int) -> [ScanEvent] {
        (0..<count).flatMap { index -> [ScanEvent] in
            let artifact = ScanArtifact(
                id: UUID(), relativePath: "Project\(index)/.build",
                language: "Swift", kind: .buildOutput
            )
            return [
                .discovered(generation: 1, artifact: artifact),
                .completed(generation: 1, artifactID: artifact.id, bytes: 4_096)
            ]
        } + [.finished(generation: 1, result: .completed)]
    }

    private func measurementOptions() -> XCTMeasureOptions {
        let options = XCTMeasureOptions()
        options.iterationCount = 5
        options.invocationOptions = [.manuallyStart, .manuallyStop]
        return options
    }
}

private struct PerformanceIdleSource: ScanEventSource {
    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        AsyncStream { $0.finish() }
    }

    func cancel(generation: UInt64) {}
}
