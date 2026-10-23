import Darwin
import Foundation
import Testing
@testable import BuildHunter

private final class MigrationFixtureBundle: NSObject {}

private struct MigrationCorpus: Decodable {
    let schemaVersion: Int
    let catalogIDs: [String]
    let cases: [MigrationTree]
    let terminalMappings: [TerminalMapping]
    let invalidPathBytes: [UInt8]
}
private struct TerminalMapping: Decodable { let rustStatus: UInt32; let gui: String }
private struct MigrationTree: Decodable {
    let id: String
    let files: [String: String]
    let directories: [String]
    let symlinks: [String: String]
    let scanRoot: String
    let rows: [String: MigrationRow]
    let runs: [MigrationRun]
}
private struct MigrationRun: Decodable {
    let id: String
    let client: String
    let expected: [String]
    let excluded: [String]
    let status: String
    let warnings: Int
}
private struct MigrationRow: Decodable {
    let path: String
    let filterID: String
    let kind: String
    let language: String
    let nested: Bool
}

@Suite("Migration baseline parity")
@MainActor
struct MigrationParityTests {
    private func corpus() throws -> MigrationCorpus {
        let url = try #require(Bundle(for: MigrationFixtureBundle.self)
            .url(forResource: "migration-parity-v1", withExtension: "json"))
        let value = try JSONDecoder().decode(MigrationCorpus.self, from: Data(contentsOf: url))
        #expect(value.schemaVersion == 1)
        #expect(SearchFilterCatalog.load().map(\.id) == value.catalogIDs)
        return value
    }

    @Test("Shared golden trees through the real GUI event source", .timeLimit(.minutes(1)))
    func guiGoldenTrees() async throws {
        for tree in try corpus().cases {
            for run in tree.runs where run.client == "gui" {
                try await check(tree, run)
            }
        }
    }

    private func check(_ tree: MigrationTree, _ run: MigrationRun) async throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("BuildHunter migration \(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        for path in tree.directories {
            try FileManager.default.createDirectory(at: root.appendingPathComponent(path), withIntermediateDirectories: true)
        }
        for (path, content) in tree.files {
            let url = root.appendingPathComponent(path)
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try Data(content.utf8).write(to: url)
        }
        for (path, target) in tree.symlinks {
            let url = root.appendingPathComponent(path)
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try FileManager.default.createSymbolicLink(atPath: url.path, withDestinationPath: target)
        }
        let suite = "BuildHunter.Migration.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        defaults.set(run.excluded, forKey: SearchFilterSettings.persistenceKey)
        let settings = SearchFilterSettings(userDefaults: defaults)
        let source = RustScanEventSource(settings: settings)
        let selected = root.appendingPathComponent(tree.scanRoot).standardizedFileURL
        var discovered: [UUID: ScanArtifact] = [:]
        var completed: [UUID: (bytes: Int64, partial: Bool)] = [:]
        var warnings: [String] = []
        var terminal: [String] = []
        for await event in source.events(for: 1, target: selected) {
            #expect(terminal.isEmpty, "No event after Finished: \(tree.id)/\(run.id)")
            switch event {
            case .discovered(_, let artifact):
                #expect(discovered[artifact.id] == nil)
                discovered[artifact.id] = artifact
            case .completed(_, let id, let bytes, let partial):
                #expect(discovered[id] != nil)
                #expect(completed[id] == nil)
                completed[id] = (bytes, partial)
            case .warning(_, let message): warnings.append(message)
            case .finished(_, let result): terminal.append(tag(result))
            case .profile: break
            }
        }
        let expected = try run.expected.map { try #require(tree.rows[$0]) }
        #expect(terminal == [run.status], "\(tree.id)/\(run.id)")
        #expect(warnings.count == run.warnings, "\(tree.id)/\(run.id): \(warnings)")
        #expect(discovered.values.map(\.relativePath).sorted() == expected.map(\.path).sorted(),
                "\(tree.id)/\(run.id)")
        #expect(Set(completed.keys) == Set(discovered.keys))
        for row in expected {
            #expect(!row.nested, "The GUI baseline contains only outer roots")
            let artifact = try #require(discovered.values.first { $0.relativePath == row.path })
            #expect(artifact.language.lowercased() == row.language)
            let kind = switch artifact.kind {
            case .buildOutput: "build"
            case .cache: "cache"
            case .environment: "environment"
            case .testEnvironment: "test-environment"
            }
            #expect(kind == row.kind)
            // GUI policy emits its folded kind codes, not CLI bytecode/metadata.
            #expect(artifact.scannerKind == nil)
            let result = try #require(completed[artifact.id])
            #expect(!result.partial)
            let url = selected.appendingPathComponent(row.path).standardizedFileURL
            #expect(result.bytes == (try allocatedBytes(url)), "\(tree.id)/\(run.id)/\(row.path)")
            let context = try facts(url)
            let classification = try #require(ClassifyArtifactRoot().decide(context))
            #expect(settings.snapshot.descriptor(matching: context, classification: classification)?.id == row.filterID)
        }
    }

    @Test("Rust terminal codes retain their distinct GUI results")
    func terminalCodes() async throws {
        for mapping in try corpus().terminalMappings {
            let channel = ScanEventChannel(capacity: 8)
            let stream = channel.makeStream(onClose: {})
            let context = bridge(channel)
            context.finish(status: mapping.rustStatus)
            context.finish(status: mapping.rustStatus) // must not duplicate Finished
            var results: [String] = []
            for await event in stream {
                if case .finished(_, let result) = event { results.append(tag(result)) }
                else { Issue.record("Unexpected event for terminal-only fixture") }
            }
            #expect(results == [mapping.gui])
        }
    }

    @Test("Lossy path decoding warns and prevents GUI success")
    func invalidUTF8Baseline() async throws {
        let bytes = try corpus().invalidPathBytes
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let context = bridge(channel)
        bytes.withUnsafeBufferPointer { buffer in
            var event = BHScanEvent()
            event.event_type = 1
            event.artifact_id = 1
            event.path = buffer.baseAddress
            event.path_len = buffer.count
            withUnsafePointer(to: &event, context.handle)
        }
        context.finish(status: 0)
        var paths: [String] = []
        var warnings = 0
        var terminal: [String] = []
        for await event in stream {
            switch event {
            case .discovered(_, let artifact): paths.append(artifact.relativePath)
            case .warning: warnings += 1
            case .finished(_, let result): terminal.append(tag(result))
            default: break
            }
        }
        #expect(paths == [String(decoding: bytes, as: UTF8.self)])
        #expect(warnings == 1)
        #expect(terminal == ["failed"])
    }

    private func bridge(_ channel: ScanEventChannel) -> RustScanBridgeContext {
        RustScanBridgeContext(generation: 1, channel: channel,
            policy: BuildHunterScanPolicy(filters: SearchFilterSnapshot(
                excludedFilterIDs: [], catalog: SearchFilterCatalog.load())))
    }
    private func tag(_ result: ScanTerminalResult) -> String {
        switch result {
        case .completed: "completed"
        case .completedWithChanges: "completed-with-changes"
        case .stopped: "stopped"
        case .failed: "failed"
        }
    }
    private func facts(_ url: URL) throws -> ArtifactPolicyContext {
        let names = ["Cargo.toml", "pyproject.toml", "setup.py", "setup.cfg", "pyvenv.cfg"]
        func markers(_ directory: URL) -> Set<String> {
            Set(names.filter { name in
                var isDirectory: ObjCBool = false
                return FileManager.default.fileExists(atPath: directory.appendingPathComponent(name).path,
                                                     isDirectory: &isDirectory) && !isDirectory.boolValue
            })
        }
        let info = try metadata(url)
        return ArtifactPolicyContext(nodeName: url.lastPathComponent,
            isDirectory: info.st_mode & S_IFMT == S_IFDIR, isSymbolicLink: false,
            hasArtifactAncestor: false, ownMarkerFiles: markers(url),
            parentMarkerFiles: markers(url.deletingLastPathComponent()))
    }
    private func metadata(_ url: URL) throws -> stat {
        var info = stat()
        let status = url.path.withCString { lstat($0, &info) }
        try #require(status == 0, "lstat fixture oracle failed: \(url.path)")
        return info
    }
    private func allocatedBytes(_ url: URL) throws -> Int64 {
        let info = try metadata(url)
        if info.st_mode & S_IFMT == S_IFLNK { return 0 }
        try #require(info.st_blocks >= 0)
        var bytes = Int64(info.st_blocks) * 512
        if info.st_mode & S_IFMT == S_IFDIR {
            for child in try FileManager.default.contentsOfDirectory(at: url, includingPropertiesForKeys: nil) {
                bytes += try allocatedBytes(child)
            }
        }
        return bytes
    }
}
