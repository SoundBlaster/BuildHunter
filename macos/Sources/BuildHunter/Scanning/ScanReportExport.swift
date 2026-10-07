import Foundation

/// A finished scan report captured for export to CSV or JSON. Building it copies the rows
/// in the chosen order; serializing it touches no UI state.
struct ScanReportExport: Sendable {
    enum Format: String, CaseIterable, Identifiable, Sendable {
        case csv
        case json

        var id: String { rawValue }
        var title: String {
            switch self {
            case .csv: "CSV"
            case .json: "JSON"
            }
        }
        var fileExtension: String { rawValue }
    }

    enum Order: String, CaseIterable, Identifiable, Sendable {
        case table
        case largestFirst

        var id: String { rawValue }
        var title: String {
            switch self {
            case .table: "As shown in the table"
            case .largestFirst: "Largest first"
            }
        }
    }

    enum Status: String, Sendable {
        case complete
        case stopped
        case incomplete

        /// Only a finished report can be exported; a running scan still changes sizes.
        init?(phase: ScanPhase) {
            switch phase {
            case .completed: self = .complete
            case .stopped: self = .stopped
            case .incomplete: self = .incomplete
            case .idle, .scanning: return nil
            }
        }
    }

    static let schemaVersion = 1

    let rootName: String
    let rootURL: URL?
    let status: Status
    let rows: [ScanRow]
    let warnings: [String]
    let excludedFilterIDs: [String]
    let elapsedSeconds: Double?
    let generatedAt: Date
    let appVersion: String

    init(rootName: String, rootURL: URL?, status: Status, rows: [ScanRow], order: Order,
         tableOrder: [ScanRowComparator], warnings: [String], excludedFilterIDs: Set<String>,
         elapsedSeconds: Double?, generatedAt: Date = Date(), appVersion: String) {
        self.rootName = rootName
        self.rootURL = rootURL
        self.status = status
        self.rows = Self.ordered(rows, order: order, tableOrder: tableOrder)
        self.warnings = warnings
        self.excludedFilterIDs = excludedFilterIDs.sorted()
        self.elapsedSeconds = elapsedSeconds
        self.generatedAt = generatedAt
        self.appVersion = appVersion
    }

    static func ordered(_ rows: [ScanRow], order: Order, tableOrder: [ScanRowComparator]) -> [ScanRow] {
        switch order {
        case .table: ScanRowComparator.sorted(rows, by: tableOrder)
        // Unknown sizes stay last in either direction; ties fall back to path.
        case .largestFirst: ScanRowComparator.sorted(rows, by: [ScanRowComparator(column: .size, order: .reverse)])
        }
    }

    func data(as format: Format) throws -> Data {
        switch format {
        case .csv: Data(csv().utf8)
        case .json: try json()
        }
    }

    /// `BuildHunter-<folder>-<yyyy-MM-dd>.<ext>`, safe for any file system.
    func suggestedFileName(for format: Format) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        let unsafe = CharacterSet(charactersIn: "/:\\").union(.controlCharacters)
        let folder = rootName.unicodeScalars.map { unsafe.contains($0) ? "-" : String($0) }.joined()
        return "BuildHunter-\(folder.isEmpty ? "Report" : folder)-\(formatter.string(from: generatedAt)).\(format.fileExtension)"
    }

    // MARK: CSV

    static let csvHeader = ["Path", "Absolute Path", "Language", "Kind", "Size (bytes)", "Size", "Size State"]

    /// RFC 4180 with CRLF line breaks and a UTF-8 byte order mark, so Excel and Numbers
    /// both read non-ASCII paths correctly. The file holds only the table; the folder and
    /// date are in its name.
    func csv() -> String {
        var lines = [Self.csvHeader.map(Self.csvField).joined(separator: ",")]
        for row in rows {
            let bytes = Self.bytes(of: row.size)
            lines.append([
                row.relativePath,
                absolutePath(of: row) ?? "",
                row.language,
                row.kind.rawValue,
                bytes.map(String.init) ?? "",
                bytes.map { ByteCountFormatter.string(fromByteCount: $0, countStyle: .binary) } ?? "",
                Self.sizeState(row.size),
            ].map(Self.csvField).joined(separator: ","))
        }
        return "\u{FEFF}" + lines.joined(separator: "\r\n") + "\r\n"
    }

    static func csvField(_ value: String) -> String {
        let needsQuotes = value.contains { $0 == "," || $0 == "\"" || $0 == "\n" || $0 == "\r" }
            || value.first == " " || value.last == " "
        guard needsQuotes else { return value }
        return "\"" + value.replacingOccurrences(of: "\"", with: "\"\"") + "\""
    }

    // MARK: JSON

    /// The fields of `build-hunter --json` (root, total_bytes, elapsed_seconds, artifacts
    /// with path/language/kind/bytes/nested, errors) with the CLI's lowercase vocabulary,
    /// so scripts written for the CLI read app exports too. Additional fields only add.
    func json() throws -> Data {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        encoder.dateEncodingStrategy = .iso8601
        return try encoder.encode(JSONReport(self))
    }

    private struct JSONReport: Encodable {
        let report: ScanReportExport
        init(_ report: ScanReportExport) { self.report = report }

        enum CodingKeys: String, CodingKey {
            case schemaVersion = "schema_version", generatedAt = "generated_at", appVersion = "app_version"
            case root, status, totalBytes = "total_bytes", elapsedSeconds = "elapsed_seconds"
            case excludedFilters = "excluded_filters", artifacts, errors
        }

        func encode(to encoder: Encoder) throws {
            var container = encoder.container(keyedBy: CodingKeys.self)
            try container.encode(ScanReportExport.schemaVersion, forKey: .schemaVersion)
            try container.encode(report.generatedAt, forKey: .generatedAt)
            try container.encode(report.appVersion, forKey: .appVersion)
            try container.encode(report.rootURL?.path ?? report.rootName, forKey: .root)
            try container.encode(report.status.rawValue, forKey: .status)
            try container.encode(report.totalBytes, forKey: .totalBytes)
            // Encoded as null when unknown, so every key the CLI writes is always present.
            try container.encode(report.elapsedSeconds.map { ($0 * 1_000).rounded() / 1_000 },
                                 forKey: .elapsedSeconds)
            try container.encode(report.excludedFilterIDs, forKey: .excludedFilters)
            try container.encode(report.rows.map { JSONArtifact(row: $0, report: report) }, forKey: .artifacts)
            try container.encode(report.warnings, forKey: .errors)
        }
    }

    private struct JSONArtifact: Encodable {
        let row: ScanRow
        let report: ScanReportExport

        enum CodingKeys: String, CodingKey {
            case path, relativePath = "relative_path", language, kind, bytes, sizeState = "size_state", nested
        }

        func encode(to encoder: Encoder) throws {
            var container = encoder.container(keyedBy: CodingKeys.self)
            try container.encode(report.absolutePath(of: row) ?? row.relativePath, forKey: .path)
            try container.encode(row.relativePath, forKey: .relativePath)
            try container.encode(row.language.lowercased(), forKey: .language)
            try container.encode(row.scannerKind ?? ScanReportExport.cliKind(row.kind), forKey: .kind)
            try container.encode(ScanReportExport.bytes(of: row.size), forKey: .bytes)
            try container.encode(ScanReportExport.sizeState(row.size), forKey: .sizeState)
            // The app reports only outermost artifact roots, never nested ones.
            try container.encode(false, forKey: .nested)
        }
    }

    // MARK: Shared values

    /// Known bytes; a partial size is a lower bound and still counts.
    var totalBytes: Int64 { rows.reduce(0) { $0 + (Self.bytes(of: $1.size) ?? 0) } }

    func absolutePath(of row: ScanRow) -> String? {
        rootURL?.appendingPathComponent(row.relativePath, isDirectory: true).path
    }

    static func bytes(of size: SizeState) -> Int64? {
        switch size {
        case .measured(let bytes): bytes
        case .partial(let bytes): bytes
        case .measuring: nil
        }
    }

    static func sizeState(_ size: SizeState) -> String {
        switch size {
        case .measured: "measured"
        case .partial: "partial"
        case .measuring: "measuring"
        }
    }

    /// The CLI's kind names (`build-hunter --json`).
    static func cliKind(_ kind: ArtifactKind) -> String {
        switch kind {
        case .buildOutput: "build"
        case .cache: "cache"
        case .environment: "environment"
        case .testEnvironment: "test-environment"
        }
    }
}
