import Foundation
import Testing
@testable import BuildHunter

@Suite("Window scan lifecycle")
@MainActor
struct WindowScanModelTests {
    @Test("Warning actions use exact event paths, reject unsafe paths and reset with the report")
    func warningActionLocations() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        let root = URL(filePath: "/tmp/warning-root", directoryHint: .isDirectory)
        model.accept(target: root)
        let generation = model.generation
        source.yield(.warning(generation: generation, message: "Unrelated display text: details",
                              relativePath: "folder: with colon/subfolder"))
        source.yield(.warning(generation: generation, message: "fake/path: text alone is not a location"))
        source.yield(.warning(generation: generation, message: "Parent escape", relativePath: "../outside"))
        source.yield(.warning(generation: generation, message: "Absolute path", relativePath: "/outside"))
        source.yield(.warning(generation: generation, message: "Embedded null", relativePath: "bad\0name"))
        source.yield(.warning(generation: generation, message: "Selected root warning", relativePath: "."))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)
        await model.waitForCurrentScan()
        #expect(model.warningActionURL(at: 0)?.path == root.appending(path: "folder: with colon/subfolder").path)
        for index in [1, 2, 3, 4, -1, 99] { #expect(model.warningActionURL(at: index) == nil) }
        #expect(model.warningActionURL(at: 5) == root)
        #expect(model.warnings[0] == "Unrelated display text: details")
        model.rescan()
        #expect(model.warningActionURL(at: 0) == nil)
        model.apply(.warning(generation: generation, message: "Late", relativePath: "old/path"))
        #expect(model.warnings.isEmpty)
        model.stop()
    }

    @Test("Deduplicated warnings with conflicting origins permanently lose their path action")
    func duplicateWarningLocations() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.accept(target: URL(filePath: "/tmp/warning-root", directoryHint: .isDirectory))
        let generation = model.generation
        for path in ["first", "second", "first"] {
            source.yield(.warning(generation: generation, message: "Same formatted warning", relativePath: path))
        }
        for _ in 0..<2 {
            source.yield(.warning(generation: generation, message: "Same origin", relativePath: "stable"))
        }
        source.yield(.warning(generation: generation, message: "Summary collision", relativePath: "real"))
        source.yield(.warning(generation: generation, message: "Summary collision"))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)
        await model.waitForCurrentScan()
        #expect(model.warnings.count == 3)
        #expect(model.warningActionURL(at: 0) == nil)
        #expect(model.warningActionURL(at: 1)?.lastPathComponent == "stable")
        #expect(model.warningActionURL(at: 2) == nil)
    }

    @Test("Diagnostic warning actions retain exact paths and respect lossy root disabling")
    func diagnosticWarningLocation() async throws {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.accept(target: URL(filePath: "/tmp/warning-root", directoryHint: .isDirectory))
        let generation = model.generation
        let diagnostic = ScanDiagnostic(relativePath: "display-only", classification: .init(
            kind: "indeterminate", matchedTargetIDs: [], unknownTargetIDs: ["a.cache"], unavailable: []),
            message: "Diagnostic details")
        source.yield(.diagnostic(generation: generation, diagnostic: diagnostic, relativePath: "exact"))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)
        await model.waitForCurrentScan()
        #expect(model.warningActionURL(at: 0)?.lastPathComponent == "exact")
        model.rescan()
        let next = model.generation
        model.apply(.catalog(generation: next, metadata: try #require(ScanCatalogMetadata.basic()), rootPathIsExact: false))
        model.apply(.diagnostic(generation: next, diagnostic: diagnostic, relativePath: "exact"))
        #expect(model.warningActionURL(at: 0) == nil)
        model.stop()
    }

    @Test("Warning bridge copies exact paths and rejects lossy or empty paths", arguments: [true, false])
    func bridgeWarningLocations(rootPathIsExact: Bool) async {
        let channel = ScanEventChannel(capacity: 8)
        let bridge = RustScanBridgeContext(generation: 1, channel: channel, rootPathIsExact: rootPathIsExact)
        let paths: [[UInt8]] = [Array("folder: name".utf8), [0xFF], []]
        for bytes in paths {
            bytes.withUnsafeBufferPointer { path in
                Array("Permission denied".utf8).withUnsafeBufferPointer { message in
                    var event = BHScanEvent()
                    event.event_type = 3
                    event.path = path.baseAddress
                    event.path_len = path.count
                    event.message = message.baseAddress
                    event.message_len = message.count
                    withUnsafePointer(to: &event) { bridge.handle($0) }
                }
            }
        }
        channel.finish()
        var locations: [String?] = []
        while let event = await channel.next() {
            if case .warning(_, _, let path) = event { locations.append(path) }
        }
        #expect(locations.count == 3)
        #expect(locations[0] == (rootPathIsExact ? "folder: name" : nil))
        #expect(locations[1] == nil && locations[2] == nil)
    }

    @Test("Stop retains warning locations alongside the final rule profile")
    func stoppedProfileKeepsWarningLocations() async throws {
        let source = ControlledScanSource(captureStoppedProfiles: true)
        let model = WindowScanModel(source: source)
        let root = URL(filePath: "/tmp/warning-root", directoryHint: .isDirectory)
        model.accept(target: root)
        let generation = model.generation
        let catalog = try #require(ScanCatalogMetadata.basic())
        model.apply(.catalog(generation: generation, metadata: catalog, rootPathIsExact: true))
        model.apply(.warning(generation: generation, message: "Unreadable", relativePath: "folder: name"))
        model.stop()
        model.apply(.warning(generation: generation, message: "Late", relativePath: "late"))
        let expected = stoppedRuleProfile(catalog: catalog)
        source.finishStoppedProfile(generation: generation, profile: expected, catalog: catalog)
        await model.waitForCurrentScan()
        #expect(model.phase == .stopped && model.profile.ruleProfile == expected)
        #expect(model.warnings == ["Unreadable"])
        #expect(model.warningActionURL(at: 0)?.path == root.appending(path: "folder: name").path)
        #expect(model.warningActionURL(at: 1) == nil)
    }

    @Test("Stop retains final rule metrics and export even before catalog delivery", arguments: [false, true])
    func stopRetainsRuleProfile(catalogArrived: Bool) async throws {
        let source = ControlledScanSource(captureStoppedProfiles: true)
        let model = WindowScanModel(source: source)
        model.accept(target: URL(filePath: "/tmp/Alpha", directoryHint: .isDirectory))
        let generation = model.generation
        let catalog = try #require(ScanCatalogMetadata.basic())
        let artifact = makeArtifact()
        if catalogArrived { model.apply(.catalog(generation: generation, metadata: catalog, rootPathIsExact: true)) }
        model.apply(.discovered(generation: generation, artifact: artifact))
        model.stop()
        #expect(model.generation != generation && model.phase == .stopped)

        model.apply(.completed(generation: generation, artifactID: artifact.id, bytes: 99))
        model.apply(.warning(generation: generation, message: "late warning"))
        model.apply(.discovered(generation: generation, artifact: ScanArtifact(
            id: UUID(), relativePath: "late/target", language: "Rust", kind: .buildOutput)))
        model.apply(.finished(generation: generation, result: .completed))
        let expected = stoppedRuleProfile(catalog: catalog)
        source.finishStoppedProfile(generation: generation, profile: expected, catalog: catalog,
                                    rootPathIsExact: catalogArrived)
        await model.waitForCurrentScan()

        #expect(model.phase == .stopped && model.rows.count == 1 && model.rows[0].size == .partial(nil))
        #expect(model.warnings.isEmpty && model.profile.ruleProfile == expected)
        #expect(model.catalogMetadata == catalog)
        #expect(model.unsafePathPrefixes.contains("") == !catalogArrived)
        #expect((model.actionURL(for: ".") != nil) == catalogArrived)
        let report = ScanReportExport(rootName: "Alpha", rootURL: nil, status: .stopped,
            rows: model.rows, order: .table, tableOrder: [], warnings: model.warnings,
            excludedFilterIDs: model.excludedFilterIDs, elapsedSeconds: nil, appVersion: "test",
            catalogMetadata: model.catalogMetadata, ruleProfile: model.profile.ruleProfile)
        let json = try #require(JSONSerialization.jsonObject(with: report.data(as: .json)) as? [String: Any])
        let profile = try #require(json["rule_profile"] as? [String: Any])
        #expect(profile["scan_status"] as? String == "cancelled")
        #expect(profile["resolved_calls"] as? Int == 1)
    }

    @Test("A stopped scan's late rule metrics cannot attach to a rescan or replacement", arguments: [false, true])
    func stoppedRuleProfileCannotCrossReports(replaceTarget: Bool) async throws {
        let source = ControlledScanSource(captureStoppedProfiles: true)
        let model = WindowScanModel(source: source)
        let catalog = try #require(ScanCatalogMetadata.basic())
        model.acceptDemoTarget(named: "Alpha")
        let stoppedGeneration = model.generation
        model.apply(.catalog(generation: stoppedGeneration, metadata: catalog, rootPathIsExact: true))
        model.stop()
        if replaceTarget { model.acceptDemoTarget(named: "Beta") } else { model.rescan() }
        let activeGeneration = model.generation
        // Deliberately reuse catalog identity: freshness must also bind to the report.
        source.yield(.catalog(generation: activeGeneration, metadata: catalog, rootPathIsExact: true))
        source.yield(.finished(generation: activeGeneration, result: .completed))
        source.finish(generation: activeGeneration)
        await model.waitForCurrentScan()
        #expect(await source.nextStoppedProfileTermination() == stoppedGeneration)
        source.finishStoppedProfile(generation: stoppedGeneration, profile: stoppedRuleProfile(catalog: catalog), catalog: catalog)
        #expect(model.phase == .completed && model.profile.ruleProfile == nil)
    }

    @Test("Stopped rule metrics reject mismatched catalog or exclusions", arguments: [false, true])
    func stoppedRuleProfileChecksIdentity(wrongExclusions: Bool) async throws {
        let source = ControlledScanSource(captureStoppedProfiles: true)
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let catalog = try #require(ScanCatalogMetadata.basic())
        model.apply(.catalog(generation: generation, metadata: catalog, rootPathIsExact: true))
        model.stop()
        source.finishStoppedProfile(generation: generation, profile: stoppedRuleProfile(catalog: catalog,
            digest: wrongExclusions ? catalog.digest : String(repeating: "0", count: 64),
            exclusions: wrongExclusions ? ["swift.build"] : catalog.excludedTargetIDs), catalog: catalog)
        await model.waitForCurrentScan()
        #expect(model.phase == .stopped && model.profile.ruleProfile == nil)
    }

    private func stoppedRuleProfile(catalog: ScanCatalogMetadata, digest: String? = nil,
                                    exclusions: [String]? = nil) -> ScanRuleProfile {
        ScanRuleProfile(format: "buildhunter.rule-profile/1", catalogDigest: digest ?? catalog.digest,
            excludedTargetIDs: exclusions ?? catalog.excludedTargetIDs,
            timingScope: "rule_evaluation_and_outcome_wall_time", preliminaryCalls: 1, resolvedCalls: 1,
            classificationErrors: 0, scanStatus: "cancelled", rules: [])
    }

    @Test("Report freezes catalog identity, diagnostics and export fields, then resets them on rescan")
    func catalogReportLifecycle() async throws {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        let root = URL(filePath: "/tmp/catalog-root", directoryHint: .isDirectory)
        model.accept(target: root)
        let generation = model.generation
        let metadata = try #require(ScanCatalogMetadata.basic())
        let diagnostic = ScanDiagnostic(relativePath: "conflict", classification: .init(
            kind: "ambiguous", matchedTargetIDs: ["a.cache", "b.cache"], unknownTargetIDs: [], unavailable: []),
            message: "Conflicting plugin targets")
        let artifact = ScanArtifact(id: UUID(), relativePath: "lossy/�/.build", language: "Swift", kind: .buildOutput,
            targetID: "swift.build", catalogDigest: metadata.digest, unsafePathPrefix: "lossy/�")
        source.yield(.catalog(generation: generation, metadata: metadata, rootPathIsExact: true))
        source.yield(.diagnostic(generation: generation, diagnostic: diagnostic))
        source.yield(.discovered(generation: generation, artifact: artifact))
        source.yield(.completed(generation: generation, artifactID: artifact.id, bytes: 123))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)
        await model.waitForCurrentScan()
        #expect(model.phase == .incomplete && model.diagnostics == [diagnostic])
        #expect(model.catalogMetadata == metadata && model.rows.first?.targetID == "swift.build")
        #expect(model.actionURL(for: "lossy") != nil)
        #expect(model.actionURL(for: "lossy/�/.build") == nil)
        let report = ScanReportExport(rootName: "catalog-root", rootURL: root, status: .incomplete,
            rows: model.rows, order: .table, tableOrder: [], warnings: model.warnings,
            excludedFilterIDs: model.excludedFilterIDs, elapsedSeconds: nil, appVersion: "test",
            catalogMetadata: model.catalogMetadata, diagnostics: model.diagnostics)
        let json = try #require(JSONSerialization.jsonObject(with: report.data(as: .json)) as? [String: Any])
        #expect(json["schema_version"] as? Int == 2)
        #expect((json["catalog"] as? [String: Any])?["digest"] as? String == metadata.digest)
        let rows = try #require(json["artifacts"] as? [[String: Any]])
        #expect(rows.first?["target_id"] as? String == "swift.build")
        #expect(rows.first?["path_is_exact"] as? Bool == false)
        #expect((json["diagnostics"] as? [[String: Any]])?.count == 1)
        #expect(report.csv().contains("Target ID,Catalog Digest,Path Is Exact"))
        model.rescan()
        #expect(model.catalogMetadata == nil && model.diagnostics.isEmpty && model.unsafePathPrefixes.isEmpty)
        #expect(model.rows.isEmpty)
        // Late metadata from the previous scan must not replace the new report.
        model.apply(.catalog(generation: generation, metadata: metadata, rootPathIsExact: false))
        #expect(model.catalogMetadata == nil)
        model.stop()
    }

    @Test("Demo source completes without manufacturing access warnings")
    func demoCompletesNormally() async {
        let model = WindowScanModel(source: DemoScanSource())
        model.acceptDemoTarget(named: "Demo")
        await model.waitForCurrentScan()
        #expect(model.rows.count == 3)
        #expect(model.phase == .completed)
        #expect(model.warnings.isEmpty)
    }

    @Test("Rust FFI scans selected folders and applies SpecificationCore classification")
    func rustScannerStreamsMeasuredArtifacts() async throws {
        let root = FileManager.default.temporaryDirectory
            .appending(path: "BuildHunter-Rust-\(UUID().uuidString)", directoryHint: .isDirectory)
        let swiftBuild = root.appending(path: "Package/.build", directoryHint: .isDirectory)
        let nestedCache = swiftBuild.appending(path: "__pycache__", directoryHint: .isDirectory)
        let rustTarget = root.appending(path: "Service/target", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: nestedCache, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: rustTarget, withIntermediateDirectories: true)
        try Data("let fixture = true".utf8).write(to: swiftBuild.appending(path: "output.o"))
        try Data("[package]".utf8).write(to: root.appending(path: "Service/Cargo.toml"))
        defer { try? FileManager.default.removeItem(at: root) }

        let source = RustScanEventSource()
        let stream = source.events(for: 41, target: root)
        var artifacts: [ScanArtifact] = []
        var profiles: [ScanProfileSnapshot] = []
        var terminal: ScanTerminalResult?
        for await event in stream {
            switch event {
            case .discovered(_, let artifact): artifacts.append(artifact)
            case .profile(_, let sample): profiles.append(sample)
            case .finished(_, let result): terminal = result
            default: break
            }
        }

        #expect(terminal == .completed)
        #expect(profiles.count >= 2)
        #expect(profiles.last?.pendingTasks == 0)
        #expect(profiles.last?.artifacts == 2)
        #expect((profiles.last?.measuredBytes ?? 0) > 0)
        #expect(profiles.last?.cpuUserNanoseconds != nil)
        #expect((profiles.last?.residentBytes ?? 0) > 0)
        #expect(artifacts.map(\.relativePath).sorted() == ["Package/.build", "Service/target"])
        #expect(artifacts.map(\.language).sorted() == ["Rust", "Swift"])
    }

    @Test("A selected artifact root is displayed as dot")
    func rustScannerDisplaysSelectedArtifactRoot() async throws {
        let parent = FileManager.default.temporaryDirectory
            .appending(path: "BuildHunter-SelectedRoot-\(UUID().uuidString)", directoryHint: .isDirectory)
        let selectedRoot = parent.appending(path: ".build", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: selectedRoot, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: parent) }

        let source = RustScanEventSource()
        let stream = source.events(for: 42, target: selectedRoot)
        var discoveredPath: String?
        var terminal: ScanTerminalResult?
        for await event in stream {
            switch event {
            case .discovered(_, let artifact): discoveredPath = artifact.relativePath
            case .finished(_, let result): terminal = result
            default: break
            }
        }

        #expect(discoveredPath == ".")
        #expect(terminal == .completed)
    }

    @Test("A continuously occupied event queue releases delivered payloads", arguments: [1, 2, 8])
    func occupiedQueueReleasesDeliveredEvents(capacity: Int) async throws {
        let channel = ScanEventChannel(capacity: capacity)
        for index in 0..<capacity {
            #expect(channel.send(.warning(generation: 1, message: "\(index)")))
        }
        for index in 0..<10_000 {
            let event = try #require(await channel.next())
            guard case .warning(_, let message, _) = event else {
                Issue.record("Expected a warning event")
                return
            }
            #expect(message == "\(index)", "FIFO must survive repeated wraparound")
            if index == 9_999 {
                #expect(channel.retainedEventCount == capacity - 1,
                        "Delivered payloads must be released even while the queue stays occupied")
            }
            #expect(channel.send(.warning(generation: 1, message: "\(index + capacity)")))
        }
        #expect(channel.bufferedCount == capacity)
        #expect(channel.retainedEventCount == capacity)
        channel.finish()
        for index in 10_000..<(10_000 + capacity) {
            let event = try #require(await channel.next())
            guard case .warning(_, let message, _) = event else {
                Issue.record("Expected a buffered warning event")
                return
            }
            #expect(message == "\(index)")
        }
        #expect(await channel.next() == nil)
        #expect(channel.retainedEventCount == 0)
    }

    @Test("Closing a partly consumed queue releases its remaining payloads")
    func closingQueueReleasesPayloads() async throws {
        let channel = ScanEventChannel(capacity: 2)
        channel.send(.warning(generation: 1, message: "first"))
        channel.send(.warning(generation: 1, message: "second"))
        _ = try #require(await channel.next())
        channel.close()
        #expect(channel.bufferedCount == 0)
        #expect(channel.retainedEventCount == 0)
        #expect(!channel.send(.warning(generation: 1, message: "closed")))
        #expect(await channel.next() == nil)
    }

    @Test("Rust events wait for buffer space instead of being dropped")
    func rustEventsApplyBackpressure() async throws {
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let artifacts = (0..<1_000).map { index in
            ScanArtifact(id: UUID(), relativePath: "Project\(index)/.build", language: "Swift", kind: .buildOutput)
        }
        let producer = ProducerProbe()
        // The Rust scanner calls back on its own thread and can outrun the consumer.
        Thread {
            for artifact in artifacts {
                channel.send(.discovered(generation: 1, artifact: artifact))
                channel.send(.completed(generation: 1, artifactID: artifact.id, bytes: 4_096))
            }
            channel.send(.finished(generation: 1, result: .completed))
            channel.finish()
            producer.markFinished()
        }.start()
        try await Task.sleep(for: .milliseconds(50))
        #expect(channel.bufferedCount == 8, "the producer waits once the buffer is full")
        #expect(!producer.isFinished)

        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Busy consumer")
        await model.waitForCurrentScan()

        #expect(model.rows.count == artifacts.count)
        #expect(model.rows.allSatisfy { $0.size == .measured(4_096) })
        #expect(model.warnings.isEmpty)
        #expect(model.phase == .completed)
    }

    @Test("Abandoning the event stream wakes a producer blocked on a full buffer")
    func abandonedStreamReleasesTheProducer() async throws {
        let channel = ScanEventChannel(capacity: 2)
        let producer = ProducerProbe()
        var stream: AsyncStream<ScanEvent>? = channel.makeStream(onClose: { producer.markClosed() })
        Thread {
            var accepted = 0
            for index in 0..<10 where channel.send(.warning(generation: 1, message: "\(index)")) {
                accepted += 1
            }
            producer.markFinished(accepted: accepted)
        }.start()
        try await Task.sleep(for: .milliseconds(50))
        #expect(!producer.isFinished)
        #expect(stream != nil)

        stream = nil
        for _ in 0..<200 where !producer.isFinished {
            try await Task.sleep(for: .milliseconds(10))
        }

        #expect(producer.isFinished)
        #expect(producer.isClosed)
        #expect(producer.accepted == 2)
    }

    @Test("Repeated warnings keep their first-seen order without duplicates")
    func repeatedWarningsAreDeduplicatedInOrder() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Unreadable home")
        let generation = model.generation
        let messages = (0..<1_000).map { "Library/Private/\($0): Permission denied" }

        for message in messages + messages.reversed() {
            model.apply(.warning(generation: generation, message: message))
        }

        #expect(model.warnings == messages)
        model.rescan()
        model.apply(.warning(generation: model.generation, message: messages[0]))
        #expect(model.warnings == [messages[0]], "a new report forgets earlier warnings")
    }

    @Test("Rust artifact IDs map to the same stable UUIDs, built from bytes")
    func stableIDsAreBuiltFromBytes() {
        #expect(stableID(0).uuidString == "00000000-0000-4000-8000-000000000000")
        #expect(stableID(0xABCDEF).uuidString == "00000000-0000-4000-8000-000000ABCDEF")
        #expect(stableID(0xFFFF_FFFF_FFFF).uuidString == "00000000-0000-4000-8000-FFFFFFFFFFFF")
        #expect(stableID(0x1_0000_0000_0001) == stableID(1), "only the low 48 bits are kept")
        #expect(Set((0..<UInt64(10_000)).map(stableID)).count == 10_000)
    }

    @Test("Terminal event rejects later events from the same generation")
    func terminalEventRejectsLateUpdates() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let artifact = makeArtifact()

        model.apply(.discovered(generation: generation, artifact: artifact))
        model.apply(.completed(generation: generation, artifactID: artifact.id, bytes: 20))
        model.apply(.finished(generation: generation, result: .completed))
        model.apply(.warning(generation: generation, message: "late warning"))
        model.apply(.completed(generation: generation, artifactID: artifact.id, bytes: 99))

        #expect(model.rows[0].size == .measured(20))
        #expect(model.warnings.isEmpty)
        #expect(model.phase == .completed)
    }

    @Test("An event stream without a terminal event becomes incomplete")
    func exhaustedStreamBecomesIncomplete() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        source.finish(generation: model.generation)

        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(model.warnings == ["Event stream ended before a terminal result."])
    }

    @Test("Stop remains distinct from completion and leaves unknown size unknown")
    func stopPreservesPartialUnknownSize() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        model.apply(.discovered(generation: generation, artifact: makeArtifact()))

        model.stop()
        await model.waitForCurrentScan()
        let terminatedGeneration = await source.nextTermination()

        #expect(model.phase == .stopped)
        #expect(model.rows.count == 1)
        #expect(model.rows[0].size == .partial(nil))
        #expect(terminatedGeneration == generation)
    }

    @Test("A completed stream applies streamed events and reaches the completed phase")
    func injectedSourceDeliversEvents() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let artifact = makeArtifact()
        source.yield(.discovered(generation: generation, artifact: artifact))
        source.yield(.completed(generation: generation, artifactID: artifact.id, bytes: 20))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)

        await model.waitForCurrentScan()

        #expect(model.rows == [ScanRow(id: artifact.id, relativePath: artifact.relativePath,
                                      language: artifact.language, kind: artifact.kind, size: .measured(20))])
        #expect(model.phase == .completed)
    }

    @Test("A warning makes the terminal report incomplete")
    func warningsMarkReportIncomplete() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        source.yield(.warning(generation: generation, message: "fixture warning"))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)

        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(model.warnings == ["fixture warning"])
    }

    @Test("A partial measurement keeps the report incomplete even without a warning")
    func partialMeasurementMarksReportIncomplete() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Partial")
        let generation = model.generation
        let artifact = makeArtifact()
        source.yield(.discovered(generation: generation, artifact: artifact))
        source.yield(.completed(generation: generation, artifactID: artifact.id, bytes: 20, partial: true))
        source.yield(.finished(generation: generation, result: .completed))
        source.finish(generation: generation)

        await model.waitForCurrentScan()

        #expect(model.rows.first?.size == .partial(20))
        #expect(model.phase == .incomplete)
    }

    @Test("Incomplete or unknown Rust terminal statuses cannot report success", arguments: [2, 99])
    func unsuccessfulRustStatusMarksReportIncomplete(status: UInt32) async {
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let bridge = RustScanBridgeContext(generation: 1, channel: channel,
                                           policy: BuildHunterScanPolicy(filters: SearchFilterSnapshot(
                                            excludedFilterIDs: [], catalog: [])))
        bridge.finish(status: status)

        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Incomplete")
        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(!model.warnings.isEmpty)
    }

    @Test("Rust changed-tree completion keeps notices and partial rows without failing the scan")
    func changedTreeCompletionKeepsPartialRows() async {
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let bridge = RustScanBridgeContext(generation: 1, channel: channel,
                                           policy: BuildHunterScanPolicy(filters: SearchFilterSnapshot(
                                            excludedFilterIDs: [], catalog: [])))
        let artifact = makeArtifact()
        channel.send(.discovered(generation: 1, artifact: artifact))
        channel.send(.completed(generation: 1, artifactID: artifact.id, bytes: 20, partial: true))
        channel.send(.warning(generation: 1, message: "Changed during scan: temporary file disappeared"))
        bridge.finish(status: 4)
        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Changing tree")
        await model.waitForCurrentScan()

        #expect(model.phase == .completed)
        #expect(model.rows.first?.size == .partial(20))
        #expect(model.warnings.count == 1)
    }

    @Test("Bridge warnings keep both ordinary and changed-tree reports incomplete",
          arguments: [UInt32(0), 4], [UInt32(1), 99])
    func bridgeWarningsPreventSuccessfulCompletion(status: UInt32, eventType: UInt32) async {
        let channel = ScanEventChannel(capacity: 8)
        let stream = channel.makeStream(onClose: {})
        let bridge = RustScanBridgeContext(generation: 1, channel: channel,
            policy: BuildHunterScanPolicy(filters: SearchFilterSnapshot(excludedFilterIDs: [], catalog: [])))
        let invalidPath: [UInt8] = [0xFF]
        invalidPath.withUnsafeBufferPointer { path in
            var event = BHScanEvent()
            event.event_type = eventType
            event.artifact_id = 1
            event.path = path.baseAddress
            event.path_len = path.count
            withUnsafePointer(to: &event, bridge.handle)
        }
        // Give any discovered row a complete measurement: the warning, rather
        // than an unfinished row, must be what prevents successful completion.
        var measured = BHScanEvent()
        measured.event_type = 2
        measured.artifact_id = 1
        measured.bytes = 20
        withUnsafePointer(to: &measured, bridge.handle)
        channel.send(.warning(generation: 1, message: "Changed during scan: temporary file disappeared"))
        bridge.finish(status: status)
        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Bridge warning")
        await model.waitForCurrentScan()

        #expect(model.phase == .incomplete)
        #expect(model.warnings.contains { $0.contains("not valid UTF-8") || $0.contains("unknown event") })
        #expect(model.rows.allSatisfy { $0.size == .measured(20) })
    }

    @Test("Duplicate and out-of-order events do not duplicate rows or invent a size")
    func duplicateAndOutOfOrderEventsAreSafe() {
        let model = WindowScanModel(source: ControlledScanSource())
        model.acceptDemoTarget(named: "Alpha")
        let generation = model.generation
        let artifact = makeArtifact()

        model.apply(.completed(generation: generation, artifactID: artifact.id, bytes: 20))
        model.apply(.discovered(generation: generation, artifact: artifact))
        model.apply(.discovered(generation: generation, artifact: artifact))

        #expect(model.rows.count == 1)
        #expect(model.rows[0].id == artifact.id)
        #expect(model.rows[0].size == .measuring)
    }

    @Test("Rescan clears the old report before accepting new events")
    func rescanClearsRowsAndWarnings() {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "Alpha")
        let firstGeneration = model.generation
        model.apply(.discovered(generation: firstGeneration, artifact: makeArtifact()))
        model.apply(.warning(generation: firstGeneration, message: "old warning"))

        model.rescan()
        model.apply(.warning(generation: firstGeneration, message: "late old warning"))

        #expect(model.rows.isEmpty)
        #expect(model.warnings.isEmpty)
        #expect(model.generation == firstGeneration + 1)
        #expect(model.targetName == "Alpha")
        #expect(model.phase == .scanning)
    }

    @Test("Artifact IDs can be reused after rescan or target replacement", arguments: [false, true])
    func reusedArtifactIDAfterRestart(replacingTarget: Bool) {
        let model = WindowScanModel(source: ControlledScanSource())
        let artifact = makeArtifact()
        model.acceptDemoTarget(named: "First")
        model.apply(.discovered(generation: model.generation, artifact: artifact))
        model.apply(.completed(generation: model.generation, artifactID: artifact.id, bytes: 20))

        if replacingTarget {
            model.acceptDemoTarget(named: "Second")
        } else {
            model.rescan()
        }
        model.apply(.discovered(generation: model.generation, artifact: artifact))
        model.apply(.completed(generation: model.generation, artifactID: artifact.id, bytes: 40))

        #expect(model.rows.count == 1)
        #expect(model.rows.first?.size == .measured(40))
        model.stop()
    }

    @Test("Replacing a target cancels its stream and isolates the new report")
    func replacementCancelsOldGeneration() async {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.acceptDemoTarget(named: "First")
        let oldGeneration = model.generation
        model.apply(.discovered(generation: oldGeneration, artifact: makeArtifact()))
        model.apply(.warning(generation: oldGeneration, message: "old warning"))

        model.acceptDemoTarget(named: "Second")
        model.apply(.warning(generation: oldGeneration, message: "late warning"))
        let terminatedGeneration = await source.nextTermination()

        #expect(terminatedGeneration == oldGeneration)
        #expect(model.targetName == "Second")
        #expect(model.rows.isEmpty)
        #expect(model.warnings.isEmpty)
        #expect(model.phase == .scanning)
    }

    @Test("Window models keep target and result state independent")
    func windowsAreIsolated() {
        let first = WindowScanModel(source: ControlledScanSource())
        let second = WindowScanModel(source: ControlledScanSource())
        first.acceptDemoTarget(named: "Alpha")
        second.acceptDemoTarget(named: "Beta")
        first.apply(.discovered(generation: first.generation, artifact: makeArtifact()))

        #expect(first.rows.count == 1)
        #expect(second.rows.isEmpty)
        #expect(first.targetName == "Alpha")
        #expect(second.targetName == "Beta")
    }

#if DEBUG
    @Test("Mock scenarios discard the previous real target and never rescan that folder")
    func mockScenarioClearsRealTarget() {
        let source = ControlledScanSource()
        let model = WindowScanModel(source: source)
        model.accept(target: URL(fileURLWithPath: "/real/project", isDirectory: true))

        model.showMockState(.results)
        #expect(model.targetURL == nil)

        model.rescan()
        #expect(source.lastTarget == nil)
        model.stop()
    }

    @Test("Debug mock scenarios render empty, streaming, completed, stopped, and incomplete states")
    func mockScenarios() {
        let model = WindowScanModel(source: ControlledScanSource())

        model.showMockState(.empty)
        #expect(model.targetName == nil)
        #expect(model.phase == .idle)
        #expect(model.rows.isEmpty)

        model.showMockState(.scanning)
        #expect(model.phase == .scanning)
        #expect(model.rows.count == 3)
        #expect(model.rows.allSatisfy { $0.size == .measuring })

        model.showMockState(.results)
        #expect(model.phase == .completed)
        #expect(model.rows.allSatisfy { if case .measured = $0.size { true } else { false } })

        model.showMockState(.stopped)
        #expect(model.phase == .stopped)
        #expect(model.rows.filter { $0.size == .partial(nil) }.count == 2)
        #expect(model.rows.filter { if case .measured = $0.size { true } else { false } }.count == 1)

        model.showMockState(.incomplete)
        #expect(model.phase == .incomplete)
        #expect(model.warnings.count == 1)
        #expect(model.rows.filter { $0.size == .partial(nil) }.count == 1)
    }
#endif

    private func makeArtifact() -> ScanArtifact {
        ScanArtifact(id: UUID(uuidString: "A0000000-0000-4000-8000-000000000001")!,
                     relativePath: "DemoFixture/Alpha/.build",
                     language: "Swift", kind: .buildOutput)
    }
}

@MainActor
private final class ControlledScanSource: ScanEventSource {
    private(set) var lastTarget: URL?
    private var continuations: [UInt64: AsyncStream<ScanEvent>.Continuation] = [:]
    private let terminationEvents: AsyncStream<UInt64>
    private let terminationContinuation: AsyncStream<UInt64>.Continuation
    private let captureStoppedProfiles: Bool
    private var stoppedProfiles: [UInt64: AsyncStream<ScanRuleProfileSnapshot>.Continuation] = [:]
    private let stoppedProfileTerminations: AsyncStream<UInt64>
    private let stoppedProfileTerminationContinuation: AsyncStream<UInt64>.Continuation

    init(captureStoppedProfiles: Bool = false) {
        self.captureStoppedProfiles = captureStoppedProfiles
        (terminationEvents, terminationContinuation) = AsyncStream.makeStream(of: UInt64.self)
        (stoppedProfileTerminations, stoppedProfileTerminationContinuation) = AsyncStream.makeStream(of: UInt64.self)
    }

    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        lastTarget = target
        let (stream, continuation) = AsyncStream.makeStream(of: ScanEvent.self)
        continuations[generation] = continuation
        let terminationContinuation = self.terminationContinuation
        continuation.onTermination = { @Sendable _ in
            terminationContinuation.yield(generation)
        }
        return stream
    }

    func cancel(generation: UInt64) {}

    func stop(generation: UInt64) -> AsyncStream<ScanRuleProfileSnapshot> {
        cancel(generation: generation)
        guard captureStoppedProfiles else { return AsyncStream { $0.finish() } }
        let (stream, continuation) = AsyncStream.makeStream(of: ScanRuleProfileSnapshot.self, bufferingPolicy: .bufferingNewest(1))
        let termination = stoppedProfileTerminationContinuation
        continuation.onTermination = { @Sendable _ in termination.yield(generation) }
        stoppedProfiles[generation] = continuation
        return stream
    }

    func nextStoppedProfileTermination() async -> UInt64? {
        var iterator = stoppedProfileTerminations.makeAsyncIterator()
        return await iterator.next()
    }

    func finishStoppedProfile(generation: UInt64, profile: ScanRuleProfile, catalog: ScanCatalogMetadata,
                              rootPathIsExact: Bool = true) {
        stoppedProfiles[generation]?.yield(ScanRuleProfileSnapshot(catalog: catalog, rootPathIsExact: rootPathIsExact, profile: profile))
        stoppedProfiles.removeValue(forKey: generation)?.finish()
    }

    func yield(_ event: ScanEvent) {
        continuations[event.generation]?.yield(event)
    }

    func finish(generation: UInt64) {
        continuations[generation]?.finish()
    }

    func nextTermination() async -> UInt64? {
        var iterator = terminationEvents.makeAsyncIterator()
        return await iterator.next()
    }
}

private final class ProducerProbe: @unchecked Sendable {
    private let lock = NSLock()
    private var finished = false
    private var closed = false
    private var acceptedCount = 0

    var isFinished: Bool { lock.withLock { finished } }
    var isClosed: Bool { lock.withLock { closed } }
    var accepted: Int { lock.withLock { acceptedCount } }

    func markFinished(accepted: Int = 0) {
        lock.withLock {
            finished = true
            acceptedCount = accepted
        }
    }

    func markClosed() {
        lock.withLock { closed = true }
    }
}

@MainActor
private struct SingleStreamScanSource: ScanEventSource {
    let stream: AsyncStream<ScanEvent>

    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        stream
    }

    func cancel(generation: UInt64) {}
}

@Suite("Artifact table sorting")
@MainActor
struct ScanTableModelTests {
    @Test("Every header sorts both ways; sizes use bytes and keep unknown values last")
    func columnsAndUnknownSizes() {
        let rows = [
            ScanRow(id: UUID(), relativePath: "Z/target", language: "Rust", kind: .buildOutput, size: .measured(20)),
            ScanRow(id: UUID(), relativePath: "A/.build", language: "Swift", kind: .buildOutput, size: .partial(100)),
            ScanRow(id: UUID(), relativePath: "B/cache", language: "Python", kind: .cache, size: .measuring),
            ScanRow(id: UUID(), relativePath: "C/cache", language: "Python", kind: .cache, size: .partial(nil))
        ]
        for column in [ScanRowComparator.Column.path, .language, .kind] {
            let forward = ScanRowComparator(column: column)
            let reverse = ScanRowComparator(column: column, order: .reverse)
            for first in rows {
                for second in rows where forward.compare(first, second) != .orderedSame {
                    #expect(forward.compare(first, second) != reverse.compare(first, second))
                }
            }
        }
        #expect(ScanRowComparator.sorted(rows, by: [.init(column: .size)]).map(\.relativePath)
                == ["Z/target", "A/.build", "B/cache", "C/cache"])
        #expect(ScanRowComparator.sorted(rows, by: [.init(column: .size, order: .reverse)]).map(\.relativePath)
                == ["A/.build", "Z/target", "B/cache", "C/cache"])
        #expect(ScanRowComparator.sorted(Array(rows.reversed()), by: [.init(column: .kind)])
                == ScanRowComparator.sorted(rows, by: [.init(column: .kind)]))
    }

    @Test("Streaming refreshes merge new rows into the same order as a full sort")
    func streamingRefreshIsIncremental() async {
        let scan = WindowScanModel(source: TableIdleSource())
        let table = ScanTableModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        let artifacts = (0..<11_000).shuffled().map { index in
            ScanArtifact(id: UUID(), relativePath: "Projects/group\(index % 97)/Project\(index)/.build",
                         language: index.isMultiple(of: 3) ? "Rust" : "Swift", kind: .buildOutput)
        }
        for artifact in artifacts.prefix(10_000) {
            scan.apply(.discovered(generation: scan.generation, artifact: artifact))
        }
        await table.refresh(from: scan)

        for round in 0..<40 {
            for artifact in artifacts[(10_000 + round * 25)..<(10_000 + (round + 1) * 25)] {
                scan.apply(.discovered(generation: scan.generation, artifact: artifact))
            }
            for artifact in artifacts[(round * 25)..<((round + 1) * 25)] {
                scan.apply(.completed(generation: scan.generation, artifactID: artifact.id, bytes: Int64(round)))
            }
            await table.refresh(from: scan)
        }

        #expect(table.rows == ScanRowComparator.sorted(scan.rows, by: table.sortOrder))
    }

    @Test("Sorting during discovery does not break the scanner's row index or late measurements")
    func streamingSort() async {
        let scan = WindowScanModel(source: TableIdleSource())
        let table = ScanTableModel()
        scan.acceptDemoTarget(named: "Fixture")
        defer { scan.stop() }
        let entries = ["Z/target", "A/.build"].map {
            ScanArtifact(id: UUID(), relativePath: $0, language: "Swift", kind: .buildOutput)
        }
        for entry in entries { scan.apply(.discovered(generation: scan.generation, artifact: entry)) }
        await table.refresh(from: scan)
        #expect(table.rows.map(\.relativePath) == ["A/.build", "Z/target"])
        table.sortOrder = [.init(column: .size, order: .reverse)]
        scan.apply(.completed(generation: scan.generation, artifactID: entries[0].id, bytes: 100))
        await table.refresh(from: scan)
        #expect(table.rows.first?.id == entries[0].id)
        scan.apply(.completed(generation: scan.generation, artifactID: entries[1].id, bytes: 200))
        await table.refresh(from: scan)
        #expect(table.rows.map(\.size) == [.measured(200), .measured(100)])
        #expect(scan.rows.map(\.size) == [.measured(100), .measured(200)])
        table.sortOrder = [.init(column: .path, order: .reverse)]
        await table.refresh(from: scan)
        #expect(table.rows.first?.id == entries[0].id)
        scan.rescan()
        await table.refresh(from: scan)
        #expect(table.rows.isEmpty)
    }
}

private struct TableIdleSource: ScanEventSource {
    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> { AsyncStream { _ in } }
    func cancel(generation: UInt64) {}
}

@Suite("Scan profile metrics")
@MainActor
struct ScanProfileTests {
    private func resourceSample(_ seconds: UInt64, user: UInt64?, system: UInt64?, rss: UInt64?) -> ScanProfileSnapshot {
        ScanProfileSnapshot(elapsedMicroseconds: seconds * 1_000_000, entries: seconds,
            directories: 0, measuredBytes: 0, artifacts: 0, warnings: 0, pendingTasks: 0,
            cpuUserNanoseconds: user.map { $0 * 1_000_000_000 },
            cpuSystemNanoseconds: system.map { $0 * 1_000_000_000 }, residentBytes: rss)
    }

    @Test("Omitted producer samples break resource series and RAM integration")
    func omittedResourceSamples() {
        var history = ScanProfileHistory()
        history.record(resourceSample(0, user: 0, system: 0, rss: 100))
        history.record(resourceSample(1, user: 1, system: 0, rss: 200))
        var after = resourceSample(4, user: 4, system: 0, rss: 800)
        after.resourceUsage = ScanResourceUsage(limits: [:], limitReached: nil,
            peakPendingTasks: 0, peakListingEntries: 0, peakListingWeight: 0, droppedProfileSamples: 2)
        history.record(after)
        #expect(history.points.last?.cpuPercent == nil)
        #expect(history.points.first?.cpuSeries != history.points.last?.cpuSeries)
        #expect(history.points.first?.memorySeries != history.points.last?.memorySeries)
        #expect(history.residentObservedSeconds == 1)
        #expect(history.averageResidentBytes == 150)
        #expect(history.latest?.resourceUsage?.droppedProfileSamples == 2)
    }

    @Test("Process CPU can exceed 100 percent; RSS average is weighted by time")
    func processResources() {
        var history = ScanProfileHistory()
        history.record(resourceSample(0, user: 0, system: 0, rss: 100))
        history.record(resourceSample(1, user: 3, system: 1, rss: 300))
        history.record(resourceSample(3, user: 4, system: 2, rss: 100))
        #expect(history.points.map(\.cpuPercent) == [400, 100])
        #expect(history.averageCPUPercent == 200)
        #expect(history.peakCPUPercent == 400)
        #expect(history.initialResidentBytes == 100)
        #expect(history.peakResidentBytes == 300)
        #expect(history.averageResidentBytes == 200)
        #expect(history.residentObservedSeconds == 3)
    }

    @Test("Missing resources are gaps; counter regressions cannot underflow")
    func resourceGaps() {
        var history = ScanProfileHistory()
        history.record(resourceSample(0, user: 0, system: 0, rss: 100))
        history.record(resourceSample(1, user: 4, system: 1, rss: 300))
        history.record(resourceSample(2, user: 3, system: 2, rss: nil))
        #expect(history.points.last?.cpuPercent == nil)
        #expect(history.averageCPUPercent == nil)
        history.record(resourceSample(3, user: 6, system: 3, rss: 200))
        history.record(resourceSample(4, user: 7, system: 4, rss: 400))
        #expect(history.peakCPUPercent == 500)
        #expect(history.averageResidentBytes == 250)
        #expect(history.residentObservedSeconds == 2)
        #expect(history.points.first?.cpuSeries != history.points.last?.cpuSeries)
        #expect(history.points.first?.memorySeries != history.points.last?.memorySeries)
    }

    @Test("Evicting chart samples preserves resource peaks for the whole scan")
    func resourceWindow() {
        var history = ScanProfileHistory()
        history.record(resourceSample(0, user: 0, system: 0, rss: 100))
        for second in 1...1_000 {
            let time = UInt64(second)
            history.record(resourceSample(time, user: time + 3, system: time,
                                          rss: second == 1 ? 9999 : 100))
        }
        #expect(history.points.count == ScanProfileHistory.capacity)
        #expect(history.peakResidentBytes == 9999)
        #expect(history.peakCPUPercent == 500)
        #expect(abs((history.averageCPUPercent ?? 0) - 200.3) < 0.0001)
    }

    private func sample(_ seconds: UInt64, entries: UInt64, bytes: UInt64 = 0) -> ScanProfileSnapshot {
        ScanProfileSnapshot(elapsedMicroseconds: seconds * 1_000_000, entries: entries,
            directories: 0, measuredBytes: bytes, artifacts: 0, warnings: 0, pendingTasks: 0)
    }

    @Test("Rates use interval deltas, while means include idle time")
    func intervalsAndIdleTime() {
        var history = ScanProfileHistory()
        history.record(sample(0, entries: 0))
        history.record(sample(1, entries: 100, bytes: 1_000))
        history.record(sample(3, entries: 100, bytes: 1_000))
        #expect(history.points.map(\.entriesPerSecond) == [100, 0])
        #expect(history.points.map(\.bytesPerSecond) == [1_000, 0])
        #expect(history.peakEntriesPerSecond == 100)
        #expect(abs(history.averageEntriesPerSecond - 100.0 / 3) < 0.001)
    }

    @Test("Rolling history preserves lifetime averages and peaks")
    func boundedHistory() {
        var history = ScanProfileHistory()
        history.record(sample(0, entries: 0))
        for seconds in 1...1_000 {
            history.record(sample(UInt64(seconds), entries: UInt64(seconds * 10 + 100)))
        }
        #expect(history.points.count == ScanProfileHistory.capacity)
        #expect(history.peakEntriesPerSecond == 110)
        #expect(history.averageEntriesPerSecond == 10.1)
        #expect(history.points.first?.seconds == 401)
    }

    @Test("Rolling chart points retain absolute identities and values")
    func rollingChartIdentity() {
        var history = ScanProfileHistory()
        history.record(sample(0, entries: 0))
        for seconds in 1...ScanProfileHistory.capacity {
            history.record(sample(UInt64(seconds), entries: UInt64(seconds * 10)))
        }
        let before = Dictionary(uniqueKeysWithValues: history.points.map { ($0.id, $0) })
        let next = UInt64(ScanProfileHistory.capacity + 1)
        history.record(sample(next, entries: next * 10))
        #expect(history.points.first?.id == 2)
        #expect(history.points.last?.id == Double(next))
        for point in history.points where before[point.id] != nil {
            #expect(before[point.id] == point, "A retained mark must keep the same time and rate")
        }
    }

    @Test("Duplicate or backwards samples cannot create invalid chart rates")
    func invalidSamples() {
        var history = ScanProfileHistory()
        history.record(sample(0, entries: 0))
        #expect(history.averageEntriesPerSecond == 0)
        history.record(sample(0, entries: 100))
        history.record(sample(2, entries: 20))
        history.record(sample(1, entries: 10))
        history.record(sample(3, entries: 1))
        #expect(history.points.count == 1)
        #expect(history.latest?.entries == 20)
        #expect(history.points.allSatisfy { $0.entriesPerSecond.isFinite && $0.bytesPerSecond.isFinite })
    }

    @Test("Buffered artifact bursts preserve live samples and terminal results")
    func bufferedBurst() async {
        let stream = AsyncStream<ScanEvent> { continuation in
            continuation.yield(.profile(generation: 1, sample: sample(0, entries: 0)))
            for index in 1...2_048 {
                let artifact = ScanArtifact(id: UUID(), relativePath: "project-\(index)/.build",
                                            language: "Swift", kind: .buildOutput)
                continuation.yield(.discovered(generation: 1, artifact: artifact))
                continuation.yield(.completed(generation: 1, artifactID: artifact.id, bytes: 1))
                if index % 128 == 0 {
                    continuation.yield(.profile(generation: 1,
                        sample: sample(UInt64(index / 128), entries: UInt64(index), bytes: UInt64(index))))
                }
            }
            continuation.yield(.finished(generation: 1, result: .completed))
            continuation.finish()
        }
        let model = WindowScanModel(source: SingleStreamScanSource(stream: stream))
        model.acceptDemoTarget(named: "Busy stream")
        await model.waitForCurrentScan()
        #expect(model.phase == .completed)
        #expect(model.rows.count == 2_048)
        #expect(model.profile.points.count == 16)
        #expect(model.profile.latest?.entries == 2_048)
    }

    @Test("Replacing or stopping a target rejects stale profile events")
    func lifecycle() {
        let model = WindowScanModel(source: TableIdleSource())
        model.acceptDemoTarget(named: "first")
        let firstGeneration = model.generation
        model.apply(.profile(generation: firstGeneration, sample: sample(0, entries: 0)))
        model.apply(.profile(generation: firstGeneration, sample: sample(1, entries: 10)))
        #expect(model.profile.averageEntriesPerSecond == 10)
        model.acceptDemoTarget(named: "second")
        #expect(model.profile.latest == nil)
        model.apply(.profile(generation: firstGeneration, sample: sample(2, entries: 20)))
        #expect(model.profile.latest == nil)
        model.apply(.profile(generation: model.generation, sample: sample(0, entries: 0)))
        model.stop()
        model.apply(.profile(generation: model.generation, sample: sample(1, entries: 100)))
        #expect(model.profile.latest?.entries == 0)
    }
}
