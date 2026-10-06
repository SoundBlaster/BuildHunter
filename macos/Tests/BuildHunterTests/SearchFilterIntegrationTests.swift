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

    @Test("Profile warning count includes invalid UTF-8 paths reported by the Swift bridge")
    func profileIncludesBridgeWarnings() async throws {
        let suite = "BuildHunter.InvalidUTF8.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let policy = BuildHunterScanPolicy(filters: SearchFilterSettings(userDefaults: defaults).snapshot)
        let context = RustScanBridgeContext(generation: 91, channel: channel, policy: policy)

        let invalidPath: [UInt8] = [0xFF]
        invalidPath.withUnsafeBufferPointer { path in
            var artifactEvent = BHScanEvent()
            artifactEvent.event_type = 1
            artifactEvent.artifact_id = 1
            artifactEvent.path = path.baseAddress
            artifactEvent.path_len = path.count
            withUnsafePointer(to: &artifactEvent, context.handle)
        }

        var rustSample = BHScanProfileSample()
        rustSample.elapsed_us = 100_000
        withUnsafePointer(to: &rustSample) { sample in
            var profileEvent = BHScanEvent()
            profileEvent.event_type = 5
            profileEvent.profile = sample
            withUnsafePointer(to: &profileEvent, context.handle)
        }
        context.finish(status: 0)
        channel.finish()

        var bridgeWarningCount = 0
        var profiles: [ScanProfileSnapshot] = []
        for await event in stream {
            switch event {
            case .warning(_, let message) where message.contains("not valid UTF-8"):
                bridgeWarningCount += 1
            case .profile(_, let sample): profiles.append(sample)
            default: break
            }
        }

        #expect(bridgeWarningCount == 1)
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
