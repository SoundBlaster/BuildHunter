import Foundation

@MainActor
final class RustScanEventSource: ScanEventSource {
    private var jobs: [UInt64: RustScanJob] = [:]
    private let settings: SearchFilterSettings

    init(settings: SearchFilterSettings = .shared) {
        self.settings = settings
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

        let job = RustScanJob(control: control, channel: channel)
        jobs[generation] = job
        // Settings changes affect the next scan, never a running report.
        let filters = settings.snapshot
        let policy = BuildHunterScanPolicy(filters: filters)
        let context = RustScanBridgeContext(generation: generation, channel: channel, policy: policy)
        let retainedContext = Unmanaged.passRetained(context).toOpaque()
        let rootBytes = Data(target.path.utf8)
        let startedSecurityScope = target.startAccessingSecurityScopedResource()
        let stream = channel.makeStream(onClose: { job.cancel() })

        Task.detached(priority: .userInitiated) {
            // Compiling all vocabulary cells is bounded but nontrivial; keep it off the UI actor.
            let table = RustCompiledPolicyTable.compile(using: context.policy)
            let result = rootBytes.withUnsafeBytes { rawBuffer -> Int32 in
                guard let baseAddress = rawBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    return 3
                }
                if let table {
                    return table.withUnsafeEntries { entries, count in
                        bh_scan_with_policy_table(
                            control,
                            baseAddress,
                            rawBuffer.count,
                            0,
                            table.version,
                            entries,
                            count,
                            buildHunterEventCallback,
                            retainedContext
                        )
                    }
                }
                return bh_scan(control, baseAddress, rawBuffer.count, 0,
                               buildHunterPolicyCallback, buildHunterEventCallback, retainedContext)
            }
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

// Rust calls both callbacks serially on its scan thread. Policies are immutable and the
// completion flag is locked; the channel is safe to use from that thread.
final class RustScanBridgeContext: @unchecked Sendable {
    let generation: UInt64
    let channel: ScanEventChannel
    let policy: BuildHunterScanPolicy
    private let lock = NSLock()
    private var didFinish = false

    init(generation: UInt64, channel: ScanEventChannel, policy: BuildHunterScanPolicy) {
        self.generation = generation
        self.channel = channel
        self.policy = policy
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

    func handle(_ event: UnsafePointer<BHScanEvent>) {
        let value = event.pointee
        let path = decode(value.path, length: value.path_len)
        switch value.event_type {
        case 1:
            if decodeUTF8(value.path, length: value.path_len) == nil {
                yield(.warning(
                    generation: generation,
                    message: "A detected path is not valid UTF-8; its displayed name may be lossy."
                ))
            }
            let language = languageName(value.language)
            let kind = artifactKind(value.kind)
            yield(.discovered(
                generation: generation,
                artifact: ScanArtifact(
                    id: stableID(value.artifact_id),
                    relativePath: path,
                    language: language,
                    kind: kind
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
        default:
            yield(.warning(generation: generation, message: "Rust scanner emitted an unknown event."))
        }
    }

    func finish(status: UInt32) {
        lock.lock()
        let shouldFinish = !didFinish
        didFinish = true
        lock.unlock()
        guard shouldFinish else { return }

        let result: ScanTerminalResult = switch status {
        case 0: .completed
        case 1: .stopped
        case 2: .failed("Some paths could not be read; results are incomplete.")
        case 3: .failed("The scanner could not complete the scan.")
        default: .failed("The Rust scanner returned an unknown status \(status).")
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
    rawDecision.pointee = bridge.policy.decision(for: rawFacts.pointee)
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

private func decode(_ bytes: UnsafePointer<UInt8>?, length: Int) -> String {
    guard let bytes, length > 0 else { return "" }
    return String(decoding: UnsafeBufferPointer(start: bytes, count: length), as: UTF8.self)
}

private func decodeUTF8(_ bytes: UnsafePointer<UInt8>?, length: Int) -> String? {
    guard let bytes else { return nil }
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
