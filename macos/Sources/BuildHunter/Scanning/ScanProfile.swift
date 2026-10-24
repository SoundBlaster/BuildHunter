import Foundation

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
