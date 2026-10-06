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
        case bytes = "Measured bytes/s"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Picker("Throughput", selection: $metric) {
                    ForEach(Metric.allCases, id: \.self) { value in
                        Text(value.rawValue).tag(value)
                    }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 320)
                Spacer()
                Text("\(profile.latest?.elapsedSeconds ?? 0, format: .number.precision(.fractionLength(1))) s")
                    .monospacedDigit().foregroundStyle(.secondary)
                    .contentTransition(.numericText())
                    .animation(sampleAnimation, value: profile.latest?.elapsedMicroseconds)
            }
            HStack(spacing: 24) {
                statistic("Current", value: isScanning ? current : 0)
                statistic("Average", value: average)
                statistic("Peak", value: peak)
            }
            Chart(profile.points) { point in
                AreaMark(x: .value("Elapsed seconds", point.seconds), y: .value(metric.rawValue, rate(point)))
                    .foregroundStyle(.blue.opacity(0.15))
                LineMark(x: .value("Elapsed seconds", point.seconds), y: .value(metric.rawValue, rate(point)))
                    .foregroundStyle(.blue)
                    .accessibilityLabel("\(point.seconds, format: .number.precision(.fractionLength(1))) seconds")
                    .accessibilityValue(formatted(rate(point)))
            }
            .chartYScale(domain: 0...max(1, profile.points.map(rate).max() ?? 0))
            .chartXAxisLabel("Elapsed seconds")
            .chartYAxis {
                AxisMarks(position: .leading) { value in
                    AxisGridLine()
                    AxisValueLabel {
                        if let number = value.as(Double.self) { Text(formatted(number)) }
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
            .nestedAccessibilityIdentifier("chart")
            if let sample = profile.latest {
                Text("\(sample.entries) entries · \(sample.directories) folders · \(sample.artifacts) artifacts · \(sample.warnings) warnings · \(sample.pendingTasks) pending tasks")
                    .font(.caption).foregroundStyle(.secondary)
            } else {
                Text("Waiting for scan samples…").font(.caption).foregroundStyle(.secondary)
            }
            Text("Measured bytes are artifact sizes from metadata, not disk read speed. Chart shows recent samples; averages and peaks cover the entire scan.")
                .font(.caption).foregroundStyle(.secondary)
        }
    }

    private var sampleAnimation: Animation? { reduceMotion ? nil : .linear(duration: 0.1) }

    private var current: Double { profile.points.last.map(rate) ?? 0 }
    private var average: Double { metric == .entries ? profile.averageEntriesPerSecond : profile.averageBytesPerSecond }
    private var peak: Double { metric == .entries ? profile.peakEntriesPerSecond : profile.peakBytesPerSecond }

    private func rate(_ point: ScanProfilePoint) -> Double {
        metric == .entries ? point.entriesPerSecond : point.bytesPerSecond
    }

    private func formatted(_ value: Double) -> String {
        if metric == .bytes {
            return ByteCountFormatter.string(fromByteCount: Int64(clamping: UInt64(min(value, Double(UInt64.max / 2)))), countStyle: .binary) + "/s"
        }
        return value.formatted(.number.precision(.fractionLength(0))) + " entries/s"
    }

    private func statistic(_ title: String, value: Double) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title).font(.caption).foregroundStyle(.secondary)
                .nestedAccessibilityIdentifier(title.lowercased())
            Text(formatted(value)).font(.headline).monospacedDigit()
                .contentTransition(.numericText(value: value))
                .animation(sampleAnimation, value: value)
        }
    }
}
