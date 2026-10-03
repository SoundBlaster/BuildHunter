import Foundation
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
