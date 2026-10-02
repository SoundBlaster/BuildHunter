import SwiftUI
import Charts
import NestedA11yIDs

struct ArtifactDiagramWindow: View {
    let scan: WindowScanModel
    @State private var diagram = ArtifactDiagramModel()

    var body: some View {
        VStack(spacing: 0) {
            DiagramHeader(targetName: scan.targetName, phase: scan.phase,
                          statistics: diagram.snapshot.root.statistics)
            Divider()
            HSplitView {
                ArtifactSunburstChart(layout: diagram.layout, bytes: diagram.focus.bytes,
                                      isScanning: scan.isScanning, statistics: diagram.focus.statistics) { sector in
                    diagram.navigate(to: sector.nodeID ?? sector.parentID)
                }
                .padding(24)
                .frame(minWidth: 420, maxWidth: .infinity, maxHeight: .infinity)

                ArtifactDiagramSidebar(model: diagram)
                    .frame(minWidth: 240, idealWidth: 280, maxWidth: 380, maxHeight: .infinity)
            }
            Divider()
            Text("Area shows known artifact sizes. Partial sizes are lower bounds; unmeasured artifacts have no sector yet.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(12)
        }
        .frame(minWidth: 760, minHeight: 540)
        .navigationTitle("\(scan.targetName ?? "BuildHunter") — Artifact Diagram")
        .task { await diagram.follow(scan) }
        .a11yRoot("buildhunter.diagram")
    }
}

private struct DiagramHeader: View {
    let targetName: String?
    let phase: ScanPhase
    let statistics: ArtifactSizeStatistics

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(targetName ?? "Choose a folder in the scan window")
                    .font(.title2.weight(.semibold))
                    .nestedAccessibilityIdentifier("target")
                Text(phase.diagramDescription)
                    .foregroundStyle(.secondary)
                    .nestedAccessibilityIdentifier("status")
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 4) {
                Text("\(statistics.artifactCount) artifacts")
                    .monospacedDigit()
                    .nestedAccessibilityIdentifier("count")
                Text("\(statistics.measuringCount) measuring · \(statistics.partialCount) partial")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(18)
    }
}

private struct ArtifactSunburstChart: View {
    let layout: ArtifactSunburstLayout
    let bytes: Double
    let isScanning: Bool
    let statistics: ArtifactSizeStatistics
    let select: (ArtifactSunburstLayout.Sector) -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var hovered: ArtifactSunburstLayout.Sector.ID?

    var body: some View {
        VStack(spacing: 12) {
            if layout.sectors.isEmpty {
                ContentUnavailableView {
                    Text(isScanning ? "Waiting for sizes" : "No measured size to display")
                } description: {
                    Text("The diagram updates as the scan measures artifacts.")
                }
            } else {
                Chart(layout.sectors) { sector in
                    SectorMark(
                        angle: .value("Share of known size", sector.angularRange),
                        innerRadius: .ratio(sector.innerRadius / sector.outerRadius),
                        outerRadius: .ratio(sector.outerRadius),
                        angularInset: 1
                    )
                    .foregroundStyle(diagramColor(sector.colorKey))
                    .opacity(hovered == sector.id ? 1 : 0.95 - Double(sector.depth) * 0.15)
                    .accessibilityLabel(sector.nodeID ?? "\(sector.parentID)/\(sector.name)")
                    .accessibilityValue("\(diagramBytes(sector.bytes))\(sector.isPartial ? ", partial" : "")")
                }
                .chartLegend(.hidden)
                .chartOverlay { proxy in
                    GeometryReader { geometry in
                        Rectangle().fill(.clear).contentShape(Rectangle())
                            .onContinuousHover { phase in
                                switch phase {
                                case .active(let location):
                                    hovered = hit(location, proxy: proxy, geometry: geometry)?.id
                                case .ended:
                                    hovered = nil
                                }
                            }
                            .onTapGesture { location in
                                if let sector = hit(location, proxy: proxy, geometry: geometry) {
                                    select(sector)
                                }
                            }
                    }
                }
                .chartBackground { proxy in
                    GeometryReader { geometry in
                        if let anchor = proxy.plotFrame {
                            let frame = geometry[anchor]
                            VStack(spacing: 3) {
                                Text("Known size").font(.caption2).foregroundStyle(.secondary)
                                Text(diagramBytes(bytes))
                                    .font(.headline)
                                    .minimumScaleFactor(0.6)
                                    .lineLimit(1)
                                    .contentTransition(reduceMotion ? .identity : .numericText())
                                    .nestedAccessibilityIdentifier("total")
                            }
                            .frame(width: min(frame.width, frame.height) * 0.21)
                            .position(x: frame.midX, y: frame.midY)
                        }
                    }
                }
                .aspectRatio(1, contentMode: .fit)
                .animation(reduceMotion ? nil : .smooth(duration: 0.25), value: layout.sectors)
                .nestedAccessibilityIdentifier("chart")
            }
            Text(hoverDescription)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(2)
                .frame(height: 34)
            Text("\(statistics.unavailableCount) unavailable · \(statistics.zeroCount) zero bytes")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private var hoverDescription: String {
        guard let sector = layout.sectors.first(where: { $0.id == hovered }) else {
            return "Select a sector or a folder to explore its contents."
        }
        return "\(sector.nodeID ?? sector.name) · \(diagramBytes(sector.bytes))\(sector.isPartial ? " · partial" : "")"
    }

    private func hit(_ point: CGPoint, proxy: ChartProxy, geometry: GeometryProxy) -> ArtifactSunburstLayout.Sector? {
        guard let anchor = proxy.plotFrame else { return nil }
        let frame = geometry[anchor]
        let maximumRadius = min(frame.width, frame.height) / 2
        guard maximumRadius > 0 else { return nil }
        let dx = point.x - frame.midX
        let dy = point.y - frame.midY
        var angle = atan2(dx, -dy) / (2 * .pi)
        if angle < 0 { angle += 1 }
        return layout.sector(angle: angle, radius: hypot(dx, dy) / maximumRadius)
    }
}

private struct ArtifactDiagramSidebar: View {
    let model: ArtifactDiagramModel
    @State private var query = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Button("All artifacts") { model.navigate(to: "") }
                    .disabled(model.focusID.isEmpty)
                    .nestedAccessibilityIdentifier("showAll")
                Button("Up") { model.navigate(to: model.focus.parentID ?? "") }
                    .disabled(model.focusID.isEmpty)
                    .nestedAccessibilityIdentifier("up")
            }
            Text(model.focusID.isEmpty ? "All artifacts" : model.focusID)
                .font(.headline)
                .textSelection(.enabled)
                .nestedAccessibilityIdentifier("focus")
            Text("\(diagramBytes(model.focus.bytes)) known · \(model.focus.statistics.artifactCount) artifacts")
                .font(.caption)
                .foregroundStyle(.secondary)
            if let artifact = model.focus.artifact {
                Text("\(artifact.language) · \(artifact.kind.rawValue)")
                Text(sizeDescription(artifact.size)).foregroundStyle(.secondary)
            }
            TextField("Filter folders", text: $query)
                .textFieldStyle(.roundedBorder)
                .nestedAccessibilityIdentifier("filter")
            List(filteredChildren) { node in
                Button {
                    model.navigate(to: node.id)
                    query = ""
                } label: {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(node.name).lineLimit(1).truncationMode(.middle)
                        Text("\(diagramBytes(node.bytes)) · \(node.statistics.artifactCount) artifacts")
                            .font(.caption).foregroundStyle(.secondary)
                        if node.statistics.measuringCount > 0 || node.statistics.partialCount > 0 {
                            Text("\(node.statistics.measuringCount) measuring · \(node.statistics.partialCount) partial")
                                .font(.caption2).foregroundStyle(.secondary)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("\(node.id), \(diagramBytes(node.bytes)) known")
            }
            .listStyle(.plain)
        }
        .padding(16)
        .onChange(of: model.focusID) { query = "" }
    }

    private var filteredChildren: [ArtifactSunburstNode] {
        query.isEmpty ? model.children : model.children.filter { $0.name.localizedStandardContains(query) }
    }

    private func sizeDescription(_ state: SizeState) -> String {
        switch state {
        case .measuring: "Measuring…"
        case .measured(let bytes): diagramBytes(Double(bytes))
        case .partial(let bytes): bytes.map { "\(diagramBytes(Double($0))) partial" } ?? "Partial · size unknown"
        }
    }
}

private func diagramBytes(_ bytes: Double) -> String {
    guard bytes >= 0, bytes < Double(Int64.max) else {
        return bytes.formatted(.number.precision(.fractionLength(0))) + " bytes"
    }
    return ByteCountFormatter.string(fromByteCount: Int64(bytes), countStyle: .binary)
}

private func diagramColor(_ key: String) -> Color {
    guard !key.isEmpty else { return .gray }
    let palette: [Color] = [.indigo, .teal, .orange, .purple, .blue, .pink, .green]
    let hash = key.utf8.reduce(UInt64(14_695_981_039_346_656_037)) { ($0 ^ UInt64($1)) &* 1_099_511_628_211 }
    return palette[Int(hash % UInt64(palette.count))]
}

private extension ScanPhase {
    var diagramDescription: String {
        switch self {
        case .idle: "No folder selected"
        case .scanning: "Scanning · live updates"
        case .completed: "Scan complete"
        case .stopped: "Scan stopped · partial results"
        case .incomplete: "Scan incomplete · review warnings in the scan window"
        }
    }
}

#if DEBUG
#Preview("Completed artifact diagram") {
    let scan = WindowScanModel()
    scan.showMockState(.results)
    return ArtifactDiagramWindow(scan: scan).frame(width: 1_000, height: 700)
}

#Preview("Partial artifact diagram") {
    let scan = WindowScanModel()
    scan.showMockState(.stopped)
    return ArtifactDiagramWindow(scan: scan).frame(width: 1_000, height: 700)
}
#endif
