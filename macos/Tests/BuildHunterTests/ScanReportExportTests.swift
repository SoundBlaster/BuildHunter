import Foundation
import Testing
@testable import BuildHunter

@Suite("Scan results export")
struct ScanReportExportTests {
    private func row(_ path: String, _ size: SizeState, language: String = "Swift",
                     kind: ArtifactKind = .buildOutput) -> ScanRow {
        ScanRow(id: UUID(), relativePath: path, language: language, kind: kind, size: size)
    }

    private func export(_ rows: [ScanRow], order: ScanReportExport.Order = .table,
                        root: URL? = URL(fileURLWithPath: "/Users/me/Projects", isDirectory: true),
                        status: ScanReportExport.Status = .complete) -> ScanReportExport {
        ScanReportExport(rootName: "Projects", rootURL: root, status: status, rows: rows, order: order,
                         tableOrder: [ScanRowComparator(column: .path)], warnings: ["Permission denied: /x"],
                         excludedFilterIDs: ["rust.target", "python.bytecode"], elapsedSeconds: 1.23456,
                         generatedAt: Date(timeIntervalSince1970: 1_790_000_000), appVersion: "1.0 (7)")
    }

    @Test("Only a finished report can be exported")
    func exportableStatus() {
        #expect(ScanReportExport.Status(phase: .completed) == .complete)
        #expect(ScanReportExport.Status(phase: .stopped) == .stopped)
        #expect(ScanReportExport.Status(phase: .incomplete) == .incomplete)
        #expect(ScanReportExport.Status(phase: .scanning) == nil)
        #expect(ScanReportExport.Status(phase: .idle) == nil)
    }

    @Test("CSV is one RFC 4180 table with a BOM, CRLF lines and quoted special characters")
    func csvTable() throws {
        let csv = export([
            row("App/.build", .measured(2_048)),
            row("Odd, \"quoted\"\nname/target", .partial(512), language: "Rust", kind: .cache),
            row("Lib/.venv", .partial(nil), language: "Python", kind: .environment),
        ]).csv()
        #expect(csv.hasPrefix("\u{FEFF}Path,Absolute Path,Language,Kind,Size (bytes),Size,Size State\r\n"))
        #expect(csv.hasSuffix("\r\n"))
        let records = csv.dropFirst().components(separatedBy: "\r\n").filter { !$0.isEmpty }
        #expect(records.count == 4)
        #expect(records[1].hasPrefix("App/.build,/Users/me/Projects/App/.build,Swift,Build output,2048,"))
        #expect(records[1].hasSuffix(",measured"))
        #expect(records[2] == "Lib/.venv,/Users/me/Projects/Lib/.venv,Python,Environment,,,partial")
        // Commas, quotes and line breaks stay inside one quoted field with doubled quotes.
        #expect(records[3].hasPrefix("\"Odd, \"\"quoted\"\"\nname/target\","))
        #expect(records[3].contains(",Rust,Cache,512,"))
        #expect(records[3].hasSuffix(",partial"))
    }

    @Test("A synthetic report without a folder leaves absolute paths empty")
    func csvWithoutRoot() {
        let csv = export([row("A/.build", .measured(1))], root: nil).csv()
        #expect(csv.contains("\r\nA/.build,,Swift,Build output,1,"))
    }

    @Test("JSON keeps every CLI field and vocabulary and adds report metadata")
    func jsonMatchesCLI() throws {
        let data = try export([
            row("App/.build", .measured(2_048)),
            row("Lib/.venv", .partial(nil), language: "Python", kind: .testEnvironment),
        ], status: .stopped).json()
        let object = try #require(try JSONSerialization.jsonObject(with: data) as? [String: Any])
        // CLI keys: root, total_bytes, elapsed_seconds, artifacts, errors.
        #expect(object["root"] as? String == "/Users/me/Projects")
        #expect((object["total_bytes"] as? NSNumber)?.int64Value == 2_048)
        #expect((object["elapsed_seconds"] as? NSNumber)?.doubleValue == 1.235)
        #expect(object["errors"] as? [String] == ["Permission denied: /x"])
        // Additions.
        #expect((object["schema_version"] as? NSNumber)?.intValue == 1)
        #expect(object["status"] as? String == "stopped")
        #expect(object["app_version"] as? String == "1.0 (7)")
        #expect(object["excluded_filters"] as? [String] == ["python.bytecode", "rust.target"])
        #expect((object["generated_at"] as? String)?.hasPrefix("2026-") == true)

        let artifacts = try #require(object["artifacts"] as? [[String: Any]])
        #expect(artifacts.count == 2)
        let build = artifacts[0]
        #expect(build["path"] as? String == "/Users/me/Projects/App/.build")
        #expect(build["relative_path"] as? String == "App/.build")
        #expect(build["language"] as? String == "swift")
        #expect(build["kind"] as? String == "build")
        #expect((build["bytes"] as? NSNumber)?.int64Value == 2_048)
        #expect(build["size_state"] as? String == "measured")
        #expect((build["nested"] as? NSNumber)?.boolValue == false)
        let venv = artifacts[1]
        #expect(venv["kind"] as? String == "test-environment")
        // An unknown size stays a present key with null, like every other CLI field.
        #expect(venv.keys.contains("bytes") && venv["bytes"] is NSNull)
        #expect(venv["size_state"] as? String == "partial")
    }

    @Test("Rows follow the table order or go largest first with unknown sizes last")
    func ordering() {
        let rows = [
            row("b", .measured(10)),
            row("a", .partial(nil)),
            row("c", .measured(30)),
            row("d", .partial(20)),
        ]
        #expect(export(rows, order: .table).rows.map(\.relativePath) == ["a", "b", "c", "d"])
        #expect(export(rows, order: .largestFirst).rows.map(\.relativePath) == ["c", "d", "b", "a"])
    }

    @Test("The suggested file name is safe and carries the folder and date")
    func fileName() {
        let report = ScanReportExport(rootName: "a/b:c", rootURL: nil, status: .complete, rows: [], order: .table,
                                      tableOrder: [], warnings: [], excludedFilterIDs: [], elapsedSeconds: nil,
                                      generatedAt: Date(timeIntervalSince1970: 1_790_000_000), appVersion: "")
        let name = report.suggestedFileName(for: .json)
        #expect(name.hasPrefix("BuildHunter-a-b-c-2026-"))
        #expect(name.hasSuffix(".json"))
        #expect(!name.contains("/") && !name.contains(":"))
    }
}
