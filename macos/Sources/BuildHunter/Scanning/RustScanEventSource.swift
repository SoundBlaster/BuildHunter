import Foundation

@MainActor
final class RustScanEventSource: ScanEventSource {
    private var jobs: [UInt64: RustScanJob] = [:]
    private let settings: SearchFilterSettings

    init(settings: SearchFilterSettings = .shared) {
        self.settings = settings
    }

    /// The scanner outruns a main actor that is busy rendering. A bounded buffer dropped
    /// events and failed the whole report, so buffering is unbounded: it holds at most the
    /// artifacts and warnings the report keeps anyway, and it drains as the model applies them.
    nonisolated static func makeEventStream() -> (AsyncStream<ScanEvent>, AsyncStream<ScanEvent>.Continuation) {
        AsyncStream.makeStream(of: ScanEvent.self, bufferingPolicy: .unbounded)
    }

    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        let (stream, continuation) = Self.makeEventStream()
        guard let target else {
            continuation.yield(.finished(generation: generation, result: .failed("No folder is selected.")))
            continuation.finish()
            return stream
        }
        guard let control = bh_scan_control_create() else {
            continuation.yield(.finished(generation: generation, result: .failed("Could not start the Rust scanner.")))
            continuation.finish()
            return stream
        }

        let job = RustScanJob(control: control)
        jobs[generation] = job
        // Settings changes affect the next scan, never a running report.
        let context = RustScanBridgeContext(generation: generation, continuation: continuation,
                                           filters: settings.snapshot)
        let retainedContext = Unmanaged.passRetained(context).toOpaque()
        let rootBytes = Data(target.path.utf8)
        let startedSecurityScope = target.startAccessingSecurityScopedResource()
        continuation.onTermination = { @Sendable _ in job.cancel() }

        Task.detached(priority: .userInitiated) {
            let result = rootBytes.withUnsafeBytes { rawBuffer -> Int32 in
                guard let baseAddress = rawBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    return 3
                }
                return bh_scan(
                    control,
                    baseAddress,
                    rawBuffer.count,
                    0,
                    buildHunterPolicyCallback,
                    buildHunterEventCallback,
                    retainedContext
                )
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

    init(control: UnsafeMutableRawPointer) {
        self.control = control
    }

    func cancel() {
        lock.lock()
        defer { lock.unlock() }
        if let control {
            bh_scan_control_cancel(control)
        }
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

// Rust calls both callbacks serially on its scan thread. Policies are immutable;
// completion/overflow flags are locked and AsyncStream.Continuation is Sendable.
final class RustScanBridgeContext: @unchecked Sendable {
    let generation: UInt64
    let continuation: AsyncStream<ScanEvent>.Continuation
    let candidatePolicy = IsScannableArtifactCandidate()
    let outerRootPolicy = IsOuterArtifactRoot()
    let classificationPolicy = ClassifyArtifactRoot()
    let filters: SearchFilterSnapshot
    private let lock = NSLock()
    private var overflowed = false
    private var didFinish = false

    init(generation: UInt64, continuation: AsyncStream<ScanEvent>.Continuation, filters: SearchFilterSnapshot) {
        self.generation = generation
        self.continuation = continuation
        self.filters = filters
    }

    @discardableResult
    func yield(_ event: ScanEvent) -> Bool {
        switch continuation.yield(event) {
        case .dropped:
            lock.lock()
            overflowed = true
            lock.unlock()
            return true
        case .enqueued, .terminated:
            return false
        @unknown default:
            return false
        }
    }

    func finishIfRustReturnedUnexpectedly(status: Int32) {
        lock.lock()
        let shouldFinish = !didFinish
        didFinish = true
        let hadOverflow = overflowed
        lock.unlock()

        if shouldFinish {
            if hadOverflow {
                yield(.warning(generation: generation, message: "Some scan events exceeded the display buffer; results are incomplete."))
                yield(.finished(generation: generation, result: .failed("The display buffer filled before the scan completed.")))
            } else {
                let message = status == 0 ? "The scanner ended without a terminal event." : "The Rust scanner stopped with status \(status)."
                yield(.warning(generation: generation, message: message))
                yield(.finished(generation: generation, result: .failed(message)))
            }
        }
        continuation.finish()
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
        let hadOverflow = overflowed
        lock.unlock()
        guard shouldFinish else { return }

        if hadOverflow {
            yield(.warning(generation: generation, message: "Some scan events exceeded the display buffer; results are incomplete."))
            yield(.finished(generation: generation, result: .failed("The display buffer filled before the scan completed.")))
        } else {
            let result: ScanTerminalResult = switch status {
            case 0: .completed
            case 1: .stopped
            case 2: .failed("Some paths could not be read; results are incomplete.")
            case 3: .failed("The scanner could not complete the scan.")
            default: .failed("The Rust scanner returned an unknown status \(status).")
            }
            if yield(.finished(generation: generation, result: result)) {
                yield(.warning(generation: generation, message: "Some scan events exceeded the display buffer; results are incomplete."))
                yield(.finished(generation: generation, result: .failed("The display buffer filled before the scan completed.")))
            }
        }
        continuation.finish()
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
    let facts = rawFacts.pointee
    guard let name = decodeUTF8(facts.node_name, length: facts.node_name_len) else {
        rawDecision.pointee.action = 0
        return 1
    }
    let ownMarkers = markerNames(facts.own_marker_files)
    let parentMarkers = markerNames(facts.parent_marker_files)
    let context = ArtifactPolicyContext(
        nodeName: name,
        isDirectory: facts.is_directory != 0,
        isSymbolicLink: facts.is_symbolic_link != 0,
        hasArtifactAncestor: facts.has_artifact_ancestor != 0,
        ownMarkerFiles: ownMarkers,
        parentMarkerFiles: parentMarkers
    )
    guard bridge.candidatePolicy.isSatisfiedBy(context) else {
        rawDecision.pointee.action = 1
        return 1
    }
    guard bridge.outerRootPolicy.isSatisfiedBy(
        ArtifactRootContext(hasArtifactAncestor: facts.has_artifact_ancestor != 0)
    ), let classification = bridge.classificationPolicy.decide(context) else {
        rawDecision.pointee.action = 0
        return 1
    }
    guard bridge.filters.includes(candidate: context, classification: classification) else {
        // Keep traversing an excluded root: eligible artifacts of another type
        // may be inside it. Exclusions do not alter an accepted root's byte size.
        rawDecision.pointee.action = 0
        return 1
    }
    rawDecision.pointee.action = 2
    rawDecision.pointee.language = languageCode(classification.language)
    rawDecision.pointee.kind = kindCode(classification.kind)
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

private func markerNames(_ flags: UInt32) -> Set<String> {
    var names = Set<String>()
    if flags & 1 != 0 { names.insert("Cargo.toml") }
    if flags & 2 != 0 { names.insert("pyproject.toml") }
    if flags & 4 != 0 { names.insert("setup.py") }
    if flags & 8 != 0 { names.insert("setup.cfg") }
    if flags & 16 != 0 { names.insert("pyvenv.cfg") }
    return names
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
