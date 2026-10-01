import Foundation

struct DemoArtifact: Identifiable, Equatable, Sendable {
    let id: UUID
    let relativePath: String
    let simulatedBytes: Int64
    let language: String
    let kind: ArtifactKind
}

enum SizeState: Equatable, Sendable {
    case measuring
    case simulated(Int64)
    case partial(Int64?)
}

enum ScanPhase: Equatable, Sendable {
    case idle
    case scanning
    case completed
    case stopped
    case incomplete
}

enum ScanTerminalResult: Sendable {
    case completed
    case stopped
    case failed(String)
}

enum ScanEvent: Sendable {
    case discovered(generation: UInt64, artifact: DemoArtifact)
    case completed(generation: UInt64, artifactID: UUID, simulatedBytes: Int64)
    case warning(generation: UInt64, message: String)
    case finished(generation: UInt64, result: ScanTerminalResult)

    var generation: UInt64 {
        switch self {
        case .discovered(let generation, _), .completed(let generation, _, _),
             .warning(let generation, _), .finished(let generation, _): generation
        }
    }
}

struct ScanRow: Identifiable, Equatable, Sendable {
    let id: UUID
    let relativePath: String
    let language: String
    let kind: ArtifactKind
    var size: SizeState
}
