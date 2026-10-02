import Foundation

struct ScanArtifact: Identifiable, Equatable, Sendable {
    let id: UUID
    let relativePath: String
    let language: String
    let kind: ArtifactKind
}

enum SizeState: Equatable, Sendable {
    case measuring
    case measured(Int64)
    case partial(Int64?)
}

enum ScanPhase: Equatable, Sendable {
    case idle
    case scanning
    case completed
    case stopped
    case incomplete
}

enum ScanTerminalResult: Equatable, Sendable {
    case completed
    case stopped
    case failed(String)
}

enum ScanEvent: Sendable {
    case discovered(generation: UInt64, artifact: ScanArtifact)
    case completed(generation: UInt64, artifactID: UUID, bytes: Int64, partial: Bool = false)
    case warning(generation: UInt64, message: String)
    case finished(generation: UInt64, result: ScanTerminalResult)

    var generation: UInt64 {
        switch self {
        case .discovered(let generation, _), .completed(let generation, _, _, _),
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
