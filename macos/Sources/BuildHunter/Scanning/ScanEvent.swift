import Foundation

struct ScanArtifact: Identifiable, Equatable, Sendable {
    let id: UUID
    let relativePath: String
    let language: String
    let kind: ArtifactKind
    /// The scanner's kind name where `kind` merges several (Python `bytecode` and
    /// `metadata` show as Cache); exports keep it so they match `build-hunter --json`.
    var scannerKind: String? = nil
    var targetID: String? = nil
    var catalogDigest: String? = nil
    /// First lossy component in display coordinates; disables actions on it and its descendants.
    var unsafePathPrefix: String? = nil
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
    case completedWithChanges
    case stopped
    case failed(String)
}

enum ScanEvent: Sendable {
    case catalog(generation: UInt64, metadata: ScanCatalogMetadata, rootPathIsExact: Bool)
    case diagnostic(generation: UInt64, diagnostic: ScanDiagnostic, relativePath: String? = nil)
    case discovered(generation: UInt64, artifact: ScanArtifact)
    case completed(generation: UInt64, artifactID: UUID, bytes: Int64, partial: Bool = false)
    case profile(generation: UInt64, sample: ScanProfileSnapshot)
    /// Only an exact decoded path may be offered as an action; never parse it from message text.
    case warning(generation: UInt64, message: String, relativePath: String? = nil)
    case finished(generation: UInt64, result: ScanTerminalResult)

    var generation: UInt64 {
        switch self {
        case .catalog(let generation, _, _), .diagnostic(let generation, _, _),
             .discovered(let generation, _), .completed(let generation, _, _, _),
             .warning(let generation, _, _), .profile(let generation, _), .finished(let generation, _): generation
        }
    }
}

struct ScanRow: Identifiable, Equatable, Sendable {
    let id: UUID
    let relativePath: String
    let language: String
    let kind: ArtifactKind
    var size: SizeState
    /// See `ScanArtifact.scannerKind`.
    var scannerKind: String? = nil
    var targetID: String? = nil
    var catalogDigest: String? = nil
    /// First lossy component in display coordinates; disables actions on it and its descendants.
    var unsafePathPrefix: String? = nil
}

/// Table sorting is a projection: the scanner retains its append-only row indices.
struct ScanRowComparator: SortComparator, Sendable {
    enum Column: Hashable, Sendable { case path, size, language, kind }
    let column: Column
    var order: SortOrder = .forward

    func compare(_ lhs: ScanRow, _ rhs: ScanRow) -> ComparisonResult {
        let result: ComparisonResult
        switch column {
        case .path: result = lhs.relativePath.localizedStandardCompare(rhs.relativePath)
        case .language: result = lhs.language.localizedStandardCompare(rhs.language)
        case .kind: result = lhs.kind.rawValue.localizedStandardCompare(rhs.kind.rawValue)
        case .size:
            switch (knownBytes(lhs.size), knownBytes(rhs.size)) {
            case let (.some(left), .some(right)):
                result = left == right ? .orderedSame : left < right ? .orderedAscending : .orderedDescending
            case (.none, .none): return .orderedSame
            case (.none, .some): return .orderedDescending
            case (.some, .none): return .orderedAscending
            }
        }
        guard order == .reverse else { return result }
        return result == .orderedSame ? .orderedSame : result == .orderedAscending ? .orderedDescending : .orderedAscending
    }

    static func sorted(_ rows: [ScanRow], by comparators: [Self]) -> [ScanRow] {
        rows.sorted { areInIncreasingOrder($0, $1, by: comparators) }
    }

    /// A total order: ties fall back to path, then identity.
    static func areInIncreasingOrder(_ first: ScanRow, _ second: ScanRow, by comparators: [Self]) -> Bool {
        for comparator in comparators {
            let comparison = comparator.compare(first, second)
            if comparison != .orderedSame { return comparison == .orderedAscending }
        }
        if first.relativePath != second.relativePath { return first.relativePath < second.relativePath }
        return first.id.uuidString < second.id.uuidString
    }

    private func knownBytes(_ size: SizeState) -> Int64? {
        switch size {
        case .measured(let bytes), .partial(.some(let bytes)): bytes >= 0 ? bytes : nil
        case .measuring, .partial(nil): nil
        }
    }
}
