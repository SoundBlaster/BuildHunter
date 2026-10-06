import Foundation

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

    var elapsedSeconds: Double { Double(elapsedMicroseconds) / 1_000_000 }
}

struct ScanProfilePoint: Identifiable, Equatable, Sendable {
    let seconds: Double
    let entriesPerSecond: Double
    let bytesPerSecond: Double
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

    var averageEntriesPerSecond: Double { mean(latest?.entries ?? 0) }
    var averageBytesPerSecond: Double { mean(latest?.measuredBytes ?? 0) }

    mutating func record(_ sample: ScanProfileSnapshot) {
        let previous = latest
        if let previous {
            guard sample.elapsedMicroseconds > previous.elapsedMicroseconds,
                  sample.entries >= previous.entries,
                  sample.measuredBytes >= previous.measuredBytes else { return }
        }
        latest = sample
        guard let previous else { return }
        let seconds = Double(sample.elapsedMicroseconds - previous.elapsedMicroseconds) / 1_000_000
        let entries = Double(sample.entries - previous.entries) / seconds
        let bytes = Double(sample.measuredBytes - previous.measuredBytes) / seconds
        peakEntriesPerSecond = max(peakEntriesPerSecond, entries)
        peakBytesPerSecond = max(peakBytesPerSecond, bytes)
        if points.count == Self.capacity { points.removeFirst() }
        points.append(ScanProfilePoint(seconds: sample.elapsedSeconds, entriesPerSecond: entries, bytesPerSecond: bytes))
    }

    private func mean(_ count: UInt64) -> Double {
        guard let seconds = latest?.elapsedSeconds, seconds > 0 else { return 0 }
        return Double(count) / seconds
    }
}
