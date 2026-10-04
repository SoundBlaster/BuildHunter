import Foundation
import XCTest
@testable import BuildHunter

/// Filesystem work is included; fixture creation and event parity are outside timing.
@MainActor
final class FinitePolicyScanPerformanceTests: XCTestCase {
    func testCompiledPolicyPreservesWholeScanAndRemovesPolicyCallbacks() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        try makeFixture(at: root)
        let catalog = SearchFilterCatalog.load()
        XCTAssertFalse(catalog.isEmpty)
        let filters = SearchFilterSnapshot(excludedFilterIDs: ["swift.build"], catalog: catalog)
        let table = try XCTUnwrap(RustCompiledPolicyTable.compile(filters: filters))
        let policy = BuildHunterScanPolicy(filters: filters)
        let baseline = try scan(root: root, policy: policy, table: nil)
        let compiled = try scan(root: root, policy: policy, table: table)
        XCTAssertEqual(baseline.status, 0)
        XCTAssertEqual(compiled.status, 0)
        XCTAssertEqual(compiled.events.sorted(), baseline.events.sorted())
        XCTAssertGreaterThan(baseline.policyCalls, 100)
        XCTAssertEqual(compiled.policyCalls, 0)
        XCTAssertGreaterThan(compiled.events.count, 100)

        var construction: [Double] = []
        var oldSamples: [Double] = []
        var newSamples: [Double] = []
        for round in 0..<7 {
            let start = ProcessInfo.processInfo.systemUptime
            let freshTable = try XCTUnwrap(RustCompiledPolicyTable.compile(filters: filters))
            construction.append(ProcessInfo.processInfo.systemUptime - start)
            for usesTable in round.isMultiple(of: 2) ? [false, true] : [true, false] {
                let result = try scan(root: root, policy: policy, table: usesTable ? freshTable : nil)
                // Assertions and sorting happen after the timed scan returns.
                XCTAssertEqual(result.status, 0)
                XCTAssertEqual(result.events.sorted(), baseline.events.sorted())
                XCTAssertEqual(result.policyCalls, usesTable ? 0 : baseline.policyCalls)
                if usesTable { newSamples.append(result.elapsed) } else { oldSamples.append(result.elapsed) }
            }
        }
        func median(_ values: [Double]) -> Double { values.sorted()[values.count / 2] }
        let oldMedian = median(oldSamples)
        let newMedian = median(newSamples)
        let mad = median(oldSamples.map { abs($0 - oldMedian) })
            + median(newSamples.map { abs($0 - newMedian) })
        // Include compilation in the acceptance gate: small scans must not hide startup cost.
        let constructionMedian = median(construction)
        let limit = oldMedian * 1.5 + 0.050 + 6 * mad
        let report: [String: Any] = [
            "fixture": "64 mixed projects, excluded Swift roots containing Python artifacts",
            "baseline_scan_seconds": oldSamples, "compiled_scan_seconds": newSamples,
            "table_construction_seconds": construction,
            "baseline_median_seconds": oldMedian, "compiled_median_seconds": newMedian,
            "construction_median_seconds": constructionMedian,
            "combined_mad_seconds": mad, "gate_limit_seconds": limit,
            "baseline_policy_callbacks": baseline.policyCalls, "compiled_policy_callbacks": compiled.policyCalls,
            "event_count": compiled.events.count, "table_cell_count": table.entries.count,
            "os": ProcessInfo.processInfo.operatingSystemVersionString
        ]
        let data = try JSONSerialization.data(withJSONObject: report, options: [.sortedKeys, .prettyPrinted])
        let attachment = XCTAttachment(data: data, uniformTypeIdentifier: "public.json")
        attachment.name = "finite-policy-whole-scan.json"
        attachment.lifetime = .keepAlways
        add(attachment)
        XCTAssertLessThanOrEqual(newMedian + constructionMedian, limit,
                                 "Table construction plus scan must stay within the broad paired baseline gate.")
    }

    private func makeFixture(at root: URL) throws {
        let manager = FileManager.default
        for index in 0..<64 {
            let project = root.appendingPathComponent("project-\(index)")
            for directory in [".build/__pycache__", "target", "build", "custom-environment",
                              "__pycache__", ".pytest_cache", ".git/.build", "Sources"] {
                let folder = project.appendingPathComponent(directory)
                try manager.createDirectory(at: folder, withIntermediateDirectories: true)
                try Data(repeating: UInt8(index), count: 256).write(to: folder.appendingPathComponent("payload"))
            }
            for file in ["Cargo.toml", "pyproject.toml", "custom-environment/pyvenv.cfg", "Sources/module.pyc"] {
                try Data("fixture".utf8).write(to: project.appendingPathComponent(file))
            }
            try manager.createSymbolicLink(at: project.appendingPathComponent("linked-cache"),
                                           withDestinationURL: project.appendingPathComponent("__pycache__"))
        }
    }

    private func scan(root: URL, policy: BuildHunterScanPolicy,
                      table: RustCompiledPolicyTable?) throws -> PolicyScanCapture {
        let control = try XCTUnwrap(bh_scan_control_create())
        defer { bh_scan_control_destroy(control) }
        let capture = PolicyScanCapture(policy: policy)
        let context = Unmanaged.passUnretained(capture).toOpaque()
        let bytes = Array(root.path.utf8)
        let start = ProcessInfo.processInfo.systemUptime
        capture.status = bytes.withUnsafeBufferPointer { buffer in
            if let table {
                return table.withUnsafeEntries { entries, count in
                    bh_scan_with_policy_table(control, buffer.baseAddress, buffer.count, 0,
                                              table.version, entries, count, capturePolicyEvent, context)
                }
            }
            return bh_scan(control, buffer.baseAddress, buffer.count, 0,
                           capturePolicyDecision, capturePolicyEvent, context)
        }
        capture.elapsed = ProcessInfo.processInfo.systemUptime - start
        return capture
    }
}

// Rust invokes both callbacks serially on this test's scan thread. Borrowed payloads
// are copied before returning; numeric IDs are normalized to paths between runs.
private final class PolicyScanCapture {
    let policy: BuildHunterScanPolicy
    var policyCalls = 0
    var paths: [UInt64: String] = [:]
    var events: [String] = []
    var status: Int32 = 3
    var elapsed = 0.0
    init(policy: BuildHunterScanPolicy) { self.policy = policy }
}

private func capturePolicyDecision(_ rawContext: UnsafeMutableRawPointer?,
                                   _ facts: UnsafePointer<BHCandidateFacts>?,
                                   _ decision: UnsafeMutablePointer<BHCandidateDecision>?) -> Int32 {
    guard let rawContext, let facts, let decision else { return 0 }
    let capture = Unmanaged<PolicyScanCapture>.fromOpaque(rawContext).takeUnretainedValue()
    capture.policyCalls += 1
    decision.pointee = capture.policy.decision(for: facts.pointee)
    return 1
}

private func capturePolicyEvent(_ rawContext: UnsafeMutableRawPointer?, _ rawEvent: UnsafePointer<BHScanEvent>?) {
    guard let rawContext, let rawEvent else { return }
    let capture = Unmanaged<PolicyScanCapture>.fromOpaque(rawContext).takeUnretainedValue()
    let event = rawEvent.pointee
    func decode(_ pointer: UnsafePointer<UInt8>?, _ length: Int) -> String {
        guard let pointer else { return "" }
        return String(decoding: UnsafeBufferPointer(start: pointer, count: length), as: UTF8.self)
    }
    let path = decode(event.path, event.path_len)
    if event.event_type == 1 { capture.paths[event.artifact_id] = path }
    let normalizedPath = path.isEmpty ? capture.paths[event.artifact_id] ?? "" : path
    capture.events.append("\(event.event_type)|\(normalizedPath)|\(event.language)|\(event.kind)|\(event.bytes)|\(event.partial)|\(event.status)|\(decode(event.message, event.message_len))")
}
