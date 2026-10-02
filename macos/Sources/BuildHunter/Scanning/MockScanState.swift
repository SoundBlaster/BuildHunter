#if DEBUG
import Foundation

enum MockScanState: String, CaseIterable, Identifiable {
    case empty
    case scanning
    case results
    case stopped
    case incomplete

    var id: Self { self }

    var title: String {
        switch self {
        case .empty: "Empty window"
        case .scanning: "Scanning"
        case .results: "Completed results"
        case .stopped: "Stopped · partial sizes"
        case .incomplete: "Incomplete · warning"
        }
    }

    var targetName: String? {
        self == .empty ? nil : "Demo Workspace"
    }

    var artifacts: [ScanArtifact] {
        guard self != .empty else { return [] }
        return [
            ScanArtifact(id: Self.swiftID, relativePath: "Packages/Core/.build",
                         language: "Swift", kind: .buildOutput),
            ScanArtifact(id: Self.rustID, relativePath: "Tools/Indexer/target",
                         language: "Rust", kind: .buildOutput),
            ScanArtifact(id: Self.pythonID, relativePath: "Services/API/.pytest_cache",
                         language: "Python", kind: .cache)
        ]
    }

    var completedArtifacts: [(ScanArtifact, Int64)] {
        let sizes: [Int64] = [1_610_612_736, 822_083_584, 12_582_912]
        let samples = Array(zip(artifacts, sizes))
        switch self {
        case .results:
            return samples
        case .stopped:
            return Array(samples.prefix(1))
        case .incomplete:
            return Array(samples.prefix(2))
        case .empty, .scanning:
            return []
        }
    }

    var warnings: [String] {
        self == .incomplete ? ["Permission denied while reading Demo Workspace/Private/.build."] : []
    }

    var terminalResult: ScanTerminalResult? {
        switch self {
        case .empty, .scanning: nil
        case .results, .incomplete: .completed
        case .stopped: .stopped
        }
    }

    private static let swiftID = UUID(uuidString: "B0000000-0000-4000-8000-000000000001")!
    private static let rustID = UUID(uuidString: "B0000000-0000-4000-8000-000000000002")!
    private static let pythonID = UUID(uuidString: "B0000000-0000-4000-8000-000000000003")!
}
#endif
