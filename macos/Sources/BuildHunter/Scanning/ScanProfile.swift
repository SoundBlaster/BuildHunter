import Foundation

struct ScanRuleProfile: Codable, Hashable, Sendable {
    struct Rule: Codable, Hashable, Identifiable, Sendable {
        let packageID: String
        let targetID: String
        let ruleID: String
        let phase: String
        let evaluations: UInt64
        let skippedByDispatch: UInt64
        let matched: UInt64
        let noMatch: UInt64
        let unknown: UInt64
        let totalNanoseconds: UInt64
        let maxNanoseconds: UInt64
        let averageNanoseconds: Double?
        var id: String { "\(packageID)/\(targetID)/\(ruleID)/\(phase)" }
        enum CodingKeys: String, CodingKey {
            case packageID = "package_id", targetID = "target_id", ruleID = "rule_id", phase, evaluations
            case skippedByDispatch = "skipped_by_dispatch", matched = "true", noMatch = "false", unknown
            case totalNanoseconds = "total_ns", maxNanoseconds = "max_ns", averageNanoseconds = "average_ns"
        }
        func encode(to encoder: Encoder) throws {
            var values = encoder.container(keyedBy: CodingKeys.self)
            try values.encode(packageID, forKey: .packageID)
            try values.encode(targetID, forKey: .targetID)
            try values.encode(ruleID, forKey: .ruleID)
            try values.encode(phase, forKey: .phase)
            try values.encode(evaluations, forKey: .evaluations)
            try values.encode(skippedByDispatch, forKey: .skippedByDispatch)
            try values.encode(matched, forKey: .matched)
            try values.encode(noMatch, forKey: .noMatch)
            try values.encode(unknown, forKey: .unknown)
            try values.encode(totalNanoseconds, forKey: .totalNanoseconds)
            try values.encode(maxNanoseconds, forKey: .maxNanoseconds)
            // Retain the shared wire shape: never-evaluated mean is explicit null.
            try values.encode(averageNanoseconds, forKey: .averageNanoseconds)
        }
    }
    let format: String
    let catalogDigest: String
    let excludedTargetIDs: [String]
    let timingScope: String
    let preliminaryCalls: UInt64
    let resolvedCalls: UInt64
    let classificationErrors: UInt64
    let scanStatus: String?
    let rules: [Rule]
    enum CodingKeys: String, CodingKey {
        case format, catalogDigest = "catalog_digest", timingScope = "timing_scope", excludedTargetIDs = "excluded_target_ids"
        case preliminaryCalls = "preliminary_calls", resolvedCalls = "resolved_calls"
        case classificationErrors = "classification_errors", scanStatus = "scan_status", rules
    }
    var statusTitle: String {
        switch scanStatus {
        case "completed": "Completed"
        case "completed_with_changes": "Completed with changes"
        case "cancelled": "Cancelled · partial metrics"
        case "incomplete": "Incomplete · partial metrics"
        case "failed": "Failed · partial metrics"
        default: "Status unavailable"
        }
    }
    var isSupported: Bool {
        format == "buildhunter.rule-profile/1" && timingScope == "rule_evaluation_and_outcome_wall_time"
            && catalogDigest.utf8.count == 64 && catalogDigest.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
            && excludedTargetIDs.count <= 2_048 && Set(excludedTargetIDs).count == excludedTargetIDs.count
            && (scanStatus.map { ["completed", "completed_with_changes", "cancelled", "incomplete", "failed"].contains($0) } ?? true)
            && rules.count <= 8_192 && Set(rules.map(\.id)).count == rules.count
            && rules.allSatisfy { ["preliminary", "resolved"].contains($0.phase)
                && [$0.packageID, $0.targetID, $0.ruleID].allSatisfy { !$0.isEmpty && $0.utf8.count <= 128 }
                && $0.matched <= $0.evaluations && $0.noMatch <= $0.evaluations - $0.matched
                && $0.unknown == $0.evaluations - $0.matched - $0.noMatch
                && $0.maxNanoseconds <= $0.totalNanoseconds
                && ($0.averageNanoseconds.map { $0.isFinite && $0 >= 0 } ?? true) }
    }
}

/// Per-scan estimates copied from catalog-event metadata. These are payload
/// limits and observations, never an exact process-wide memory ceiling.
struct ScanResourceUsage: Codable, Equatable, Sendable {
    let limits: [String: UInt64]
    let limitReached: String?
    let peakPendingTasks: UInt64
    let peakListingEntries: UInt64
    let peakListingWeight: UInt64
    let droppedProfileSamples: UInt64
}

/// Cumulative coordinator counters, copied from the Rust callback. Bytes describe
/// artifact sizes discovered through metadata, never physical disk I/O.
struct ScanProfileSnapshot: Equatable, Sendable {
    let elapsedMicroseconds: UInt64
    let entries: UInt64
    let directories: UInt64
    let measuredBytes: UInt64
    let artifacts: UInt64
    let warnings: UInt64
    let pendingTasks: UInt64
    /// Process-wide deltas from scan start, not CPU attributed to a single window.
    var cpuUserNanoseconds: UInt64? = nil
    var cpuSystemNanoseconds: UInt64? = nil
    var residentBytes: UInt64? = nil
    var resourceUsage: ScanResourceUsage? = nil

    var elapsedSeconds: Double { Double(elapsedMicroseconds) / 1_000_000 }
}

struct ScanProfilePoint: Identifiable, Equatable, Sendable {
    let seconds: Double
    let entriesPerSecond: Double
    let bytesPerSecond: Double
    /// Cumulative found volume at this point, for the total line.
    var measuredBytes: UInt64 = 0
    /// 100% is one logical core; parallel scans may legitimately exceed 100%.
    var cpuPercent: Double? = nil
    var residentBytes: UInt64? = nil
    /// Separate line series prevent Charts from interpolating across missing readings.
    var cpuSeries: Double = 0
    var memorySeries: Double = 0
    var id: Double { seconds }
}

/// The chart retains the latest minute at the nominal 10 Hz cadence. Lifetime
/// means and peaks are independent of this rolling display window.
struct ScanProfileHistory: Sendable {
    static let capacity = 600
    private(set) var points: [ScanProfilePoint] = []
    private(set) var latest: ScanProfileSnapshot?
    var ruleProfile: ScanRuleProfile?
    private(set) var peakEntriesPerSecond: Double = 0
    private(set) var peakBytesPerSecond: Double = 0
    private(set) var peakCPUPercent: Double?
    private(set) var initialResidentBytes: UInt64?
    private(set) var peakResidentBytes: UInt64?
    private(set) var residentObservedSeconds: Double = 0
    private var residentIntegral: Double = 0
    private var cpuSeries: Double = 0
    private var memorySeries: Double = 0

    var averageEntriesPerSecond: Double { mean(latest?.entries ?? 0) }
    var averageBytesPerSecond: Double { mean(latest?.measuredBytes ?? 0) }
    var averageCPUPercent: Double? {
        guard let latest, latest.elapsedSeconds > 0,
              let user = latest.cpuUserNanoseconds, let system = latest.cpuSystemNanoseconds else { return nil }
        return (Double(user) + Double(system)) / 1_000_000_000 / latest.elapsedSeconds * 100
    }
    var averageResidentBytes: Double? {
        residentObservedSeconds > 0 ? residentIntegral / residentObservedSeconds : nil
    }

    mutating func record(_ input: ScanProfileSnapshot) {
        var sample = input
        let previous = latest
        if let previous {
            guard sample.elapsedMicroseconds > previous.elapsedMicroseconds,
                  sample.entries >= previous.entries,
                  sample.measuredBytes >= previous.measuredBytes else { return }
            if let user = sample.cpuUserNanoseconds, let previousUser = previous.cpuUserNanoseconds,
               let system = sample.cpuSystemNanoseconds, let previousSystem = previous.cpuSystemNanoseconds,
               user < previousUser || system < previousSystem {
                sample.cpuUserNanoseconds = nil
                sample.cpuSystemNanoseconds = nil
            }
        } else {
            initialResidentBytes = sample.residentBytes
        }
        if let resident = sample.residentBytes {
            peakResidentBytes = max(peakResidentBytes ?? 0, resident)
        }
        latest = sample
        guard let previous else { return }
        let lostSamples = (sample.resourceUsage?.droppedProfileSamples ?? 0) > (previous.resourceUsage?.droppedProfileSamples ?? 0)
        if lostSamples {
            cpuSeries = sample.elapsedSeconds
            memorySeries = sample.elapsedSeconds
        }
        let seconds = Double(sample.elapsedMicroseconds - previous.elapsedMicroseconds) / 1_000_000
        let entries = Double(sample.entries - previous.entries) / seconds
        let bytes = Double(sample.measuredBytes - previous.measuredBytes) / seconds
        var cpu: Double?
        if !lostSamples, let user = sample.cpuUserNanoseconds, let previousUser = previous.cpuUserNanoseconds,
           let system = sample.cpuSystemNanoseconds, let previousSystem = previous.cpuSystemNanoseconds {
            let percent = (Double(user - previousUser) + Double(system - previousSystem)) / 1_000_000_000 / seconds * 100
            cpu = percent
            peakCPUPercent = max(peakCPUPercent ?? 0, percent)
        } else {
            cpuSeries = sample.elapsedSeconds
        }
        if !lostSamples, let resident = sample.residentBytes, let previousResident = previous.residentBytes {
            residentIntegral += (Double(resident) / 2 + Double(previousResident) / 2) * seconds
            residentObservedSeconds += seconds
        } else {
            memorySeries = sample.elapsedSeconds
        }
        peakEntriesPerSecond = max(peakEntriesPerSecond, entries)
        peakBytesPerSecond = max(peakBytesPerSecond, bytes)
        if points.count == Self.capacity { points.removeFirst() }
        points.append(ScanProfilePoint(seconds: sample.elapsedSeconds, entriesPerSecond: entries,
                                       bytesPerSecond: bytes, measuredBytes: sample.measuredBytes,
                                       cpuPercent: cpu, residentBytes: sample.residentBytes,
                                       cpuSeries: cpuSeries, memorySeries: memorySeries))
    }

    private func mean(_ count: UInt64) -> Double {
        guard let seconds = latest?.elapsedSeconds, seconds > 0 else { return 0 }
        return Double(count) / seconds
    }
}
