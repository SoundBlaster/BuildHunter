import Foundation
import Testing
@testable import BuildHunter

@Suite("Rust bridge search filters")
@MainActor
struct SearchFilterIntegrationTests {
    @Test("Rule profiling is opt-in and a finished profile survives JSON export")
    func ruleProfiling() async throws {
        let fixture = try makeFixture()
        defer { try? FileManager.default.removeItem(at: fixture) }
        let suite = "BuildHunter.RuleProfiling.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let settings = SearchFilterSettings(userDefaults: defaults)
        #expect(!settings.ruleProfilingEnabled)
        settings.ruleProfilingEnabled = true
        #expect(SearchFilterSettings(userDefaults: defaults).ruleProfilingEnabled)
        let templates = TemplateLibrary(userDefaults: defaults)
        let source = RustScanEventSource(settings: settings, templates: templates)
        var copied: ScanRuleProfile?
        var catalog: ScanCatalogMetadata?
        var order: [String] = []
        for await event in source.events(for: 901, target: fixture) {
            switch event {
            case .catalog(_, let value, _): catalog = value
            case .ruleProfile(_, let profile): copied = profile; order.append("rules")
            case .finished: order.append("finished")
            default: break
            }
        }
        let profile = try #require(copied)
        #expect(profile.isSupported && profile.catalogDigest == catalog?.digest)
        #expect(profile.scanStatus == "completed")
        #expect(profile.rules.contains { $0.evaluations > 0 })
        #expect(order == ["rules", "finished"])
        let report = ScanReportExport(rootName: "Fixture", rootURL: fixture, status: .complete,
            rows: [], order: .table, tableOrder: [], warnings: [], excludedFilterIDs: [],
            elapsedSeconds: nil, appVersion: "test", catalogMetadata: catalog, ruleProfile: profile)
        let json = try #require(JSONSerialization.jsonObject(with: report.data(as: .json)) as? [String: Any])
        let exported = try #require(json["rule_profile"] as? [String: Any])
        #expect(exported["catalog_digest"] as? String == profile.catalogDigest)
        let ruleRows = try #require(exported["rules"] as? [[String: Any]])
        #expect(ruleRows.allSatisfy { $0.keys.contains("average_ns") })
        #expect(ruleRows.filter { ($0["evaluations"] as? Int) == 0 }.allSatisfy { $0["average_ns"] is NSNull })
        settings.ruleProfilingEnabled = false
        var unexpected = false
        for await event in source.events(for: 902, target: fixture) {
            if case .ruleProfile = event { unexpected = true }
        }
        #expect(!unexpected)
    }

    @Test("Catalog profile metadata is copied before callback return")
    func resourceMetadataOwnership() async throws {
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let context = RustScanBridgeContext(generation: 93, channel: channel)
        let expected = ScanResourceUsage(limits: ["queuedTasks": 12], limitReached: nil,
            peakPendingTasks: 9, peakListingEntries: 7, peakListingWeight: 1024, droppedProfileSamples: 17)
        var bytes = Array(try JSONEncoder().encode(expected))
        bytes.withUnsafeBufferPointer { metadata in
            var sample = BHScanProfileSample()
            sample.elapsed_us = 1_000_000
            withUnsafePointer(to: &sample) { profile in
                var event = BHCatalogScanEvent()
                event.version = 1
                event.base.event_type = 5
                event.base.profile = profile
                event.metadata = metadata.baseAddress
                event.metadata_len = metadata.count
                withUnsafePointer(to: &event, context.handleCatalog)
            }
        }
        bytes = Array(repeating: 0, count: bytes.count)
        context.finish(status: 0)
        var copied: ScanResourceUsage?
        for await event in stream {
            if case .profile(_, let sample) = event { copied = sample.resourceUsage }
        }
        #expect(copied == expected)
    }

    @Test("A running scan keeps its filter snapshot; the next scan uses changed settings")
    func immutableScanFilters() async throws {
        let fixture = try makeFixture()
        defer { try? FileManager.default.removeItem(at: fixture) }
        let suite = "BuildHunter.FilterIntegration.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        defaults.set(["future.framework"], forKey: SearchFilterSettings.persistenceKey)
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
        #expect(settings.excludedFilterIDs.contains("future.framework"))
        #expect(defaults.stringArray(forKey: SearchFilterSettings.persistenceKey)?.contains("future.framework") == true)
    }

    @Test("Catalog callback copies target identity and marks lossy path actions unsafe")
    func catalogIdentityAndLossyPaths() async throws {
        let metadata = try #require(ScanCatalogMetadata.basic())
        let channel = ScanEventChannel(capacity: 32)
        let stream = channel.makeStream(onClose: {})
        let context = RustScanBridgeContext(generation: 92, channel: channel)
        sendMetadata(try JSONEncoder().encode(metadata), context: context)
        let path = Array("safe/".utf8) + [0xFF] + Array("/cache".utf8)
        let target = Array("swift.build".utf8)
        path.withUnsafeBufferPointer { pathBytes in
            target.withUnsafeBufferPointer { targetBytes in
                var event = BHCatalogScanEvent()
                event.version = 1
                event.base.event_type = 1
                event.base.artifact_id = 7
                event.base.path = pathBytes.baseAddress
                event.base.path_len = pathBytes.count
                event.target_id = targetBytes.baseAddress
                event.target_id_len = targetBytes.count
                withUnsafePointer(to: &event, context.handleCatalog)
            }
        }
        context.finish(status: 0)
        var artifacts: [ScanArtifact] = []
        var catalog: ScanCatalogMetadata?
        var result: ScanTerminalResult?
        for await event in stream {
            switch event {
            case .catalog(_, let value, _): catalog = value
            case .discovered(_, let value): artifacts.append(value)
            case .finished(_, let value): result = value
            default: break
            }
        }
        #expect(catalog == metadata)
        let artifact = try #require(artifacts.first)
        #expect(artifact.targetID == "swift.build" && artifact.catalogDigest == metadata.digest)
        #expect(artifact.unsafePathPrefix == "safe/�")
        #expect(artifact.relativePath == "safe/�/cache")
        if case .failed = result {} else { Issue.record("Lossy bridge paths must make a completed scan incomplete") }
        let root = URL(filePath: "/tmp/catalog-fixture", directoryHint: .isDirectory)
        let prefixes: Set<String> = [try #require(artifact.unsafePathPrefix)]
        #expect(ArtifactFolderLocation.actionURL(root: root, relativePath: "safe", unsafePrefixes: prefixes) != nil)
        for unsafe in ["safe/�", "safe/�/cache", "safe/�/cache/child", "../escape", "/absolute"] {
            #expect(ArtifactFolderLocation.actionURL(root: root, relativePath: unsafe, unsafePrefixes: prefixes) == nil)
        }
        #expect(ArtifactFolderLocation.actionURL(root: root, relativePath: "safe/�other", unsafePrefixes: prefixes) != nil)
        #expect(ArtifactFolderLocation.actionURL(root: root, relativePath: "safe", unsafePrefixes: [""]) == nil)
        #expect(ArtifactFolderLocation.actionURL(root: root, relativePath: ".", unsafePrefixes: []) == root)
    }

    @Test("Malformed catalog metadata fails closed without accepting discoveries")
    func malformedCatalogs() async throws {
        let metadata = try #require(ScanCatalogMetadata.basic())
        var object = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(metadata)) as? [String: Any])
        var targets = try #require(object["targets"] as? [[String: Any]])
        targets.append(try #require(targets.first))
        object["targets"] = targets
        let channel = ScanEventChannel(capacity: 16)
        let stream = channel.makeStream(onClose: {})
        let context = RustScanBridgeContext(generation: 93, channel: channel)
        sendMetadata(try JSONSerialization.data(withJSONObject: object), context: context)
        // Invalid lengths must not enter Data(bytes:count:) or turn into an allocation.
        var byte: UInt8 = 0
        withUnsafePointer(to: &byte) { pointer in
            var event = BHCatalogScanEvent()
            event.version = 1
            event.base.event_type = 6
            event.metadata = pointer
            event.metadata_len = -1
            withUnsafePointer(to: &event, context.handleCatalog)
        }
        context.finish(status: 0)
        var warned = 0
        for await event in stream {
            switch event {
            case .catalog, .discovered: Issue.record("Malformed metadata must not be accepted")
            case .warning: warned += 1
            default: break
            }
        }
        #expect(warned == 2)
    }

    private func sendMetadata(_ bytes: Data, context: RustScanBridgeContext) {
        bytes.withUnsafeBytes { buffer in
            var event = BHCatalogScanEvent()
            event.version = 1
            event.base.event_type = 6
            event.metadata = buffer.bindMemory(to: UInt8.self).baseAddress
            event.metadata_len = buffer.count
            withUnsafePointer(to: &event, context.handleCatalog)
        }
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
