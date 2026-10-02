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
                ArtifactSunburstChart(layout: diagram.layout, palette: diagram.palette, bytes: diagram.focus.bytes,
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
    let palette: ArtifactSunburstPalette
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
                        .nestedAccessibilityIdentifier("emptyStatus")
                } description: {
                    Text("The diagram updates as the scan measures artifacts.")
                }
            } else {
                GeometryReader { chartGeometry in
                    let plotRadius = min(chartGeometry.size.width, chartGeometry.size.height) / 2
                    Chart(layout.sectors) { sector in
                        let decoration = sector.decoration(plotRadius: plotRadius)
                        SectorMark(
                            angle: .value("Share of known size", sector.angularRange),
                            innerRadius: .ratio(sector.innerRadiusRelativeToOuter),
                            outerRadius: .ratio(sector.outerRadius),
                            angularInset: CGFloat(decoration.angularInset)
                        )
                        .cornerRadius(CGFloat(decoration.cornerRadius))
                        .foregroundStyle(diagramColor(palette.color(for: sector)))
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
                    .animation(reduceMotion ? nil : .smooth(duration: 0.25), value: layout.sectors)
                    .nestedAccessibilityIdentifier("chart")
                }
                .aspectRatio(1, contentMode: .fit)
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

private func diagramColor(_ swatch: ArtifactSunburstPalette.Swatch) -> Color {
    Color(hue: swatch.hue, saturation: swatch.saturation, brightness: swatch.brightness)
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

#Preview("Compact artifact diagram") {
    let scan = WindowScanModel()
    scan.showMockState(.results)
    return ArtifactDiagramWindow(scan: scan).frame(width: 760, height: 540)
}

#Preview("Folder color continuity") {
    let rows = [
        ("Apps/Alpha/.build", Int64(240_000_000)),
        ("Apps/Beta/target", Int64(400_000_000)),
        ("Apps/Gamma/.venv", Int64(160_000_000)),
        ("Tools/Linter/__pycache__", Int64(180_000_000)),
        ("Server/target", Int64(320_000_000))
    ].map { path, bytes in
        ScanRow(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput, size: .measured(bytes))
    }
    let snapshot = ArtifactSunburstSnapshot(rows: rows)
    let overview = ArtifactSunburstLayout(snapshot: snapshot)
    let focused = ArtifactSunburstLayout(snapshot: snapshot, focusID: "Apps")
    var palette = ArtifactSunburstPalette()
    palette.include(overview)
    palette.include(focused)
    return HStack(spacing: 24) {
        VStack {
            Text("All artifacts").font(.headline)
            ArtifactSunburstChart(layout: overview, palette: palette, bytes: snapshot.root.bytes,
                                  isScanning: false, statistics: snapshot.root.statistics) { _ in }
        }
        VStack {
            Text("Inside Apps — same folder colors").font(.headline)
            ArtifactSunburstChart(layout: focused, palette: palette, bytes: snapshot.nodes["Apps"]!.bytes,
                                  isScanning: false, statistics: snapshot.nodes["Apps"]!.statistics) { _ in }
        }
    }
    .padding(24)
    .frame(width: 1_060, height: 580)
}
#endif
