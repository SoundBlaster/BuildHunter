import SwiftUI
import Charts

struct ScanProfilePanel: View {
    let profile: ScanProfileHistory
    let isScanning: Bool
    @State private var expanded = false
    @State private var metric = Metric.entries

    private enum Metric: String, CaseIterable {
        case entries = "Entries/s"
        case bytes = "Measured bytes/s"
    }

    var body: some View {
        DisclosureGroup(isExpanded: $expanded) {
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
                .frame(height: 140)
                .accessibilityIdentifier("buildhunter.report.profile.chart")
                if let sample = profile.latest {
                    Text("\(sample.entries) entries · \(sample.directories) folders · \(sample.artifacts) artifacts · \(sample.warnings) warnings · \(sample.pendingTasks) pending tasks")
                        .font(.caption).foregroundStyle(.secondary)
                } else {
                    Text("Waiting for scan samples…").font(.caption).foregroundStyle(.secondary)
                }
                Text("Measured bytes are artifact sizes from metadata, not disk read speed. Chart shows recent samples; averages and peaks cover the entire scan.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(.top, 8)
        } label: {
            Label("Scan profile", systemImage: "chart.xyaxis.line")
        }
        .accessibilityIdentifier("buildhunter.report.profile")
    }

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
                .accessibilityIdentifier("buildhunter.report.profile.\(title.lowercased())")
            Text(formatted(value)).font(.headline).monospacedDigit()
        }
    }
}
