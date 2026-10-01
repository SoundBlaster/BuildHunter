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

    var artifacts: [DemoArtifact] {
        guard self != .empty else { return [] }
        return [
            DemoArtifact(id: Self.swiftID, relativePath: "Packages/Core/.build",
                         simulatedBytes: 1_610_612_736, language: "Swift", kind: .buildOutput),
            DemoArtifact(id: Self.rustID, relativePath: "Tools/Indexer/target",
                         simulatedBytes: 822_083_584, language: "Rust", kind: .buildOutput),
            DemoArtifact(id: Self.pythonID, relativePath: "Services/API/.pytest_cache",
                         simulatedBytes: 12_582_912, language: "Python", kind: .cache)
        ]
    }

    var completedArtifacts: [(DemoArtifact, Int64)] {
        switch self {
        case .results:
            return artifacts.map { ($0, $0.simulatedBytes) }
        case .stopped:
            return artifacts.first.map { [($0, $0.simulatedBytes)] } ?? []
        case .incomplete:
            return artifacts.prefix(2).map { ($0, $0.simulatedBytes) }
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
