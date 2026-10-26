import Foundation

@MainActor
final class RustScanEventSource: ScanEventSource {
    private var jobs: [UInt64: RustScanJob] = [:]
    private let settings: SearchFilterSettings
    private let templates: TemplateLibrary

    init(settings: SearchFilterSettings = .shared, templates: TemplateLibrary = .shared) {
        self.settings = settings
        self.templates = templates
    }

    /// Undelivered events held for the model. When it is full the scan thread waits instead
    /// of dropping events, so memory stays bounded and the report stays complete.
    nonisolated static let eventCapacity = 4_096

    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        let channel = ScanEventChannel(capacity: Self.eventCapacity)
        guard let target else {
            channel.send(.finished(generation: generation, result: .failed("No folder is selected.")))
            channel.finish()
            return channel.makeStream(onClose: {})
        }
        guard let control = bh_scan_control_create() else {
            channel.send(.finished(generation: generation, result: .failed("Could not start the Rust scanner.")))
            channel.finish()
            return channel.makeStream(onClose: {})
        }

        bh_scan_control_set_profiling(control, 1)
        bh_scan_control_set_rule_profiling(control, settings.ruleProfilingEnabled ? 1 : 0)
        let job = RustScanJob(control: control, channel: channel)
        jobs[generation] = job
        // Settings changes affect the next scan, never a running report.
        let filters = settings.snapshot
        let templateFiles = templates.references
        let templateStorageProblem = templates.storageProblem
        let rootBytes = target.withUnsafeFileSystemRepresentation { pointer -> Data in
            guard let pointer else { return Data() }
            var count = 0
            while pointer[count] != 0 { count += 1 }
            return Data(bytes: pointer, count: count)
        }
        let context = RustScanBridgeContext(generation: generation, channel: channel,
            rootPathIsExact: String(data: rootBytes, encoding: .utf8) != nil)
        let retainedContext = Unmanaged.passRetained(context).toOpaque()
        let startedSecurityScope = target.startAccessingSecurityScopedResource()
        let stream = channel.makeStream(onClose: { job.cancel() })

        Task.detached(priority: .userInitiated) {
            let result: Int32 = {
                let catalog: UnsafeMutableRawPointer
                do {
                    if let templateStorageProblem {
                        throw TemplateProblem(code: "invalid_registry", location: "", message: templateStorageProblem)
                    }
                    catalog = try TemplateFiles.createCatalog(templateFiles, excluded: filters.excludedFilterIDs)
                } catch {
                    context.yield(.warning(generation: generation, message: "Template catalog: \(error.localizedDescription) Fix or disable the template in Settings → Templates."))
                    context.finish(status: 3)
                    return 3
                }
                defer { bh_catalog_destroy(catalog) }
                return rootBytes.withUnsafeBytes { root in
                    bh_scan_catalog_v1(control, catalog, root.baseAddress?.assumingMemoryBound(to: UInt8.self), root.count,
                        0, 0, buildHunterCatalogEventCallback, retainedContext)
                }
            }()
            job.finish()
            if startedSecurityScope {
                target.stopAccessingSecurityScopedResource()
            }
            let bridgeContext = Unmanaged<RustScanBridgeContext>.fromOpaque(retainedContext).takeRetainedValue()
            bridgeContext.finishIfRustReturnedUnexpectedly(status: result)
            _ = await MainActor.run {
                self.jobs.removeValue(forKey: generation)
            }
        }
        return stream
    }

    func cancel(generation: UInt64) {
        jobs[generation]?.cancel()
    }
}

private final class RustScanJob: @unchecked Sendable {
    private let lock = NSLock()
    private var control: UnsafeMutableRawPointer?
    private let channel: ScanEventChannel

    init(control: UnsafeMutableRawPointer, channel: ScanEventChannel) {
        self.control = control
        self.channel = channel
    }

    /// Stops the scan and wakes its thread if it is waiting for buffer space.
    func cancel() {
        lock.lock()
        if let control {
            bh_scan_control_cancel(control)
        }
        lock.unlock()
        channel.close()
    }

    func finish() {
        lock.lock()
        defer { lock.unlock() }
        if let control {
            self.control = nil
            bh_scan_control_destroy(control)
        }
    }
}

/// A single-producer, single-consumer event queue with blocking backpressure. The Rust scan
/// thread waits in `send` while `capacity` events are undelivered. Closing the queue, which
/// happens on Stop, cancellation or when the stream is abandoned, discards pending events and
/// wakes the producer, so Stop never waits for the consumer.
final class ScanEventChannel: @unchecked Sendable {
    private let condition = NSCondition()
    private let capacity: Int
    // Fixed storage bounds payload retention even when the queue never becomes empty.
    private var events: [ScanEvent?]
    private var head = 0
    private var occupiedCount = 0
    private var waiter: CheckedContinuation<ScanEvent?, Never>?
    private var finished = false
    private var closed = false

    init(capacity: Int) {
        precondition(capacity > 0)
        self.capacity = capacity
        events = Array(repeating: nil, count: capacity)
    }

    var bufferedCount: Int { condition.withLock { occupiedCount } }

    /// Test probe: counts payloads retained by backing storage, including delivered events.
    /// Scans every slot (O(capacity)); not used by the scan or app UI.
    var retainedEventCount: Int {
        condition.withLock { events.reduce(0) { $0 + ($1 == nil ? 0 : 1) } }
    }

    /// Blocks while the buffer is full. Returns `false` once the queue is closed.
    @discardableResult
    func send(_ event: ScanEvent) -> Bool {
        condition.lock()
        while !closed, waiter == nil, occupiedCount >= capacity {
            condition.wait()
        }
        guard !closed else {
            condition.unlock()
            return false
        }
        let waiter = self.waiter
        if waiter == nil {
            events[(head + occupiedCount) % capacity] = event
            occupiedCount += 1
        } else {
            self.waiter = nil
        }
        condition.unlock()
        waiter?.resume(returning: event)
        return true
    }

    /// Ends the stream after the buffered events are delivered.
    func finish() {
        condition.lock()
        finished = true
        let waiter = self.waiter
        self.waiter = nil
        condition.unlock()
        waiter?.resume(returning: nil)
    }

    /// Ends the stream now and releases a waiting producer.
    func close() {
        condition.lock()
        closed = true
        for index in events.indices { events[index] = nil }
        head = 0
        occupiedCount = 0
        let waiter = self.waiter
        self.waiter = nil
        condition.broadcast()
        condition.unlock()
        waiter?.resume(returning: nil)
    }

    func next() async -> ScanEvent? {
        await withCheckedContinuation { continuation in
            condition.lock()
            if occupiedCount > 0 {
                let event = events[head]
                events[head] = nil
                head = (head + 1) % capacity
                occupiedCount -= 1
                condition.signal()
                condition.unlock()
                continuation.resume(returning: event)
            } else if finished || closed {
                condition.unlock()
                continuation.resume(returning: nil)
            } else {
                waiter = continuation
                condition.unlock()
            }
        }
    }

    /// The stream closes the queue and calls `onClose` when it is cancelled or released.
    func makeStream(onClose: @escaping @Sendable () -> Void) -> AsyncStream<ScanEvent> {
        let lifetime = ScanEventStreamLifetime(channel: self, onClose: onClose)
        return AsyncStream(unfolding: { await lifetime.channel.next() },
                           onCancel: { lifetime.close() })
    }
}

private final class ScanEventStreamLifetime: Sendable {
    let channel: ScanEventChannel
    private let onClose: @Sendable () -> Void

    init(channel: ScanEventChannel, onClose: @escaping @Sendable () -> Void) {
        self.channel = channel
        self.onClose = onClose
    }

    func close() {
        channel.close()
        onClose()
    }

    deinit {
        close()
    }
}

// Rust calls callbacks serially on its scan thread. Only that thread mutates the
// catalog cache/warning counter; the completion flag is locked and the channel is thread-safe.
final class RustScanBridgeContext: @unchecked Sendable {
    let generation: UInt64
    let channel: ScanEventChannel
    let policy: BuildHunterScanPolicy?
    private let rootPathIsExact: Bool
    private var receivedRuleProfile = false
    private var catalogMetadata: ScanCatalogMetadata?
    private var catalogTargets: [String: ScanCatalogMetadata.Target] = [:]
    private let lock = NSLock()
    private var didFinish = false
    private var bridgeGeneratedWarnings: UInt64 = 0

    init(generation: UInt64, channel: ScanEventChannel, policy: BuildHunterScanPolicy? = nil, rootPathIsExact: Bool = true) {
        self.generation = generation
        self.channel = channel
        self.policy = policy
        self.rootPathIsExact = rootPathIsExact
    }

    func yield(_ event: ScanEvent) {
        channel.send(event)
    }

    func finishIfRustReturnedUnexpectedly(status: Int32) {
        lock.lock()
        let shouldFinish = !didFinish
        didFinish = true
        lock.unlock()

        if shouldFinish {
            let message = status == 0 ? "The scanner ended without a terminal event." : "The Rust scanner stopped with status \(status)."
            yield(.warning(generation: generation, message: message))
            yield(.finished(generation: generation, result: .failed(message)))
        }
        channel.finish()
    }

    func handle(_ event: UnsafePointer<BHScanEvent>) { handle(event, resourceUsage: nil) }

    private func handle(_ event: UnsafePointer<BHScanEvent>, resourceUsage: ScanResourceUsage?) {
        let value = event.pointee
        let path = decode(value.path, length: value.path_len)
        switch value.event_type {
        case 1:
            if decodeUTF8(value.path, length: value.path_len) == nil {
                warnAboutBridge("A detected path is not valid UTF-8; its displayed name may be lossy.")
            }
            let language = languageName(value.language)
            let kind = artifactKind(value.kind)
            yield(.discovered(
                generation: generation,
                artifact: ScanArtifact(
                    id: stableID(value.artifact_id),
                    relativePath: path,
                    language: language,
                    kind: kind,
                    scannerKind: scannerKindName(value.kind),
                    unsafePathPrefix: firstUnsafePathPrefix(value.path, length: value.path_len)
                )
            ))
        case 2:
            yield(.completed(
                generation: generation,
                artifactID: stableID(value.artifact_id),
                bytes: Int64(clamping: value.bytes),
                partial: value.partial != 0
            ))
        case 3:
            let detail = decode(value.message, length: value.message_len)
            yield(.warning(
                generation: generation,
                message: path.isEmpty ? detail : "\(path): \(detail)"
            ))
        case 4:
            finish(status: value.status)
        case 5:
            if let sample = value.profile?.pointee {
                // Copy every borrowed counter before returning to Rust.
                yield(.profile(generation: generation, sample: ScanProfileSnapshot(
                    elapsedMicroseconds: sample.elapsed_us, entries: sample.entries,
                    directories: sample.directories, measuredBytes: sample.measured_bytes,
                    artifacts: sample.artifacts,
                    warnings: sample.warnings > .max - bridgeGeneratedWarnings
                        ? .max : sample.warnings + bridgeGeneratedWarnings,
                    pendingTasks: sample.pending_tasks,
                    cpuUserNanoseconds: sample.resource_flags & 1 != 0 ? sample.cpu_user_ns : nil,
                    cpuSystemNanoseconds: sample.resource_flags & 1 != 0 ? sample.cpu_system_ns : nil,
                    residentBytes: sample.resource_flags & 2 != 0 ? sample.resident_bytes : nil,
                    resourceUsage: resourceUsage
                )))
            }
        default:
            warnAboutBridge("Rust scanner emitted an unknown event.")
        }
    }

    /// Called serially by ABI 1. Copy all borrowed data before returning to Rust.
    func handleCatalog(_ pointer: UnsafePointer<BHCatalogScanEvent>) {
        let value = pointer.pointee
        guard value.version == 1 else { warnAboutBridge("Unsupported catalog event ABI."); return }
        switch value.base.event_type {
        case 6:
            guard let bytes = value.metadata, (1...4_194_304).contains(value.metadata_len),
                  let metadata = try? JSONDecoder().decode(ScanCatalogMetadata.self,
                    from: Data(bytes: bytes, count: value.metadata_len)), metadata.isSupported,
                  catalogMetadata == nil else { warnAboutBridge("Invalid or repeated scan catalog metadata."); return }
            catalogMetadata = metadata
            catalogTargets = Dictionary(uniqueKeysWithValues: metadata.targets.map { ($0.id, $0) })
            yield(.catalog(generation: generation, metadata: metadata, rootPathIsExact: rootPathIsExact))
            if !rootPathIsExact { warnAboutBridge("The selected path is not valid UTF-8; path actions are disabled.") }
        case 1:
            guard let catalogMetadata, let id = decodeUTF8(value.target_id, length: value.target_id_len),
                  let target = catalogTargets[id], let kind = target.artifactKind else {
                warnAboutBridge("A discovery refers to an unknown catalog target."); return
            }
            let path = decode(value.base.path, length: value.base.path_len)
            let firstInvalid = firstUnsafePathPrefix(value.base.path, length: value.base.path_len)
            let unsafePrefix = rootPathIsExact ? firstInvalid : ""
            if firstInvalid != nil { warnAboutBridge("A detected path is not valid UTF-8; path actions are disabled for its lossy components.") }
            yield(.discovered(generation: generation, artifact: ScanArtifact(
                id: stableID(value.base.artifact_id), relativePath: path, language: target.language,
                kind: kind, targetID: target.id, catalogDigest: catalogMetadata.digest, unsafePathPrefix: unsafePrefix)))
        case 5:
            var usage: ScanResourceUsage?
            if value.metadata_len > 0 {
                if let bytes = value.metadata, value.metadata_len <= 16_384 {
                    usage = try? JSONDecoder().decode(ScanResourceUsage.self,
                        from: Data(bytes: bytes, count: value.metadata_len))
                }
                if usage == nil { warnAboutBridge("Invalid scan resource metadata.") }
            }
            withUnsafePointer(to: value.base) { handle($0, resourceUsage: usage) }
        case 8:
            guard !receivedRuleProfile, let catalogMetadata,
                  let bytes = value.metadata, (1...16_777_216).contains(value.metadata_len),
                  let profile = try? JSONDecoder().decode(ScanRuleProfile.self,
                    from: Data(bytes: bytes, count: value.metadata_len)), profile.isSupported,
                  profile.catalogDigest == catalogMetadata.digest,
                  profile.excludedTargetIDs.sorted() == catalogMetadata.excludedTargetIDs.sorted(),
                  profile.rules.allSatisfy({ catalogTargets[$0.targetID] != nil }) else {
                warnAboutBridge("Invalid or repeated rule profile."); return
            }
            receivedRuleProfile = true
            yield(.ruleProfile(generation: generation, profile: profile))
        case 7:
            guard let bytes = value.metadata, (1...4_194_304).contains(value.metadata_len),
                  let classification = try? JSONDecoder().decode(ScanDiagnostic.Classification.self,
                    from: Data(bytes: bytes, count: value.metadata_len)),
                  ["ambiguous", "indeterminate"].contains(classification.kind) else {
                warnAboutBridge("Invalid plugin diagnostic."); return
            }
            let path = decode(value.base.path, length: value.base.path_len)
            let message = decode(value.base.message, length: value.base.message_len)
            yield(.diagnostic(generation: generation, diagnostic: ScanDiagnostic(relativePath: path,
                classification: classification, message: path.isEmpty ? message : "\(path): \(message)")))
        default:
            withUnsafePointer(to: value.base) { handle($0) }
        }
    }

    private func warnAboutBridge(_ message: String) {
        bridgeGeneratedWarnings = bridgeGeneratedWarnings == .max ? .max : bridgeGeneratedWarnings + 1
        yield(.warning(generation: generation, message: message))
    }

    func finish(status: UInt32) {
        lock.lock()
        let shouldFinish = !didFinish
        didFinish = true
        lock.unlock()
        guard shouldFinish else { return }

        // Rust knows only its own warnings. A lossy path or unknown bridge event
        // must still make a completed/changed report incomplete in the GUI.
        let result: ScanTerminalResult
        if (status == 0 || status == 4), bridgeGeneratedWarnings > 0 {
            result = .failed("The Swift bridge could not preserve all scanner data; results are incomplete.")
        } else {
            result = switch status {
            case 0: .completed
            case 4: .completedWithChanges
            case 1: .stopped
            case 2: .failed("Some paths were skipped or could not be read; results are incomplete.")
            case 3: .failed("The scanner could not complete the scan.")
            default: .failed("The Rust scanner returned an unknown status \(status).")
            }
        }
        yield(.finished(generation: generation, result: result))
        channel.finish()
    }
}

/// The sole Swift implementation of a scan decision. Both the legacy callback and the
/// finite table compiler pass raw ABI facts through this function, preserving the UTF-8
/// guard before any policy context is created.
struct BuildHunterScanPolicy {
    private let candidatePolicy = IsScannableArtifactCandidate()
    private let outerRootPolicy = IsOuterArtifactRoot()
    private let classificationPolicy = ClassifyArtifactRoot()
    let filters: SearchFilterSnapshot

    init(filters: SearchFilterSnapshot) {
        self.filters = filters
    }

    func decision(for facts: BHCandidateFacts) -> BHCandidateDecision {
        guard let name = decodeUTF8(facts.node_name, length: facts.node_name_len) else {
            return BHCandidateDecision(action: 0, language: 0, kind: 0)
        }
        let context = ArtifactPolicyContext(
            nodeName: name,
            isDirectory: facts.is_directory != 0,
            isSymbolicLink: facts.is_symbolic_link != 0,
            hasArtifactAncestor: facts.has_artifact_ancestor != 0,
            ownMarkerFacts: ArtifactMarkerFacts(rawValue: facts.own_marker_files),
            parentMarkerFacts: ArtifactMarkerFacts(rawValue: facts.parent_marker_files)
        )
        return decision(for: context)
    }

    func decision(for context: ArtifactPolicyContext) -> BHCandidateDecision {
        guard candidatePolicy.isSatisfiedBy(context) else {
            return BHCandidateDecision(action: 1, language: 0, kind: 0)
        }
        guard outerRootPolicy.isSatisfiedBy(ArtifactRootContext(hasArtifactAncestor: context.hasArtifactAncestor)),
              let classification = classificationPolicy.decide(context) else {
            return BHCandidateDecision(action: 0, language: 0, kind: 0)
        }
        guard filters.includes(candidate: context, classification: classification) else {
            // Keep traversing an excluded root: eligible artifacts of another type
            // may be inside it. Exclusions do not alter an accepted root's byte size.
            return BHCandidateDecision(action: 0, language: 0, kind: 0)
        }
        return BHCandidateDecision(action: 2, language: languageCode(classification.language),
                                   kind: kindCode(classification.kind))
    }
}

/// Immutable, scan-owned decisions compiled over the Rust vocabulary. Unsupported
/// descriptor matching remains on the extensible callback path.
final class RustCompiledPolicyTable: @unchecked Sendable {
    let version: UInt32
    let entries: [BHCandidateDecision]

    private init(version: UInt32, entries: [BHCandidateDecision]) {
        self.version = version
        self.entries = entries
    }

    static func compile(filters: SearchFilterSnapshot) -> RustCompiledPolicyTable? {
        compile(using: BuildHunterScanPolicy(filters: filters))
    }

    static func compile(using policy: BuildHunterScanPolicy) -> RustCompiledPolicyTable? {
        let cellCount = Int(BH_POLICY_TABLE_CELL_COUNT)
        guard bh_policy_table_version() == UInt32(BH_POLICY_TABLE_VERSION),
              bh_policy_table_cell_count() == cellCount,
              isRepresentable(policy.filters) else { return nil }

        var entries: [BHCandidateDecision] = []
        entries.reserveCapacity(cellCount)
        for index in 0..<cellCount {
            var facts = BHCandidateFacts(node_name: nil, node_name_len: 0, is_directory: 0,
                                         is_symbolic_link: 0, has_artifact_ancestor: 0,
                                         own_marker_files: 0, parent_marker_files: 0)
            guard bh_policy_table_cell_facts(index, &facts) != 0 else { return nil }
            entries.append(policy.decision(for: facts))
        }
        guard entries.count == cellCount else { return nil }
        return RustCompiledPolicyTable(version: bh_policy_table_version(), entries: entries)
    }

    func withUnsafeEntries<R>(_ body: (UnsafePointer<BHCandidateDecision>, Int) -> R) -> R {
        entries.withUnsafeBufferPointer { buffer in
            guard let baseAddress = buffer.baseAddress else {
                preconditionFailure("A validated policy table cannot be empty.")
            }
            return body(baseAddress, buffer.count)
        }
    }

    private static func isRepresentable(_ filters: SearchFilterSnapshot) -> Bool {
        let exactNames: Set<String> = [
            ".git", ".build", "target", "__pycache__", ".pytest_cache", ".mypy_cache",
            ".ruff_cache", ".pytype", ".tox", ".nox", "build", "dist", ".venv", "venv"
        ]
        let suffixes: Set<String> = [".egg-info", ".pyc", ".pyo"]
        return filters.catalog.allSatisfy { descriptor in
            if descriptor.requiresVenvMarker { return true }
            return descriptor.nodeNames.allSatisfy(exactNames.contains)
                && descriptor.suffixes.allSatisfy(suffixes.contains)
        }
    }
}

@_cdecl("build_hunter_policy_callback")
private func buildHunterPolicyCallback(
    _ rawContext: UnsafeMutableRawPointer?,
    _ rawFacts: UnsafePointer<BHCandidateFacts>?,
    _ rawDecision: UnsafeMutablePointer<BHCandidateDecision>?
) -> Int32 {
    guard let rawContext, let rawFacts, let rawDecision else { return 0 }
    let bridge = Unmanaged<RustScanBridgeContext>.fromOpaque(rawContext).takeUnretainedValue()
    guard let policy = bridge.policy else { return 0 }
    rawDecision.pointee = policy.decision(for: rawFacts.pointee)
    return 1
}

@_cdecl("build_hunter_event_callback")
private func buildHunterEventCallback(
    _ rawContext: UnsafeMutableRawPointer?,
    _ rawEvent: UnsafePointer<BHScanEvent>?
) {
    guard let rawContext, let rawEvent else { return }
    Unmanaged<RustScanBridgeContext>.fromOpaque(rawContext).takeUnretainedValue().handle(rawEvent)
}

@_cdecl("build_hunter_catalog_event_callback")
private func buildHunterCatalogEventCallback(_ rawContext: UnsafeMutableRawPointer?, _ rawEvent: UnsafePointer<BHCatalogScanEvent>?) {
    guard let rawContext, let rawEvent else { return }
    Unmanaged<RustScanBridgeContext>.fromOpaque(rawContext).takeUnretainedValue().handleCatalog(rawEvent)
}

/// Keeps safe ancestors usable while rejecting any displayed path containing the
/// first undecodable component. Does not manufacture an action URL from replacement text.
func firstUnsafePathPrefix(_ bytes: UnsafePointer<UInt8>?, length: Int) -> String? {
    guard let bytes, length > 0 else { return nil }
    let data = Data(bytes: bytes, count: length)
    if String(data: data, encoding: .utf8) != nil { return nil }
    var components: [String] = []
    for part in data.split(separator: 47, omittingEmptySubsequences: false) {
        components.append(String(decoding: part, as: UTF8.self))
        if String(bytes: part, encoding: .utf8) == nil { return components.joined(separator: "/") }
    }
    return nil
}

private func decode(_ bytes: UnsafePointer<UInt8>?, length: Int) -> String {
    guard let bytes, length > 0 else { return "" }
    return String(decoding: UnsafeBufferPointer(start: bytes, count: length), as: UTF8.self)
}

private func decodeUTF8(_ bytes: UnsafePointer<UInt8>?, length: Int) -> String? {
    guard let bytes, length >= 0 else { return nil }
    return String(bytes: UnsafeBufferPointer(start: bytes, count: length), encoding: .utf8)
}

private func languageCode(_ language: String) -> UInt32 {
    switch language {
    case "Swift": 1
    case "Rust": 2
    case "Python": 3
    default: 0
    }
}

private func kindCode(_ kind: ArtifactKind) -> UInt32 {
    switch kind {
    case .buildOutput: 1
    case .cache: 2
    case .environment: 3
    case .testEnvironment: 4
    }
}

private func languageName(_ code: UInt32) -> String {
    switch code {
    case 1: "Swift"
    case 2: "Rust"
    case 3: "Python"
    default: "Unknown"
    }
}

/// The scanner's name for kinds that `artifactKind` folds into Cache.
private func scannerKindName(_ code: UInt32) -> String? {
    switch code {
    case 5: "bytecode"
    case 6: "metadata"
    default: nil
    }
}

private func artifactKind(_ code: UInt32) -> ArtifactKind {
    switch code {
    case 1: .buildOutput
    case 2, 5, 6: .cache
    case 3: .environment
    case 4: .testEnvironment
    default: .cache
    }
}

/// Maps a Rust artifact ID to `00000000-0000-4000-8000-<low 48 bits>` from bytes; formatting
/// and parsing a UUID string cost about a microsecond per event on the scan thread.
func stableID(_ value: UInt64) -> UUID {
    func byte(_ shift: UInt64) -> UInt8 { UInt8(truncatingIfNeeded: value >> shift) }
    return UUID(uuid: (0, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0,
                       byte(40), byte(32), byte(24), byte(16), byte(8), byte(0)))
}
