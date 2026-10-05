import AppKit
import SwiftUI
import Charts
import OSLog
import NestedA11yIDs

struct ArtifactDiagramWindow: View {
    let scan: WindowScanModel
    @State private var diagram: ArtifactDiagramModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var navigation: DiagramNavigationPresentation?
    @State private var fade = 0.0
    @State private var expansion = 0.0
    @State private var reveal = 0.0

    init(scan: WindowScanModel, diagram: ArtifactDiagramModel = ArtifactDiagramModel()) {
        self.scan = scan
        _diagram = State(initialValue: diagram)
    }

    var body: some View {
        VStack(spacing: 0) {
            DiagramHeader(targetName: scan.targetName, phase: scan.phase,
                          statistics: diagram.snapshot.root.statistics)
            Divider()
            GeometryReader { geometry in
                let preferredPaneWidth = max(0, (geometry.size.width - 1) / 2)
                HSplitView {
                    ArtifactSunburstChart(layout: diagram.layout, palette: diagram.palette, bytes: diagram.focus.bytes,
                                          isScanning: scan.isScanning, statistics: diagram.focus.statistics,
                                          focusID: diagram.focusID, reportID: scan.reportID,
                                          canNavigateUp: diagram.canNavigateUp,
                                          navigation: navigation, fade: fade, expansion: expansion, reveal: reveal,
                                          goUp: { navigate(to: diagram.focus.parentID ?? "") },
                                          hover: { diagram.preview($0.map { $0.nodeID ?? $0.parentID }) }) { sector in
                        navigate(to: sector.nodeID ?? sector.parentID)
                    }
                    .padding(24)
                    .frame(minWidth: 320, idealWidth: preferredPaneWidth, maxWidth: .infinity, maxHeight: .infinity)

                    ArtifactDiagramSidebar(model: diagram, navigate: { navigate(to: $0) })
                        .disabled(navigation != nil)
                        .frame(minWidth: 240, idealWidth: preferredPaneWidth, maxWidth: .infinity, maxHeight: .infinity)
                }
            }
            WindowStatusBar {
                Label("Known artifact sizes. Tiny folders are enlarged for visibility; partial sizes are lower bounds.",
                      systemImage: "chart.pie.fill")
            }
        }
        .frame(minWidth: 760, minHeight: 540)
        .navigationTitle("\(scan.targetName ?? "BuildHunter") — Artifact Diagram")
        .task { await diagram.follow(scan) }
        .task(id: navigation?.id) {
            if let token = navigation?.id { fadeNeighbors(token: token) }
        }
        .onChange(of: scan.reportID) { cancelNavigation() }
        .onChange(of: reduceMotion) { if reduceMotion { cancelNavigation() } }
        .onDisappear { cancelNavigation() }
        .a11yRoot("buildhunter.diagram")
    }

    private func navigate(to path: String) {
        guard navigation == nil, path != diagram.focusID, diagram.snapshot.nodes[path] != nil else { return }
        let descending = !path.isEmpty && (diagram.focusID.isEmpty || path.hasPrefix(diagram.focusID + "/"))
        let ascending = path.isEmpty || diagram.focusID.hasPrefix(path + "/")
        guard descending || ascending, !reduceMotion else {
            diagram.navigate(to: path)
            return
        }
        let source = diagram.layout
        let palette = diagram.palette
        let selectedID = ascending ? diagram.focusID : path
        let direction: ArtifactSunburstNavigation.Direction = ascending ? .ascend : .descend
        // The normal Chart receives the final layout without interpolating navigation.
        // A frozen overlay controls the selected branch's geometry in explicit phases.
        var transaction = Transaction(animation: nil)
        transaction.disablesAnimations = true
        withTransaction(transaction) {
            diagram.navigate(to: path)
            if let plan = ArtifactSunburstNavigation(source: source, destination: diagram.layout,
                                                     selectedID: selectedID, direction: direction) {
                navigation = DiagramNavigationPresentation(plan: plan, sourcePalette: palette, destinationPalette: diagram.palette)
                diagram.setNavigationTransitionActive(true)
                fade = 0; expansion = 0; reveal = 0
            }
        }
    }

    private func fadeNeighbors(token: UUID) {
        guard navigation?.id == token else { return }
        let ascending = navigation?.plan.direction == .ascend
        recordNavigationPhase(ascending ? "restore-branch" : "fade")
        withAnimation(.easeOut(duration: ascending ? 0.18 : 0.14), completionCriteria: .removed) {
            fade = 1
        } completion: {
            expandBranch(token: token)
        }
    }

    private func expandBranch(token: UUID) {
        guard navigation?.id == token else { return }
        recordNavigationPhase(navigation?.plan.direction == .ascend ? "contract" : "expand")
        withAnimation(.smooth(duration: 0.46), completionCriteria: .removed) {
            expansion = 1
        } completion: {
            revealChildren(token: token)
        }
    }

    private func revealChildren(token: UUID) {
        guard navigation?.id == token else { return }
        let ascending = navigation?.plan.direction == .ascend
        recordNavigationPhase(ascending ? "reveal-neighbors" : "reveal")
        withAnimation(.easeInOut(duration: ascending ? 0.14 : 0.18), completionCriteria: .removed) {
            reveal = 1
        } completion: {
            if navigation?.id == token { cancelNavigation() }
        }
    }

    private func cancelNavigation() {
        if navigation != nil { recordNavigationPhase("finished-or-cancelled") }
        var transaction = Transaction(animation: nil)
        transaction.disablesAnimations = true
        withTransaction(transaction) {
            navigation = nil
            diagram.setNavigationTransitionActive(false)
            fade = 0; expansion = 0; reveal = 0
        }
    }

    private func recordNavigationPhase(_ phase: String) {
#if DEBUG
        Logger(subsystem: "BuildHunter", category: "ChartNavigation").notice("ChartNavigation phase=\(phase, privacy: .public)")
#endif
    }
}

private struct DiagramHeader: View {
    let targetName: String?
    let phase: ScanPhase
    let statistics: ArtifactSizeStatistics

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                HStack(alignment: .firstTextBaseline, spacing: 7) {
                    Image(systemName: "folder.fill").accessibilityHidden(true)
                    Text(targetName ?? "Choose a folder in the scan window")
                        .nestedAccessibilityIdentifier("target")
                }
                .font(.title2.weight(.semibold))
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
    var focusID = ""
    var reportID: UUID?
    var canNavigateUp = false
    var navigation: DiagramNavigationPresentation?
    var fade = 0.0
    var expansion = 0.0
    var reveal = 0.0
    var goUp: () -> Void = {}
    var hover: (ArtifactSunburstLayout.Sector?) -> Void = { _ in }
    let select: (ArtifactSunburstLayout.Sector) -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var hovered: ArtifactSunburstLayout.Sector.ID?
    @State private var pointer = CGPoint.zero

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
                        .cornerRadius(0) // Diagnostic comparison: keep animation and gap, disable rounding.
                        .foregroundStyle(diagramColor(palette.color(for: sector)))
                        .opacity(navigation == nil ? max(0.7, 1 - Double(sector.depth) * 0.14) : 0)
                        .accessibilityLabel(sector.nodeID ?? "\(sector.parentID)/\(sector.name)")
                        .accessibilityValue("\(diagramBytes(sector.bytes))\(sector.isPartial ? ", partial" : "")")
                    }
                    .chartLegend(.hidden)
                    .chartOverlay { proxy in
                        GeometryReader { geometry in
                            if let anchor = proxy.plotFrame {
                                let frame = geometry[anchor]
                                let centerDiameter = min(frame.width, frame.height) * 0.21
                                ZStack(alignment: .topLeading) {
                                    if let navigation {
                                        DiagramNavigationLayer(presentation: navigation, fade: fade,
                                                               expansion: expansion, reveal: reveal)
                                            .frame(width: frame.width, height: frame.height)
                                            .position(x: frame.midX, y: frame.midY)
                                            .allowsHitTesting(false)
                                            .accessibilityHidden(true)
                                    }
                                    Rectangle().fill(.clear).contentShape(Rectangle())
                                        .onTapGesture { location in
                                            guard navigation == nil else { return }
                                            if let sector = hit(location, proxy: proxy, geometry: geometry) {
                                                setHover(nil)
                                                select(sector)
                                            }
                                        }
                                    Button(action: goUp) {
                                        VStack(spacing: 3) {
                                            HStack(spacing: 3) {
                                                Image(systemName: canNavigateUp ? "arrow.up.circle" : "square.stack.3d.up.fill")
                                                    .accessibilityHidden(true)
                                                Text(canNavigateUp ? "Up · Known size" : "Known size")
                                            }
                                            .font(.system(size: min(11, centerDiameter * 0.12)))
                                            .lineLimit(1)
                                            .minimumScaleFactor(0.5)
                                            .foregroundStyle(.secondary)
                                            Text(diagramBytes(bytes))
                                                .foregroundStyle(Color.primary)
                                                .font(.system(size: min(17, centerDiameter * 0.19), weight: .semibold))
                                                .minimumScaleFactor(0.5)
                                                .lineLimit(1)
                                                .contentTransition(reduceMotion ? .identity : .numericText())
                                                .nestedAccessibilityIdentifier("total")
                                        }
                                        .frame(width: centerDiameter, height: centerDiameter)
                                        .contentShape(Circle())
                                    }
                                    .buttonStyle(DiagramCenterButtonStyle())
                                    .disabled(!canNavigateUp || navigation != nil)
                                    .accessibilityLabel(canNavigateUp ? "Go to parent folder" : "All artifacts")
                                    .help(canNavigateUp ? "Go to parent folder" : "All artifacts")
                                    .nestedAccessibilityIdentifier("centerUp")
                                    .position(x: frame.midX, y: frame.midY)

                                    if let sector = layout.sectors.first(where: { $0.id == hovered }) {
                                        DiagramTooltip(name: sector.name, pointer: pointer, bounds: geometry.size)
                                            .allowsHitTesting(false)
                                            .accessibilityHidden(true)
                                    }
                                }
                                // Track the entire overlay, including the center button.
                                // Attaching hover after that button's .position expands
                                // its tracking region across the plot and masks the rings.
                                .onContinuousHover { phase in
                                    switch phase {
                                    case .active(let location):
                                        guard navigation == nil else { return }
                                        pointer = location
                                        setHover(hit(location, proxy: proxy, geometry: geometry))
                                    case .ended:
                                        setHover(nil)
                                    }
                                }
                            }
                        }
                    }
                    .animation(reduceMotion || navigation != nil ? nil : .smooth(duration: 0.25), value: layout.sectors)
                    // Preserve Chart identity so streaming insertions and navigation animate.
                    // Debug evidence records our inputs separately from Charts' interpolation.
                    .onAppear { recordChartInputs(size: chartGeometry.size) }
                    .onChange(of: layout.sectors) { recordChartInputs(size: chartGeometry.size) }
                    .onChange(of: chartGeometry.size) { recordChartInputs(size: chartGeometry.size) }
                    .nestedAccessibilityIdentifier("chart")
                }
                .aspectRatio(1, contentMode: .fit)
            }
            Text(hoverDescription)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(2)
                .frame(height: 34)
            Text(sizeNotes)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(minHeight: 16)
        }
        .onChange(of: focusID) { setHover(nil) }
        .onChange(of: reportID) { setHover(nil) }
        .onChange(of: layout.sectors) {
            if !layout.sectors.contains(where: { $0.id == hovered }) { setHover(nil) }
        }
        .onDisappear { setHover(nil) }
    }

    private func recordChartInputs(size: CGSize) {
#if DEBUG
        let logger = Logger(subsystem: "BuildHunter", category: "ChartInput")
        let radius = min(size.width, size.height) / 2
        var invalid = !size.width.isFinite || !size.height.isFinite || size.width < 0 || size.height < 0
        let inputs = layout.sectors.enumerated().map { index, sector in
            let decoration = sector.decoration(plotRadius: radius)
            let values = [sector.start, sector.end, sector.innerRadiusRelativeToOuter,
                          sector.outerRadius, decoration.angularInset, 0]
            if !values.allSatisfy(\.isFinite) || sector.end <= sector.start
                || sector.innerRadiusRelativeToOuter < 0 || sector.innerRadiusRelativeToOuter >= 1
                || sector.outerRadius <= 0 || sector.outerRadius > 1
                || decoration.angularInset < 0 {
                invalid = true
            }
            return "\(index):d=\(sector.depth),a=\(sector.start)..\(sector.end),inner=\(sector.innerRadiusRelativeToOuter),outer=\(sector.outerRadius),gap=\(decoration.angularInset),corner=0"
        }.joined(separator: ";")
        logger.notice("ChartInput size=\(size.width)x\(size.height) sectors=\(layout.sectors.count) invalid=\(invalid) reduceMotion=\(reduceMotion)")
        logger.debug("ChartInput sectors: \(inputs, privacy: .public)")
        if invalid {
            logger.error("Invalid ChartInput sectors: \(inputs, privacy: .public)")
        }
#endif
    }

    private var sizeNotes: String {
        var notes: [String] = []
        if statistics.unavailableCount > 0 {
            let count = statistics.unavailableCount
            notes.append("\(count) \(count == 1 ? "artifact" : "artifacts") with unknown size")
        }
        if statistics.zeroCount > 0 {
            let count = statistics.zeroCount
            notes.append("\(count) \(count == 1 ? "artifact" : "artifacts") with size 0 B")
        }
        return notes.joined(separator: " · ")
    }

    private func setHover(_ sector: ArtifactSunburstLayout.Sector?) {
        guard hovered != sector?.id else { return }
        hovered = sector?.id
        hover(sector)
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

private struct DiagramNavigationPresentation: Identifiable {
    let id = UUID()
    let plan: ArtifactSunburstNavigation
    let sourcePalette: ArtifactSunburstPalette
    let destinationPalette: ArtifactSunburstPalette

    func color(for frame: ArtifactSunburstNavigation.Frame, reveal: Double) -> Color {
        guard case .node(let path) = frame.id else { return diagramColor(.other) }
        let source = sourcePalette.color(for: path) ?? .other
        return diagramColor(reveal == 0 ? source : destinationPalette.color(for: path) ?? source)
    }
}

/// Only navigation uses these paths. Streaming values continue to use Swift Charts.
/// Keeping both endpoint layouts fixed prevents producer events from retargeting a zoom.
private struct DiagramNavigationLayer: View {
    let presentation: DiagramNavigationPresentation
    let fade: Double
    let expansion: Double
    let reveal: Double

    var body: some View {
        GeometryReader { geometry in
            let radius = min(geometry.size.width, geometry.size.height) / 2
            ZStack {
                ForEach(presentation.plan.frames(fade: fade, expansion: expansion, reveal: reveal)) { frame in
                    DiagramNavigationSector(start: frame.start, end: frame.end,
                                            inner: frame.innerRadius, outer: frame.outerRadius,
                                            inset: min(2, (frame.outerRadius - frame.innerRadius) * radius / 4))
                        .fill(presentation.color(for: frame, reveal: presentation.plan.direction == .ascend ? fade : reveal))
                        .opacity(frame.opacity * max(0.7, 1 - frame.depth * 0.14))
                }
            }
        }
    }
}

private struct DiagramNavigationSector: Shape {
    var start: Double
    var end: Double
    var inner: Double
    var outer: Double
    var inset: Double

    var animatableData: AnimatablePair<AnimatablePair<Double, Double>, AnimatablePair<AnimatablePair<Double, Double>, Double>> {
        get { .init(.init(start, end), .init(.init(inner, outer), inset)) }
        set {
            start = newValue.first.first; end = newValue.first.second
            inner = newValue.second.first.first; outer = newValue.second.first.second
            inset = newValue.second.second
        }
    }

    func path(in rect: CGRect) -> Path {
        guard [start, end, inner, outer, inset].allSatisfy(\.isFinite), end > start,
              outer > inner, inner >= 0, outer <= 1 else { return Path() }
        let radius = min(rect.width, rect.height) / 2
        guard radius > 0 else { return Path() }
        let center = CGPoint(x: rect.midX, y: rect.midY)
        let gap = end - start >= 1 - 1e-12 ? 0 : min((end - start) / 8, inset / (max(1, inner * radius) * 2 * .pi))
        let lower = (start + gap) * 2 * .pi - .pi / 2
        let upper = (end - gap) * 2 * .pi - .pi / 2
        var path = Path()
        path.move(to: CGPoint(x: center.x + cos(lower) * inner * radius,
                              y: center.y + sin(lower) * inner * radius))
        path.addLine(to: CGPoint(x: center.x + cos(lower) * outer * radius,
                                y: center.y + sin(lower) * outer * radius))
        path.addArc(center: center, radius: outer * radius,
                    startAngle: .radians(lower), endAngle: .radians(upper), clockwise: false)
        path.addLine(to: CGPoint(x: center.x + cos(upper) * inner * radius,
                                y: center.y + sin(upper) * inner * radius))
        path.addArc(center: center, radius: inner * radius,
                    startAngle: .radians(upper), endAngle: .radians(lower), clockwise: true)
        path.closeSubpath()
        return path
    }
}

private struct DiagramCenterButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        // Disabling Up at the root must not dim the report's central total.
        configuration.label.opacity(configuration.isPressed ? 0.7 : 1)
    }
}

private struct DiagramTooltip: View {
    let name: String
    let pointer: CGPoint
    let bounds: CGSize
    @State private var size = CGSize(width: 160, height: 36)

    var body: some View {
        Text(name)
            .font(.callout.weight(.medium))
            .lineLimit(2)
            .minimumScaleFactor(0.75)
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .frame(maxWidth: max(1, min(240, bounds.width - 16)))
            // Ask for the capped ideal size instead of stretching short names
            // to the full maximum width. Long names wrap within the plot.
            .fixedSize(horizontal: true, vertical: true)
            .background(.regularMaterial, in: .rect(cornerRadius: 8))
            .shadow(color: .black.opacity(0.15), radius: 5, y: 2)
            .onGeometryChange(for: CGSize.self) { $0.size } action: { size = $0 }
            .position(x: min(max(size.width / 2, pointer.x + 14 + size.width / 2), bounds.width - size.width / 2),
                      y: pointer.y + size.height + 20 < bounds.height
                        ? pointer.y + 16 + size.height / 2 : max(size.height / 2, pointer.y - 12 - size.height / 2))
    }
}

private struct ArtifactDiagramSidebar: View {
    @Bindable var model: ArtifactDiagramModel
    let navigate: (String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Button("All artifacts", systemImage: "square.stack.3d.up.fill") { navigate("") }
                    .disabled(!model.canNavigateUp)
                    .nestedAccessibilityIdentifier("showAll")
                Button("Up", systemImage: "arrow.up.circle") { navigate(model.focus.parentID ?? "") }
                    .disabled(!model.canNavigateUp)
                    .nestedAccessibilityIdentifier("up")
            }
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Image(systemName: "folder.fill").accessibilityHidden(true)
                    Text(model.displayedPath)
                        .font(.headline)
                        .textSelection(.enabled)
                        .fixedSize(horizontal: false, vertical: true)
                        .nestedAccessibilityIdentifier("focus")
                }
                Button("Copy path", systemImage: "document.on.document") {
                    guard let path = model.displayedURL?.path else { return }
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(path, forType: .string)
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
                .disabled(model.displayedURL == nil)
                .help("Copy full path")
                .nestedAccessibilityIdentifier("copyPath")
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text("\(diagramBytes(model.displayedFolder.bytes)) known · \(model.displayedFolder.statistics.artifactCount) artifacts")
                .font(.caption)
                .foregroundStyle(.secondary)
            if let artifact = model.displayedFolder.artifact {
                Text("\(artifact.language) · \(artifact.kind.rawValue)")
                Text(sizeDescription(artifact.size)).foregroundStyle(.secondary)
            }
            TextField("Filter folders", text: $model.query)
                .textFieldStyle(.roundedBorder)
                .nestedAccessibilityIdentifier("filter")
            ScrollView {
                LazyVStack(spacing: 4) {
                    ForEach(model.filteredChildren) { node in
                        ArtifactDiagramFolderRow(node: node, swatch: model.palette.color(for: node.id)) {
                            navigate(node.id)
                        }
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .nestedAccessibilityIdentifier("folders")
        }
        .padding(16)
    }

    private func sizeDescription(_ state: SizeState) -> String {
        switch state {
        case .measuring: "Measuring…"
        case .measured(let bytes): diagramBytes(Double(bytes))
        case .partial(let bytes): bytes.map { "\(diagramBytes(Double($0))) partial" } ?? "Partial · size unknown"
        }
    }
}

private struct ArtifactDiagramFolderRow: View {
    let node: ArtifactSunburstNode
    let swatch: ArtifactSunburstPalette.Swatch?
    let select: () -> Void
    @State private var isHovered = false

    var body: some View {
        Button(action: select) {
            HStack(alignment: .top, spacing: 8) {
                Image(systemName: "folder.fill")
                    .foregroundStyle(diagramColor(swatch ?? .other))
                    .padding(.top, 1)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(node.name).lineLimit(1).truncationMode(.middle)
                        ArtifactLanguageBadges(languages: node.languages)
                    }
                    Text("\(diagramBytes(node.bytes)) · \(node.statistics.artifactCount) artifacts")
                        .font(.caption).foregroundStyle(.secondary)
                    if node.statistics.measuringCount > 0 || node.statistics.partialCount > 0 {
                        Text("\(node.statistics.measuringCount) measuring · \(node.statistics.partialCount) partial")
                            .font(.caption2).foregroundStyle(.secondary)
                    }
                }
                Spacer(minLength: 0)
            }
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.accentColor.opacity(isHovered ? 0.16 : 0), in: .rect(cornerRadius: 6))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { isHovered = $0 }
        .accessibilityLabel("\(node.id), \(diagramBytes(node.bytes)) known")
        .accessibilityValue(ArtifactLanguage.allCases.filter { node.languages.contains($0) }
            .map(\.rawValue).joined(separator: ", "))
        .nestedAccessibilityIdentifier("folder.\(node.id)")
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
    let diagram = ArtifactDiagramModel(snapshot: ArtifactSunburstSnapshot(rows: scan.rows))
    return ArtifactDiagramWindow(scan: scan, diagram: diagram).frame(width: 1_000, height: 700)
}

#Preview("Partial artifact diagram") {
    let scan = WindowScanModel()
    scan.showMockState(.stopped)
    let diagram = ArtifactDiagramModel(snapshot: ArtifactSunburstSnapshot(rows: scan.rows))
    return ArtifactDiagramWindow(scan: scan, diagram: diagram).frame(width: 1_000, height: 700)
}

#Preview("Compact artifact diagram") {
    let scan = WindowScanModel()
    scan.showMockState(.results)
    let diagram = ArtifactDiagramModel(snapshot: ArtifactSunburstSnapshot(rows: scan.rows))
    return ArtifactDiagramWindow(scan: scan, diagram: diagram).frame(width: 760, height: 540)
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
    let model = ArtifactDiagramModel(snapshot: snapshot)
    let overview = model.layout
    let overviewPalette = model.palette
    model.navigate(to: "Apps")
    let focused = model.layout
    let palette = model.palette
    return HStack(spacing: 24) {
        VStack {
            Text("All artifacts").font(.headline)
            ArtifactSunburstChart(layout: overview, palette: overviewPalette, bytes: snapshot.root.bytes,
                                  isScanning: false, statistics: snapshot.root.statistics) { _ in }
        }
        VStack {
            Text("Inside Apps — Beta keeps the parent color").font(.headline)
            ArtifactSunburstChart(layout: focused, palette: palette, bytes: snapshot.nodes["Apps"]!.bytes,
                                  isScanning: false, statistics: snapshot.nodes["Apps"]!.statistics) { _ in }
        }
    }
    .padding(24)
    .frame(width: 1_060, height: 580)
}
#Preview("Folder navigation and full path") {
    let scan = WindowScanModel(source: DiagramPreviewSource())
    scan.accept(target: URL(fileURLWithPath: "/Users/egor/Development/GitHub", isDirectory: true))
    for (artifact, bytes) in MockScanState.results.completedArtifacts {
        scan.apply(.discovered(generation: scan.generation, artifact: artifact))
        scan.apply(.completed(generation: scan.generation, artifactID: artifact.id, bytes: bytes))
    }
    scan.apply(.finished(generation: scan.generation, result: .completed))
    let diagram = ArtifactDiagramModel(snapshot: ArtifactSunburstSnapshot(rows: scan.rows),
                                       targetURL: scan.targetURL, targetName: scan.targetName)
    return ArtifactDiagramWindow(scan: scan, diagram: diagram).frame(width: 1_060, height: 740)
}

#Preview("Tiny folder minimum angles") {
    let rows = [
        ("Large/Compiler/target", Int64(1_000_000_000)),
        ("Tiny/One/.build", Int64(1)),
        ("Tiny/Two/target", Int64(2)),
        ("Small/Cache/__pycache__", Int64(1_024))
    ].map { path, bytes in
        ScanRow(id: UUID(), relativePath: path, language: "Swift", kind: .buildOutput, size: .measured(bytes))
    }
    let snapshot = ArtifactSunburstSnapshot(rows: rows)
    let model = ArtifactDiagramModel(snapshot: snapshot)
    return ArtifactSunburstChart(layout: model.layout, palette: model.palette, bytes: snapshot.root.bytes,
                                 isScanning: false, statistics: snapshot.root.statistics) { _ in }
        .padding(24)
        .frame(width: 600, height: 600)
}

private struct DiagramPreviewSource: ScanEventSource {
    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> { AsyncStream { _ in } }
    func cancel(generation: UInt64) {}
}
#endif

#Preview("Compact center and tooltips") {
    HStack(spacing: 12) {
        ForEach([false, true], id: \.self) { navigatesUp in
            let snapshot = ArtifactSunburstSnapshot(rows: [
                ScanRow(id: UUID(), relativePath: "Package/target", language: "Rust", kind: .buildOutput,
                        size: .measured(421_108_121))
            ])
            let model = ArtifactDiagramModel(snapshot: snapshot)
            ArtifactSunburstChart(layout: model.layout, palette: model.palette, bytes: snapshot.root.bytes,
                                 isScanning: false, statistics: snapshot.root.statistics,
                                 canNavigateUp: navigatesUp) { _ in }
                .frame(width: 180, height: 280)
        }
    }
    .overlay(alignment: .topLeading) {
        ZStack(alignment: .topLeading) {
            DiagramTooltip(name: "Development", pointer: CGPoint(x: 25, y: 20), bounds: CGSize(width: 372, height: 280))
            DiagramTooltip(name: "A very long folder name that must wrap inside a compact window",
                           pointer: CGPoint(x: 370, y: 220), bounds: CGSize(width: 372, height: 280))
        }
    }
    .padding(16)
    .frame(width: 404, height: 312)
}

#Preview("Descendant language badges") {
    let snapshot = ArtifactSunburstSnapshot(rows: [
        ScanRow(id: UUID(), relativePath: "Projects/Deep/Swift/.build", language: "Swift",
                kind: .buildOutput, size: .measured(100_000)),
        ScanRow(id: UUID(), relativePath: "Projects/Deep/Rust/target", language: "Rust",
                kind: .buildOutput, size: .measured(200_000)),
        ScanRow(id: UUID(), relativePath: "Projects/Python/__pycache__", language: "Python",
                kind: .cache, size: .measuring)
    ])
    VStack {
        ForEach(["Projects", "Projects/Deep", "Projects/Python"], id: \.self) { path in
            if let node = snapshot.nodes[path] {
                ArtifactDiagramFolderRow(node: node, swatch: nil) {}
            }
        }
    }
    .padding()
    .frame(width: 380)
}
