import Foundation
import Darwin
import Testing
@testable import BuildHunter

@Suite("Rust bridge search filters")
@MainActor
struct SearchFilterIntegrationTests {
    @Test("A running scan keeps its filter snapshot; the next scan uses changed settings")
    func immutableScanFilters() async throws {
        let fixture = try makeFixture()
        defer { try? FileManager.default.removeItem(at: fixture) }
        let suite = "BuildHunter.FilterIntegration.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let settings = SearchFilterSettings(userDefaults: defaults)
        let source = RustScanEventSource(settings: settings)
        let first = source.events(for: 1, target: fixture)
        settings.setEnabled(false, for: "swift.build")
        settings.setEnabled(false, for: "python.pytest-cache")
        let firstPaths = await discoveredPaths(first)
        #expect(firstPaths == [".build", "target", ".pytest_cache"])
        let secondPaths = await discoveredPaths(source.events(for: 2, target: fixture))
        #expect(secondPaths == [".build/Nested/__pycache__", "target"],
                "Excluded outer roots must remain traversable to find included child types")
        for descriptor in settings.snapshot.catalog {
            settings.setEnabled(false, for: descriptor.id)
        }
        let emptyPaths = await discoveredPaths(source.events(for: 3, target: fixture))
        #expect(emptyPaths.isEmpty, "Disabling all supported filters yields an empty report")
    }

    @Test("Profile warning count includes invalid UTF-8 paths reported by the Swift bridge")
    func profileIncludesBridgeWarnings() async throws {
        let root = FileManager.default.temporaryDirectory
            .appending(path: "BuildHunter-InvalidUTF8-\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }

        var invalidParent = Array(root.path.utf8) + [UInt8(ascii: "/"), 0xFF, 0]
        let parentResult = invalidParent.withUnsafeBufferPointer { bytes in
            Darwin.mkdir(UnsafeRawPointer(bytes.baseAddress!).assumingMemoryBound(to: CChar.self), 0o700)
        }
        #expect(parentResult == 0)

        var artifactPath = Array(root.path.utf8) + [UInt8(ascii: "/"), 0xFF]
        artifactPath += Array("/.build".utf8) + [0]
        let artifactResult = artifactPath.withUnsafeBufferPointer { bytes in
            Darwin.mkdir(UnsafeRawPointer(bytes.baseAddress!).assumingMemoryBound(to: CChar.self), 0o700)
        }
        #expect(artifactResult == 0)

        let suite = "BuildHunter.InvalidUTF8.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let source = RustScanEventSource(settings: SearchFilterSettings(userDefaults: defaults))
        var warningCount = 0
        var profiles: [ScanProfileSnapshot] = []
        for await event in source.events(for: 91, target: root) {
            switch event {
            case .warning: warningCount += 1
            case .profile(_, let sample): profiles.append(sample)
            default: break
            }
        }

        #expect(warningCount == 1)
        #expect(profiles.last?.warnings == 1)
    }

    private func discoveredPaths(_ stream: AsyncStream<ScanEvent>) async -> Set<String> {
        var paths = Set<String>()
        var completed = false
        for await event in stream {
            switch event {
            case .discovered(_, let artifact): paths.insert(artifact.relativePath)
            case .finished(_, .completed): completed = true
            case .finished(_, let result): Issue.record("Unexpected scanner result: \(result)")
            default: break
            }
        }
        #expect(completed, "Filtered scans must still emit the completed terminal event")
        return paths
    }

    private func makeFixture() throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("BuildHunter Filter Fixture \(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        do {
            try Data("[package]\nname = \"fixture\"\n".utf8).write(to: root.appendingPathComponent("Cargo.toml"))
            for path in [".build/Nested/__pycache__", "target", ".pytest_cache"] {
                let directory = root.appendingPathComponent(path, isDirectory: true)
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                try Data(repeating: 65, count: 64).write(to: directory.appendingPathComponent("fixture.dat"))
            }
            return root
        } catch {
            try? FileManager.default.removeItem(at: root)
            throw error
        }
    }
}
