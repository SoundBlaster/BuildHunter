import SwiftUI
import Charts
import NestedA11yIDs

/// Status bar entry: a CPU-history style bar chart of recent entries/s that opens
/// the full profile. Bars persist after the scan; Rescan resets the history.
struct ScanProfileStatusItem: View {
    let profile: ScanProfileHistory
    let isScanning: Bool
    @Environment(\.accessibilityPrefix) private var prefix
    @State private var presented = false

    var body: some View {
        Button {
            presented.toggle()
        } label: {
            ScanProfileSparkline(points: profile.points)
        }
        .buttonStyle(.borderless)
        .help("Scan profile · \(summary)")
        .accessibilityLabel("Scan profile")
        .accessibilityValue(summary)
        .nestedAccessibilityIdentifier("profile")
        .popover(isPresented: $presented, arrowEdge: .top) {
            ScanProfilePanel(profile: profile, isScanning: isScanning)
                // The status bar's single-line secondary caption style must not leak
                // into the popover.
                .lineLimit(nil)
                .font(.body)
                .foregroundStyle(.primary)
                .padding(14)
                .frame(width: 520)
                // Children keep the "<report>.profile.*" identifiers without a second
                // element claiming the button's own identifier.
                .environment(\.accessibilityPrefix, prefix.isEmpty ? "profile" : "\(prefix).profile")
        }
    }

    /// Live rate while scanning; the whole-scan average once it has finished.
    private var summary: String {
        let rate = isScanning ? profile.points.last?.entriesPerSecond ?? 0 : profile.averageEntriesPerSecond
        let text = rate.formatted(.number.precision(.fractionLength(0))) + " entries/s"
        return isScanning ? text : "average " + text
    }
}

/// One bar per profile interval, newest at the trailing edge. New samples push
/// older bars to the left; the scale follows the tallest visible bar.
struct ScanProfileSparkline: View {
    static let barCount = 48
    private static let barWidth: CGFloat = 2
    private static let spacing: CGFloat = 1
    private static let height: CGFloat = 16

    let points: [ScanProfilePoint]
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        let recent = Array(points.suffix(Self.barCount))
        let peak = max(recent.map(\.entriesPerSecond).max() ?? 0, 1)
        HStack(alignment: .bottom, spacing: Self.spacing) {
            ForEach(recent) { point in
                Rectangle()
                    .fill(Color.accentColor)
                    .frame(width: Self.barWidth,
                           height: max(1, Self.height * point.entriesPerSecond / peak))
            }
        }
        .frame(width: CGFloat(Self.barCount) * (Self.barWidth + Self.spacing) - Self.spacing,
               height: Self.height, alignment: .bottomTrailing)
        .clipped()
        .padding(3)
        .contentShape(Rectangle())
        // Points keep absolute timestamp identities, so bars slide instead of morphing.
        .animation(reduceMotion ? nil : .linear(duration: 0.1), value: points.last?.id)
        .accessibilityHidden(true)
    }
}

struct ScanProfilePanel: View {
    let profile: ScanProfileHistory
    let isScanning: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var metric = Metric.entries

    private enum Metric: String, CaseIterable {
        case entries = "Entries/s"
        case bytes = "Bytes/s"
        case cpu = "CPU"
        case memory = "RAM"
        var isResource: Bool { self == .cpu || self == .memory }
    }

    var body: some View {
        // Computed once per render; the chart reads them for every point.
        let rateTop = max(1, profile.points.compactMap(rate).max() ?? 0)
        // Found volume is cumulative, so the newest point is the largest.
        let foundTop = max(1, Double(profile.points.last?.measuredBytes ?? 0))
        let foundScale = rateTop / foundTop
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Picker("Metric", selection: $metric) {
                    ForEach(Metric.allCases, id: \.self) { value in
                        Text(value.rawValue).tag(value)
                    }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 400)
                .nestedAccessibilityIdentifier("metric")
                Spacer()
                Text("\(profile.latest?.elapsedSeconds ?? 0, format: .number.precision(.fractionLength(1))) s")
                    .monospacedDigit().foregroundStyle(.secondary)
                    .contentTransition(.numericText())
                    .animation(sampleAnimation, value: profile.latest?.elapsedMicroseconds)
            }
            HStack(spacing: 12) {
                statistic("Current", value: metric.isResource || isScanning ? current : 0)
                statistic("Average", value: average)
                statistic("Peak", value: peak)
            }
            HStack(spacing: 16) {
                legend(metric.rawValue, color: Self.rateColor)
                if !metric.isResource { legend("Found volume", color: Self.foundColor) }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            Chart(profile.points) { point in
                if !metric.isResource {
                    // Total found volume shares throughput plots only.
                    AreaMark(x: .value("Elapsed seconds", point.seconds),
                             y: .value("Found volume", Double(point.measuredBytes) * foundScale))
                        .foregroundStyle(LinearGradient(colors: [Self.foundColor.opacity(0.35),
                                                                 Self.foundColor.opacity(0.02)],
                                                        startPoint: .top, endPoint: .bottom))
                    LineMark(x: .value("Elapsed seconds", point.seconds),
                             y: .value("Found volume", Double(point.measuredBytes) * foundScale),
                             series: .value("Series", "Found volume"))
                        .foregroundStyle(Self.foundColor)
                        .accessibilityLabel("Found volume at \(point.seconds, format: .number.precision(.fractionLength(1))) seconds")
                        .accessibilityValue(bytesLabel(Double(point.measuredBytes)))
                }
                if let value = rate(point) {
                    LineMark(x: .value("Elapsed seconds", point.seconds),
                             y: .value(metric.rawValue, value),
                             series: .value("Series", series(point)))
                        .foregroundStyle(Self.rateColor)
                        .accessibilityLabel("\(point.seconds, format: .number.precision(.fractionLength(1))) seconds")
                        .accessibilityValue(formatted(value))
                    if metric.isResource {
                        PointMark(x: .value("Elapsed seconds", point.seconds),
                                  y: .value(metric.rawValue, value))
                            .symbolSize(8).foregroundStyle(Self.rateColor)
                            .accessibilityHidden(true)
                    }
                }
            }
            .chartYScale(domain: 0...rateTop)
            .chartXAxisLabel("Elapsed seconds")
            .chartYAxis {
                AxisMarks(position: .leading) { value in
                    AxisGridLine()
                    AxisValueLabel {
                        if let number = value.as(Double.self) { Text(formatted(number)) }
                    }
                }
                if !metric.isResource {
                    AxisMarks(position: .trailing, values: [0, rateTop / 2, rateTop]) { value in
                        AxisValueLabel {
                            if let number = value.as(Double.self) {
                                Text(bytesLabel(number / foundScale))
                            }
                        }
                    }
                }
            }
            // Points keep absolute timestamp identities, including when the rolling
            // history evicts its oldest interval. Animate new samples, not slot indices.
            .animation(sampleAnimation, value: profile.latest?.elapsedMicroseconds)
            .animation(reduceMotion ? nil : .easeInOut(duration: 0.25), value: metric)
            .transaction { transaction in
                // A fresh scan resets the timeline; never morph the previous scan into it.
                if profile.points.count < 2 {
                    transaction.animation = nil
                    transaction.disablesAnimations = true
                }
            }
            .frame(height: 140)
            .overlay {
                if metric.isResource && profile.points.compactMap(rate).isEmpty {
                    Text("No resource readings available").font(.caption).foregroundStyle(.secondary)
                }
            }
            // Units change between metrics; never interpolate bytes into CPU percentages.
            .id(metric)
            .nestedAccessibilityIdentifier("chart")
            if let sample = profile.latest {
                Text("\(sample.entries) entries · \(sample.directories) folders · \(sample.artifacts) artifacts · \(sample.warnings) warnings · \(sample.pendingTasks) pending tasks")
                    .font(.caption).foregroundStyle(.secondary)
            } else {
                Text("Waiting for scan samples…").font(.caption).foregroundStyle(.secondary)
            }
            if metric == .cpu, let sample = profile.latest,
               let user = sample.cpuUserNanoseconds, let system = sample.cpuSystemNanoseconds {
                Text("CPU time: \(Double(user) / 1e9, format: .number.precision(.fractionLength(2))) s user · \(Double(system) / 1e9, format: .number.precision(.fractionLength(2))) s system")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Text(metric.isResource
                 ? "Entire BuildHunter process, including UI and other scans. CPU 100% = one logical core. RAM is resident memory (RSS); sampled peaks can miss brief spikes. Averages and peaks cover the entire scan."
                 : "Measured bytes are artifact sizes from metadata, not disk read speed. Chart shows recent samples; averages and peaks cover the entire scan.")
                .font(.caption).foregroundStyle(.secondary)
        }
    }

    private var sampleAnimation: Animation? { reduceMotion ? nil : .linear(duration: 0.1) }

    private static let rateColor = Color.blue
    private static let foundColor = Color.green

    private func bytesLabel(_ bytes: Double) -> String {
        // Clamp before converting: Double(UInt64.max) does not fit back into an integer.
        let clamped = min(max(0, bytes), Double(Int64.max / 2))
        return ByteCountFormatter.string(fromByteCount: Int64(clamped), countStyle: .binary)
    }

    private func legend(_ title: String, color: Color) -> some View {
        Label {
            Text(title)
        } icon: {
            Circle().fill(color).frame(width: 7, height: 7)
        }
    }

    private var current: Double? {
        if metric == .memory { return profile.latest?.residentBytes.map { Double($0) } }
        return profile.points.last.flatMap(rate)
    }
    private var average: Double? {
        switch metric {
        case .entries: profile.averageEntriesPerSecond
        case .bytes: profile.averageBytesPerSecond
        case .cpu: profile.averageCPUPercent
        case .memory: profile.averageResidentBytes
        }
    }
    private var peak: Double? {
        switch metric {
        case .entries: profile.peakEntriesPerSecond
        case .bytes: profile.peakBytesPerSecond
        case .cpu: profile.peakCPUPercent
        case .memory: profile.peakResidentBytes.map { Double($0) }
        }
    }

    private func rate(_ point: ScanProfilePoint) -> Double? {
        switch metric {
        case .entries: point.entriesPerSecond
        case .bytes: point.bytesPerSecond
        case .cpu: point.cpuPercent
        case .memory: point.residentBytes.map { Double($0) }
        }
    }

    private func series(_ point: ScanProfilePoint) -> String {
        switch metric {
        case .cpu: "CPU-\(point.cpuSeries)"
        case .memory: "RAM-\(point.memorySeries)"
        default: "Rate"
        }
    }

    private func formatted(_ value: Double) -> String {
        switch metric {
        case .bytes: return bytesLabel(value) + "/s"
        case .memory: return bytesLabel(value)
        case .cpu: return value.formatted(.number.precision(.fractionLength(0))) + "%"
        case .entries: return value.formatted(.number.precision(.fractionLength(0))) + " entries/s"
        }
    }

    private func statistic(_ title: String, value: Double?) -> some View {
        let display = statisticDisplay(value)
        return VStack(alignment: .leading, spacing: 2) {
            HStack(alignment: .firstTextBaseline, spacing: 4) {
                Text(title)
                    .nestedAccessibilityIdentifier(title.lowercased())
                Text(display.unit)
                    .fontWeight(.regular)
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .lineLimit(1)
            Text(display.amount)
                .font(.headline)
                .monospacedDigit()
                .lineLimit(1)
                .minimumScaleFactor(0.65)
                // Keep the complete measurement available to assistive technology.
                .accessibilityValue(value.map(formatted) ?? "—")
                .nestedAccessibilityIdentifier("\(title.lowercased()).value")
                .contentTransition(.numericText(value: value ?? 0))
                .animation(sampleAnimation, value: value)
        }
        // Each statistic owns an equal column, independent of live digit counts.
        .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading)
    }

    private func statisticDisplay(_ value: Double?) -> (amount: String, unit: String) {
        switch metric {
        case .entries, .cpu:
            return (value?.formatted(.number.precision(.fractionLength(0))) ?? "—",
                    metric == .entries ? "entries/s" : "%")
        case .bytes, .memory:
            guard let value else { return ("—", metric == .bytes ? "bytes/s" : "bytes") }
            let bytes = Int64(min(max(0, value), Double(Int64.max / 2)))
            let formatter = ByteCountFormatter()
            formatter.countStyle = .binary
            formatter.allowsNonnumericFormatting = false
            formatter.includesUnit = false
            let amount = formatter.string(fromByteCount: bytes)
            formatter.includesUnit = true
            formatter.includesCount = false
            let unit = formatter.string(fromByteCount: bytes)
            return (amount, unit + (metric == .bytes ? "/s" : ""))
        }
    }
}
